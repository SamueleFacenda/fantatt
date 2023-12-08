# nix comment
{
  stdenv
, lib
, gradle_8
, jdk17
, perl
, jre17_minimal
, makeWrapper
}:

let
  pname = "fantatt-backend";
  version = "0.0.1";
  deps = stdenv.mkDerivation {
      pname = "${pname}-deps";
      inherit version;
      src = ./.. ;
      nativeBuildInputs = [ gradle_8 perl ];
      buildPhase = ''
        export GRADLE_USER_HOME=$(mktemp -d)
        gradle --no-daemon resolveDependencies
      '';
      # perl code mavenizes pathes (com.squareup.okio/okio/1.13.0/a9283170b7305c8d92d25aff02a6ab7e45d06cbe/okio-1.13.0.jar -> com/squareup/okio/okio/1.13.0/okio-1.13.0.jar)
      installPhase = ''
        find $GRADLE_USER_HOME/caches/modules-2 -type f -regex '.*\.\(jar\|pom\)' \
          | perl -pe 's#(.*/([^/]+)/([^/]+)/([^/]+)/[0-9a-f]{30,40}/([^/\s]+))$# ($x = $2) =~ tr|\.|/|; "install -Dm444 $1 \$out/$x/$3/$4/$5" #e' \
          | sh
      '';
      outputHashMode = "recursive";
      outputHash = lib.fakeHash;
  };
in

stdenv.mkDerivation {
  inherit pname version;
  src = ./.. ; 
  nativeBuildInputs = [ gradle_8 makeWrapper ];
  patchPhase = ''
    # point to offline repo
    # copied from mindustry derivation
    sed -ie "1ipluginManagement { repositories { maven { url '${deps}' } } }; " backend/build.gradle.kts
    sed -ie "s#mavenCentral()#mavenCentral(); maven { url '${deps}' }#g" backend/build.gradle.kts
  '';
  
  buildPhase = ''
    gradle --offline --no-daemon build -Pversion=${version}
  '';
  installPhase =  ''
    mkdir -pv $out/share/java $out/bin
    cp backend/build/libs $out/share/java
    
    makeWrapper ${jre17_minimal}/bin/java $out/bin/fantatt-backend \
        --add-flags "-jar $out/share/java/backend-${version}.jar"
  '';
}
