# Roadmap

**Status:** written 2026-11-27 after v0.5.0. The direction is decided: see [Decisions](#decisions) at the end. The plan itself lives in the [waves](https://github.com/n8behavior/bevy-td-sandbox/milestones), one GitHub milestone per wave, and each wave is one release. This file keeps the reasoning behind them.

## Where things stand

- **The game:** a playable open-field mazing tower defense, with a 20-wave Classic mode (now with a win screen, [#76]) and an Endless mode. It has 7 tower types, 4 enemy types, terrain, upgrades and repair.
- **Engineering:** about 350 tests, zero warnings, CI, a scripted release, and a web build on [itch.io](https://n8behavior.itch.io/bevy-td-sandbox). It's up to date on Bevy 0.20 and bevy_northstar 0.8.
- **History:** most of the work happened in March–April 2026. After 2026-05-01 there was a seven-month pause; work resumed on 2026-11-27.
- **Bugs:** none open. #66, #76 and #78 are closed. The Raider enemy was removed until it can be made real ([#82]).

## The tower editor brainstorm

`TOWER_EDITOR_BRAINSTORM.md` was never on `main`. It lives on the unmerged branch [`editor-brainstorm`](https://github.com/n8behavior/bevy-td-sandbox/tree/editor-brainstorm): eight commits from 2026-04-25 to 04-28, 1,642 lines. Besides the brainstorm, the branch has:
- a full modding manual (an mdbook published through a GitHub Pages workflow) covering Lua tower recipes, an atom reference, worked examples, validation and the editor UI.
- `just book` and `just book-serve` recipes for building it.

Its state:

- **Dead links:** the comments on [#75] and [#55] link to the brainstorm on `main`, so both links are 404s.
- **Out of date:** the branch forked before the [#81] refactor. #81 and the enemy refactor (9706a31) built the brainstorm's "components + interfaces" idea, but under different names (`Turret`, `SlowOnHit`, `UpgradeTrack`, `PanelSections` instead of Trigger, Acquirer, Deliverer, Payload).
- **Ahead of the code:** the manual describes a Lua recipe system that doesn't exist yet as if it already shipped.

## The open issues fall into four themes

1. **Make it fun and readable:**
   - [#69] pile growth is tied to the scrap you can spend.
   - [#73] visual identity.
   - [#82] Raiders as a real enemy.
   - Balance, and win and score ([#76], [#84]).
2. **See what's happening:**
   - [#77] diagnostics.
   - [#83] dev mode with "Show pathing".
   - [#84] player stats.
   - [#55] event-driven hits.
3. **Make it extensible:** [#75] plus the brainstorm branch: towers and enemies defined by data or scripts, and the editor.
4. **Distribution and infrastructure:** [#71] save/load, [#79] crates.io, [#74] CI Docker image.

### Theme 2 shares one foundation

[#77], [#83], [#84] and [#55] all need the same thing: a stream of gameplay events with credit attached, saying who damaged, killed, collected or stole what, and when.

- [#77] adds up the stream for developers.
- [#83] draws it live for developers.
- [#84] adds it up for players.
- [#55] is how hits become events.

Build the stream once, and the four issues mostly become views on it. Today nothing records which tower did what: projectiles don't carry the tower that fired them, a kill event doesn't say who landed it, and scrap pickup isn't credited to a collector.

## A candidate north star

The repo name says sandbox, the brainstorm says creator platform ("easiest and most fun tower-building editor ever"), and most issues say polished game. Each needs different work. One framing ties the threads together:

> **A tower defense where you design towers, test them, and see exactly how they performed.**

The editor creates towers, the stats and advice judge them, and dev mode shows the mechanics. Under this framing the observability work isn't a side quest; it's the core of the testing loop.

## A possible order

1. **The event stream.** Add the source tower to every hit, kill and pickup, and route hits through events ([#55], plus [#77]'s core). Players won't notice it, but everything after this depends on it.
2. **Dev mode with Show pathing ([#83]).** It speeds up everything after it; the Raider freeze ([#82]) would have taken a glance to diagnose.
3. **Core feel: [#69] and [#73].** Do these before the editor, since every new tower type makes the "colored rectangles" problem worse.
4. **Stats, phases 1–2 ([#84]).** The tower-panel stats, then the post-wave summary and end-of-run report.
5. **The editor track ([#75]).** Start by reconciling the brainstorm with the capability model `main` actually has. Settle the recipe format before building UI.

## Risk to check early: Lua in the web build

The manual assumes Lua through `mlua` runs "identically on browser and native." The itch.io build targets `wasm32-unknown-unknown`. mlua 0.12.3 supports that target, but only "with a WASI C runtime", and its `docs/wasm32.md` says "The host must provide the matching WASI imports and support Wasm exceptions." So the open question is whether our browser build (Bevy and wasm-bindgen, served on itch.io) can provide those. If it can't, the recipe format would need to be a data format (e.g. RON with Bevy reflection) or a pure-Rust script engine. A quick spike would settle it before more design goes into Lua. The game also ships on Windows, Mac and Linux (see [Decisions](#decisions)), so any format must work on all four platforms.

## Park or close

- **[#79] crates.io:** low value for a game; players use itch.io and developers clone the repo.
- **[#74] CI Docker image:** CI's 14–18 minutes are mostly compiling, and the image would only save toolchain setup.
- **[#71] save/load:** matters mostly for long Endless runs, and gets easier once the game's state is described by data.

## Housekeeping

- GAMEPLAY.md says you start with 200 scrap; the code uses 250 (`STARTING_SCRAP`).
- GAMEPLAY.md's tower table lists 6 towers and has no Mortar, but `src/tower/mortar` exists.
- [#75]'s title, "Factor towers & enemies into separate components," is mostly done by [#81]. What's left is config, scripts and the editor.
- Decide whether to merge, rebase or retire the `editor-brainstorm` branch, and fix the dead brainstorm links in [#55] and [#75].
- Remove the leftover `upgrade-probe` worktree (`.claude/worktrees/upgrade-probe`) from the Bevy 0.20 upgrade.

## Decisions

Mike answered the open questions on 2026-11-27.

1. **Who is this primarily for?** Players. It will release on itch.io and other online stores. It will ship with an editor for modders, so it is also for players who want to mod the game. But first and foremost it must be a fun game to play, with no modding needed out of the box.
2. **Is the web build the main platform?** No. It will ship on Windows, Mac, Linux and web. No current plans for mobile, but not off the table.
3. **What would the next release mean?** Not decided yet. We work it out from this roadmap and other goals as they come. Each wave in riff is one release.
4. **The editor and Lua track versus polishing the core game?** The core game is the priority: make it fun, then make it pretty. Visuals and sound stay at just enough to work through the core mechanics. The editor matters as a way to design and test towers and enemies with different abilities, and to buff or nerf them quickly, always in service of improving the core game.

[#55]: https://github.com/n8behavior/bevy-td-sandbox/issues/55
[#69]: https://github.com/n8behavior/bevy-td-sandbox/issues/69
[#71]: https://github.com/n8behavior/bevy-td-sandbox/issues/71
[#73]: https://github.com/n8behavior/bevy-td-sandbox/issues/73
[#74]: https://github.com/n8behavior/bevy-td-sandbox/issues/74
[#75]: https://github.com/n8behavior/bevy-td-sandbox/issues/75
[#76]: https://github.com/n8behavior/bevy-td-sandbox/issues/76
[#77]: https://github.com/n8behavior/bevy-td-sandbox/issues/77
[#79]: https://github.com/n8behavior/bevy-td-sandbox/issues/79
[#81]: https://github.com/n8behavior/bevy-td-sandbox/issues/81
[#82]: https://github.com/n8behavior/bevy-td-sandbox/issues/82
[#83]: https://github.com/n8behavior/bevy-td-sandbox/issues/83
[#84]: https://github.com/n8behavior/bevy-td-sandbox/issues/84
