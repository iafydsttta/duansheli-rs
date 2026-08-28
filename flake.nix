{
  description = "Duansheli - Keeping directories clean with a small Rust CLI";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
       url = "github:oxalica/rust-overlay";
       inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }:
    let supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
    ];

    forAllSystems =
      f: nixpkgs.lib.genAttrs supportedSystems (
        system: f (
          import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
        }
      )
    );

    cargoToml = nixpkgs.lib.importTOML ./Cargo.toml;
    in {

      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          # ...
        };
        meta = {
          # ...
        };
      });

      # apps = ...
      # devShells ...
      # checks ... 
      # formatter
    };
}
