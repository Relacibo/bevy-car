# Bevy 0.18.x API Guidelines for AI Code Generation

This document provides critical API guidance for Bevy 0.18.x to help avoid using deprecated or removed functionality.

## Current Version
- **Bevy 0.18.1** (March 2026)
- AI models are often trained on Bevy 0.14 or earlier code
- Many patterns have changed significantly

---

## Query Patterns for Unique Entities

### ✅ CORRECT: Use `Single` system parameter
```rust
fn system(window: Single<&Window, With<PrimaryWindow>>) {
    println!("Window size: {:?}", window.resolution);
}

fn camera_system(camera: Single<&Camera, With<MainCamera>>) {
    // Use camera directly
}
```

### ❌ WRONG: Using `Query::single()` or `Query::single_mut()`
```rust
// DISCOURAGED - returns Result, but Single<> is preferred
fn system(query: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = query.single() else { return; };
    // Use window...
}

// DISCOURAGED - prefer Single<> parameter instead
fn system(mut query: Query<&mut Player>) {
    let Ok(mut player) = query.single_mut() else { return; };
    // Use player...
}
```

### ❌ WRONG: Using `Query::get_single()` or `Query::get_single_mut()`
```rust
// DON'T DO THIS - does not compile
fn system(query: Query<&Window, With<PrimaryWindow>>) {
    let window = query.get_single();
}

// DON'T DO THIS - does not compile
fn system(mut query: Query<&mut Player>) {
    let Ok(player) = query.get_single_mut() else { return };
}
```

**For optional single entities**: Use `Option<Single<T>>`

```rust
// When the entity might not exist
fn system(player: Option<Single<&Transform, With<Player>>>) {
    if let Some(player) = player {
        let transform = player.into_inner();
        println!("Player position: {:?}", transform.translation);
    } else {
        // No player found - system still runs
        println!("No player in the game");
    }
}

// Example: Conditional UI updates
fn update_ui(
    camera: Option<Single<&Transform, With<Camera3d>>>, 
    mut ui_text: Single<&mut Text, With<PositionDisplay>>,
) {
    let mut text = ui_text.into_inner();
    
    if let Some(camera) = camera {
        let cam_transform = camera.into_inner();
        **text = format!("Camera at: {:.2?}", cam_transform.translation);
    } else {
        **text = "No camera found".to_string();
    }
}
```

---

## 2D Mesh and Material Spawning

### ✅ CORRECT: Direct component composition
```rust
commands.spawn((
    Mesh2d(meshes.add(Rectangle::default())),
    MeshMaterial2d(materials.add(Color::from(PURPLE))),
    Transform::default().with_scale(Vec3::splat(128.)),
));
```

### ❌ WRONG: Using `MaterialMesh2dBundle`
```rust
// DON'T DO THIS - MaterialMesh2dBundle is deprecated/removed
commands.spawn(MaterialMesh2dBundle {
    mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
    material: materials.add(SomeMaterial::default()),
    ..Default::default()
});
```

**Why**: Bevy moved away from predefined bundles for mesh rendering toward explicit component composition. This gives more flexibility and clarity.

---

## Text Rendering

### ✅ CORRECT: Direct `Text` component (since 0.15+)
```rust
commands.spawn((
    Text::new("Hello World"),
    TextFont {
        font_size: 32.0,
        ..default()
    },
    TextColor(Color::WHITE),
));

// Or with sections:
commands.spawn(Text::from_section(
    "Score: 100",
    TextFont {
        font_size: 24.0,
        ..default()
    }
));
```

### ❌ WRONG: Using `TextBundle`
```rust
// DON'T DO THIS - TextBundle is deprecated since 0.15
commands.spawn(TextBundle {
    text: Text::from_section("Hello World", text_style),
    ..Default::default()
});
```

**Why**: `TextBundle` was deprecated in Bevy 0.15 in favor of "Required Components". Use explicit `Text`, `TextFont`, and `TextColor` components instead.

---

## Query Error Handling

### ✅ CORRECT: Query methods return `Result`
```rust
fn system(query: Query<&Player>) {
    if let Ok(player) = query.get_single() {
        // Handle player
    } else {
        // Handle error case
    }
}
```

### ❌ WRONG: Expecting panics
```rust
// Old behavior - query.single() would panic
// Now returns Result instead
```

**Why**: Since Bevy 0.16, query methods like `get_single()` return `Result` for better error handling.

---

## Entity Relationships & Hierarchy

- **Built-in one-to-many relationships** since 0.16
- Parent-child hierarchy handling has changed
- Despawning entities and adding/removing children works differently
- Always use the current ECS relationship APIs

---

## Events and Observers (0.18+)

### Events are now split into two types:

1. **Messages**: Buffered queue events for system-to-system communication
   - Use `MessageReader` and `MessageWriter` system parameters
   - Double-buffered to ensure reliable delivery

2. **EntityEvent**: Immediate observer-triggered behavior for per-entity logic

### Example:
```rust
fn reader_system(mut messages: MessageReader<MyMessage>) {
    for message in messages.read() {
        // Process message
    }
}

fn writer_system(mut messages: MessageWriter<MyMessage>) {
    messages.write(MyMessage { /* ... */ });
}
```

---

## Asset Handles

### ✅ CORRECT (0.17+): Use `uuid_handle!` macro
```rust
let handle: Handle<Image> = uuid_handle!("some-uuid");
```

### ❌ WRONG: Using `weak_handle!` or `Handle::weak_from_u128()`
```rust
// DON'T DO THIS - deprecated in 0.16+
let handle = Handle::weak_from_u128(some_id);
```

**Why**: Handle weak reference APIs were replaced with UUID-based handles for better type safety.

---

## Component Spawning Best Practices

### ✅ CORRECT: Tuple-based component insertion
```rust
commands.spawn((
    Transform::default(),
    Visibility::default(),
    MyComponent,
));
```

### Custom bundles are still allowed:
```rust
#[derive(Bundle)]
struct MyBundle {
    transform: Transform,
    visibility: Visibility,
}
```

But prefer direct component tuples for clarity unless you need reusable bundles.

---

## Entity Hierarchies and Children

### ✅ PREFERRED: Use `children![]` macro for declarative hierarchies
```rust
commands.spawn((
    Name::new("Parent"),
    Transform::default(),
    children![
        (Name::new("Child1"), Transform::default()),
        (
            Name::new("Child2"),
            Transform::default(),
            children![
                (Name::new("Grandchild"), Transform::default())
            ]
        )
    ]
));
```

**Why**: The `children![]` macro is the most declarative and clean way to define hierarchies at spawn time.

### ✅ ALSO VALID: Use `with_children` only when you need dynamic logic
```rust
// Use this when you need loops, conditions, or runtime decisions
commands.spawn((Name::new("Parent"), Transform::default()))
    .with_children(|parent| {
        for i in 0..count {
            parent.spawn((Name::new(format!("Child{}", i)), Transform::default()));
        }
        
        if some_condition {
            parent.spawn((Name::new("Conditional"), Transform::default()));
        }
    });
```

**When to use `with_children`**:
- You need loops to spawn multiple similar children
- Children depend on runtime conditions
- You need to call methods on child entities immediately after spawning

### ❌ AVOID: Manual child setup
```rust
// This works but is verbose and error-prone
let parent = commands.spawn((Name::new("Parent"), Transform::default())).id();
let child = commands.spawn((Name::new("Child"), Transform::default())).id();
commands.entity(parent).push_children(&[child]);
```

---

## System Organization Best Practices

### ✅ CORRECT: Use run conditions instead of checking inputs in system
```rust
use bevy::prelude::*;

fn toggle_debug_ui() {
    // System logic without input checking
    println!("F3 pressed!");
}

fn main() {
    App::new()
        .add_systems(Update, toggle_debug_ui.run_if(input_just_pressed(KeyCode::F3)))
        .run();
}
```

### ❌ WRONG: Manually checking input in every system
```rust
// Don't do this - pollutes system logic with input checking
fn toggle_debug_ui(input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::F3) {
        println!("F3 pressed!");
    }
}
```

**Why**: Run conditions keep system logic clean and make dependencies explicit.

---

### ✅ CORRECT: Group systems with tuples
```rust
App::new()
    .add_systems(Startup, (setup1, setup2, setup3))
    .add_systems(Update, (system1, system2, system3))
```

### ❌ WRONG: Chaining individual `.add_systems()` calls
```rust
// Verbose and harder to read
App::new()
    .add_systems(Startup, setup1)
    .add_systems(Startup, setup2)
    .add_systems(Update, system1)
    .add_systems(Update, system2)
```

**Why**: Tuples are more concise and show which systems belong together.

---

### ✅ CORRECT: Use `.after()` or `.before()` for ordering
```rust
App::new()
    .add_systems(Update, (
        physics_system,
        render_system.after(physics_system),
    ))
```

### ❌ WRONG: Assuming order from code position
```rust
// This does NOT guarantee execution order!
App::new()
    .add_systems(Update, (physics_system, render_system))
```

**Why**: Systems in tuples run in **parallel by default** unless you explicitly specify ordering with `.after()` or `.before()`.

---

## General Guidelines

1. **Avoid panic-prone APIs**: Prefer `Single` over `Query::single()`
2. **Use explicit components**: Don't rely on deprecated bundles like `MaterialMesh2dBundle`
3. **Handle Results**: Query methods return `Result` types
4. **Check version**: Always verify which Bevy version patterns you're using
5. **Read migration guides**: https://bevy.org/learn/migration-guides/introduction/
6. **Use run conditions**: Keep input checking out of system logic
7. **Group related systems**: Use tuples to organize systems clearly
8. **Explicit ordering**: Use `.after()`/`.before()` when order matters

---

## Common Deprecated/Removed Items

- `MaterialMesh2dBundle` → Use `Mesh2d` + `MeshMaterial2d` components
- `TextBundle` → Use `Text`, `TextFont`, `TextColor` components
- `Query::get_single()` / `Query::get_single_mut()` → **REMOVED** - Use `Single<>` parameter
- `Query::single()` / `Query::single_mut()` → **DISCOURAGED** - Use `Single<>` or handle `Result`
- `Handle::weak_from_u128()` → Use `uuid_handle!` macro
- Old event system → Use `MessageReader`/`MessageWriter` or `EntityEvent`
- `world.iter_entities()` → Use `world.query::<EntityMut>()`
- Manual child entity setup → Prefer `children![]` macro or `with_children`

---

## Resources

- Migration Guides: https://bevy.org/learn/migration-guides/introduction/
- Official Examples: https://bevy.org/examples/
- Release Notes: https://github.com/bevyengine/bevy/releases
- Bevy Cheat Book: https://bevy-cheatbook.github.io/

---

## Rapier Physics (bevy_rapier3d 0.33.0)

### ✅ CORRECT: Plugin initialization
```rust
use bevy_rapier3d::prelude::*;

App::new()
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
    .add_plugins(RapierDebugRenderPlugin::default())
```

### ❌ WRONG: Using `pixels_per_meter`
```rust
// DON'T DO THIS - pixels_per_meter was removed in 0.33
.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
```

**Why**: The `pixels_per_meter` method was removed in bevy_rapier3d 0.33.0. Use `::default()` instead.

---

### RapierContext Access

`RapierContext` is a **SystemParam**, not a `Res` or `Component`.

#### ❌ WRONG: Treating it as Resource or Component
```rust
fn system(rapier: Res<RapierContext>) { }  // Compile error!
fn system(rapier: Single<&RapierContext>) { }  // Compile error!
```

#### ✅ CORRECT: Use ReadRapierContext SystemParam
```rust
fn system(rapier_read: ReadRapierContext) {
    let Ok(rapier_context) = rapier_read.single() else {
        return;
    };
    
    // Now use rapier_context for queries
    if let Some((entity, toi)) = rapier_context.cast_ray(
        origin,
        direction,
        max_distance,
        solid,
        QueryFilter::default(),
    ) {
        println!("Hit entity {:?} at distance {}", entity, toi);
    }
}
```

#### Key Points:
- Use `ReadRapierContext` for read-only physics queries
- Call `.single()` to extract the actual `RapierContext`
- Returns `Result` - handle with `let Ok(...) = ... else { return; }`
- `cast_ray()` returns `Option<(Entity, f32)>` where `f32` is hit distance

#### Raycasting Example:
```rust
fn raycast_system(rapier_read: ReadRapierContext) {
    let Ok(rapier) = rapier_read.single() else { return; };
    
    let ray_pos = Vec3::ZERO;
    let ray_dir = Vec3::X;
    let max_distance = 100.0;
    
    if let Some((entity, toi)) = rapier.cast_ray(
        ray_pos,
        ray_dir,
        max_distance,
        true,  // solid
        QueryFilter::default(),
    ) {
        println!("Raycast hit at distance: {}", toi);
    }
}
```
