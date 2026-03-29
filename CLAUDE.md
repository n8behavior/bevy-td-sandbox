# Bevy TD Sandbox - AI Assistant Reference

## Project Overview

Post-apocalyptic tower defense game. Open-field mazing, scavenging economy, wave system.

## Tech Stack

- **Bevy 0.18.1** (2D feature set)
- **bevy_northstar 0.6.1** (grid-based A* pathfinding)
- **rand 0.9**

## CRITICAL: Bevy 0.18 API Patterns

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

### No TilemapChunk

Bevy 0.18 does NOT have built-in tilemap support. Render grids with individual sprites or a mesh.

## bevy_northstar 0.6.1 API

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

## Build Commands

```bash
cargo check          # fast compile check
cargo run            # run the game
cargo doc --open     # browse local API docs
```

## Local Docs

After `cargo doc`, authoritative API docs are at:
- `target/doc/bevy/index.html`
- `target/doc/bevy_northstar/index.html`

When uncertain about ANY API, read these docs -- do not guess from training data.
