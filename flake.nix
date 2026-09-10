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
          config = {
            allowUnfree = true;
            cudaSupport = true;
          };
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
          NIX_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              zlib
              glib
              cudaPackages.cudatoolkit
              stdenv.cc.cc
              libxcb
              libGL
            ]
          );

          # NIX_LD = pkgs.lib.fileContents "${pkgs.stdenv.cc}/nix-support/dynamic-linker";
          packages = with pkgs; [
            pkg-config
            nodejs
            bun
            yt-dlp
            uv
          ];

          buildInputs = with pkgs; [
            zlib
            glib
            cudaPackages.cudatoolkit
            stdenv.cc.cc
            libxcb
            libGL
          ];

          shellHook = ''
            export CUDA_PATH=${pkgs.cudaPackages.cudatoolkit}
            export LD_LIBRARY_PATH=${pkgs.stdenv.cc.cc.lib}:/run/opengl-driver/lib:$NIX_LD_LIBRARY_PATH:$LD_LIBRARY_PATH
          '';
        };
      }
    );
}
