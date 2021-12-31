{
  description = "A very basic flake with a shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      #inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      fenixPkgs = fenix.packages.${system};
      haasApiPackage = { rustPlatform, system, systemd ? false, ... }: with pkgs; let
        isDarwin = lib.hasSuffix "darwin" system;
        isLinux = lib.hasSuffix "linux" system;
      in rustPlatform.buildRustPackage rec {
        pname = "haas-api";
        version = "0.0.0";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = {
            "caddy-0.1.0" = "sha256-kiTfY6bz9+rat6UkP+7u7jbp7AQVULl7jWMok12S5D4=";
          };
        };
        buildInputs = [
          (if systemd then postgresql else postgresql.override { systemd = false; })
        ] ++ lib.optional isDarwin [
          darwin.apple_sdk.frameworks.Security
        ] ++ lib.optional isLinux [
          openssl
        ];
        nativeBuildInputs = with pkgs; lib.optional isLinux [
          pkg-config
        ];
        enableParallelBuilding = true;
        doCheck = false; # FIXME
      };
      caddyfile = pkgs.writeText "Caddyfile" ''
        {
          debug
          auto_https off
          admin 0.0.0.0:2019 {
            origins localhost:2019 127.0.0.1:2019 0.0.0.0:2019 *:2019
          }
        }
        http://test.haas.hackclub.com {
          respond "It works!"
        }
      '';
    in rec {
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (fenixPkgs.latest.withComponents [
            "cargo"
            "rustc"
            "rust-src"
            "rustfmt"
          ])
          fenixPkgs.rust-analyzer
          diesel-cli
          cargo-edit
          libiconv
          caddy
          (postgresql.override {
            systemd = false;
          })
        ] ++ lib.optional (lib.hasSuffix "darwin" system) [
          darwin.apple_sdk.frameworks.Security
        ];
        # save as env var
        CADDYFILE_DEV = builtins.toString caddyfile;
      };

      packages.haas-api = pkgs.callPackage haasApiPackage { inherit system; };
      packages.haas-api-dockerImage = if pkgs.lib.hasSuffix "linux" system then pkgs.dockerTools.buildLayeredImage {
        name = "haas-api";
        contents = pkgs.callPackage haasApiPackage { inherit system; /* FIXME */ systemd = true; };
        extraCommands = ''
          cp ${./Rocket.toml} .
        '';
        config = {
          Cmd = [ "haas_api" ];
        };
      } else null;

      defaultPackage = packages.haas-api;
    });
}
