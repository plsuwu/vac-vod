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

        craneLib = crane.mkLib pkgs;
      in
      {
        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            pkg-config
            nodejs
            bun

            yt-dlp
          ];
        };
      }
    );
}

# {
#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
#     utils.url = "github:numtide/flake-utils";
#   };
#   outputs =
#     {
#       self,
#       nixpkgs,
#       utils,
#     }:
#     utils.lib.eachDefaultSystem (
#       system:
#       let
#         pkgs = nixpkgs.legacyPackages.${system};
#       in
#       {
#         devShell = pkgs.mkShell {
#         };
#       }
#     );
# }
