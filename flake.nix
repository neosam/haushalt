{
  description = "Household Manager - Full-stack Rust application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        packages = {
          backend = pkgs.callPackage ./default.nix {
            pkgs = pkgs;
          };

          frontend = let
            cargoVendorDir = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
            # Build wasm-bindgen-cli with matching version
            wasm-bindgen-cli = pkgs.rustPlatform.buildRustPackage rec {
              pname = "wasm-bindgen-cli";
              version = "0.2.108";
              src = pkgs.fetchCrate {
                inherit pname version;
                hash = "sha256-UsuxILm1G6PkmVw0I/JF12CRltAfCJQFOaT4hFwvR8E=";
              };
              cargoHash = "sha256-iqQiWbsKlLBiJFeqIYiXo3cqxGLSjNM8SOWXGM9u43E=";
              nativeBuildInputs = [ pkgs.pkg-config ];
              buildInputs = [ pkgs.openssl ] ++
                pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.curl ];
              doCheck = false;
            };
          in pkgs.stdenv.mkDerivation {
            pname = "haushalt-frontend";
            version = "1.0.0-dev";

            src = pkgs.lib.cleanSource ./.;

            nativeBuildInputs = [
              rustToolchain
              pkgs.trunk
              wasm-bindgen-cli
              pkgs.binaryen
              pkgs.lld
            ];

            buildPhase = ''
              runHook preBuild

              export HOME=$TMPDIR
              export CARGO_HOME=$TMPDIR/.cargo
              mkdir -p $CARGO_HOME

              # Setup vendored dependencies
              mkdir -p .cargo
              cat > .cargo/config.toml << EOF
              [source.crates-io]
              replace-with = "vendored-sources"

              [source.vendored-sources]
              directory = "${cargoVendorDir}"
              EOF

              cd frontend

              # Build with trunk (handles WASM, assets, and post_build hooks)
              trunk build --release --offline

              runHook postBuild
            '';

            installPhase = ''
              runHook preInstall
              mkdir -p $out
              cp -r dist/* $out/
              runHook postInstall
            '';
          };

          default = pkgs.callPackage ./default.nix {
            pkgs = pkgs;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain with WASM target
            rustToolchain
            cargo-watch

            # SQLite and sqlx
            sqlite
            sqlx-cli

            # Frontend build tools
            trunk
            wasm-bindgen-cli
            binaryen
            lld

            # Development tools
            pkg-config
            openssl
          ];

          shellHook = ''
            echo "Household Manager Development Environment"
            echo ""
            echo "Commands:"
            echo "  cargo run -p backend     - Run the backend server"
            echo "  cd frontend && trunk serve - Run the frontend dev server"
            echo "  cargo test --workspace   - Run all tests"
            echo ""
          '';

          # For sqlx offline mode
          SQLX_OFFLINE = "true";
        };
      }
    );
}
