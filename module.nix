# NixOS module for Inventurly service
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
          type = lib.types.nullOr lib.types.package;
          description = "Haushalt package to use.";
          default = (builtins.getFlake "path:${toString ./.}").packages.${pkgs.system};
        };
        
        frontendPackage = lib.mkOption {
          type = lib.types.package;
          description = "Inventurly frontend package to use";
          default = (builtins.getFlake "path:${toString ./.}").packages.${pkgs.system}.frontend;
        };
        
        port = lib.mkOption {
          type = lib.types.port;
          default = 3000;
          description = "Port to listen on";
        };
        
        host = lib.mkOption {
          type = lib.types.str;
          default = "127.0.0.1";
          description = "Host to bind to";
        };
        
        logLevel = lib.mkOption {
          type = lib.types.str;
          default = "haushalt=debug,tower_http=debug";
          description = "Rust log level configuration";
        };
        
        domain = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Domain name for nginx reverse proxy. If set, enables nginx reverse proxy.";
        };
        
        enableSSL = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Enable SSL/TLS with Let's Encrypt (only used when domain is set)";
        };
        
        extraEnvironment = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = {};
          description = "Additional environment variables";
        };

        jwtSecret = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = "se!cret";
          description = "JWT secret - keep this secret.";
        };
        
      };
    });
    default = {};
    description = "Inventurly service instances";
  };
  
  config = lib.mkMerge [
    # Systemd services
    {
      systemd.services = lib.mapAttrs' (name: instanceCfg:
        let
          # Determine package based on OIDC configuration
          actualPackage = instanceCfg.package;

            #else pkgs.callPackage (./default.nix) { 
            #  inherit pkgs; 
            #  features = if instanceCfg.oidc.enable then ["oidc"] else ["mock_auth"]; 
            #};
            
          # Auto-derive APP_URL if not specified
          appUrl = if instanceCfg.oidc.appUrl != null
            then instanceCfg.oidc.appUrl
            else if instanceCfg.domain != null
              then "https://${instanceCfg.domain}"
              else "http://${instanceCfg.host}:${toString instanceCfg.port}";
              
          # Base environment variables
          baseEnv = {
            SERVER_ADDRESS = "${instanceCfg.host}:${toString instanceCfg.port}";
            RUST_LOG = instanceCfg.logLevel;
            BASE_PATH = "${appUrl}/api";
          };
          
        in
        lib.nameValuePair "haushalt-${name}" (lib.mkIf instanceCfg.enable {
          description = "Inventurly Service (${name})";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];
          
          environment =  {
            DATABASE_URL = "sqlite:/var/lib/haushalt-${name}/haushalt.db";
            JWT_SECRET = "secr!t123";
            PORT = toString instanceCfg.port;
          };
          
          serviceConfig = {
            Type = "simple";
            ExecStart = "${actualPackage}/bin/backend";
            StateDirectory = "haushalt-${name}";
            WorkingDirectory = "/var/lib/haushalt-${name}";
            Restart = "on-failure";
          };
          
          preStart = ''
            # Initialize database
            if [ ! -f /var/lib/haushalt-${name}/haushalt.db ]; then
              ${pkgs.sqlite}/bin/sqlite3 /var/lib/haushalt-${name}/haushalt.db "VACUUM;"
            fi
            
            # Copy and run migrations
            if [ ! -d /var/lib/haushalt-${name}/migrations ]; then
              cp -r ${actualPackage}/migrations /var/lib/haushalt-${name}/
            fi
            
            # Run migrations
            cd /var/lib/haushalt-${name}
            ${pkgs.sqlx-cli}/bin/sqlx database setup --source ./migrations/sqlite || true
          '';
        })
      ) cfg;

    }

    {
      # Create etc directories
      environment.etc = lib.mapAttrs' (name: instanceCfg: 
        lib.nameValuePair "haushalt-${name}/config.json" {
          text = lib.mkIf instanceCfg.enable ''
            {
              "backend": "https://${instanceCfg.domain}/api"
            }
          '';
        }) cfg;
    }

    # Nginx configuration for instances with domains
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

            locations."= /authenticate" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."= /logout" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."/swagger-ui/" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."/api-docs/" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."/api/" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
              extraConfig = ''
                rewrite ^/(.*)$ /$1 break;
                proxy_connect_timeout 60s;
                proxy_send_timeout 1200s;
                proxy_read_timeout 1200s;
              '';
            };
            locations."= /config.json" = {
              alias = "/etc/haushalt-${name}/config.json";
              extraConfig = "add_header ContentType application/json;";
              priority = 200;
            };
            locations."= /assets/config.json" = {
              alias = "/etc/haushalt-${name}/config.json";
              extraConfig = "add_header ContentType application/json;";
              priority = 200;
            };
            locations."/" = {
              root = instanceCfg.frontendPackage;
              priority = 300;
              tryFiles = "$uri /index.html =200";
            };
          }
        ) (lib.filterAttrs (_: instanceCfg: instanceCfg.enable && instanceCfg.domain != null) cfg);
      };
    })
    
    # ACME configuration for SSL
    (lib.mkIf (lib.any (instanceCfg: instanceCfg.enable && instanceCfg.domain != null && instanceCfg.enableSSL) (lib.attrValues cfg)) {
      security.acme = {
        acceptTerms = lib.mkDefault true;
        #defaults.email = lib.mkDefault "admin@example.com"; # Users should override this
      };
    })
  ];
}
