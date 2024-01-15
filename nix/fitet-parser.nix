# nix comment
{
  lib,
  python3Packages,
  version
}:
python3Packages.buildPythonApplication {
  pname = "fitet-parser";
  src = ./.. ;
  inherit version;
  pyproject = true;
  #format = "pyproject";

  # version dynamic in pyproject
  preBuild = ''
    echo ${version} > VERSION
  '';

  propagatedBuildInputs = with python3Packages; [
    beautifulsoup4
    requests
    icecream
    sqlalchemy
  ];

  nativeBuildInputs = [
    python3Packages.setuptools
  ];
}
