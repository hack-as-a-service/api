{
  description = "A very basic flake with a shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, naersk }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      lib = nixpkgs.lib;
      fenixPkgs = fenix.packages.${system};
      # FIXME: once nmattie/naersk#191 is merged
      rust = fenixPkgs.combine [
        fenixPkgs.latest.rustc
        fenixPkgs.latest.cargo
      ];
      naerskLib = naersk.lib."${system}".override {
        cargo = rust;
        rustc = rust;
      };
      haas-api-spec = {
        pname = "haas_api";
        root = ./.;
        buildInputs = with pkgs; [
          postgresql
        ] ++ lib.optionals (lib.hasSuffix system "linux") (with pkgs; [
          pkg-config
          openssl.dev
        ]);
      };
    in rec {
      packages.haas-api = naerskLib.buildPackage haas-api-spec;
      # FIXME: compile on non-macOS
      packages.haas-api-docker = 
        let
          pkgsCross = pkgs.pkgsCross.musl64;
          target = "x86_64-unknown-linux-musl";
          toolchain = with fenixPkgs; combine [
            latest.rustc
            latest.cargo
            targets.${target}.latest.rust-std
          ];
          cc-naerskLib = naersk.lib."${system}".override {
            cargo = toolchain;
            rustc = toolchain;
          };
          cc-haas-api = cc-naerskLib.buildPackage {
            pname = "haas_api";
            root = ./.;
            buildInputs = with pkgsCross; [
              postgresql
              pkg-config
              openssl.dev
            ];
            CARGO_BUILD_TARGET = target;
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgsCross.stdenv.cc}/bin/${target}-gcc";
            #OPENSSL_STATIC = "1";
            #OPENSSL_LIB_DIR = "${pkgsCross.openssl.dev}/lib";
            #OPENSSL_INCLUDE_DIR = "${pkgsCross.openssl.dev}/lib";
          };
        in
          pkgs.dockerTools.buildImage {
            name = "haas-api";
            config = {
              Cmd = [ "${cc-haas-api}/bin/haas_api" ];
            };
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
        ];
      };
    });
}
