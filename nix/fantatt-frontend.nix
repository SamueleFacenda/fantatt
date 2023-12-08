# nix comment
{
  stdenv,
  lib,
  mkYarnModules,
  yarn
}:

let
  node-modules = mkYarnModules {
    version = "0.1.0";
    pname = "node-modules";
    yarnLock = ../frontend/yarn.lock;
    packageJSON = ../frontend/package.json;
    outputHashMode = "recursive";
    outputHash = lib.fakeSha256;
  };
in

stdenv.mkDerivation {
  name = "fantatt-frontend";
  src = ../frontend; 
  nativeBuildInputs = [
    yarn
    node-modules
  ];
  buildPhase = ''
    mkdir node_modules
    find -L ${node-modules}/node_modules -maxdepth 1 -exec ln -s -t node_modules "{}" +
    ${yarn}/bin/yarn build
  '';
  installPhase =  ''
    mkdir $out
    mv build/* $out
  '';
}
