# Bevy TD Sandbox

A post-apocalyptic tower defense game built with Rust and Bevy.

## Prerequisites

### Rust

Install via [rustup](https://rustup.rs/).

### System Dependencies

**macOS** — No extra packages needed. Xcode Command Line Tools are sufficient (`xcode-select --install`).

**Windows** — No extra packages needed. The Visual Studio C++ Build Tools are required, but `rustup` prompts you to install them during Rust setup.

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev
```

## Build & Run

```bash
cargo run
```

## Development

```bash
cargo check    # fast compile check
cargo doc      # build local API docs
```
