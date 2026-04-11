# Release Process

## Pre-release checklist

- [ ] `just ci` passes (format + clippy + tests)
- [ ] Test native build: `just run`
  - Terrain features visible (brown rubble, blue puddles, green radioactive)
  - Enemies spawn and path correctly
  - Towers place, fire, and collect scrap
  - Audio plays (shots, deaths, scrap collection)
- [ ] Test web build: `just web-serve` (opens at http://localhost:4000)
  - Same checks as native above
  - Browser console (F12) has no unexpected warnings or errors
  - "Generated terrain:" info log appears in console

## Cut the release

1. Update `CHANGELOG.md` with a new version section
2. Bump `version` in `Cargo.toml`
3. Run `cargo fmt` (regenerates `Cargo.lock`)
4. Run `just ci` to re-verify
5. Commit: `git add CHANGELOG.md Cargo.toml Cargo.lock && git commit -m "Release vX.Y.Z"`
6. Tag: `git tag vX.Y.Z`
7. Push: `git push && git push --tags`
8. Create release: `just release vX.Y.Z`

The `just release` recipe flight-checks everything (clean tree, tag exists and is pushed, Cargo.toml version matches, CHANGELOG entry present, CI green) and then creates a GitHub release, which triggers the Release workflow.

## Post-release

1. Watch workflows: `gh run list --limit 3`
2. After the Release workflow completes, verify the itch.io build:
   - Game loads and plays correctly
   - Browser console has no unexpected warnings
3. Verify the web zip is attached to the GitHub release
