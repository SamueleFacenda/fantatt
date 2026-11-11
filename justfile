# Fantatt command runner. Run inside the dev shell:
#   nix develop -c just <recipe>

default:
    @just --list

# Enter the Nix dev shell.
dev:
    nix develop

# Build every crate in the workspace.
build:
    cargo build --workspace

# Run the full test suite.
test:
    cargo nextest run --workspace

# Pre-merge gate: format check + clippy + tests.
check: fmt-check lint test

# Lint with clippy, failing on any warning.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Verify formatting without writing.
fmt-check:
    cargo fmt --all -- --check

# Apply formatting.
fmt:
    cargo fmt --all

# Dependency advisory scan (requires network).
security:
    cargo audit

# Remove build artifacts.
clean:
    cargo clean
