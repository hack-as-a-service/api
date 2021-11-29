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
    in {
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
      };
    });
}
