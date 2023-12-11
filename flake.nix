# nix comments
{
  description = "Fantasy tabletennis!";

  inputs.nixpkgs.url = "nixpkgs/nixpkgs-unstable";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
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
          fantatt-frontend = pkgs.callPackage (import ./nix/fantatt-frontend.nix) {} ;
          fantatt-backend = pkgs.callPackage (import ./nix/fantatt-backend.nix) {} ;
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
