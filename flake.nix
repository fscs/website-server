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

    theme = {
      url = "github:fscs/website-theme";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };

    website = {
      type = "git";
      url = "ssh://git@git.hhu.de/fscs/website.git";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        theme.follows = "theme";
      };
    };
  };

  outputs = {
    self,
    website,
    flake-utils,
    crane,
    nixpkgs,
    rust-overlay,
    theme,
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

          buildInputs = [
            # Add additional build inputs here
            pkgs.openssl
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
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
        my-crate = craneLib.buildPackage (commonArgs // {
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
          fullWebsite = pkgs.stdenv.mkDerivation {
            name = "with-files";
            src = builtins.filterSource (path: type: false) ./.;
            buildInputs = [my-crate];

            postInstall = ''
              mkdir -p $out/bin
              ln -s ${website.defaultPackage.${system}} $out/bin/static
              cp ${defaultPackage}/bin/fscs-website-backend $out/bin/fscs-website-backend
              cp -r ${defaultPackage}/bin/migrations $out/bin/migrations
            '';
          };

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

          # For `nix run`:
          run = pkgs.stdenv.mkDerivation {
            name = "fscs-website-run";
            src = builtins.filterSource (path: type: false) ./.;
            postInstall = ''
              mkdir -p $out/bin
              echo "#!/bin/bash" >> $out/bin/run.sh
              echo "echo \"Initializing the Database\"" >> $out/bin/run.sh
              echo "mkdir -p ./db/data" >> $out/bin/run.sh
              echo "mkdir -p ./db/sockets" >> $out/bin/run.sh
              echo "${pkgs.postgresql}/bin/initdb -D ./db/data" >> $out/bin/run.sh
              echo "echo \"Starting the db\"" >> $out/bin/run.sh
              echo "sockets=\$PWD/db/sockets" >> $out/bin/run.sh
              echo "${pkgs.postgresql}/bin/pg_ctl -D ./db/data -o \"-k \$sockets -h \\\"\\\"\" start" >> $out/bin/run.sh
              echo "sockets=\$(echo \$sockets | sed 's/\\//%2f/g')" >> $out/bin/run.sh
              echo "export DATABASE_URL=\"postgresql://\$sockets:5432/postgres\"" >> $out/bin/run.sh
              echo "echo \"Starting the server\"" >> $out/bin/run.sh
              echo "${packages.fullWebsite}/bin/fscs-website-backend --database-url \$DATABASE_URL --use-executable-dir" >> $out/bin/run.sh
              echo "echo \"Stopping the database\"" >> $out/bin/run.sh
              echo "${pkgs.postgresql}/bin/pg_ctl -D ./db/data stop" >> $out/bin/run.sh
              chmod +x $out/bin/run.sh
            '';
          };
        };

        apps.default = flake-utils.lib.mkApp {
          drv = packages.fullWebsite;
          exePath = "/bin/fscs-website-backend";
        };

        apps.full = flake-utils.lib.mkApp {
          drv = packages.run;
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
          ];
        };
      }
    );
}
