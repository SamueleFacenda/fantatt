# nix comments
{
  description = "Fantasy tabletennis!";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
  
    dream2nix.url = "github:nix-community/dream2nix";
    nixpkgs.follows = "dream2nix/nixpkgs";
  };
  
  outputs = { self, nixpkgs, flake-utils, dream2nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (nixpkgs.legacyPackages.${system}.extend (final: prev: {
          # overlays
        }));
      in
      {

        packages = rec {
          default = fitet-parser;
          fitet-parser = pkgs.callPackage (import ./nix/fitet-parser.nix) {} ;
          fantatt-backend = pkgs.callPackage (import ./nix/fantatt-backend.nix) {} ;
          fantatt-frontend = dream2nix.lib.evalModules {
            packageSets.nixpkgs = pkgs;
            modules = [
              ./nix/fantatt-frontend.nix
            ];
            projectRoot = ./.;
            projectRootFile = "flake.nix";
            package = ./.;
          };
        };

        apps = {
          default = {
            type = "app";
            program = "${self.packages.${system}.fitet-parser}/bin/fitet-runner";
          };
        };

        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs; [
              (python3.withPackages (ps: with ps; [
                beautifulsoup4
                requests
                icecream
                tqdm
                sqlalchemy
              ]))
              
              jdk17
              gradle_8
              
              nodejs_20
              yarn-berry
            ];
          };

        };
      }
    );
}
