# nix comment
{
  lib,
  python3Packages,
  version ? "0.0.1"
}:
python3Packages.buildPythonApplication {
  pname = "fitet-parser";
  src = ./.. ;
  inherit version;
  pyproject = true;
  #format = "pyproject";

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
