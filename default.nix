{ pkgs ? import <nixpkgs> {}, ... }:
let
  specificPkgs = pkgs;
  src = specificPkgs.lib.cleanSource ./.;
  rustPlatform = specificPkgs.rustPlatform;
in
  rustPlatform.buildRustPackage {
    pname = "haushalt-service";
    version = "1.0.0-dev";
    src = src;
    nativeBuildInputs = with specificPkgs; [curl pkg-config openssl];
    buildInputs = with specificPkgs; [sqlite openssl];
    cargoBuildFlags = [ "-p" "backend" ];
    SQLX_OFFLINE = "true";

    postInstall = ''
  cp -r $src/backend/migrations $out/

  # Create the start script
  echo "#!${specificPkgs.bash}/bin/bash" > $out/bin/start.sh
  echo "set -e" >> $out/bin/start.sh
  echo "${specificPkgs.sqlx-cli}/bin/sqlx db setup --source $out/migrations" >> $out/bin/start.sh
  echo "$out/bin/backend" >> $out/bin/start.sh
  chmod a+x $out/bin/start.sh
  '';

    cargoLock = {
      lockFile = ./Cargo.lock;
    };
  }
