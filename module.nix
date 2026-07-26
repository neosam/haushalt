# NixOS module for the Haushalt service
{ config, lib, pkgs, ... }:

let
  cfg = config.services.haushalt;
in
{
  options.services.haushalt = lib.mkOption {
    type = lib.types.attrsOf (lib.types.submodule {
      options = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Enable this Haushalt instance";
        };

        package = lib.mkOption {
          type = lib.types.package;
          description = "Haushalt backend package to use.";
          default = (builtins.getFlake "path:${toString ./.}").packages.${pkgs.system}.backend;
          defaultText = lib.literalExpression "haushalt.packages.\${system}.backend";
        };

        frontendPackage = lib.mkOption {
          type = lib.types.package;
          description = "Haushalt frontend package to use (served as static files by nginx).";
          default = (builtins.getFlake "path:${toString ./.}").packages.${pkgs.system}.frontend;
          defaultText = lib.literalExpression "haushalt.packages.\${system}.frontend";
        };

        port = lib.mkOption {
          type = lib.types.port;
          default = 3000;
          description = "Port the backend listens on.";
        };

        host = lib.mkOption {
          type = lib.types.str;
          default = "127.0.0.1";
          description = "Address the backend binds to.";
        };

        logLevel = lib.mkOption {
          type = lib.types.str;
          default = "backend=info,actix_web=info";
          description = "Value for RUST_LOG.";
        };

        domain = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Domain for the nginx reverse proxy. If set, enables nginx for this instance.";
        };

        enableSSL = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Enable SSL/TLS via Let's Encrypt (only used when domain is set).";
        };

        jwtSecretFile = lib.mkOption {
          type = lib.types.nullOr lib.types.path;
          default = null;
          example = "/run/secrets/haushalt-jwt";
          description = ''
            Path to a systemd EnvironmentFile containing the JWT secret, i.e. a
            file with a single line `JWT_SECRET=<secret>`.

            This is the recommended way to configure the secret: the file is read
            at service start and never ends up in the world-readable Nix store.
            Takes precedence over `jwtSecret`.
          '';
        };

        jwtSecret = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = ''
            JWT secret as a plain string. WARNING: this lands in the Nix store and
            is readable by every local user. Only for local/test instances — use
            `jwtSecretFile` for anything reachable from the network.
          '';
        };

        accessTokenExpirationMinutes = lib.mkOption {
          type = lib.types.int;
          default = 15;
          description = "Lifetime of an access token in minutes.";
        };

        refreshTokenExpirationDays = lib.mkOption {
          type = lib.types.int;
          default = 30;
          description = "Lifetime of a refresh token in days.";
        };

        corsOrigins = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [];
          example = [ "https://haushalt.example.com" ];
          description = ''
            Allowed CORS origins. Leave empty when the frontend is served from the
            same domain as the API (the default nginx setup does exactly that).
          '';
        };

        legalDir = lib.mkOption {
          type = lib.types.nullOr lib.types.path;
          default = null;
          description = "Directory containing the legal pages served under /api/legal.";
        };

        extraEnvironment = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = {};
          description = "Additional environment variables for the service.";
        };
      };
    });
    default = {};
    description = "Haushalt service instances";
  };

  config = lib.mkMerge [
    # Fail early on a misconfigured instance instead of starting with a default secret.
    {
      assertions = lib.flatten (lib.mapAttrsToList (name: instanceCfg: [
        {
          assertion = !instanceCfg.enable
            || instanceCfg.jwtSecretFile != null
            || instanceCfg.jwtSecret != null;
          message = ''
            services.haushalt.${name}: either jwtSecretFile (recommended) or
            jwtSecret must be set — the backend refuses to start without JWT_SECRET.
          '';
        }
      ]) cfg);
    }

    # Systemd services
    {
      systemd.services = lib.mapAttrs' (name: instanceCfg:
        let
          stateDir = "/var/lib/haushalt-${name}";
        in
        lib.nameValuePair "haushalt-${name}" (lib.mkIf instanceCfg.enable {
          description = "Haushalt Service (${name})";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];

          # The backend runs its embedded migrations itself (sqlx::migrate! in
          # main.rs), so there is no preStart migration step. `mode=rwc` lets
          # SQLite create the database file on first start.
          environment = {
            HOST = instanceCfg.host;
            PORT = toString instanceCfg.port;
            RUST_LOG = instanceCfg.logLevel;
            DATABASE_URL = "sqlite:${stateDir}/haushalt.db?mode=rwc";
            ACCESS_TOKEN_EXPIRATION_MINUTES = toString instanceCfg.accessTokenExpirationMinutes;
            REFRESH_TOKEN_EXPIRATION_DAYS = toString instanceCfg.refreshTokenExpirationDays;
          }
          // lib.optionalAttrs (instanceCfg.corsOrigins != []) {
            CORS_ORIGINS = lib.concatStringsSep "," instanceCfg.corsOrigins;
          }
          // lib.optionalAttrs (instanceCfg.legalDir != null) {
            LEGAL_DIR = toString instanceCfg.legalDir;
          }
          // lib.optionalAttrs (instanceCfg.jwtSecretFile == null && instanceCfg.jwtSecret != null) {
            JWT_SECRET = instanceCfg.jwtSecret;
          }
          // instanceCfg.extraEnvironment;

          serviceConfig = {
            Type = "simple";
            ExecStart = "${instanceCfg.package}/bin/backend";
            StateDirectory = "haushalt-${name}";
            WorkingDirectory = stateDir;
            Restart = "on-failure";
            RestartSec = 5;

            DynamicUser = true;
            NoNewPrivileges = true;
            PrivateTmp = true;
            ProtectSystem = "strict";
            ProtectHome = true;
          } // lib.optionalAttrs (instanceCfg.jwtSecretFile != null) {
            EnvironmentFile = instanceCfg.jwtSecretFile;
          };
        })
      ) cfg;
    }

    # Nginx reverse proxy for instances with a domain.
    # The frontend hardcodes API_BASE = "/api" and does not fetch any runtime
    # config, so /api/ must be proxied through unrewritten and everything else
    # falls back to the SPA's index.html.
    (lib.mkIf (lib.any (instanceCfg: instanceCfg.enable && instanceCfg.domain != null) (lib.attrValues cfg)) {
      services.nginx = {
        enable = lib.mkDefault true;
        recommendedGzipSettings = lib.mkDefault true;
        recommendedOptimisation = lib.mkDefault true;
        recommendedProxySettings = lib.mkDefault true;
        recommendedTlsSettings = lib.mkDefault true;

        virtualHosts = lib.mapAttrs' (name: instanceCfg:
          lib.nameValuePair instanceCfg.domain {
            forceSSL = instanceCfg.enableSSL;
            enableACME = instanceCfg.enableSSL;

            locations."/api/" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              # /api/ws is an actix WebSocket route and needs the upgrade headers.
              proxyWebsockets = true;
              priority = 100;
              extraConfig = ''
                proxy_connect_timeout 60s;
                proxy_send_timeout 1200s;
                proxy_read_timeout 1200s;
              '';
            };

            locations."/" = {
              root = instanceCfg.frontendPackage;
              priority = 300;
              tryFiles = "$uri /index.html =404";
            };
          }
        ) (lib.filterAttrs (_: instanceCfg: instanceCfg.enable && instanceCfg.domain != null) cfg);
      };
    })

    # ACME for SSL
    (lib.mkIf (lib.any (instanceCfg: instanceCfg.enable && instanceCfg.domain != null && instanceCfg.enableSSL) (lib.attrValues cfg)) {
      security.acme.acceptTerms = lib.mkDefault true;
    })
  ];
}
