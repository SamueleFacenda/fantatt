# nix comment
{
  lib,
  python3Packages
}:
python3Packages.buildPythonApplication {
  pname = "fitet-parser";
  src = ../ ;
  version = 0.1.0;
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
