{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    flake-utils,
    crane,
    nixpkgs,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
        };
        
        craneLib = crane.mkLib pkgs;
        inherit (pkgs) lib;

        queryFilter = path: _type: null != builtins.match ".*/query-.*\.json$" path;
        sqlFilter = path: _type: null != builtins.match ".*\.sql$" path;
        sqlOrQueryOrCargo = path: type: (queryFilter path type) || (sqlFilter path type) || (craneLib.filterCargoSources path type);

        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = sqlOrQueryOrCargo;
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
                # Add additional build inputs here
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                # Additional darwin specific inputs can be set here
                pkgs.libiconv
              ];

            doCheck = false;
          });
      in rec {
        checks = {
          inherit my-crate;
        };

        formatter = pkgs.alejandra;

        # For `nix build` & `nix run`:
        defaultPackage = my-crate;

        packages = {
          docker = pkgs.dockerTools.buildImage {
            name = "fscs-website";
            tag = "latest";

            config = {
              Env = ["SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"];
              Cmd = [
                "${defaultPackage}/bin/fscs-website-backend"
                "--host"
                "0.0.0.0"
                "--port"
                "8080"
                "--content-dir"
                "test/static"
                "--private-content-dir"
                "test/static_auth"
                "--hidden-content-dir"
                "test/hidden_static"
              ];
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

            trap "${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR stop; exit" SIGINT

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
              ALREADY_RUNNING=true
            fi

            if [ "$ALREADY_RUNNING" = false ]; then
              echo Initializing the Database
              ${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR -o "-k $SOCKET_DIR -h \"\"" start
              trap "${pkgs.postgresql}/bin/pg_ctl -D $DATA_DIR stop; exit" SIGINT
            fi

            echo Starting the server
            ${defaultPackage}/bin/fscs-website-backend --database-url $DATABASE_URL --content-dir test/static --private-content-dir test/static_auth --hidden-content-dir test/static_hidden $@

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

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            cargo-binutils
            sqlx-cli
            postgresql
            docker-compose
          ];
        };
      }
    );
}
