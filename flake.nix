{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = {
    self,
    flake-utils,
    crane,
    nixpkgs,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = crane.lib.${system};
        inherit (pkgs) lib;

        sqlFilter = path: _type: null != builtins.match ".*sql$" path;
        sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);

        src = lib.cleanSourceWith {
          src = craneLib.path ./.; # The original, unfiltered source
          filter = sqlOrCargo;
        };

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = [
            pkgs.pkg-config
          ];

          buildInputs =
            [
              # Add additional build inputs here
              pkgs.openssl
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.Security
            ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;

            nativeBuildInputs = with pkgs; [pkg-config];
            buildInputs =
              [
                pkgs.openssl
                pkgs.sqlx-cli
                pkgs.postgresql
                # Add additional build inputs here
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                # Additional darwin specific inputs can be set here
                pkgs.libiconv
              ];

            # Start postgreSQL and run migrations
            preBuild = ''
              ${pkgs.postgresql}/bin/initdb -D ./db
              sockets=$(mktemp -d)
              ${pkgs.postgresql}/bin/pg_ctl -D ./db -o "-k $sockets -h \"\"" start
              # Write the postgresql socket to DATABASE_URL
              sockets=$(echo $sockets | sed 's/\//%2f/g')
              export DATABASE_URL="postgresql://$sockets:5432/postgres"
              echo "DATABASE_URL=$DATABASE_URL"
              ${pkgs.sqlx-cli}/bin/sqlx migrate run --source ./migrations  --database-url $DATABASE_URL
            '';

            postInstall = ''
              ${pkgs.postgresql}/bin/pg_ctl -D ./db stop
              cp -r migrations $out/bin/migrations
            '';

            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          });
      in rec {
        checks = {
          inherit my-crate;
        };
        # For `nix build` & `nix run`:
        defaultPackage = my-crate;

        packages = {
          docker = pkgs.dockerTools.buildImage {
            name = "fscs-website";
            tag = "latest";

            config = {
              Cmd = ["${packages.fullWebsite}/bin/fscs-website-backend" "--host" "0.0.0.0"];
              ExposedPorts = {
                "8080/tcp" = {};
              };
            };
          };

          database = pkgs.writeScriptBin "run.sh" ''
            #!/usr/bin/env bash
            DATA_DIR="$PWD/db/data"
            SOCKET_DIR="$PWD/db/sockets"
            SOCKET_URL="$(echo $SOCKET_DIR | sed 's/\//%2f/g')"
            export DATABASE_URL="postgresql://$SOCKET_URL:5432/postgres"

            mkdir -p "$DATA_DIR" "$SOCKET_DIR"

            echo Initializing the Database
            ${pkgs.postgresql}/bin/initdb -D "$DATA_DIR" --locale=C.utf8

            ${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR -o "-k $SOCKET_DIR" start

            ${pkgs.sqlx-cli}/bin/sqlx migrate run --source ./migrations  --database-url $DATABASE_URL

            read -p "Press enter to stop the database"

            ${pkgs.postgresql}/bin/pg_ctl -D "$DATA_DIR" stop
          '';

          full = pkgs.writeScriptBin "run.sh" ''
            #!/usr/bin/env bash
            DATA_DIR="$PWD/db/data"
            SOCKET_DIR="$PWD/db/sockets"
            SOCKET_URL="$(echo $SOCKET_DIR | sed 's/\//%2f/g')"
            export DATABASE_URL="postgresql://$SOCKET_URL:5432/postgres"

            mkdir -p "$DATA_DIR" "$SOCKET_DIR"


            ${pkgs.postgresql}/bin/initdb -D "$DATA_DIR" --locale=C.utf8

            # Check if the database is already running
            ALREADY_RUNNING=false
            if ${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR status; then
              echo Initializing the Database
              ALREADY_RUNNING=true
            fi

            if [ "$ALREADY_RUNNING" = false ]; then
              ${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR -o "-k $SOCKET_DIR -h \"\"" start
            fi

            echo Starting the server
            ${defaultPackage}/bin/fscs-website-backend --database-url $DATABASE_URL --use-executable-dir

            if [ "$ALREADY_RUNNING" = false ]; then
              echo Stopping the Database
              ${pkgs.postgresql}/bin/pg_ctl -D "$DATA_DIR" stop
            fi
          '';
        };

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.full;
          exePath = "/bin/run.sh";
        };

        apps.database = flake-utils.lib.mkApp {
          drv = self.packages.${system}.database;
          exePath = "/bin/run.sh";
        };

        # For `nix develop`:
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src"];
              targets = ["wasm32-unknown-unknown"];
            })
            rustc
            cargo
            wasm-pack
            cargo-binutils
            sqlx-cli
            postgresql
            docker-compose
          ];
        };
      }
    );
}
