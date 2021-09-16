{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust2nix = {
      url = "github:anirudhb/rust2nix";
      #url = "path:../../rust2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, rust2nix }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      fenixPkgs = fenix.packages.${system};
      rust = fenixPkgs.combine [
        fenixPkgs.latest.rustc
        fenixPkgs.latest.cargo
      ];
      rust2nixLib = rust2nix.lib.${system};
    in rec {
      packages.haas-api = rust2nixLib.mkRustApp {
        pname = "haas_api";
        src = ./.;
        cargo = rust;
        rustc = rust;
        #buildInputs = [
        #  pkgs.darwin.apple_sdk.frameworks.Security
        #];
        #LDFLAGS = "-F ${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks";
        #NIX_DEBUG = "1";
      };
      defaultPackage = packages.haas-api;

      apps.haas-api = flake-utils.lib.mkApp {
        drv = packages.haas-api;
      };
      defaultApp = apps.haas-api;

      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          fenixPkgs.stable.defaultToolchain
          fenixPkgs.rust-analyzer
          jq
          libiconv
        ];
      };
    });
}
