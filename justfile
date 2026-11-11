# Fantatt command runner. Run inside the dev shell:
#   nix develop -c just <recipe>

# Every local gate. Run this before pushing a branch.
default: check

# Enter the Nix dev shell.
dev:
    nix develop

# Build every crate in the workspace.
build:
    cargo build --workspace

# Run the full test suite.
test:
    cargo nextest run --workspace

# Full local gate suite: format, clippy, tests, comment lint.
check: fmt-check lint test comment-lint

# Lint with clippy, failing on any warning.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Verify formatting without writing.
fmt-check:
    cargo fmt --all -- --check

# Apply formatting.
fmt:
    cargo fmt --all

# Reject forbidden comment leads (why-only comments per the coding rules).
comment-lint:
    scripts/comment-lint

# Validate conventional-commit headers. The commit-msg hook passes the message
# file; CI passes a commit range.
commit-lint +args:
    scripts/commit-lint {{args}}

# Install the lefthook git hooks (once, inside the dev shell).
install-hooks:
    lefthook install

# Dependency advisory scan (requires network).
security:
    cargo audit

# Remove build artifacts.
clean:
    cargo clean
