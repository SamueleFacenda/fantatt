{ lib,config, dream2nix }: {
  imports = [
    dream2nix.modules.dream2nix.nodejs-package-lock-v3
    dream2nix.modules.dream2nix.nodejs-granular-v3
  ];
  
  mkDerivation.src = ./..;
  
  nodejs-package-lock-v3 = {
    packageLockFile = "${config.mkDerivation.src}/yarn.lock";
  };
}
