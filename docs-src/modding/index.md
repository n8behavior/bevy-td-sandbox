# Modding Overview

Bevy TD Sandbox is built to be modded. The game's content — towers, and eventually maps and enemies — is authored in **Lua** and loaded at runtime. You don't need to recompile the game (or know any Rust) to add a new tower; you write a small Lua file, drop it in a folder, and the game picks it up.

## Why Lua

- **Small and friendly.** Lua's syntax is forgiving and compact. You can read most recipes top-to-bottom without referring to a manual.
- **Familiar to modders.** A lot of game-modding ecosystems already use Lua (Factorio, Garry's Mod, World of Warcraft, Love2D, Defold, …). If you've modded a game before, this won't feel foreign.
- **Real language.** Beyond simple data, you can compute, parameterize, and share helpers between recipes when you want to. See [Advanced Patterns](towers/advanced.md).
- **Sandboxed.** Recipes run in a restricted Lua environment. Filesystem and network access are off; only `string`, `math`, and `table` from the stdlib are available. Recipes you download from someone else can't read your files or call out to the internet.

## What you can mod today

- **[Towers](towers/index.md)** — fully supported. Compose triggers, acquirers, deliverers, payloads, and modifiers from the atom catalog.

## What's planned

- **Maps** — author paths, terrain, and waves.
- **Enemies** — author enemy types with their own component-and-atom model.
- **Shareable mod packs** — bundle recipes (and eventually maps and enemies) into one redistributable folder.

These will follow the same general shape as tower modding: a Lua surface, a sandboxed runtime, an atom-style catalog, and a manual section explaining the role of each piece. Until they're ready, this section of the manual is tower-only.
