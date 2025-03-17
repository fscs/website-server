{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      flake-utils,
      crane,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        craneLib = crane.mkLib pkgs;
        inherit (pkgs) lib;

        queryFilter = path: _type: null != builtins.match ".*/query-.*\.json$" path;
        sqlFilter = path: _type: null != builtins.match ".*\.sql$" path;
        sqlOrQueryOrCargo =
          path: type:
          (queryFilter path type) || (sqlFilter path type) || (craneLib.filterCargoSources path type);

        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = sqlOrQueryOrCargo;
        };

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
          ];
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        website-server = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;

            doCheck = false;
            meta.mainProgram = "fscs-website-backend";
          }
        );
      in
      {
        checks = {
          inherit website-server;

          website-server-tests = craneLib.mkCargoDerivation (
            commonArgs
            // {
              inherit cargoArtifacts;

              pnameSuffix = "-test";

              buildPhaseCargoCommand = ''
                TEMP_DIR=$(mktemp -d)
                DATA_DIR=$TEMP_DIR/data
                SOCKET_DIR="$TEMP_DIR/sockets"
                SOCKET_URL="$(echo $SOCKET_DIR | sed 's/\//%2f/g')"
                export DATABASE_URL="postgresql://$SOCKET_URL:5432/postgres"

                mkdir -p "$DATA_DIR" "$SOCKET_DIR"

                ${pkgs.postgresql_16}/bin/initdb -D "$DATA_DIR" --locale=C.utf8

                ${pkgs.postgresql_16}/bin/pg_ctl -D $DATA_DIR -o "-k $SOCKET_DIR -h \"\"" start

                ${pkgs.sqlx-cli}/bin/sqlx migrate run

                cargoWithProfile test --locked

                ${pkgs.postgresql_16}/bin/pg_ctl -D "$DATA_DIR" stop
              '';
            }
          );
        };

        formatter = pkgs.alejandra;

        packages = rec {
          default = website-server;

          database = pkgs.writeScriptBin "run.sh" ''
            #!/usr/bin/env bash
            DATA_DIR="$PWD/db/data"
            SOCKET_DIR="$PWD/db/sockets"
            SOCKET_URL="$(echo $SOCKET_DIR | sed 's/\//%2f/g')"
            export DATABASE_URL="postgresql://$SOCKET_URL:5432/postgres"

            mkdir -p "$DATA_DIR" "$SOCKET_DIR"

            echo Initializing the Database
            ${pkgs.postgresql_16}/bin/initdb -D "$DATA_DIR" --locale=C.utf8

            ${pkgs.postgresql_16}/bin/pg_ctl -D $DATA_DIR -o "-k $SOCKET_DIR" start

            trap "${pkgs.postgresql_16}/bin/pg_ctl -D $DATA_DIR stop; exit" SIGINT

            ${pkgs.sqlx-cli}/bin/sqlx migrate run --source ./migrations  --database-url $DATABASE_URL

            read -p "Press enter to stop the database"

            ${pkgs.postgresql_16}/bin/pg_ctl -D "$DATA_DIR" stop
          '';

          full = pkgs.writeScriptBin "run.sh" ''
            #!/usr/bin/env bash
            POSTGRES_DATA_DIR="$PWD/db/data"
            DATA_DIR="$PWD"
            SOCKET_DIR="$PWD/db/sockets"
            SOCKET_URL="$(echo $SOCKET_DIR | sed 's/\//%2f/g')"
            export DATABASE_URL="postgresql://$SOCKET_URL:5432/postgres"

            mkdir -p "$POSTGRES_DATA_DIR" "$SOCKET_DIR" "$DATA_DIR"


            ${pkgs.postgresql_16}/bin/initdb -D "$POSTGRES_DATA_DIR" --locale=C.utf8

            ${pkgs.postgresql_16}/bin/pg_ctl -D $POSTGRES_DATA_DIR status
            ALREADY_RUNNING=$?

            if [ ! "$ALREADY_RUNNING" -eq 0 ]; then
              echo Initializing the Database
              ${pkgs.postgresql_16}/bin/pg_ctl -D $POSTGRES_DATA_DIR -o "-k $SOCKET_DIR -h \"\"" start
              trap "${pkgs.postgresql_16}/bin/pg_ctl -D $POSTGRES_DATA_DIR stop; exit" SIGINT
            fi

            echo Starting the server
            echo "DATABASE_URL: $DATABASE_URL"
            ${lib.getExe default} \
              --database-url $DATABASE_URL \
              --content-dir test \
              --data-dir $DATA_DIR \
              --max-file-size 10485760 \
              --auth-url https://auth.inphima.de/application/o/authorize/ \
              --token-url https://auth.inphima.de/application/o/token/ \
              --user-info https://auth.inphima.de/application/o/userinfo/ \
              $@


            if [ ! "$ALREADY_RUNNING" -eq 0  ]; then
              echo Stopping the Database
              ${pkgs.postgresql_16}/bin/pg_ctl -D "$POSTGRES_DATA_DIR" stop
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

        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              sqlx-cli
              postgresql
              cargo-watch
            ];
          };

          attic = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.attic-client
            ];
          };
        };
      }
    )
    // {
      nixosModules.fscs-website-server = nixpkgs.lib.modules.importApply ./module.nix {
        inherit (self) outputs;
      };
    };
}
