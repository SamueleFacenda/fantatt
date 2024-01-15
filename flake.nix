# nix comments
{
  description = "Fantasy tabletennis!";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";
  };
  
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (nixpkgs.legacyPackages.${system}.extend (final: prev: {
          # overlays
        }));
        version = "0.0.1";
      in
      {

        packages = rec {
          default = fitet-parser;
          fitet-parser = pkgs.callPackage (import ./nix/fitet-parser.nix) {inherit version;};
          fantatt-backend = pkgs.callPackage (import ./nix/fantatt-backend.nix) {inherit version;};      
          fantatt-frontend = pkgs.callPackage (import ./nix/fantatt-frontend.nix) {inherit version;};
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
              (gradle_8.override {java = jdk17;})
              
              nodejs_20
            ];
          };

        };
      }
    );
}
