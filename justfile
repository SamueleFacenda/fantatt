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

# Line coverage for the domain core and the API shell. The invariant code of
# fantatt-core must stay fully covered; the workspace gate is 80% lines
# (docs/15-testing.md). Needs llvm-tools matching the pinned toolchain's LLVM
# version: either rustup component add llvm-tools-preview or
# LLVM_COV/LLVM_PROFDATA pointing at that exact LLVM release (a different
# llvm-cov rejects the raw profile).
coverage:
    cargo llvm-cov -p fantatt-core -p fantatt-api --summary-only

# Verify the recorded FITET fixture tree: every file against
# manifest.sha256, every declared match array, and the record and surface
# counts (tests/fixtures/fitet).
fixtures-verify:
    cargo run -q --bin fantatt-fixtures -- verify

# Smoke the discovery sweep against the live portal: walk portale.fitet.org
# through the polite fetch client, record the pages in the dev database, and
# queue the work list. Needs network access; a second sweep follows to prove
# the ledger cache prunes. Structural assertions only - the golden
# expectations live against the recorded fixtures (G1.3).
# why: the portal's edge resets non-browser TLS clients (rustls) with a
# connection reset while the same client reads the plain-HTTP origin fine, so
# the smoke walks over http:// until the edge accepts the rustls client.
discover-smoke: db-migrate
    #!/usr/bin/env bash
    set -euo pipefail
    export DATABASE_URL="{{db-url}}"
    export FANTATT_SMOKE_BASE_URL="http://portale.fitet.org/"
    cargo run -q --example live_sweep -p fantatt-jobs

# Re-record the live FITET surfaces behind tests/fixtures/fitet/html, rewrite
# capture.json and manifest.sha256, then verify the refreshed tree. Needs
# network access to portale.fitet.org; a surface that fails to fetch fails the
# recipe instead of writing a partial file.
fixtures-capture:
    scripts/fixtures-capture
    cargo run -q --bin fantatt-fixtures -- verify

# Full local gate suite: format, clippy, tests, fixture tree, comment lint,
# secret scan, offline query data freshness, docs links, dependency watchdog.
check: fmt-check lint test fixtures-verify comment-lint secrets db-offline-check docs-check watchdog watchdog-selftest

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

# Secret scan over staged changes. CI scans the full history with gitleaks
# directly, which is stronger than the staged-only hook.
secrets:
    gitleaks git --staged --no-banner

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

# Fail when a direct dependency lacks a watchdog row, the row disagrees with
# Cargo.lock, or a newer upstream release stayed unreviewed for 90 days
# (docs/dependency-watchdog.md).
watchdog:
    scripts/watchdog-check

# Prove the stale-row failure path on a throwaway copy of the table.
watchdog-selftest:
    scripts/watchdog-selftest

# Markdown link check. Relative links resolve offline; --external also fetches
# http(s) targets with curl.
docs-check *args:
    scripts/docs-link-check {{args}}

# Local Postgres for development and gates. The dev database lives in
# .dev/postgres and listens on 5433 on loopback with trust auth; nothing but
# the local gates ever reach it. Integration tests do not use it: they spawn
# their own ephemeral instance (crates/fantatt-test-support).
db-data := ".dev/postgres"
db-port := "5433"
db-name := "fantatt_dev"
db-url := "postgres://postgres@127.0.0.1:" + db-port + "/" + db-name

# Start the local dev Postgres (idempotent) and print its DATABASE_URL.
db-start:
    #!/usr/bin/env bash
    set -euo pipefail
    data="$(pwd)/{{db-data}}"
    if [ ! -f "$data/PG_VERSION" ]; then
        mkdir -p "$data"
        initdb -D "$data" -U postgres --auth=trust --encoding=UTF8 >/dev/null
    fi
    # why: Postgres resolves unix_socket_directories against its own working
    # directory, so the recipe passes absolute paths only.
    pg_ctl -D "$data" -l "$(pwd)/.dev/postgres.log" \
        -o "-p {{db-port}} -c listen_addresses=127.0.0.1 -c unix_socket_directories=$data" \
        start >/dev/null 2>&1 || echo "postgres already running"
    createdb -h 127.0.0.1 -p {{db-port}} -U postgres {{db-name}} 2>/dev/null || true
    echo "DATABASE_URL={{db-url}}"

# Stop the local dev Postgres.
db-stop:
    #!/usr/bin/env bash
    pg_ctl -D "$(pwd)/{{db-data}}" stop -m fast

# Apply all migrations to the dev database.
db-migrate: db-start
    #!/usr/bin/env bash
    export DATABASE_URL="{{db-url}}"
    sqlx migrate run --source migrations

# Rebuild the dev database from scratch and apply all migrations.
db-reset: db-start
    #!/usr/bin/env bash
    export DATABASE_URL="{{db-url}}"
    dropdb -h 127.0.0.1 -p {{db-port}} -U postgres --if-exists {{db-name}}
    createdb -h 127.0.0.1 -p {{db-port}} -U postgres {{db-name}}
    sqlx migrate run --source migrations

# Regenerate the committed offline query data (.sqlx) from the dev database.
db-prepare: db-migrate
    #!/usr/bin/env bash
    export DATABASE_URL="{{db-url}}"
    cargo sqlx prepare --workspace -- --all-targets

# Fail when the committed offline query data no longer matches the schema.
db-offline-check: db-migrate
    #!/usr/bin/env bash
    export DATABASE_URL="{{db-url}}"
    cargo sqlx prepare --check --workspace -- --all-targets

# Remove build artifacts.
clean:
    cargo clean

# Serve the API shell (G0.6) against the dev database. The CSRF secret falls
# back to a dev-only value when the environment does not carry one; a
# deployment always sets FANTATT__CRYPTO__SESSION_SECRET.
api-serve: db-migrate
    #!/usr/bin/env bash
    set -euo pipefail
    export FANTATT__DATABASE__URL="{{db-url}}"
    export FANTATT__CRYPTO__SESSION_SECRET="${FANTATT__CRYPTO__SESSION_SECRET:-dev-only-insecure-secret}"
    cargo run --bin fantatt-api

# Demo the API shell (G0.6): the health check, a CSRF-rejected logout, a fresh
# session with its CSRF token, and the OpenAPI document.
api-demo: db-migrate
    #!/usr/bin/env bash
    set -euo pipefail
    export FANTATT__DATABASE__URL="{{db-url}}"
    export FANTATT__CRYPTO__SESSION_SECRET="demo-only-insecure-secret"
    cargo run --bin fantatt-api &
    server=$!
    trap 'kill "$server" 2>/dev/null || true' EXIT
    sleep 3
    echo "== GET /healthz =="
    curl -sS http://127.0.0.1:8080/healthz
    echo
    echo "== DELETE /auth/session without a CSRF token =="
    curl -sS -X DELETE http://127.0.0.1:8080/auth/session
    echo
    echo "== GET /auth/csrf (a fresh session and its token) =="
    curl -sS -c "$(pwd)/.dev/demo-cookies.txt" http://127.0.0.1:8080/auth/csrf
    echo
    echo "== GET /openapi.json (first lines) =="
    curl -sS http://127.0.0.1:8080/openapi.json | head -8
