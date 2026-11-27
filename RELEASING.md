# Release Process

## Quick start

```bash
just release patch   # 0.4.0 → 0.4.1
just release minor   # 0.4.0 → 0.5.0
just release major   # 0.4.0 → 1.0.0
```

The new version comes from bumping the `Cargo.toml` version. You can't
pass a version string.

If a release is interrupted, re-run the same command and it picks up
where it stopped. While `Cargo.toml` is at a version with no GitHub
release yet, the command finishes that release instead of bumping again.
It refuses if the bump you pass doesn't match the one that started it.

This single command handles the entire release:

1. **Pre-flight** — before changing anything, verifies you're on `main` with a clean tree, `gh` is logged in, `main` matches `origin/main` (after a fetch), and the tag doesn't exist locally or on origin; then runs `just ci`
2. **Prep** — bumps the version in `Cargo.toml` and `Cargo.lock`, drafts changelog from commits, opens `$EDITOR`
3. **Test gate** — pauses so you can test native (`just run`) and web (`just web-serve`)
4. **Publish** — commits, tags, pushes, waits for CI, creates GitHub release

The GitHub release triggers the Release workflow, which builds the web bundle and deploys to itch.io.

## What to test before confirming

When the script pauses at "Continue with release?", test in separate terminals:

- [ ] **Native** (`just run`): terrain visible, enemies spawn/path, towers fire, audio works
- [ ] **Web** (`just web-serve` → http://localhost:4000): same checks plus browser console (F12) shows "Generated terrain:" log with no unexpected warnings

## Post-release

1. Watch workflows: `gh run list --limit 3`
2. After the Release workflow completes, verify the itch.io build loads and plays correctly
3. Verify the web zip is attached to the GitHub release
