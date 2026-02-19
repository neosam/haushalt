{ pkgs ? import <nixpkgs> {}, ... }:
let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgsWithOverlay = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rustToolchain = pkgsWithOverlay.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-unknown-unknown" ];
  };
  src = pkgs.lib.cleanSource ./..;

  cargoVendorDir = pkgs.rustPlatform.importCargoLock {
    lockFile = ../Cargo.lock;
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
in
pkgs.stdenv.mkDerivation {
  pname = "haushalt-frontend";
  version = "1.0.0-dev";

  inherit src;

  nativeBuildInputs = [
    rustToolchain
    wasm-bindgen-cli
    pkgs.trunk
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

  meta = with pkgs.lib; {
    description = "Household Manager Frontend - Built with Leptos";
    license = licenses.mit;
    platforms = platforms.all;
  };
}
