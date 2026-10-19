use bevy::prelude::*;
use std::f32::consts::PI;

use crate::resources::{InertialGrid, SceneRootMarker, SceneRootRef, SimConfig};
use lift_core::ViewFrame;

const CABIN_HALF: f32 = 1.5;
const CABIN_H: f32 = 3.0;
const TARGET: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Resource, Debug)]
pub struct OrbitCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: TARGET,
            yaw: 0.6947,
            pitch: 0.2864,
            distance: 8.142,
        }
    }
}

impl OrbitCamera {
    fn pose(&self) -> (Vec3, Vec3) {
        let pitch_cos = self.pitch.cos();
        let offset = Vec3::new(
            self.yaw.cos() * pitch_cos,
            self.pitch.sin(),
            self.yaw.sin() * pitch_cos,
        ) * self.distance;
        (self.target + offset, self.target)
    }
}

pub fn setup_camera(mut commands: Commands, orbit: Res<OrbitCamera>) {
    let (pos, target) = orbit.pose();
    let mut transform = Transform::from_translation(pos);
    transform.look_at(target, Vec3::Y);
    // Контекст egui привязывается к этой камере явно (T06): см.
    // комментарий к EguiGlobalSettings в lib.rs — авто-привязка попала
    // бы в сущность AmbientLight (в Bevy 0.20 она несёт Camera).
    commands.spawn((Camera3d::default(), transform, bevy_egui::PrimaryEguiContext));
}

/// Орбитальная камера работает только в кадре лифта.
/// Ввод гейтится на egui: вращение/зум не должны срабатывать, когда
/// указатель над панелью или тянется слайдер (T06 §8).
#[allow(clippy::too_many_arguments)] // SystemParam-набор Bevy — норма
pub fn update_orbit_camera(
    sim: Res<SimConfig>,
    time: Res<Time>,
    mut orbits: ResMut<OrbitCamera>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut prev_cursor: Local<Option<Vec2>>,
    mut contexts: bevy_egui::EguiContexts,
) {
    if sim.view != ViewFrame::Lift {
        return;
    }
    // egui использует указатель (слайдер, панель) — камеру не вращаем.
    if contexts
        .ctx_mut()
        .map(|ctx| ctx.egui_wants_pointer_input() || ctx.egui_wants_keyboard_input())
        .unwrap_or(false)
    {
        return;
    }
    let dt = time.delta_secs();
    let rotate_speed = 1.8;

    if keys.pressed(KeyCode::ArrowLeft) {
        orbits.yaw += rotate_speed * dt;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        orbits.yaw -= rotate_speed * dt;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        orbits.pitch = (orbits.pitch + rotate_speed * dt).min(PI / 2.0 - 0.05);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        orbits.pitch = (orbits.pitch - rotate_speed * dt).max(0.05);
    }
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        orbits.distance = (orbits.distance - 4.0 * dt).max(3.0);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        orbits.distance = (orbits.distance + 4.0 * dt).min(20.0);
    }

    if let Ok(window) = windows.single() {
        let cursor = window.cursor_position();
        let dragging = mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right);
        if dragging {
            if let (Some(cur), Some(prev)) = (cursor, *prev_cursor) {
                let delta = cur - prev;
                orbits.yaw -= delta.x * 0.006;
                orbits.pitch = (orbits.pitch + delta.y * 0.006).clamp(0.05, PI / 2.0 - 0.05);
            }
        }
        *prev_cursor = if dragging { cursor } else { None };
    }

    if let Ok(mut transform) = cameras.single_mut() {
        let (pos, target) = orbits.pose();
        transform.translation = pos;
        transform.look_at(target, Vec3::Y);
    }
}

/// Визуальная сцена: корень `SceneRoot` (двигается во внешнем кадре),
/// кабина-стрекало, свет, инерциальная сетка.
///
/// Физические сущности (коллайдеры кабины, мячи) спавнятся отдельно
/// в `systems::physics` и живут в корне мира — см. SceneRootMarker.
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scene_root = commands
        .spawn((SceneRootMarker, Transform::IDENTITY, Visibility::Inherited))
        .id();

    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.75, 0.85, 0.95, 0.15),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let back_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.72, 0.75),
        ..default()
    });
    let frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.28),
        ..default()
    });
    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.75, 0.3, 0.35),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.entity(scene_root).with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.0, 0.1, 3.0))),
            MeshMaterial3d(floor_mat),
            Transform::from_xyz(0.0, -0.05, 0.0),
        ));

        for x in [-1.0_f32, 1.0] {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.1, CABIN_H, 3.0))),
                MeshMaterial3d(wall_mat.clone()),
                Transform::from_xyz(x * (CABIN_HALF + 0.05), CABIN_H / 2.0, 0.0),
            ));
        }

        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(3.1, CABIN_H, 0.1))),
            MeshMaterial3d(back_mat),
            Transform::from_xyz(0.0, CABIN_H / 2.0, -(CABIN_HALF + 0.05)),
        ));

        let t = 0.05;
        let top = CABIN_H - t / 2.0;
        let mut spawn_bar = |parent: &mut ChildSpawnerCommands, size: Vec3, pos: Vec3| {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_translation(pos),
            ));
        };
        for z in [-1.0_f32, 1.0] {
            spawn_bar(
                parent,
                Vec3::new(3.0 + t, t, t),
                Vec3::new(0.0, top, z * (CABIN_HALF + t / 2.0)),
            );
        }
        for x in [-1.0_f32, 1.0] {
            spawn_bar(
                parent,
                Vec3::new(t, t, 3.0 + t),
                Vec3::new(x * (CABIN_HALF + t / 2.0), top, 0.0),
            );
        }
        for x in [-1.0_f32, 1.0] {
            for z in [-1.0_f32, 1.0] {
                spawn_bar(
                    parent,
                    Vec3::new(t, CABIN_H, t),
                    Vec3::new(
                        x * (CABIN_HALF + t / 2.0),
                        CABIN_H / 2.0,
                        z * (CABIN_HALF + t / 2.0),
                    ),
                );
            }
        }
    });

    // Инерциальная референс-сетка на y = 0 — НЕ ребёнок SceneRoot.
    // Видна только во внешнем кадре (frame_bridge переключает Visibility).
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(14.0, 0.02, 14.0))),
            MeshMaterial3d(grid_mat),
            Transform::from_xyz(0.0, -0.01, 0.0),
            InertialGrid,
            Visibility::Hidden,
        ))
        .insert(Name::new("InertialGrid"));

    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));

    commands.spawn(AmbientLight {
        color: Color::WHITE,
        brightness: 400.0,
        ..default()
    });

    commands.insert_resource(SceneRootRef(scene_root));
}
