{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        toolchainFor =
          p:
          p.rust-bin.selectLatestNightlyWith (
            tc:
            tc.default.override {
              extensions = [ "rust-src" ];
              targets = [ "x86_64-unknown-linux-gnu" ];
            }
          );
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchainFor;
      in
      {
        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            pkg-config

            nodejs
            bun

            yt-dlp
            uv
          ];
        };
      }
    );
}
