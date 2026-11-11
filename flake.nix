# Fantatt — fantasy table tennis.
#
# Rust backend whose game logic lives in a knowledge graph evaluated by
# reasoning rules, with a mobile-first PWA built entirely through Nix.
#
# Build framework: flake-parts + crane (Rust) + rust-overlay (docs/adr/0007).

{
  description = "Fantatt — fantasy table tennis — Rust backend, knowledge-graph reasoning rules, mobile-first PWA";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      flake-parts,
      crane,
      rust-overlay,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Phase 0 evaluates and builds on Linux x86_64. Additional systems (and
      # Darwin) are declared once remote builders are available.
      systems = [
        "x86_64-linux"
      ];

      perSystem =
        { system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.stable."1.99.1".default;

          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

          # Filter crate sources (drops .git, target/, node_modules) for
          # stable dependency hashing.
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              (craneLib.fileset.cargoTomlAndLock ./.)
            ];
          };

          pname = "fantatt";
          version = "0.1.0";

          cargoArtifacts = craneLib.buildDepsOnly {
            inherit
              src
              pname
              version
              nativeBuildInputs
              buildInputs
              ;
          };

          # The dependency tree links native code: aws-lc-sys (reqwest's
          # rustls provider) and any bindgen codegen need a C toolchain and
          # cmake; bindgenHook provides LIBCLANG_PATH from libclang.lib. TLS is
          # rustls-only (lettre uses tokio1-rustls-tls), so no OpenSSL.
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            perl
            binutils
            clang
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs; [
            libclang.lib
            openssl
          ];

          craneConf = {
            inherit
              src
              cargoArtifacts
              nativeBuildInputs
              buildInputs
              pname
              version
              ;
          };
        in
        {
          packages = {
            default = craneLib.buildPackage craneConf;

            fantatt-core = craneLib.buildPackage (
              craneConf
              // {
                pname = "fantatt-core";
                cargoExtraArgs = "-p fantatt-core";
              }
            );
          };

          checks = {
            cargo-test = craneLib.cargoTest craneConf;

            cargo-clippy = craneLib.cargoClippy (
              craneConf
              // {
                cargoClippyExtraArgs = "-- --deny warnings --deny clippy::all";
              }
            );

            rustfmt = craneLib.cargoFmt (craneConf // { cargoFmtCommand = "cargo fmt --all -- --check"; });
          };

          devShells.default = pkgs.mkShell {
            packages = [
              rustToolchain
            ]
            ++ (with pkgs; [
              just
              cargo-nextest
              cargo-audit
              pkg-config
              cmake
              perl
              binutils
              clang
              libclang.lib
              jq
              git
              ripgrep
            ]);

            shellHook = ''
              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
              export RUST_BACKTRACE=1
              echo "fantatt dev shell — $(rustc --version)"
            '';
          };

          formatter = pkgs.nixfmt;
        };
    };
}
