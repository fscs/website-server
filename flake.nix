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
    theme
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = crane.lib.${system};

        inherit (pkgs) lib;

        my-crate = craneLib.buildPackage {
          src = lib.cleanSourceWith {
            src = ./.; # The original, unfiltered source
            filter = path: type:
              (lib.hasSuffix "\.html" path)
              ||
              # Default filter from crane (allow .rs files)
              (craneLib.filterCargoSources path type);
          };
          strictDeps = true;

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

          postInstall = ''
            cp -r templates $out/bin/templates
          '';

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };
      in rec {
        checks = {
          inherit my-crate;
        };
        # For `nix build` & `nix run`:
        defaultPackage = my-crate;

        packages = {
          fullWebsite = pkgs.stdenv.mkDerivation {
            name = "with-files";
            src = ./.;
            buildInputs = [ my-crate ];

            postInstall = ''
              mkdir -p $out/bin
              ln -s ${website.defaultPackage.${system}} $out/bin/static
              ln -s ${defaultPackage}/bin/templates $out/bin/templates
              cp ${defaultPackage}/bin/backend $out/bin/backend
            '';
          };

          docker = pkgs.dockerTools.buildImage {
            name = "fscs-website";
            tag = "latest";

            config = {
              Cmd = [ "${packages.fullWebsite}/bin/backend" "--host" "0.0.0.0"];
              ExposedPorts = {
                "8080/tcp" = {};
              };
            };
          };
        };

        apps.default = flake-utils.lib.mkApp {
          drv = packages.fullWebsite;
          exePath = "/bin/backend";
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
          ];
        };
      }
    );
}
