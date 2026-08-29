{
  description = "Duansheli - Keeping directories clean with a small Rust CLI";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );

      cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
    in
    {

      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          # Tools needed on the build machine
          nativeBuildInputs = [ ];
          # For linking external libs into the proj
          buildInputs = [ ];

          meta = {
            description = "Directory declutter and archival tool";
            license = pkgs.lib.licenses.mit;
            mainProgram = cargoToml.package.name;
            platforms = pkgs.lib.platforms.unix;
          };
        };
      });

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = pkgs.lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {

          packages = [
            (pkgs.rust-bin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
                "clippy"
                "rustfmt"
              ];
            })
            # pkgs.cargo-nextest
            # pkgs.cargo-watch
            pkgs.just
          ];

          env = {
            RUST_BACKTRACE = "1";
            RUST_LOG = "debug";
          };

          shellHook = ''
            echo "dev shell: ${cargoToml.package.name} ${cargoToml.package.version}"
          '';
        };

      });
      checks = forAllSystems (pkgs: {
        build = self.packages.${pkgs.stdenv.hostPlatform.system}.default;

        fmt =
          pkgs.runCommand "check-fmt"
            {
              nativeBuildInputs = [ pkgs.rust-bin.stable.latest.default ];
            }
            ''
              cargo fmt --manifest-path ${./.}/Cargo.toml --check
              touch $out
            '';
      });

      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);
    };
}
