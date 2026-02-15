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
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain with WASM target
            rustToolchain

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
