# Nix build modules

The Rust workspace build (crane) and the dev shell are defined in the root
`flake.nix`. `modules/` collects shareable NixOS module sources
(Phase 4). Frontend (`buildNpmPackage`) and container builders join here in
Phase 3–4.

See `docs/adr/0007-nix-build-system.md`.
