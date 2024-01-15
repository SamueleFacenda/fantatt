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

        packages = pkgs.lib.genAttrs [ "fitet-parser" "fantatt-backend" "fantatt-frontend" ]
          (name: pkgs.callPackage (import ./nix/${name}.nix) {inherit version;});

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
              
              jdk21
              (gradle_8.override {java = jdk21_headless;})
              
              nodejs_20
            ];
          };

        };
      }
    );
}
