{ stdenv
, lib
, fetchNpmDeps
, nodejs_20
, version ? "0.0.1"
}:

let
  fetchNodeModules = {src, hash}:
    let 
      npmCache = fetchNpmDeps {inherit src hash; };
    in stdenv.mkDerivation {
      name = "node-modules";
      dontUnpack = true;      
      nativeBuildInputs = [ nodejs_20 ];
      buildPhase = ''
        echo 'cache = ${npmCache}' >> .npmrc
        ln -st . ${src}/package-lock.json ${src}/package.json
        npm clean-install --offline
      '';
      installPhase = ''
        mkdir $out
        cp -rf node_modules $out
      '';
    };
  nodeDeps = fetchNodeModules {
    src = ./..;
    hash = "sha256-KSaRB9G9Yp6Ny9P8ldbP7pM7uGJDAseEvfB3FKgJfaM=";
  };
in

stdenv.mkDerivation {
  inherit version;
  pname = "fantatt-frontend";
  src = ./..;
  nativeBuildInputs = [ nodejs_20 ];
  
  # link the entries in node_modules alones to have a writeable node_modules/.cache
  buildPhase = ''
    runHook preBuild
    
    mkdir node_modules
    ln -st ./node_modules ${nodeDeps}/node_modules/*
    npm run build
    
    runHook postBuild
  '';
  
  installPhase = ''
    mkdir $out
    cp -r build/* $out
  '';
}
