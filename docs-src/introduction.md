# Bevy TD Sandbox Manual

This is the manual for **[Bevy TD Sandbox](https://github.com/n8behavior/bevy-td-sandbox)** — a post-apocalyptic tower defense game built in Rust with Bevy.

The manual is the practical reference: how to add things to the game, what each piece does, and how to put pieces together. Right now that means **towers**. Maps and enemies are planned next.

## What's in this manual

- **[Modding](modding/index.md)** — how to extend the game with your own content.
  - **[Towers](modding/towers/index.md)** — author towers in Lua. Drop a recipe file in, and it shows up in the build menu. Includes a quick start, a full atom reference, and worked examples for every built-in tower plus stretch designs.

## Where the design rationale lives

This manual tells you *how* to use the system. The *why* behind the design — the role/atom model, the runtime pipeline, the open questions still being chewed on — lives in the project's design brainstorm:

- [`TOWER_EDITOR_BRAINSTORM.md`](https://github.com/n8behavior/bevy-td-sandbox/blob/main/TOWER_EDITOR_BRAINSTORM.md)

Read the brainstorm if you want to understand or push back on the underlying decisions. Read this manual if you want to ship a tower.

## Building this manual locally

```bash
just book        # build to ./book/
just book-serve  # serve with live reload
```

Requires [`mdbook`](https://rust-lang.github.io/mdBook/) — install with `cargo install mdbook`.
