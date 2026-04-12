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

# Cut a release: prep, test, commit, tag, push, publish
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{VERSION}}"
    bare_version="${version#v}"

    # ── Phase 1: Pre-flight ──────────────────────────────────────────
    echo "=== Phase 1: Pre-flight ==="

    if [ -n "$(git status --porcelain)" ]; then
        echo "FAIL: working tree is not clean"; exit 1
    fi

    current="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
    if [ "$current" = "$bare_version" ]; then
        echo "FAIL: Cargo.toml already at $bare_version"; exit 1
    fi

    if git rev-parse "$version" >/dev/null 2>&1; then
        echo "FAIL: tag $version already exists"; exit 1
    fi

    echo "Current version: v$current → $version"
    echo "Running CI..."
    just ci
    echo "CI passed."

    # ── Phase 2: Prep ────────────────────────────────────────────────
    echo ""
    echo "=== Phase 2: Prep ==="

    # Bump Cargo.toml version
    sed -i "0,/^version = \"$current\"/s//version = \"$bare_version\"/" Cargo.toml
    echo "Bumped Cargo.toml to $bare_version"

    # Auto-generate changelog draft from commits since last tag
    last_tag="$(git describe --tags --abbrev=0)"
    commits="$(git log --oneline "$last_tag"..HEAD)"

    # Build changelog section
    changelog_section="## $version\n\n"
    while IFS= read -r line; do
        # Strip the short SHA prefix
        msg="${line#* }"
        changelog_section+="- $msg\n"
    done <<< "$commits"

    # Insert after "# Changelog" header
    sed -i "/^# Changelog$/a\\\\n${changelog_section}" CHANGELOG.md
    echo "Drafted changelog entry from $(echo "$commits" | wc -l) commits."

    # Open editor for review
    echo "Opening CHANGELOG.md in editor..."
    ${EDITOR:-vi} CHANGELOG.md

    # Regenerate Cargo.lock
    cargo fmt
    echo "Cargo.lock regenerated."

    # ── Human testing gate ───────────────────────────────────────────
    echo ""
    echo "=== Test before release ==="
    echo "  Native:  just run       (in another terminal)"
    echo "  Web:     just web-serve  (in another terminal)"
    echo ""
    read -p "Continue with release? [y/N] " answer
    if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
        echo "Aborted. Changes are unstaged — review or discard with: git checkout -- ."
        exit 1
    fi

    # ── Phase 3: Commit, tag, push ───────────────────────────────────
    echo ""
    echo "=== Phase 3: Commit, tag, push ==="
    git add CHANGELOG.md Cargo.toml Cargo.lock
    git commit -m "Release $version"
    git tag "$version"
    git push
    git push --tags
    echo "Pushed commit and tag."

    # ── Phase 4: Wait for CI and publish ─────────────────────────────
    echo ""
    echo "=== Phase 4: Waiting for CI ==="
    tag_sha="$(git rev-parse "$version")"
    elapsed=0
    timeout=1200
    while [ $elapsed -lt $timeout ]; do
        ci_status="$(gh run list --commit "$tag_sha" --workflow CI --json conclusion -q '.[0].conclusion' 2>/dev/null || echo "")"
        if [ "$ci_status" = "success" ]; then
            echo "CI passed."
            break
        elif [ "$ci_status" = "failure" ]; then
            echo "FAIL: CI failed. Fix and re-release."; exit 1
        fi
        echo "  CI pending... (${elapsed}s)"
        sleep 10
        elapsed=$((elapsed + 10))
    done
    if [ $elapsed -ge $timeout ]; then
        echo "FAIL: CI timed out after ${timeout}s"; exit 1
    fi

    echo ""
    echo "=== Creating GitHub release ==="
    gh release create "$version" \
        --title "$version" \
        --notes "See [CHANGELOG.md](https://github.com/n8behavior/bevy-td-sandbox/blob/main/CHANGELOG.md) for details."
    echo ""
    echo "Release created. The Release workflow will now build and deploy to itch.io."
    echo "Monitor: gh run list --limit 3"
