//! The 3D office: floor, desks, coffee machine, board, colleagues, movement,
//! camera, followers.

use crate::{Game, Mode};
use bevy::prelude::*;
use sim::Role;

pub const COFFEE_POS: Vec3 = Vec3::new(0.0, 0.0, 4.5);
pub const BOARD_POS: Vec3 = Vec3::new(0.0, 0.0, -6.0);
pub const TALK_RANGE: f32 = 2.2;
pub const STATION_RANGE: f32 = 3.0;
const WALK_SPEED: f32 = 4.5;
const NPC_SPEED: f32 = 3.2;
const CAM_OFFSET: Vec3 = Vec3::new(0.0, 13.0, 10.0);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_office)
            .add_systems(
                Update,
                (move_player, switch_character, npc_walk).run_if(in_state(Mode::Office)),
            )
            .add_systems(Update, (highlight_active, follow_camera));
    }
}

/// A colleague's body in the world.
#[derive(Component)]
pub struct Colleague {
    pub id: String,
    pub desk: Vec3,
}

/// Where a body stands in idle, the desk chair.
fn chair(desk: Vec3) -> Vec3 {
    desk + Vec3::new(0.0, 0.0, 0.9)
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
struct ActiveRing;

fn role_color(role: Option<Role>) -> Color {
    match role {
        Some(Role::Strategist) => Color::srgb(0.30, 0.55, 0.95),
        Some(Role::Creative) => Color::srgb(0.95, 0.40, 0.35),
        Some(Role::Account) => Color::srgb(0.35, 0.80, 0.50),
        Some(Role::Producer) => Color::srgb(0.95, 0.75, 0.25),
        None => Color::srgb(0.65, 0.65, 0.70),
    }
}

fn spawn_office(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game: Res<Game>,
) {
    // Floor + walls
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(18.0, 0.1, 16.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.78, 0.74, 0.68))),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    let wall = materials.add(Color::srgb(0.92, 0.91, 0.88));
    for (pos, size) in [
        (Vec3::new(0.0, 1.0, -8.0), Vec3::new(18.0, 2.0, 0.2)),
        (Vec3::new(-9.0, 1.0, 0.0), Vec3::new(0.2, 2.0, 16.0)),
        (Vec3::new(9.0, 1.0, 0.0), Vec3::new(0.2, 2.0, 16.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(wall.clone()),
            Transform::from_translation(pos),
        ));
    }

    // Coffee machine
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 1.4, 0.6))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.18, 0.14))),
        Transform::from_translation(COFFEE_POS + Vec3::Y * 0.7),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(1.4, 0.02))),
        MeshMaterial3d(materials.add(Color::srgb(0.55, 0.40, 0.30))),
        Transform::from_translation(COFFEE_POS + Vec3::Y * 0.01),
    ));

    // Brief board
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 0.1))),
        MeshMaterial3d(materials.add(Color::srgb(0.97, 0.97, 0.95))),
        Transform::from_translation(BOARD_POS + Vec3::new(0.0, 1.4, -1.1)),
    ));
    let sticky = [
        Color::srgb(1.0, 0.9, 0.4),
        Color::srgb(0.6, 0.9, 1.0),
        Color::srgb(1.0, 0.6, 0.7),
    ];
    for (row, color) in sticky.into_iter().enumerate() {
        for col in 0..3 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.6, 0.45, 0.02))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_translation(
                    BOARD_POS + Vec3::new(-1.2 + col as f32 * 1.2, 2.0 - row as f32 * 0.6, -1.03),
                ),
            ));
        }
    }

    // Desks and colleagues
    let desk_mat = materials.add(Color::srgb(0.45, 0.33, 0.22));
    let desk_mesh = meshes.add(Cuboid::new(1.6, 0.75, 0.8));
    let body_mesh = meshes.add(Capsule3d::new(0.3, 0.9));
    let head_mesh = meshes.add(Sphere::new(0.22));
    let skin = materials.add(Color::srgb(0.96, 0.82, 0.70));
    for c in &game.sim.content.colleagues {
        let desk = Vec3::new(c.desk.0, 0.0, c.desk.1);
        commands.spawn((
            Mesh3d(desk_mesh.clone()),
            MeshMaterial3d(desk_mat.clone()),
            Transform::from_translation(desk + Vec3::Y * 0.375),
        ));
        commands
            .spawn((
                Colleague { id: c.id.clone(), desk },
                Transform::from_translation(chair(desk)),
                Visibility::default(),
            ))
            .with_children(|b| {
                b.spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(materials.add(role_color(c.role))),
                    Transform::from_xyz(0.0, 0.75, 0.0),
                ));
                b.spawn((
                    Mesh3d(head_mesh.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(0.0, 1.6, 0.0),
                ));
            });
    }

    // Marker under whoever you play
    commands.spawn((
        ActiveRing,
        Mesh3d(meshes.add(Torus::new(0.45, 0.55))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0),
            emissive: LinearRgba::rgb(2.0, 2.0, 2.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.03, 0.0),
    ));

    // Light + camera
    commands.spawn((
        DirectionalLight { illuminance: 9_000.0, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight { brightness: 400.0, ..default() });
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        // Narrow FOV from further away: less vertical-line skew at screen edges.
        Projection::Perspective(PerspectiveProjection { fov: 32f32.to_radians(), ..default() }),
        Transform::from_xyz(0.0, 13.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game: Res<Game>,
    mut q: Query<(&Colleague, &mut Transform)>,
) {
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir == Vec3::ZERO {
        return;
    }
    let dir = dir.normalize();
    for (c, mut t) in &mut q {
        if c.id == game.active {
            t.translation += dir * WALK_SPEED * time.delta_secs();
            t.translation.x = t.translation.x.clamp(-8.5, 8.5);
            t.translation.z = t.translation.z.clamp(-7.5, 7.5);
            let target = t.translation + dir;
            t.look_at(target, Vec3::Y);
        }
    }
}

/// Tab: step into the next playable colleague. Whoever you leave walks back to their desk.
fn switch_character(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    let playable: Vec<String> = game
        .sim
        .content
        .colleagues
        .iter()
        .filter(|c| c.role.is_some())
        .map(|c| c.id.clone())
        .collect();
    let i = playable.iter().position(|id| *id == game.active).unwrap_or(0);
    let next = playable[(i + 1) % playable.len()].clone();
    if game.follower.as_deref() == Some(next.as_str()) {
        game.follower = None;
    }
    game.active = next.clone();
    let role = game.sim.content.colleague(&next).and_then(|c| c.role).map(|r| r.label_pl()).unwrap_or("");
    let name = game.name_of(&next);
    game.say(format!("Jesteś teraz: {name} ({role})."));
}

/// Non-controlled colleagues: follow you if invited for coffee, otherwise go back to the desk.
fn npc_walk(time: Res<Time>, game: Res<Game>, mut q: Query<(&Colleague, &mut Transform)>) {
    let player_pos = q
        .iter()
        .find(|(c, _)| c.id == game.active)
        .map(|(_, t)| t.translation)
        .unwrap_or(Vec3::ZERO);
    for (c, mut t) in &mut q {
        if c.id == game.active {
            continue;
        }
        let (target, stop) = if game.follower.as_deref() == Some(c.id.as_str()) {
            (player_pos, 1.2)
        } else {
            (chair(c.desk), 0.05)
        };
        let to = (target - t.translation).with_y(0.0);
        let d = to.length();
        if d > stop {
            let step = to / d * NPC_SPEED * time.delta_secs();
            t.translation += step.clamp_length_max(d - stop);
            let look = t.translation + to;
            t.look_at(look, Vec3::Y);
        }
    }
}

fn highlight_active(
    game: Res<Game>,
    bodies: Query<(&Colleague, &Transform), Without<ActiveRing>>,
    mut ring: Query<&mut Transform, With<ActiveRing>>,
) {
    let Ok(mut r) = ring.single_mut() else { return };
    if let Some((_, t)) = bodies.iter().find(|(c, _)| c.id == game.active) {
        r.translation = t.translation.with_y(0.03);
    }
}

fn follow_camera(
    time: Res<Time>,
    game: Res<Game>,
    bodies: Query<(&Colleague, &Transform), Without<MainCamera>>,
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut cam) = cam.single_mut() else { return };
    let Some((_, t)) = bodies.iter().find(|(c, _)| c.id == game.active) else { return };
    let focus = t.translation;
    let want = focus + CAM_OFFSET;
    let k = 1.0 - (-6.0 * time.delta_secs()).exp();
    cam.translation = cam.translation.lerp(want, k);
    let look = cam.translation - CAM_OFFSET;
    cam.look_at(look, Vec3::Y);
}
