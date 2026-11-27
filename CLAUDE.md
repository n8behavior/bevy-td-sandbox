# Bevy TD Sandbox - AI Assistant Reference

## Project Overview

Post-apocalyptic tower defense game. Open-field mazing, scavenging economy, wave system.

## Tech Stack

- **Bevy 0.20.1** (`2d` + `ui` feature sets, plus `bevy_audio` + `wav`)
- **bevy_northstar 0.8.0** (grid-based A* pathfinding)
- **rand 0.10**

## CRITICAL: Bevy 0.20 API Patterns

**DO NOT** use patterns from older Bevy versions. The following are verified from source.

### Required Components (NOT Bundles)

Bundles are deprecated. Use `#[require(...)]` on Component derive:

```rust
#[derive(Component)]
#[require(Sprite, Transform)]
struct Tower;
```

### Spawning Sprites

```rust
// Colored rectangle (our placeholder art)
commands.spawn(Sprite::from_color(Color::srgb(0.8, 0.2, 0.2), Vec2::new(20.0, 20.0)));

// Sprite automatically requires Transform, Visibility, VisibilityClass, Anchor
```

### Camera

```rust
commands.spawn(Camera2d);
// Camera, OrthographicProjection, Frustum, Transform all auto-added
```

### States & SubStates

```rust
#[derive(States, Default, PartialEq, Eq, Hash, Debug, Clone)]
enum GameState {
    #[default]
    MainMenu,
    Playing,
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
enum PlayPhase {
    #[default]
    Building,
    Defending,
}

// In app setup:
app.init_state::<GameState>();
app.add_sub_state::<PlayPhase>();
```

### Auto-Despawn on State Exit

```rust
commands.spawn((
    Sprite::from_color(Color::WHITE, Vec2::splat(10.0)),
    DespawnOnExit(GameState::Playing),
));
// NOT StateScoped -- that doesn't exist
```

`NextState::set()` re-runs the transition (and these despawns) even when the
target equals the current state. Use `set_if_different()` to skip that
(the 0.19 name `set_if_neq` no longer exists).

### Events vs Messages

Both exist with different purposes:

**Events** -- Observer-based, immediate, for reactive triggers (NO registration needed):
```rust
#[derive(Event)]
struct EnemyDied { position: Vec2, loot_value: u32 }

// Trigger (no add_event needed! observers auto-register)
commands.trigger(EnemyDied { position, loot_value });

// Register observer
world.add_observer(|trigger: On<EnemyDied>| { /* immediate */ });
```

**Lifecycle observers** put the component inside the event type:
```rust
app.add_observer(|add: On<Add<Tower>>| { /* add.entity */ });  // NOT On<Add, Tower>
```

**EntityEvent** -- Observer event targeted at a specific entity:
```rust
#[derive(EntityEvent)]
struct DamageEvent { amount: f32 }
```

**Messages** -- Buffered, poll-based, for batched per-frame processing:
```rust
#[derive(Message)]
struct GridChanged;

// Must register: app.add_message::<GridChanged>();  (NOT add_event!)

fn write_change(mut writer: MessageWriter<GridChanged>) {
    writer.write(GridChanged);
}

fn read_changes(mut reader: MessageReader<GridChanged>) {
    for _change in reader.read() { /* process */ }
}
```

### Timer API

```rust
timer.tick(time.delta());
timer.is_finished()     // true if elapsed >= duration
timer.just_finished()   // true only on the tick it finishes
timer.elapsed_secs()    // elapsed time as f32
// NO timer.finished() method -- use is_finished()
```

### Tilemaps

Bevy ships `TilemapChunk` (in `bevy_sprite_render`), but this project renders
its grid with individual sprites.

### Text

`TextFont::font_size` is a `FontSize`, not an `f32`:

```rust
TextFont { font_size: FontSize::Px(18.0), ..default() }
```

`TextFont::default()` is `FontSize::Rem(1.)` (20px at the default `RemSize`).

### UI Buttons

`bevy::ui::Button` and `Interaction` are deprecated. Use the headless widget,
add `Hovered` yourself, and react to `Activate`:

```rust
use bevy::picking::hover::Hovered;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};

commands.spawn((Button, ActivateOnPress, Hovered::default(), Node::default()));
app.add_observer(|activate: On<Activate>| { /* activate.entity */ });
// Styling: read `Hovered::get()` and `Has<bevy::ui::Pressed>`.
```

Without `ActivateOnPress`, `Activate` fires on release (a completed click).

### Shaders are WESL

Custom shaders are `.wesl` files (naga_oil `#import` is gone):

```wgsl
import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: MyMaterial;
```

### Resources Are Entities

`#[derive(Resource)]` also implements `Component`, so a type can't be both.
Broad queries (`Query<Entity>`, `Query<()>`, `Query<Option<&T>>`, `EntityRef`)
also match resource entities. Always add a `With<...>` filter (or
`Without<IsResource>`).

`Assets::get_mut` returns an `AssetMut` guard: bind it with `if let Some(mut a) = ...`.

### Cargo Features

`2d` no longer implies `ui` or `audio`; this project enables `ui`,
`bevy_audio` and `wav` explicitly.

## bevy_northstar 0.8 API

### Grid Setup

```rust
let settings = GridSettingsBuilder::new_2d(width, height)
    .chunk_size(8)
    .build();
let grid = CardinalGrid::new(&settings);
commands.spawn(grid);
```

### Cell Navigation

```rust
grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
grid.set_nav(UVec3::new(x, y, 0), Nav::Impassable);
grid.build(); // REQUIRED after modifications
```

### Agent Entities

```rust
commands.spawn((
    AgentPos(UVec3::new(x, y, 0)),
    AgentOfGrid(grid_entity),
    Pathfind::new_2d(goal_x, goal_y),
));
// Plugin auto-inserts NextPos component with next step
```

### Movement Pattern

```rust
fn move_agent(
    mut query: Query<(Entity, &mut AgentPos, &NextPos, &mut Transform)>,
    mut commands: Commands,
) {
    for (entity, mut agent_pos, next_pos, mut transform) in &mut query {
        agent_pos.0 = next_pos.0;
        transform.translation = grid_to_world(next_pos.0);
        commands.entity(entity).remove::<NextPos>();
    }
}
```

### Direct Path Validation (for tower placement)

```rust
let result = grid.pathfind(&mut PathfindArgs::new(start, goal).astar());
if result.is_some() { /* path exists */ }

// Also available:
grid.is_path_viable(start, goal) // -> bool
```

### Type Aliases

- `CardinalGrid` = `Grid<CardinalNeighborhood>` (4-dir)
- `OrdinalGrid` = `Grid<OrdinalNeighborhood>` (8-dir)

## Tuning data

A type's tuning numbers (health, speed, loot, ability values) live in a RON
file next to its module, not in Rust constants: `src/enemy/<name>/stats.ron`.
Which components a type has stays in Rust. The machinery is `src/tuning.rs`.

- **Hot reload:** with `just run` from the repository root, save the file and
  the game picks up the new values (`embedded_watcher`, `native` feature).
  Web and release builds use the values compiled in.
- **A reload changes only new spawns.** Code reads `Tuned<T>` when it spawns
  something; entities already on the field keep their values.
- **Errors:** a mistyped field is an error (`deny_unknown_fields`), and so is
  a value that `validate()` rejects. A bad file on reload is logged with its
  path, line and column, and the old values stay.

To add a file for a new type:

1. Write `stats.ron` next to the module.
2. Add the struct: `#[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]`
   and `#[serde(deny_unknown_fields)]`, and implement `TuningData::validate`
   (`require_positive` for health, speed, cooldowns and ranges).
3. In the plugin's `build`: `embedded_tuning!(app, MyStats, "stats.ron");`.
   It embeds the file and adds `TuningPlugin::<MyStats>`.
4. Read the values where the type spawns, with a deferred entity command, so
   wave scaling (an `EnemySpawned` observer) still runs after it:
   ```rust
   cmds.queue(|mut entity: EntityWorldMut| {
       let stats = entity.resource::<Tuned<MyStats>>().0.clone();
       entity.insert(Health { current: stats.health, max: stats.health });
   });
   ```
5. Test that the file parses (`tuning::parse`) and that a spawn matches it.
   Tests that load assets use `test_app_with_assets()` (the watcher is off);
   `enemy_app()` and `spawn_enemy()` in `test_helpers.rs` set up enemies.

## Build Commands

```bash
just setup           # bootstrap a fresh clone (system libs, toolchain, git hooks, web tools)
just run             # run the game (dynamic linking for fast recompiles)
just check           # fast compile check
just clippy          # lint check (MUST pass with zero warnings before every commit)
just test            # run all tests (unit, integration, doctests)
just ci              # full CI check (format + clippy + tests)
just doc             # browse local API docs
just web-serve       # local web testing in browser (port 4000)
just release patch   # flight-check and create GitHub release; patch|minor|major (see RELEASING.md)
```

## Zero Warnings Policy

**Both `cargo check` and `cargo clippy` must produce zero warnings before every commit.**

- `clippy::too_many_arguments` and `clippy::type_complexity` are allowed crate-wide in `main.rs` (standard Bevy practice -- ECS systems use dependency injection)
- All other warnings must be fixed, not suppressed
- Remove unused code rather than keeping it "for later"

## Local Docs

After `cargo doc`, authoritative API docs are at:
- `target/doc/bevy/index.html`
- `target/doc/bevy_northstar/index.html`

When uncertain about ANY API, read these docs -- do not guess from training data.
