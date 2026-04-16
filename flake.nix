{
  description = "git_progress_sync synchronizes git changes between multiple devices";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rustToolchain = pkgs.rust-bin.stable.latest.default;
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "git_progress_sync";
        version = "0.2.1";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.openssl ];
      };

      overlays.default = final: prev: {
        git_progress_sync = self.packages.${final.stdenv.hostPlatform.system}.default;
      };

      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rustToolchain
          pkgs.pkg-config
          pkgs.openssl
        ];
      };
    };
}
