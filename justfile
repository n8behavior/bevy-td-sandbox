# Bevy CLI release for web builds (the binary CI's binstall resolves to)
bevy_cli_tag := "cli-v0.1.0-alpha.2"

# List available recipes
default:
    @just --list

# Bootstrap a fresh clone: system libs, Rust components, git hooks, web tooling (safe to re-run)
setup: _setup-system _setup-rust _setup-hooks _setup-web
    @echo "[setup] Done. Try: just run"

# Native libraries for Bevy (plus OpenSSL headers for building the Bevy CLI); sudo only if something is missing
[private]
_setup-system:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
        Linux) ;;
        Darwin)
            if xcode-select -p >/dev/null 2>&1; then
                echo "[system] Xcode Command Line Tools present."; exit 0
            fi
            echo "[system] Install Xcode Command Line Tools first: xcode-select --install"; exit 1 ;;
        *)
            echo "[system] Skipping: see README Prerequisites for this platform."; exit 0 ;;
    esac
    . /etc/os-release
    missing=()
    case " $ID ${ID_LIKE:-} " in
        *" debian "*|*" ubuntu "*)
            command -v cc >/dev/null || missing+=(build-essential)
            command -v pkg-config >/dev/null || missing+=(pkgconf)
            for p in libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev libssl-dev; do
                dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q 'ok installed' || missing+=("$p")
            done
            install() { sudo apt-get update && sudo apt-get install -y "$@"; } ;;
        *" fedora "*)
            command -v cc >/dev/null || missing+=(gcc)
            command -v pkg-config >/dev/null || missing+=(pkgconf-pkg-config)
            for p in wayland-devel libxkbcommon-devel alsa-lib-devel systemd-devel openssl-devel; do
                rpm -q "$p" >/dev/null 2>&1 || missing+=("$p")
            done
            install() { sudo dnf install -y "$@"; } ;;
        *" arch "*)
            mapfile -t missing < <(pacman -T base-devel pkgconf wayland libxkbcommon alsa-lib systemd openssl || true)
            install() { sudo pacman -S --needed --noconfirm "$@"; } ;;
        *)
            echo "[system] Unrecognized distro '$ID': install the README Prerequisites by hand."; exit 1 ;;
    esac
    if [ ${#missing[@]} -eq 0 ]; then
        echo "[system] Native build dependencies present."
        exit 0
    fi
    echo "[system] Installing: ${missing[*]}"
    install "${missing[@]}"

# Toolchain components CI uses (fmt, clippy, wasm target)
[private]
_setup-rust:
    @command -v rustup >/dev/null || { echo "[rust] rustup not found: install it from https://rustup.rs, then re-run just setup"; exit 1; }
    rustup component add rustfmt clippy
    rustup target add wasm32-unknown-unknown

# Use the repo's pre-commit hook (fmt + clippy)
[private]
_setup-hooks:
    git config core.hooksPath .githooks

# Bevy CLI plus the wasm-bindgen-cli that matches Cargo.lock (cargo skips anything already installed)
[private]
_setup-web:
    #!/usr/bin/env bash
    set -euo pipefail
    wb_version="$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p' || true)"
    if [ -z "$wb_version" ]; then
        echo "[web] Could not find wasm-bindgen in Cargo.lock"; exit 1
    fi
    cargo install --locked wasm-bindgen-cli --version "$wb_version"
    cargo install --locked --git https://github.com/TheBevyFlock/bevy_cli --tag {{bevy_cli_tag}} bevy_cli

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

# Cut a release: BUMP is patch, minor, or major (re-run the same command to resume)
release BUMP:
    #!/usr/bin/env bash
    set -euo pipefail
    bump="{{BUMP}}"
    case "$bump" in
        patch|minor|major) ;;
        *) echo "Usage: just release patch|minor|major"; exit 1 ;;
    esac
    # Checked first: an unauthenticated gh would make every release look unpublished
    if ! gh auth status >/dev/null 2>&1; then
        echo "FAIL: gh is not logged in (run: gh auth login)"; exit 1
    fi
    bumped() {  # bumped X.Y.Z patch|minor|major -> X.Y.Z
        local major minor patch
        IFS=. read -r major minor patch <<< "$1"
        case "$2" in
            major) echo "$((major + 1)).0.0" ;;
            minor) echo "$major.$((minor + 1)).0" ;;
            patch) echo "$major.$minor.$((patch + 1))" ;;
        esac
    }
    current="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
    if ! [[ "$current" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "FAIL: Cargo.toml version '$current' is not X.Y.Z"; exit 1
    fi
    if gh release view "v$current" >/dev/null 2>&1; then
        version="v$(bumped "$current" "$bump")"
        echo "=== Releasing $version (v$current + $bump) ==="
    else
        # Cargo.toml is at a version with no GitHub release: an earlier run was
        # interrupted after the bump, so finish that release instead of bumping again.
        previous="$(git tag --list 'v*.*.*' --sort=-v:refname | grep -vxF "v$current" | head -1 || true)"
        if [ -z "$previous" ]; then
            echo "FAIL: no earlier release tag to bump from"; exit 1
        fi
        expected="v$(bumped "${previous#v}" "$bump")"
        if [ "$expected" != "v$current" ]; then
            echo "FAIL: v$current is in progress, but $previous + $bump would be $expected."
            echo "      Re-run with the bump that started it, or discard the uncommitted bump."
            exit 1
        fi
        version="v$current"
        echo "=== Resuming $version ($previous + $bump) ==="
    fi
    just _release-prep "$version"
    just _release-ship "$version"
    just _release-publish "$version"
    echo ""
    echo "=== $version released ==="

# Prep: pre-flight checks, CI, version bump, changelog, editor, cargo fmt
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
    # Pre-flight: fail before anything changes
    branch="$(git branch --show-current)"
    if [ "$branch" != "main" ]; then
        echo "[prep] FAIL: on branch '$branch', releases are cut from main"; exit 1
    fi
    if [ -n "$(git status --porcelain)" ]; then
        echo "[prep] FAIL: working tree is not clean"; exit 1
    fi
    git fetch -q origin main
    if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
        echo "[prep] FAIL: main is not in sync with origin/main (pull or push first)"
        git status -sb | head -1
        exit 1
    fi
    if git rev-parse -q --verify "refs/tags/$version" >/dev/null; then
        echo "[prep] FAIL: tag $version already exists locally"; exit 1
    fi
    if git ls-remote --exit-code --tags origin "refs/tags/$version" >/dev/null; then
        echo "[prep] FAIL: tag $version already exists on origin"; exit 1
    fi
    echo "[prep] Pre-flight passed."
    echo "[prep] Running CI (this may take a minute)..."
    if ! just ci > /tmp/release-ci.log 2>&1; then
        echo "[prep] FAIL: CI failed. See /tmp/release-ci.log for details."
        tail -20 /tmp/release-ci.log
        exit 1
    fi
    echo "[prep] CI passed."
    # Bump Cargo.toml version
    sed -i "0,/^version = \"$current\"/s//version = \"$bare_version\"/" Cargo.toml
    # Record the new version in Cargo.lock too, or CI's --locked builds fail on the release commit
    cargo update --workspace --quiet
    echo "[prep] Bumped Cargo.toml and Cargo.lock to $bare_version"
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
    # watch_run RUN_ID [LOG]: succeeds only if the run concluded with success.
    # gh run watch also exits non-zero on GitHub API errors (an HTTP 502 cut the
    # v0.5.0 CI watch short while the run was still green), so the verdict comes
    # from the run itself and a lost watch is resumed.
    watch_run() {
        local id="$1" log="${2:-}" state
        while true; do
            if [ -n "$log" ]; then
                gh run watch "$id" > "$log" 2>&1 || true
            else
                gh run watch "$id" || true
            fi
            state="$(gh run view "$id" --json status,conclusion -q '.status + " " + .conclusion' 2>/dev/null || true)"
            case "$state" in
                "completed success") return 0 ;;
                completed*) return 1 ;;
            esac
            echo "[publish] Lost the watch on run $id (${state:-no status}); retrying in 10s..."
            sleep 10
        done
    }
    echo "[publish] Waiting for CI..."
    tag_sha="$(git rev-parse "$version^{commit}")"
    # Wait for a non-cancelled CI run to appear
    run_id=""
    elapsed=0
    while [ -z "$run_id" ] && [ $elapsed -lt 120 ]; do
        run_id="$(gh run list --commit "$tag_sha" --workflow CI \
            --json databaseId,conclusion \
            -q '[.[] | select(.conclusion != "cancelled")] | .[0].databaseId' \
            2>/dev/null || echo "")"
        [ -z "$run_id" ] || [ "$run_id" = "null" ] && run_id="" && sleep 5 && elapsed=$((elapsed + 5))
    done
    if [ -z "$run_id" ]; then
        echo "[publish] FAIL: CI run not found after 120s"; exit 1
    fi
    echo "[publish] Watching CI run $run_id..."
    if ! watch_run "$run_id"; then
        echo "[publish] FAIL: CI failed. Fix, then re-run the same just release command to finish $version"; exit 1
    fi
    echo "[publish] CI passed. Creating GitHub release..."
    gh release create "$version" \
        --title "$version" \
        --notes "See [CHANGELOG.md](https://github.com/n8behavior/bevy-td-sandbox/blob/main/CHANGELOG.md) for details."
    echo "[publish] Release created — waiting for deploy..."
    # Watch the Release workflow run for this tag; matching headBranch keeps it
    # from picking up the previous release's (already finished) deploy run
    deploy_id=""
    elapsed=0
    while [ -z "$deploy_id" ] && [ $elapsed -lt 120 ]; do
        deploy_id="$(gh run list --workflow Release --json databaseId,event,headBranch \
            -q ".[] | select(.event == \"release\" and .headBranch == \"$version\") | .databaseId" \
            2>/dev/null | head -1 || echo "")"
        [ -z "$deploy_id" ] && sleep 2 && elapsed=$((elapsed + 2))
    done
    if [ -z "$deploy_id" ]; then
        echo "[publish] WARNING: Deploy workflow not found. Check manually: gh run list --limit 3"
        exit 0
    fi
    echo "[publish] Watching deploy run $deploy_id..."
    if ! watch_run "$deploy_id" /tmp/release-deploy-watch.log; then
        echo "[publish] FAIL: Deploy failed. See /tmp/release-deploy-watch.log for details."
        tail -20 /tmp/release-deploy-watch.log
        exit 1
    fi
    echo "[publish] Deploy complete — live on itch.io."
