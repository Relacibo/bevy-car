use bevy::ecs::relationship::Relationship;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(DebugTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .add_systems(Startup, setup_scene)
        .add_systems(Startup, setup_debug_ui)
        .add_systems(Update, update_sensors)
        .add_systems(
            Update,
            toggle_ai_system.run_if(input_just_pressed(KeyCode::KeyP)),
        )
        .add_systems(Update, ai_parking_system.after(toggle_ai_system))
        .add_systems(Update, car_control_system.after(ai_parking_system))
        .add_systems(Update, update_debug_ui)
        .add_systems(
            Update,
            toggle_debug_ui.run_if(input_just_pressed(KeyCode::F3)),
        )
        .add_systems(Update, debug_sensors)
        .add_systems(Update, draw_sensor_gizmos)
        .run();
}

#[derive(Resource)]
struct DebugTimer(Timer);

#[derive(Component)]
struct DebugUI;

#[derive(Component)]
struct DebugText;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParkingState {
    Reversing,
    Approach,
    Align,
    Enter,
    Adjust,
    Parked,
}

#[derive(Component)]
struct AiDriver {
    enabled: bool,
    decision_timer: Timer,
    state: ParkingState,
    target_angle: f32,
}

#[derive(Component)]
struct ParkingSpot;

#[derive(Component)]
struct PlayerCar;

#[derive(Component)]
struct ParkedCar;

#[derive(Component)]
struct CarControls {
    acceleration: f32,
    max_speed: f32,
    turn_speed: f32,
    brake_force: f32,
}

#[derive(Component)]
struct CarInput {
    throttle: f32, // -1.0 bis 1.0 (negativ = rückwärts)
    steering: f32, // -1.0 bis 1.0 (links/rechts)
    brake: bool,
}

impl Default for CarInput {
    fn default() -> Self {
        Self {
            throttle: 0.0,
            steering: 0.0,
            brake: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DistanceSensorPosition {
    Front,
    Back,
    Left,
    Right,
    FrontLeft,
    FrontRight,
    BackLeft,
    BackRight,
}

#[derive(Component)]
struct DistanceSensor {
    position: DistanceSensorPosition,
    max_range: f32,
    last_distance: f32,
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera - weiter raus für bessere Übersicht
    commands.spawn((
        Camera3d::default(),
        // Kamera etwas weiter nach unten (Süden) und weiter weg für mehr Übersicht
        Transform::from_xyz(0.0, 26.0, 11.0).looking_at(Vec3::new(0.0, 0.0, -4.0), Vec3::Y),
    ));

    // Licht
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.3, 0.0)),
    ));

    // Zaun (Iron Fence) ersetzt die Wand - 7 Segmente, damit er länger ist
    let fence_segment_width = 5.7;
    let top_fence_segments = 7;
    let top_fence_half_length = fence_segment_width * top_fence_segments as f32 / 2.0;
    let top_fence_z = -12.5;

    for i in -(top_fence_segments / 2)..=(top_fence_segments / 2) {
        let x = i as f32 * fence_segment_width;
        commands.spawn((
            SceneRoot(asset_server.load("models/cc0_iron_fence.glb#Scene0")),
            Transform::from_xyz(x, 0.0, top_fence_z)
                .with_scale(Vec3::splat(2.0))
                .with_rotation(Quat::from_rotation_y(0.0)),
        ));
    }

    // Durchgängiger Collider über die komplette obere Zaunlänge
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, top_fence_z),
        Collider::cuboid(top_fence_half_length, 1.76, 0.04),
    ));

    // Seitliche Zäune links/rechts, die an den oberen Zaun anschließen und nach unten laufen
    let side_fence_x = top_fence_half_length;
    let side_start_z = top_fence_z;
    let side_end_z = 15.0; // unteres Ende des Spielbereichs
    let side_segment_spacing = fence_segment_width;

    // Positioniere die seitlichen Zaun-Segmente so, dass ihr oberes Ende genau am oberen Zaun anliegt
    let mut z = side_start_z + side_segment_spacing * 0.5;
    while z < side_end_z + side_segment_spacing * 0.5 {
        // rechte Seite
        commands.spawn((
            SceneRoot(asset_server.load("models/cc0_iron_fence.glb#Scene0")),
            Transform::from_xyz(side_fence_x, 0.0, z)
                .with_scale(Vec3::splat(2.0))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));

        // linke Seite
        commands.spawn((
            SceneRoot(asset_server.load("models/cc0_iron_fence.glb#Scene0")),
            Transform::from_xyz(-side_fence_x, 0.0, z)
                .with_scale(Vec3::splat(2.0))
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        ));

        z += side_segment_spacing;
    }

    // Durchgängige Collider für die seitlichen Zäune
    let side_length = side_end_z - side_start_z;
    let side_center_z = (side_start_z + side_end_z) * 0.5;

    commands.spawn((
        Transform::from_xyz(side_fence_x, 0.0, side_center_z),
        Collider::cuboid(0.04, 1.76, side_length * 0.5),
    ));
    commands.spawn((
        Transform::from_xyz(-side_fence_x, 0.0, side_center_z),
        Collider::cuboid(0.04, 1.76, side_length * 0.5),
    ));

    // Unsichtbare Wand etwas nördlich des unteren Bildschirmrands (nur Collider)
    let bottom_wall_z = 8.0;
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, bottom_wall_z),
        Collider::cuboid(top_fence_half_length, 1.76, 0.04),
    ));

    // Boden (Parkplatz) - Größe exakt an Zaun-Rechteck angepasst
    let ground_width = top_fence_half_length * 2.0;
    let ground_depth = side_end_z - top_fence_z;
    let ground_center_z = (top_fence_z + side_end_z) * 0.5;

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(ground_width, ground_depth)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.35),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, ground_center_z),
        Collider::cuboid(ground_width * 0.5, 0.1, ground_depth * 0.5),
    ));

    // Randstein (Bürgersteig zwischen Parkplätzen und Wand) - mit Collider
    // Länge ist ein Vielfaches der Zaunbreite (7 * 5.7), damit er genau mit dem oberen Zaun endet
    let curb_depth = 2.0;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(top_fence_half_length * 2.0, 0.2, curb_depth))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.7, 0.7),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.1, top_fence_z + curb_depth * 0.5),
        Collider::cuboid(top_fence_half_length, 0.1, curb_depth * 0.5),
    ));

    // Seitliche Randsteine links/rechts, die mit ihrer oberen Kante bündig an die vordere Kante
    // des oberen Randsteins anschließen (nicht nur zentriert)
    let side_curb_start_z = top_fence_z + curb_depth;
    let side_curb_end_z = side_end_z;
    let side_curb_length = side_curb_end_z - side_curb_start_z;
    let side_curb_center_z = (side_curb_start_z + side_curb_end_z) * 0.5;

    // Seitliche Randsteine sollen mit ihrer äußeren Ecke exakt an die rechte/linke Ecke
    // des oberen Randsteins anschließen. Dafür rücken wir sie in X-Richtung um die
    // halbe Randstein-Tiefe nach innen.
    let side_curb_x = side_fence_x - curb_depth * 0.5;
    for &x in &[-side_curb_x, side_curb_x] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(curb_depth, 0.2, side_curb_length))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.7),
                ..default()
            })),
            Transform::from_xyz(x, 0.1, side_curb_center_z),
            Collider::cuboid(curb_depth * 0.5, 0.1, side_curb_length * 0.5),
        ));
    }

    let white_line = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        ..default()
    });

    let red_line = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.1),
        emissive: LinearRgba::rgb(0.3, 0.0, 0.0),
        ..default()
    });

    // Parkplatz-Markierungen (parallel zur Wand, VOR dem Randstein)
    let spot_width = 3.0;
    let spot_depth = 5.0;
    let line_thickness = 0.12;
    let spot_y = -8.0; // Vor dem Randstein

    // 7 Parkplätze nebeneinander - nur rote Umrandung für den mittleren
    for i in -3..=3 {
        let x = i as f32 * (spot_width + 0.3);

        if i == 0 {
            // Roter Rahmen für Ziel-Parkplatz (Mitte)
            // Vordere Linie
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(spot_width, 0.02, line_thickness))),
                MeshMaterial3d(red_line.clone()),
                Transform::from_xyz(x, 0.01, spot_y + spot_depth / 2.0),
            ));

            // Hintere Linie
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(spot_width, 0.02, line_thickness))),
                MeshMaterial3d(red_line.clone()),
                Transform::from_xyz(x, 0.01, spot_y - spot_depth / 2.0),
            ));

            // Linke Linie
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(line_thickness, 0.02, spot_depth))),
                MeshMaterial3d(red_line.clone()),
                Transform::from_xyz(x - spot_width / 2.0, 0.01, spot_y),
            ));

            // Rechte Linie
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(line_thickness, 0.02, spot_depth))),
                MeshMaterial3d(red_line.clone()),
                Transform::from_xyz(x + spot_width / 2.0, 0.01, spot_y),
            ));

            commands.spawn((Transform::from_xyz(x, 0.0, spot_y), ParkingSpot));
        }
    }

    // Weiße Trennlinien ZWISCHEN den Parkplätzen
    for i in -3..=3 {
        let x = (i as f32 + 0.5) * (spot_width + 0.3);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(line_thickness, 0.02, spot_depth))),
            MeshMaterial3d(white_line.clone()),
            Transform::from_xyz(x, 0.01, spot_y),
        ));
    }

    // Geparktes Auto (links) - echtes 3D-Modell mit getrennter Rotation
    commands
        .spawn((
            Transform::from_xyz(-3.3, 0.0, spot_y).with_rotation(Quat::from_rotation_y(0.08)),
            RigidBody::Dynamic,
            Collider::cuboid(0.85, 0.75, 1.9),
            Sleeping::disabled(),
            Damping {
                linear_damping: 2.0,
                angular_damping: 2.0,
            },
            ParkedCar,
        ))
        .with_children(|parent| {
            // Modell als Kind - um 180° gedreht und nach unten verschoben
            parent.spawn((
                SceneRoot(
                    asset_server
                        .load("models/kenney_car-kit/Models/GLB format/sedan-sports.glb#Scene0"),
                ),
                Transform::from_xyz(0.0, -0.45, 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::splat(1.5)),
            ));
        });

    // Spieler-Auto - echtes 3D-Modell mit getrennter Rotation
    commands
        .spawn((
            Transform::from_xyz(-3.5, 0.0, -1.5),
            RigidBody::Dynamic,
            Collider::cuboid(0.9, 0.7, 2.1),
            Sleeping::disabled(),
            Damping {
                linear_damping: 0.3,
                angular_damping: 1.0,
            },
            Friction {
                coefficient: 0.3,
                combine_rule: CoefficientCombineRule::Min,
            },
            ExternalForce::default(),
            ExternalImpulse::default(),
            Velocity::default(),
            LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            PlayerCar,
            AiDriver {
                enabled: true,
                decision_timer: Timer::from_seconds(0.25, TimerMode::Repeating),
                state: ParkingState::Approach,
                target_angle: 0.0,
            },
            CarControls {
                acceleration: 10.0,
                max_speed: 10.0,
                turn_speed: 2.0,
                brake_force: 15.0,
            },
            CarInput::default(),
        ))
        .with_children(|parent| {
            // Modell als Kind - um 180° gedreht damit Motorhaube nach vorne zeigt
            parent.spawn((
                SceneRoot(
                    asset_server.load(
                        "models/kenney_car-kit/Models/GLB format/hatchback-sports.glb#Scene0",
                    ),
                ),
                Transform::from_xyz(0.0, -0.45, 0.0)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
                    .with_scale(Vec3::splat(1.5)),
            ));

            // Sensoren an den Ecken/Seiten des Autos
            // Auto: Breite ~1.8 (collider 0.9*2), Länge ~4.2 (collider 2.1*2)
            let sensor_height = 0.5;
            let car_half_width = 0.9;
            let car_half_length = 2.1;
            let sensor_max_range = 10.0;

            parent.spawn((
                Transform::from_xyz(0.0, sensor_height, -car_half_length)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                DistanceSensor {
                    position: DistanceSensorPosition::Front,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(0.0, sensor_height, car_half_length)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                DistanceSensor {
                    position: DistanceSensorPosition::Back,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(-car_half_width, sensor_height, 0.0).with_rotation(
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
                        * Quat::from_rotation_y(std::f32::consts::PI),
                ),
                DistanceSensor {
                    position: DistanceSensorPosition::Left,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(car_half_width, sensor_height, 0.0).with_rotation(
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
                        * Quat::from_rotation_y(std::f32::consts::PI),
                ),
                DistanceSensor {
                    position: DistanceSensorPosition::Right,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(-car_half_width, sensor_height, -car_half_length)
                    .with_rotation(
                        Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)
                            * Quat::from_rotation_y(std::f32::consts::PI),
                    ),
                DistanceSensor {
                    position: DistanceSensorPosition::FrontLeft,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(car_half_width, sensor_height, -car_half_length).with_rotation(
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)
                        * Quat::from_rotation_y(std::f32::consts::PI),
                ),
                DistanceSensor {
                    position: DistanceSensorPosition::FrontRight,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(-car_half_width, sensor_height, car_half_length).with_rotation(
                    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4 + std::f32::consts::PI)
                        * Quat::from_rotation_y(std::f32::consts::PI),
                ),
                DistanceSensor {
                    position: DistanceSensorPosition::BackLeft,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));

            parent.spawn((
                Transform::from_xyz(car_half_width, sensor_height, car_half_length).with_rotation(
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_4 + std::f32::consts::PI)
                        * Quat::from_rotation_y(std::f32::consts::PI),
                ),
                DistanceSensor {
                    position: DistanceSensorPosition::BackRight,
                    max_range: sensor_max_range,
                    last_distance: sensor_max_range,
                },
            ));
        });

    // Bäume auf dem Randstein (echte 3D-Modelle)
    for x in [-9.0, -6.0, 6.0, 9.0].iter() {
        let z = -11.5;

        commands.spawn((
            SceneRoot(
                asset_server
                    .load("models/kenney_nature-kit/Models/GLTF format/tree_blocks.glb#Scene0"),
            ),
            Transform::from_xyz(*x, 0.0, z).with_scale(Vec3::splat(3.0)),
            // Nur Stamm als Collider, nicht die ganze Krone
            Collider::cylinder(0.5, 0.3),
        ));
    }
}

fn setup_debug_ui(mut commands: Commands) {
    // Debug UI Container
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Visible,
            DebugUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Press F3 to toggle\nPress P for AI"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                DebugText,
            ));
        });
}

fn toggle_debug_ui(debug_ui: Single<&mut Visibility, With<DebugUI>>) {
    let mut visibility = debug_ui.into_inner();
    *visibility = match *visibility {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

fn update_debug_ui(
    car_query: Single<(&Transform, &Velocity, &CarInput, &AiDriver, &Children), With<PlayerCar>>,
    spot_query: Single<&Transform, With<ParkingSpot>>,
    sensor_query: Query<&DistanceSensor>,
    debug_text: Single<&mut Text, With<DebugText>>,
) {
    let (car_transform, velocity, input, ai, children) = car_query.into_inner();
    let mut text = debug_text.into_inner();
    let spot_transform = spot_query.into_inner();

    // Calculate all values needed for debug display
    let car_pos = car_transform.translation;
    let target_pos = spot_transform.translation;
    let distance = (target_pos - car_pos).length();
    let current_speed = velocity.linvel.length();

    // Calculate alignment
    let car_forward = car_transform.forward();
    let car_forward_xz = Vec3::new(car_forward.x, 0.0, car_forward.z).normalize();
    let target_forward_xz = Vec3::new(0.0, 0.0, -1.0);
    let alignment_angle = car_forward_xz.dot(target_forward_xz).acos();

    // Read actual sensor values from sensor entities
    let mut front = 20.0;
    let mut back = 20.0;
    let mut left = 20.0;
    let mut right = 20.0;
    let mut front_left = 20.0;
    let mut front_right = 20.0;
    let mut back_left = 20.0;
    let mut back_right = 20.0;

    for child in children.iter() {
        if let Ok(sensor) = sensor_query.get(child) {
            match sensor.position {
                DistanceSensorPosition::Front => front = sensor.last_distance,
                DistanceSensorPosition::Back => back = sensor.last_distance,
                DistanceSensorPosition::Left => left = sensor.last_distance,
                DistanceSensorPosition::Right => right = sensor.last_distance,
                DistanceSensorPosition::FrontLeft => front_left = sensor.last_distance,
                DistanceSensorPosition::FrontRight => front_right = sensor.last_distance,
                DistanceSensorPosition::BackLeft => back_left = sensor.last_distance,
                DistanceSensorPosition::BackRight => back_right = sensor.last_distance,
            }
        }
    }

    if !ai.enabled {
        **text = format!(
            "=== MANUAL CONTROL ===\n\
            Press P to enable AI parking\n\n\
            Pos: ({:.2}, {:.2})\n\
            Target: ({:.2}, {:.2})\n\
            Dist: {:.2}m\n\
            Speed: {:.2} m/s\n\
            Align: {:.1}°\n\n\
            SENSORS:\n\
            F:{:.2} B:{:.2} L:{:.2} R:{:.2}\n\
            FL:{:.2} FR:{:.2}\n\n\
            CONTROLS: WASD/Arrows\n\
            W/↑: Accelerate  S/↓: Brake/Reverse\n\
            A/←: Left  D/→: Right",
            car_pos.x,
            car_pos.z,
            target_pos.x,
            target_pos.z,
            distance,
            current_speed,
            alignment_angle.to_degrees(),
            front,
            back,
            left,
            right,
            front_left,
            front_right
        );
        return;
    }

    // Determine state
    let state = if distance < 0.4 && alignment_angle < 0.15 && current_speed < 0.3 {
        "PARKED"
    } else if distance < 2.5 {
        "FINAL APPROACH"
    } else if distance < 5.0 {
        "SETUP PHASE"
    } else {
        "NAVIGATION"
    };

    **text = format!(
        "=== AI PARKING DEBUG ===\n\
        State: {}\n\n\
        Pos: ({:.2}, {:.2})\n\
        Target: ({:.2}, {:.2})\n\
        Dist: {:.2}m\n\
        Align: {:.1}°\n\
        Speed: {:.2} m/s\n\n\
        SENSORS:\n\
        F:{:.2}m {} B:{:.2}m {}\n\
        L:{:.2}m {} R:{:.2}m {}\n\
        FL:{:.2}m {} FR:{:.2}m {}\n\n\
        INPUT:\n\
        Thr:{:.2} Str:{:.2} Brk:{}",
        state,
        car_pos.x,
        car_pos.z,
        target_pos.x,
        target_pos.z,
        distance,
        alignment_angle.to_degrees(),
        current_speed,
        front,
        if front > 2.0 { "✅" } else { "⚠️" },
        back,
        if back > 2.0 { "✅" } else { "⚠️" },
        left,
        if left > 1.5 { "✅" } else { "⚠️" },
        right,
        if right > 1.5 { "✅" } else { "⚠️" },
        front_left,
        if front_left > 1.8 { "✅" } else { "⚠️" },
        front_right,
        if front_right > 1.8 { "✅" } else { "⚠️" },
        input.throttle,
        input.steering,
        input.brake
    );
}

fn car_control_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    player: Single<
        (
            &CarControls,
            &mut Velocity,
            &Transform,
            &mut CarInput,
            &mut AiDriver,
        ),
        With<PlayerCar>,
    >,
    time: Res<Time>,
) {
    let (controls, mut velocity, transform, mut input, ai) = player.into_inner();

    let dt = time.delta_secs();

    // Manuelle Steuerung nur wenn AI aus ist (P-Toggle ist jetzt in toggle_ai_system)
    if !ai.enabled {
        input.throttle = 0.0;
        input.steering = 0.0;
        input.brake = false;

        // Vorwärts/Rückwärts (W/S oder Pfeiltasten)
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            input.throttle = 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            input.throttle = -1.0;
        }

        // Bremsen (Leertaste)
        if keyboard.pressed(KeyCode::Space) {
            input.brake = true;
        }

        // Lenken (A/D oder Links/Rechts)
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            input.steering = 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            input.steering = -1.0;
        }
    }

    // Input anwenden mit direkter Geschwindigkeitsänderung (OHNE dt-Multiplikation für stärkere Wirkung)
    if input.throttle.abs() > 0.01 {
        let forward = transform.forward();
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize();

        let accel_multiplier = if input.throttle < 0.0 { 0.7 } else { 1.0 };
        let force = forward_xz * input.throttle * controls.acceleration * accel_multiplier;

        velocity.linvel += force * dt;
    }

    // Bremsen
    if input.brake {
        let current_speed = velocity.linvel.length();
        if current_speed > 0.1 {
            let brake_direction = -velocity.linvel.normalize();
            velocity.linvel += brake_direction * controls.brake_force * dt;
        } else {
            velocity.linvel = Vec3::ZERO;
        }
    }

    // Geschwindigkeit begrenzen
    let current_speed = velocity.linvel.length();
    if current_speed > controls.max_speed {
        velocity.linvel = velocity.linvel.normalize() * controls.max_speed;
    }

    // Anti-Drift: Seitliche Geschwindigkeit dämpfen (wie echte Auto-Reifen)
    if current_speed > 0.1 {
        let forward = transform.forward();
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let right = transform.right();
        let right_xz = Vec3::new(right.x, 0.0, right.z).normalize();

        // Geschwindigkeit in Vorwärts- und Seitwärts-Komponenten zerlegen
        let forward_speed = velocity.linvel.dot(forward_xz);
        let lateral_speed = velocity.linvel.dot(right_xz);

        // Seitliche Geschwindigkeit stark dämpfen (Grip-Effekt)
        let grip_factor = 0.95;
        let damped_lateral = lateral_speed * (1.0 - grip_factor);

        // Neue Geschwindigkeit: Vorwärts bleibt, Seitwärts gedämpft
        velocity.linvel = forward_xz * forward_speed + right_xz * damped_lateral;
    }

    // Lenken nur bei Bewegung (Geschwindigkeitsabhängig)
    if current_speed > 0.5 && input.steering.abs() > 0.01 {
        // Lenkung basiert auf aktueller Geschwindigkeit
        let turn_factor = (current_speed / controls.max_speed).min(1.0);

        // Prüfe ob wir vorwärts oder rückwärts fahren
        let forward = transform.forward();
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let is_moving_forward = velocity.linvel.dot(forward_xz) > 0.0;

        // Bei Rückwärtsfahrt Lenkung umkehren
        let steering = if is_moving_forward {
            input.steering * controls.turn_speed
        } else {
            -input.steering * controls.turn_speed
        };

        velocity.angvel.y = steering * turn_factor;
    } else {
        velocity.angvel.y = 0.0;
    }
}

fn update_sensors(
    car_entity: Single<Entity, With<PlayerCar>>,
    car_transform: Single<&Transform, With<PlayerCar>>,
    mut sensors: Query<(&mut DistanceSensor, &GlobalTransform, &ChildOf)>,
    rapier_context: ReadRapierContext,
) {
    let car_entity = *car_entity;
    let car_pos = car_transform.translation;

    let Ok(ctx) = rapier_context.single() else {
        return;
    };

    // Vorwärts- und Rechtsrichtung des Autos im XZ-Plane
    let car_forward = car_transform.forward();
    let mut car_forward_xz = Vec3::new(car_forward.x, 0.0, car_forward.z);
    if car_forward_xz.length_squared() == 0.0 {
        return;
    }
    car_forward_xz = car_forward_xz.normalize();

    let car_right = car_transform.right();
    let mut car_right_xz = Vec3::new(car_right.x, 0.0, car_right.z);
    if car_right_xz.length_squared() == 0.0 {
        return;
    }
    car_right_xz = car_right_xz.normalize();

    for (mut sensor, global_transform, parent) in sensors.iter_mut() {
        // Nur Sensoren des Spieler-Autos aktualisieren
        if parent.get() != car_entity {
            continue;
        }

        let sensor_pos = global_transform.translation();

        // Richtung abhängig von der Sensor-Position bestimmen
        let sensor_dir = match sensor.position {
            DistanceSensorPosition::Front => car_forward_xz,
            DistanceSensorPosition::Back => -car_forward_xz,
            DistanceSensorPosition::Left => -car_right_xz,
            DistanceSensorPosition::Right => car_right_xz,
            DistanceSensorPosition::FrontLeft => (car_forward_xz - car_right_xz).normalize(),
            DistanceSensorPosition::FrontRight => (car_forward_xz + car_right_xz).normalize(),
            DistanceSensorPosition::BackLeft => (-car_forward_xz - car_right_xz).normalize(),
            DistanceSensorPosition::BackRight => (-car_forward_xz + car_right_xz).normalize(),
        };

        // Raycast mit Ausschluss des eigenen Autos
        let filter = QueryFilter::default().exclude_rigid_body(car_entity);

        if let Some((_, toi)) = ctx.cast_ray(sensor_pos, sensor_dir, sensor.max_range, true, filter)
        {
            sensor.last_distance = toi;
        } else {
            sensor.last_distance = sensor.max_range;
        }
    }
}

fn debug_sensors(
    mut timer: ResMut<DebugTimer>,
    time: Res<Time>,
    player: Single<(&Transform, &Velocity, &AiDriver, &CarInput), With<CarControls>>,
    car_entity: Single<Entity, With<PlayerCar>>,
    sensors: Query<(&DistanceSensor, &ChildOf)>,
) {
    // Timer ticken lassen
    timer.0.tick(time.delta());

    if !timer.0.just_finished() {
        return;
    }

    let (car_transform, velocity, ai, input) = player.into_inner();
    let car_entity = *car_entity;

    // Sensor-Werte sammeln
    let mut front = 10.0;
    let mut back = 10.0;
    let mut left = 10.0;
    let mut right = 10.0;
    let mut front_left = 10.0;
    let mut front_right = 10.0;
    let mut back_left = 10.0;
    let mut back_right = 10.0;

    for (sensor, parent) in sensors.iter() {
        if parent.get() != car_entity {
            continue;
        }

        match sensor.position {
            DistanceSensorPosition::Front => front = sensor.last_distance,
            DistanceSensorPosition::Back => back = sensor.last_distance,
            DistanceSensorPosition::Left => left = sensor.last_distance,
            DistanceSensorPosition::Right => right = sensor.last_distance,
            DistanceSensorPosition::FrontLeft => front_left = sensor.last_distance,
            DistanceSensorPosition::FrontRight => front_right = sensor.last_distance,
            DistanceSensorPosition::BackLeft => back_left = sensor.last_distance,
            DistanceSensorPosition::BackRight => back_right = sensor.last_distance,
        }
    }

    let car_pos = car_transform.translation;
    let car_forward = car_transform.forward();
    let car_rotation = car_transform
        .rotation
        .to_euler(EulerRot::YXZ)
        .0
        .to_degrees();

    let sensor_data = serde_json::json!({
        "timestamp": time.elapsed_secs(),
        "ai_enabled": ai.enabled,
        "car_position": {
            "x": car_pos.x,
            "y": car_pos.y,
            "z": car_pos.z
        },
        "car_rotation_y_deg": car_rotation,
        "car_forward": {
            "x": car_forward.x,
            "z": car_forward.z
        },
        "car_speed": velocity.linvel.length(),
        "velocity": {
            "x": velocity.linvel.x,
            "z": velocity.linvel.z
        },
        "ai_input": {
            "throttle": input.throttle,
            "steering": input.steering,
            "brake": input.brake
        },
        "sensors": {
            "front": front,
            "back": back,
            "left": left,
            "right": right,
            "front_left": front_left,
            "front_right": front_right,
            "back_left": back_left,
            "back_right": back_right
        }
    });

    println!("{}", serde_json::to_string_pretty(&sensor_data).unwrap());
}

fn ai_parking_system(
    car_query: Single<(Entity, &Transform, &Velocity, &mut AiDriver), With<PlayerCar>>,
    input_query: Single<&mut CarInput, With<PlayerCar>>,
    spot_query: Single<&Transform, With<ParkingSpot>>,
    sensors: Query<(&DistanceSensor, &ChildOf)>,
) {
    const SAFE_DISTANCE: f32 = 1.5;
    let (car_entity, car_transform, velocity, mut ai) = car_query.into_inner();

    if !ai.enabled {
        return;
    }

    let mut input = input_query.into_inner();

    let spot_transform = spot_query.into_inner();
    let target_pos = spot_transform.translation;
    let car_pos = car_transform.translation;

    // Sensor-Werte auslesen
    let mut front = 10.0;
    let mut back = 10.0;
    let mut left = 10.0;
    let mut right = 10.0;
    let mut front_left = 10.0;
    let mut front_right = 10.0;
    let mut back_left = 10.0;
    let mut back_right = 10.0;

    for (sensor, parent) in sensors.iter() {
        if parent.get() != car_entity {
            continue;
        }

        match sensor.position {
            DistanceSensorPosition::Front => front = sensor.last_distance,
            DistanceSensorPosition::Back => back = sensor.last_distance,
            DistanceSensorPosition::Left => left = sensor.last_distance,
            DistanceSensorPosition::Right => right = sensor.last_distance,
            DistanceSensorPosition::FrontLeft => front_left = sensor.last_distance,
            DistanceSensorPosition::FrontRight => front_right = sensor.last_distance,
            DistanceSensorPosition::BackLeft => back_left = sensor.last_distance,
            DistanceSensorPosition::BackRight => back_right = sensor.last_distance,
        }
    }

    let to_target = target_pos - car_pos;
    let distance = (Vec2::new(to_target.x, to_target.z)).length();
    let current_speed = velocity.linvel.length();

    // Auto-Vorwärtsrichtung
    let car_forward = car_transform.forward();
    let car_forward_xz = Vec3::new(car_forward.x, 0.0, car_forward.z).normalize();

    // Ziel-Ausrichtung (parallel zum Zaun = -Z-Richtung)
    let target_forward = Vec3::NEG_Z;
    let alignment_angle = car_forward_xz.dot(target_forward).acos();

    // Steering helper - bestimmt Richtung zum Ziel
    let car_right = car_transform.right();
    let car_right_xz = Vec3::new(car_right.x, 0.0, car_right.z).normalize();
    let side_offset = to_target.normalize().dot(car_right_xz);

    // Dynamisches Throttle basierend auf Sensor-Distanz
    let calculate_safe_throttle = |sensor_distance: f32, base_throttle: f32| -> f32 {
        if sensor_distance < 1.0 {
            base_throttle * 0.3 // Sehr langsam
        } else if sensor_distance < 2.0 {
            base_throttle * 0.6 // Mittel
        } else {
            base_throttle // Voll
        }
    };

    // State Machine Logik - Forward Parking
    match ai.state {
        ParkingState::Reversing => {
            // Phase 0: Rückwärts fahren, um Platz zu schaffen
            // Prüfe nur BACK-Sensoren (da wir rückwärts fahren)
            let min_back_distance = back.min(back_left).min(back_right);

            if min_back_distance < 1.5 {
                // Kein Platz mehr hinten - stopp und wechsel zu Approach
                ai.state = ParkingState::Approach;
                info!(
                    "AI: Reversing -> Approach (back obstacle at {:.2}m)",
                    min_back_distance
                );
                input.throttle = 0.0;
                input.brake = true;
            } else if front > 4.5 {
                // Genug Platz geschaffen - wechsel zu Approach
                ai.state = ParkingState::Approach;
                info!(
                    "AI: Reversing -> Approach (enough space created, front={:.2}m)",
                    front
                );
                input.throttle = 0.0;
                input.brake = true;
            } else {
                // Weiter rückwärts - dynamisches Throttle basierend auf hinteren Sensoren
                let base_reverse_throttle = -0.7;
                input.throttle = calculate_safe_throttle(min_back_distance, base_reverse_throttle);
                input.steering = 0.0;
                input.brake = false;
            }
        }

        ParkingState::Approach => {
            // Phase 1: Fahre in Richtung Parkplatz
            // Prüfe nur FRONT-Sensoren (da wir vorwärts fahren)
            let min_front_distance = front.min(front_left).min(front_right);

            if min_front_distance < 2.0 && back > 3.0 {
                // Zu nah vorne, frei hinten - erstmal rückwärts
                ai.state = ParkingState::Reversing;
                info!(
                    "AI: Approach -> Reversing (front blocked at {:.2}m, creating space)",
                    min_front_distance
                );
                let min_back_distance = back.min(back_left).min(back_right);
                let base_reverse_throttle = -0.7;
                input.throttle = calculate_safe_throttle(min_back_distance, base_reverse_throttle);
                input.steering = 0.0;
                input.brake = false;
            } else if min_front_distance < 2.5 {
                // Zu nah - wechsle zu Align
                ai.state = ParkingState::Align;
                info!(
                    "AI: Approach -> Align (obstacle at {:.2}m)",
                    min_front_distance
                );
                input.throttle = 0.0;
                input.brake = true;
            } else if distance > 6.0 {
                // Weit weg - fahre mit dynamischem Throttle
                let base_throttle = 0.5;
                input.throttle = calculate_safe_throttle(min_front_distance, base_throttle);
                input.steering = (side_offset * 0.7).clamp(-1.0, 1.0);
                input.brake = false;
            } else {
                // Nahe genug - wechsle zu Align
                ai.state = ParkingState::Align;
                info!(
                    "AI: Approach -> Align (close enough, distance={:.2}m)",
                    distance
                );
            }
        }

        ParkingState::Align => {
            // Phase 2: Richte Auto parallel zum Parkplatz aus
            let is_aligned = alignment_angle < 0.08; // ~4.5 Grad Toleranz
            let min_front_distance = front.min(front_left).min(front_right);

            if is_aligned {
                ai.state = ParkingState::Enter;
                info!(
                    "AI: Align -> Enter (aligned, angle={:.2}rad)",
                    alignment_angle
                );
            } else if min_front_distance < 1.5 {
                // Zu nah am Hindernis - stopp
                input.throttle = 0.0;
                input.brake = true;
                input.steering = 0.0;
                info!(
                    "AI: Align paused (front obstacle at {:.2}m)",
                    min_front_distance
                );
            } else {
                // Drehe das Auto zur richtigen Ausrichtung
                let cross = car_forward_xz.cross(target_forward).y;
                input.steering = if cross > 0.0 { 1.0 } else { -1.0 };

                // Langsam vorwärts während des Drehens - dynamisches Throttle
                let base_throttle = 0.3;
                input.throttle = calculate_safe_throttle(min_front_distance, base_throttle);
                input.brake = false;
            }
        }

        ParkingState::Enter => {
            // Phase 3: Fahre vorwärts in den Parkplatz
            let lateral_offset = car_pos.x - target_pos.x;
            let z_distance = (car_pos.z - target_pos.z).abs();
            let min_front_distance = front.min(front_left).min(front_right);

            if z_distance < 0.5 && lateral_offset.abs() < 0.3 {
                // Nah genug - zur Feinabstimmung
                ai.state = ParkingState::Adjust;
                info!("AI: Enter -> Adjust (close to target)");
            } else if min_front_distance < 1.0 {
                // Zu nah - stopp!
                input.throttle = 0.0;
                input.brake = true;
                ai.state = ParkingState::Adjust;
                info!(
                    "AI: Enter -> Adjust (front obstacle at {:.2}m)",
                    min_front_distance
                );
            } else {
                // Fahre vorwärts und korrigiere seitlich - dynamisches Throttle
                let base_throttle = 0.4;
                input.throttle = calculate_safe_throttle(min_front_distance, base_throttle);
                input.brake = false;

                // Lenke, um zu zentrieren
                if lateral_offset.abs() > 0.1 {
                    input.steering = (-lateral_offset * 0.5).clamp(-1.0, 1.0);
                } else {
                    input.steering = 0.0;
                }
            }
        }

        ParkingState::Adjust => {
            // Phase 4: Feinabstimmung
            let lateral_offset = car_pos.x - target_pos.x;
            let z_distance = (car_pos.z - target_pos.z).abs();
            let is_centered = lateral_offset.abs() < 0.15;
            let is_at_target = z_distance < 0.3;
            let is_aligned = alignment_angle < 0.05;
            let is_stopped = current_speed < 0.05;

            if is_centered && is_at_target && is_aligned && is_stopped {
                ai.state = ParkingState::Parked;
                info!("AI: Adjust -> Parked! SUCCESS!");
            } else {
                // Sehr langsame Korrektur
                if !is_centered {
                    input.throttle = 0.15;
                    input.steering = (-lateral_offset * 0.8).clamp(-0.5, 0.5);
                    input.brake = false;
                } else if !is_at_target && front > 0.5 {
                    input.throttle = 0.2;
                    input.steering = 0.0;
                    input.brake = false;
                } else {
                    input.throttle = 0.0;
                    input.brake = true;
                    input.steering = 0.0;
                }
            }
        }

        ParkingState::Parked => {
            // Phase 5: Erfolgreich geparkt!
            input.throttle = 0.0;
            input.steering = 0.0;
            input.brake = true;
        }
    }

    // SICHERHEITSPRÜFUNG: Nur stoppen wenn SCHNELL fahrend UND Kollision droht
    let current_speed = velocity.linvel.length();
    if current_speed > 1.0 {
        if input.throttle > 0.0 && (front < 0.8 || front_left < 0.8 || front_right < 0.8) {
            input.throttle = 0.0;
            input.brake = true;
            input.steering = 0.0;
            info!(
                "AI: EMERGENCY BRAKE - Moving forward into obstacle! (speed: {:.2}, front: {:.2}, fl: {:.2}, fr: {:.2})",
                current_speed, front, front_left, front_right
            );
        }

        if input.throttle < 0.0 && (back < 0.8 || back_left < 0.8 || back_right < 0.8) {
            input.throttle = 0.0;
            input.brake = true;
            input.steering = 0.0;
            info!(
                "AI: EMERGENCY BRAKE - Moving backward into obstacle! (speed: {:.2})",
                current_speed
            );
        }
    }
}

fn toggle_ai_system(ai_query: Single<&mut AiDriver, With<PlayerCar>>) {
    info!("P key pressed - toggling AI");
    let mut ai = ai_query.into_inner();
    ai.enabled = !ai.enabled;

    if ai.enabled {
        info!("AI Parking: ENABLED");
    } else {
        info!("Manual Control: ENABLED");
    }
}

fn draw_sensor_gizmos(
    car_transform: Single<&Transform, With<PlayerCar>>,
    sensors: Query<(&GlobalTransform, &DistanceSensor)>,
    mut gizmos: Gizmos,
) {
    // Vorwärts- und Rechtsrichtung des Autos im XZ-Plane
    let car_forward = car_transform.forward();
    let mut car_forward_xz = Vec3::new(car_forward.x, 0.0, car_forward.z);
    if car_forward_xz.length_squared() == 0.0 {
        return;
    }
    car_forward_xz = car_forward_xz.normalize();

    let car_right = car_transform.right();
    let mut car_right_xz = Vec3::new(car_right.x, 0.0, car_right.z);
    if car_right_xz.length_squared() == 0.0 {
        return;
    }
    car_right_xz = car_right_xz.normalize();

    for (global_transform, sensor) in sensors.iter() {
        let start_pos = global_transform.translation();

        // Gleiche Richtungslogik wie in `update_sensors`
        let direction = match sensor.position {
            DistanceSensorPosition::Front => car_forward_xz,
            DistanceSensorPosition::Back => -car_forward_xz,
            DistanceSensorPosition::Left => -car_right_xz,
            DistanceSensorPosition::Right => car_right_xz,
            DistanceSensorPosition::FrontLeft => (car_forward_xz - car_right_xz).normalize(),
            DistanceSensorPosition::FrontRight => (car_forward_xz + car_right_xz).normalize(),
            DistanceSensorPosition::BackLeft => (-car_forward_xz - car_right_xz).normalize(),
            DistanceSensorPosition::BackRight => (-car_forward_xz + car_right_xz).normalize(),
        };

        // Bestimme Farbe basierend auf Distanz
        let base_color = if sensor.last_distance < 2.0 {
            LinearRgba::RED
        } else if sensor.last_distance < 4.0 {
            LinearRgba::rgb(1.0, 1.0, 0.0) // Yellow
        } else {
            LinearRgba::GREEN
        };

        // Zeichne Startpunkt
        gizmos.sphere(start_pos, 0.15, base_color);

        // Zeichne Strahl mit Fading
        let actual_distance = sensor.last_distance.min(sensor.max_range);
        let segments = 10;
        for i in 0..segments {
            let t1 = i as f32 / segments as f32;
            let t2 = (i + 1) as f32 / segments as f32;
            let p1 = start_pos + direction * actual_distance * t1;
            let p2 = start_pos + direction * actual_distance * t2;

            // Fade out as we go further from car (fade to fully transparent)
            let alpha = 1.0 - t1;
            let color = LinearRgba::new(base_color.red, base_color.green, base_color.blue, alpha);

            gizmos.line(p1, p2, color);
        }

        // Zeichne Endpunkt wenn etwas getroffen wurde
        if sensor.last_distance < sensor.max_range {
            let end_pos = start_pos + direction * actual_distance;
            gizmos.sphere(end_pos, 0.2, LinearRgba::RED);
        }
    }
}
