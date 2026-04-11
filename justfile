# List available recipes
default:
    @just --list

# Run the game (dynamic linking for fast recompiles)
run *ARGS:
    cargo run -F dynamic {{ARGS}}

# Fast compile check
check:
    cargo check

# Lint (must pass with zero warnings)
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests (unit, integration, doctests)
test:
    cargo test --workspace

# Check formatting
fmt-check:
    cargo fmt --check

# Auto-format code
fmt:
    cargo fmt

# Build local API docs
doc:
    cargo doc --open

# Full CI check (runs format, clippy, and tests)
ci: fmt-check clippy test

# Web dev build via Bevy CLI
web:
    bevy build web

# Optimized web release build
web-release:
    bevy build --release web

# Serve web build locally for browser testing (port 4000)
web-serve:
    bevy run web --port 4000

# Cut a release: flight-check, then create GitHub release to trigger deploy
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Flight check ==="
    # 1. Working tree clean
    if [ -n "$(git status --porcelain)" ]; then
        echo "FAIL: working tree is not clean"; exit 1
    fi
    # 2. Tag exists locally
    if ! git rev-parse "{{VERSION}}" >/dev/null 2>&1; then
        echo "FAIL: tag {{VERSION}} not found"; exit 1
    fi
    # 3. Tag is pushed
    if ! git ls-remote --tags origin "{{VERSION}}" | grep -q "{{VERSION}}"; then
        echo "FAIL: tag {{VERSION}} not pushed to origin"; exit 1
    fi
    # 4. Cargo.toml version matches tag
    cargo_version="v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
    if [ "$cargo_version" != "{{VERSION}}" ]; then
        echo "FAIL: Cargo.toml version ($cargo_version) != {{VERSION}}"; exit 1
    fi
    # 5. CHANGELOG.md has entry
    if ! grep -q "## {{VERSION}}" CHANGELOG.md; then
        echo "FAIL: CHANGELOG.md missing entry for {{VERSION}}"; exit 1
    fi
    # 6. CI passed on the tagged commit
    tag_sha=$(git rev-parse "{{VERSION}}")
    ci_status=$(gh run list --commit "$tag_sha" --workflow CI --json conclusion -q '.[0].conclusion' 2>/dev/null || echo "unknown")
    if [ "$ci_status" != "success" ]; then
        echo "FAIL: CI status is '$ci_status' for tag commit"; exit 1
    fi
    echo "All checks passed."
    echo ""
    echo "=== Creating GitHub release ==="
    gh release create "{{VERSION}}" \
        --title "{{VERSION}}" \
        --notes "See [CHANGELOG.md](https://github.com/n8behavior/bevy-td-sandbox/blob/main/CHANGELOG.md) for details."
    echo ""
    echo "Release created. The Release workflow will now build and deploy to itch.io."
    echo "Monitor: gh run list --limit 3"
