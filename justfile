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

# Cut a release (idempotent — safe to re-run if interrupted)
release VERSION: (_release-prep VERSION) (_release-ship VERSION) (_release-publish VERSION)
    @echo ""
    @echo "=== {{VERSION}} released ==="
    @echo "Monitor deploy: gh run list --limit 3"

# Prep: CI, version bump, changelog, editor, cargo fmt
[private]
_release-prep VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{VERSION}}"
    bare_version="${version#v}"
    current="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
    if [ "$current" = "$bare_version" ]; then
        echo "[prep] Cargo.toml already at $bare_version — skipping."
        exit 0
    fi
    echo "[prep] v$current → $version"
    if [ -n "$(git status --porcelain)" ]; then
        echo "[prep] FAIL: working tree is not clean"; exit 1
    fi
    echo "[prep] Running CI..."
    just ci
    echo "[prep] CI passed."
    # Bump Cargo.toml version
    sed -i "0,/^version = \"$current\"/s//version = \"$bare_version\"/" Cargo.toml
    echo "[prep] Bumped Cargo.toml to $bare_version"
    # Auto-generate changelog draft from commits since last tag
    last_tag="$(git describe --tags --abbrev=0)"
    commits="$(git log --oneline "$last_tag"..HEAD)"
    changelog_section="## $version\n\n"
    while IFS= read -r line; do
        msg="${line#* }"
        changelog_section+="- $msg\n"
    done <<< "$commits"
    sed -i "/^# Changelog$/a\\\\n${changelog_section}" CHANGELOG.md
    echo "[prep] Drafted changelog from $(echo "$commits" | wc -l) commits."
    echo "[prep] Opening CHANGELOG.md in editor..."
    ${EDITOR:-vi} CHANGELOG.md
    cargo fmt
    echo "[prep] Done."

# Ship: test gate, commit, tag, push
[private]
_release-ship VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{VERSION}}"
    if git ls-remote --tags origin "$version" 2>/dev/null | grep -q "$version"; then
        echo "[ship] Tag $version already on remote — skipping."
        exit 0
    fi
    if git rev-parse "$version" >/dev/null 2>&1; then
        echo "[ship] Tag $version exists locally but not pushed — pushing."
        git push
        git push --tags
        echo "[ship] Pushed."
        exit 0
    fi
    # Test gate
    echo ""
    echo "[ship] === Test before release ==="
    echo "  Native:  just run       (in another terminal)"
    echo "  Web:     just web-serve  (in another terminal)"
    echo ""
    read -p "Continue with release? [y/N] " answer
    if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
        echo "[ship] Aborted. Changes are unstaged — review or discard with: git checkout -- ."
        exit 1
    fi
    git add CHANGELOG.md Cargo.toml Cargo.lock
    git commit -m "Release $version"
    git tag "$version"
    git push
    git push --tags
    echo "[ship] Committed, tagged, and pushed."

# Publish: wait for CI, create GitHub release
[private]
_release-publish VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{VERSION}}"
    if gh release view "$version" >/dev/null 2>&1; then
        echo "[publish] Release $version already exists — skipping."
        exit 0
    fi
    echo "[publish] Waiting for CI..."
    tag_sha="$(git rev-parse "$version")"
    run_id=""
    elapsed=0
    while [ -z "$run_id" ] && [ $elapsed -lt 120 ]; do
        run_id="$(gh run list --commit "$tag_sha" --workflow CI --json databaseId -q '.[0].databaseId' 2>/dev/null || echo "")"
        [ -z "$run_id" ] && sleep 2 && elapsed=$((elapsed + 2))
    done
    if [ -z "$run_id" ]; then
        echo "[publish] FAIL: CI run not found after 120s"; exit 1
    fi
    echo "[publish] Watching CI run $run_id..."
    if ! gh run watch "$run_id" --exit-status; then
        echo "[publish] FAIL: CI failed. Fix and re-run: just release $version"; exit 1
    fi
    echo "[publish] Creating GitHub release..."
    gh release create "$version" \
        --title "$version" \
        --notes "See [CHANGELOG.md](https://github.com/n8behavior/bevy-td-sandbox/blob/main/CHANGELOG.md) for details."
    echo "[publish] Release created — deploy workflow triggered."
