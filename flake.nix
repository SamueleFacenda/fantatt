# first line
{
  description = "Fantasy tabletennis!";

  # Nixpkgs / NixOS version to use.
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
          fitet-parser = pkgs.python3.pkgs.buildPythonApplication {
            pname = "fitet-parser";
            src = ./.;
            version = 0.1 .0;
            pyproject = true;
            #format = "pyproject";

            propagatedBuildInputs = with pkgs.python3.pkgs; [
              beautifulsoup4
              requests
              icecream
              sqlalchemy
            ];

            nativeBuildInputs = with pkgs; [
              python3.pkgs.setuptools
            ];

          };
        };

        apps = {
          default = {
            type = "app";
            program = "${self.packages.${system}.default}/bin/fitet-runner";
          };
        };

        devShells = {
          default = pkgs.mkShell {
            packages = with pkgs; [
              php
              (python3.withPackages (ps: with ps; [
                beautifulsoup4
                requests
                icecream
                pillow
                tqdm
                pandas
                pyautogui
                keyboard
                sqlalchemy
              ]))
            ];

            #shellHook = ''
            #  exec zsh
            #'';

          };

        };
      }
    );
}
