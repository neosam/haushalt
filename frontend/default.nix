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

    # Build WASM
    cargo build --target wasm32-unknown-unknown --release --offline

    # Run wasm-bindgen
    mkdir -p dist
    wasm-bindgen \
      --target web \
      --out-dir dist \
      --out-name frontend \
      ../target/wasm32-unknown-unknown/release/frontend.wasm

    # Optimize WASM
    wasm-opt -Oz -o dist/frontend_bg.wasm dist/frontend_bg.wasm || true

    # Copy static assets
    cp index.html dist/
    cp styles.css dist/
    cp manifest.json dist/
    cp sw.js dist/
    cp favicon.svg dist/
    cp -r icons dist/

    # Update index.html to load the WASM module
    sed -i 's|<link data-trunk rel="rust" data-wasm-opt="z" />|<script type="module">import init from "/frontend.js"; init();</script>|' dist/index.html
    sed -i 's|<link data-trunk rel="css" href="styles.css">|<link rel="stylesheet" href="/styles.css">|' dist/index.html
    sed -i 's|<link data-trunk rel="copy-file"[^>]*>||g' dist/index.html
    sed -i 's|<link data-trunk rel="copy-dir"[^>]*>||g' dist/index.html

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
