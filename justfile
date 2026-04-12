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
release VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    if gh release view "{{VERSION}}" >/dev/null 2>&1; then
        echo "Release {{VERSION}} already exists — nothing to do."
        exit 0
    fi
    just _release-prep "{{VERSION}}"
    just _release-ship "{{VERSION}}"
    just _release-publish "{{VERSION}}"
    echo ""
    echo "=== {{VERSION}} released ==="

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
    echo "[prep] Running CI (this may take a minute)..."
    if ! just ci > /tmp/release-ci.log 2>&1; then
        echo "[prep] FAIL: CI failed. See /tmp/release-ci.log for details."
        tail -20 /tmp/release-ci.log
        exit 1
    fi
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
    echo "[prep] Drafted changelog from $(echo "$commits" | wc -l) commits — opening editor."
    ${EDITOR:-vi} CHANGELOG.md
    cargo fmt --quiet
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
        git push -q --follow-tags
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
    git diff --cached --quiet || git commit -q -m "Release $version"
    git tag -a "$version" -m "Release $version"
    git push -q --follow-tags
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
    echo "[publish] Waiting for CI to pass..."
    tag_sha="$(git rev-parse "$version")"
    elapsed=0
    timeout=1200
    while [ $elapsed -lt $timeout ]; do
        # Look for a successful CI run on this exact commit
        status="$(gh run list --commit "$tag_sha" --workflow CI \
            --json conclusion -q '[.[] | select(.conclusion == "success")] | length' \
            2>/dev/null || echo "0")"
        if [ "$status" -gt 0 ] 2>/dev/null; then
            echo "[publish] CI passed."
            break
        fi
        # Check for a real failure (not cancellation)
        failed="$(gh run list --commit "$tag_sha" --workflow CI \
            --json conclusion -q '[.[] | select(.conclusion == "failure")] | length' \
            2>/dev/null || echo "0")"
        if [ "$failed" -gt 0 ] 2>/dev/null; then
            echo "[publish] FAIL: CI failed on $tag_sha"; exit 1
        fi
        printf "\r[publish] CI pending... (%ds)" "$elapsed"
        sleep 10
        elapsed=$((elapsed + 10))
    done
    if [ $elapsed -ge $timeout ]; then
        echo "[publish] FAIL: CI timed out after ${timeout}s"; exit 1
    fi
    echo "[publish] CI passed. Creating GitHub release..."
    gh release create "$version" \
        --title "$version" \
        --notes "See [CHANGELOG.md](https://github.com/n8behavior/bevy-td-sandbox/blob/main/CHANGELOG.md) for details."
    echo "[publish] Release created — waiting for deploy..."
    # Watch the Release workflow triggered by the GH release
    deploy_id=""
    elapsed=0
    while [ -z "$deploy_id" ] && [ $elapsed -lt 120 ]; do
        deploy_id="$(gh run list --workflow Release --json databaseId,event,headBranch -q '.[] | select(.event=="release") | .databaseId' 2>/dev/null | head -1 || echo "")"
        [ -z "$deploy_id" ] && sleep 2 && elapsed=$((elapsed + 2))
    done
    if [ -z "$deploy_id" ]; then
        echo "[publish] WARNING: Deploy workflow not found. Check manually: gh run list --limit 3"
        exit 0
    fi
    echo "[publish] Watching deploy run $deploy_id..."
    if ! gh run watch "$deploy_id" --exit-status > /tmp/release-deploy-watch.log 2>&1; then
        echo "[publish] FAIL: Deploy failed. See /tmp/release-deploy-watch.log for details."
        tail -20 /tmp/release-deploy-watch.log
        exit 1
    fi
    echo "[publish] Deploy complete — live on itch.io."
