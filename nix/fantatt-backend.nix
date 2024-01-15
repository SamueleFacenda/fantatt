# nix comment
{
  stdenv
, lib
, gradle_8
, jdk21_headless
, jre_minimal
, perl
, makeWrapper
, version
}:

let
  gradle = gradle_8.override {java = jdk21_headless;};
  jre = jre_minimal.override {
    jdk = jdk21_headless;
    # retrived with 'find /nix/store/<hash>fantatt-backend-deps-0.0.1 -name '*.jar' -exec jdeps --list-deps --ignore-missing-deps '{}' ';' | sort | uniq'
    modules = [
      "java.base"
      "java.compiler"
      "java.datatransfer"
      "java.desktop"
      "java.instrument"
      "java.logging"
      "java.management"
      "java.naming"
      "java.net.http"
      "java.prefs"
      "java.rmi"
      "java.scripting"
      "java.security.jgss"
      "java.sql"
      "java.sql.rowset"
      "java.transaction.xa"
      "java.xml"
    ];
  };
  pname = "fantatt-backend"; # !! defined also in settings.gradle.kts TODO override here
  deps = stdenv.mkDerivation {
    pname = "${pname}-deps";
    inherit version;
    src = ./.. ;
    nativeBuildInputs = [ gradle perl ];
    buildPhase = ''
      export GRADLE_USER_HOME=$(mktemp -d)
      gradle --no-daemon resolveDependencies
    '';
    # perl code mavenizes pathes (com.squareup.okio/okio/1.13.0/a9283170b7305c8d92d25aff02a6ab7e45d06cbe/okio-1.13.0.jar -> com/squareup/okio/okio/1.13.0/okio-1.13.0.jar)
    # custom sed , remove the gradle version from some jar archives
    installPhase = ''
      find $GRADLE_USER_HOME/caches/modules-2 -type f -regex '.*\.\(jar\|pom\)' \
        | perl -pe 's#(.*/([^/]+)/([^/]+)/([^/]+)/[0-9a-f]{30,40}/([^/\s]+))$# ($x = $2) =~ tr|\.|/|; "install -Dm444 $1 \$out/$x/$3/$4/$5" #e' \
        | sed 's/-gradle[0-9]\+//2' \
        | sh
    '';
    outputHashMode = "recursive";
    outputHash = "sha256-Du4MZozefjiXmu/YA67G0YRU9ND6Dg+buFH3jL5LQHs=";
  };
in

stdenv.mkDerivation {
  inherit pname version;
  src = ./.. ; 
  nativeBuildInputs = [ gradle makeWrapper ];
  postPatch = ''
    # point to offline repo
    # copied from mindustry derivation
    sed -i "1ipluginManagement { repositories { maven { url = uri(\"${deps}\") } } }" settings.gradle.kts
    sed -i "s#mavenCentral()#mavenCentral(); maven { url = uri(\"${deps}\") }#g" build.gradle.kts
  '';
  
  buildPhase = ''
    gradle --offline --no-daemon build -Pversion=${version}
  '';
  installPhase =  ''
    mkdir -pv $out/share/java $out/bin
    cp build/libs/* $out/share/java
    
    makeWrapper ${jre}/bin/java $out/bin/${pname} \
        --add-flags "-jar $out/share/java/${pname}-${version}.jar"
  '';
}
