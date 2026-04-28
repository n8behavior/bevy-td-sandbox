# Bevy TD Sandbox

![Scrap Defence](cover-image.png)

A post-apocalyptic tower defense game built with Rust and Bevy.

**[Play in your browser on itch.io](https://n8behavior.itch.io/bevy-td-sandbox)** | **[Gameplay Guide](GAMEPLAY.md)**

## Prerequisites

### Rust

Install via [rustup](https://rustup.rs/).

### System Dependencies

**macOS** — Xcode Command Line Tools (`xcode-select --install`).

**Windows** — Visual Studio C++ Build Tools (`rustup` prompts you to install them during Rust setup).

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev
```

**Linux (Fedora):**

```bash
sudo dnf install -y wayland-devel libxkbcommon-devel alsa-lib-devel systemd-devel
```

**Linux (Arch):**

```bash
sudo pacman -S wayland libxkbcommon alsa-lib systemd
```

## Build & Run

```bash
just run       # run the game (dynamic linking for fast recompiles)
just test      # run all tests (unit, integration, doctests)
just ci        # full local CI check (format + clippy + tests)
```

All dev workflows are in the [`justfile`](justfile). Run `just` to see
all available recipes. Install [just](https://github.com/casey/just) with
`cargo install just`.

<details>
<summary>Raw cargo commands (without just)</summary>

```bash
cargo run -F dynamic   # run with dynamic linking
cargo run              # run without dynamic linking (slower recompile)
cargo check            # fast compile check
cargo clippy           # lint
cargo test --workspace # all tests
cargo fmt              # auto-format
cargo doc --open       # local API docs
```

</details>

## Manual

The modding and reference manual lives in [`docs-src/`](docs-src/) and is
published to **[GitHub Pages](https://n8behavior.github.io/bevy-td-sandbox/)**
on every push to `main`. Build and serve locally with:

```bash
just book        # build to ./book/
just book-serve  # serve with live reload
```

Requires [`mdbook`](https://rust-lang.github.io/mdBook/) — install with
`cargo install mdbook`.

## Development

### Feature flags

| Feature    | Contents                                     | When active               |
|------------|----------------------------------------------|---------------------------|
| `native`   | `web` + file/embedded watchers (hot reload) | **Default** — desktop dev |
| `web`      | Dev tools, UI debug, location tracking       | Web dev builds            |
| `dynamic`  | `bevy/dynamic_linking` (shared lib linking)  | Opt-in via `just run`     |
| *(none)*   | Bare dependencies, static linking            | Release, web release      |

`just run` automatically adds `-F dynamic` for fast incremental
recompiles. `just test` uses the defaults (no dynamic linking) so
doctests work correctly.

### Build profiles

| Profile        | Purpose                                       | Key settings                          |
|----------------|-----------------------------------------------|---------------------------------------|
| `dev`          | Local development                             | `opt-level = 1`, deps at `opt-level = 3` |
| `release`      | Native release                                | LTO thin, 1 codegen unit             |
| `web-release`  | WASM release (`bevy build --release web`)     | `opt-level = "s"`, strip debuginfo    |

### Tracing & debugging

Bevy uses the `tracing` crate for logging. Set `RUST_LOG` to see game
diagnostics:

```bash
RUST_LOG=bevy_td_sandbox=debug just run                 # all game diagnostics
RUST_LOG=bevy_td_sandbox::tower=debug just run           # tower placement only
RUST_LOG=bevy_td_sandbox::enemy=debug just run           # enemy spawns & movement
RUST_LOG=bevy_td_sandbox::pathfinding=warn just run      # pathfinding failures & stuck enemies
RUST_LOG=debug just run                                  # everything (very noisy)
```

### Web / WASM builds

The game compiles to WASM for browser play. Web builds require the
`wasm32-unknown-unknown` target and special RUSTFLAGS for `getrandom`.
The [Bevy CLI](https://github.com/TheBevyFlock/bevy_cli) handles this
automatically:

```bash
just web               # dev web build
just web-release       # optimized web bundle
```

The `[package.metadata.bevy_cli.*]` sections in `Cargo.toml` configure
feature flags for each target automatically.

### CI pipeline

A single GitHub Actions workflow (`.github/workflows/ci.yaml`) runs on
every push to `main` and on pull requests. Four parallel jobs:

| Job           | What it checks                                           |
|---------------|----------------------------------------------------------|
| **Format**    | `cargo fmt --check`                                      |
| **Clippy**    | `cargo clippy --all-targets --all-features` (zero warnings) |
| **Test**      | `cargo test --workspace` (unit, integration, doctests)   |
| **Check web** | `cargo check --no-default-features --target wasm32-unknown-unknown` |

Run `just ci` locally before pushing to catch issues early.

### Release pipeline

`.github/workflows/release.yaml` triggers on version tags (`v*.*.*`) or
manual dispatch. It builds a WASM web bundle via `bevy build --release web`,
uploads it as a GitHub release artifact, and optionally pushes to itch.io.
