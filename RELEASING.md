# Release Process

## Quick start

```bash
just release vX.Y.Z
```

This single command handles the entire release:

1. **Pre-flight** — verifies clean tree, no tag conflict, runs `just ci`
2. **Prep** — bumps `Cargo.toml` version, drafts changelog from commits, opens `$EDITOR`
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
