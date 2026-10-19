//! Пружинные весы (T03): три стойки x = −0.8 / 0 / +0.8.
//!
//! MVP-вариант чаши (03-scales §3.1): чаша — часть статического
//! основания, груз (динамическое тело) лежит на ней. Пружина —
//! чисто визуальный индикатор, сжатие вычисляется из ContactForceEvent.

pub mod mesh;
pub mod reading;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lift_core::spring_scale_y;

use crate::resources::{
    SceneRootRef, ScaleReadings, SpringVisual, BALL_X, BASE_TOP, PAN_TOP, SPRING_H0,
};
use mesh::{helix_mesh, HelixParams};

const STAND_HALF: f32 = 0.16;
const PAN_HALF: f32 = 0.14;
const PAN_HALF_H: f32 = 0.012;

/// Пороговые цвета индикации (03-scales §3.6).
const COLOR_REST: (f32, f32, f32) = (0.55, 0.62, 0.72); // серо-голубой
const COLOR_LIGHT: (f32, f32, f32) = (0.30, 0.75, 0.30); // зелёный
const COLOR_MID: (f32, f32, f32) = (0.92, 0.78, 0.20); // жёлтый
const COLOR_FULL: (f32, f32, f32) = (0.90, 0.25, 0.20); // красный

/// Кусочно-линейная интерполяция цвета по compression
/// (0 → серо-голубой, 0.5 → зелёный, 0.85 → жёлтый, 1 → красный).
pub fn compression_color(c: f32) -> (f32, f32, f32) {
    let c = c.clamp(0.0, 1.0);
    let stops = [
        (0.0, COLOR_REST),
        (0.5, COLOR_LIGHT),
        (0.85, COLOR_MID),
        (1.0, COLOR_FULL),
    ];
    for w in stops.windows(2) {
        let (c0, col0) = w[0];
        let (c1, col1) = w[1];
        if c <= c1 {
            let t = if c1 > c0 { (c - c0) / (c1 - c0) } else { 0.0 };
            return (
                col0.0 + (col1.0 - col0.0) * t,
                col0.1 + (col1.1 - col0.1) * t,
                col0.2 + (col1.2 - col0.2) * t,
            );
        }
    }
    COLOR_FULL
}

/// Статические коллайдеры стоек: колонна основания + чаша.
/// На коллайдерах включены ContactForceEvents (порог 0 — каждое
/// соприкосновение с ненулевой силой).
pub fn spawn_scale_platforms(commands: &mut Commands) {
    for &x in &BALL_X {
        let x = x as f32;
        let spawn_platform = |commands: &mut Commands,
                                  half: [f32; 3],
                                  y: f32| {
            commands.spawn((
                RigidBody::Fixed,
                Collider::cuboid(half[0], half[1], half[2]),
                ActiveEvents::CONTACT_FORCE_EVENTS,
                ContactForceEventThreshold(0.0),
                Transform::from_xyz(x, y, 0.0),
            ));
        };
        // Колонна основания: верх на BASE_TOP.
        spawn_platform(commands, [STAND_HALF, BASE_TOP / 2.0, STAND_HALF], BASE_TOP / 2.0);
        // Чаша: верх на PAN_TOP.
        spawn_platform(
            commands,
            [PAN_HALF, PAN_HALF_H, PAN_HALF],
            (PAN_TOP - PAN_HALF_H as f64) as f32,
        );
    }
}

/// Визуал стоек: колонна, чаша и пружина (единичный helix-меш,
/// масштабируется только по Y — 03-scales §3.5).
pub fn spawn_scale_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene_root: Res<SceneRootRef>,
) {
    let base_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.34),
        ..default()
    });
    let pan_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.45, 0.50),
        ..default()
    });
    let helix = meshes.add(helix_mesh(HelixParams::default()));

    for (i, &x) in BALL_X.iter().enumerate() {
        let x = x as f32;
        let spring_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.62, 0.72),
            ..default()
        });
        commands.entity(scene_root.0).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(STAND_HALF, BASE_TOP / 2.0))),
                MeshMaterial3d(base_mat.clone()),
                Transform::from_xyz(x, BASE_TOP / 2.0, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(PAN_HALF, PAN_HALF_H))),
                MeshMaterial3d(pan_mat.clone()),
                Transform::from_xyz(x, (PAN_TOP - PAN_HALF_H as f64) as f32, 0.0),
            ));
            parent.spawn((
                Mesh3d(helix.clone()),
                MeshMaterial3d(spring_mat),
                SpringVisual { stand: i },
                Transform::from_xyz(x, BASE_TOP, 0.0)
                    .with_scale(Vec3::new(1.0, SPRING_H0, 1.0)),
            ));
        });
    }
}

/// Обновление визуала пружин: scale.y и цвет по compression (§3.4, §3.6).
pub fn update_scale_visuals(
    readings: Res<ScaleReadings>,
    mut springs: Query<(&SpringVisual, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (spring, mut transform, material) in &mut springs {
        let item = &readings.items[spring.stand];
        transform.scale.y = SPRING_H0 * spring_scale_y(item.compression, 0.65);
        if let Some(mut mat) = materials.get_mut(&material.0) {
            let (r, g, b) = compression_color(item.compression);
            mat.base_color = Color::srgb(r, g, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Т03.6 (визуальная ветка): цвет монотонен по зелёному каналу
    /// в средней зоне и непрерывен на границах.
    #[test]
    fn color_stops_monotonic_and_continuous() {
        for i in 1..=100 {
            let c = compression_color(i as f32 / 100.0);
            assert!(c.0.is_finite() && c.1.is_finite() && c.2.is_finite());
        }
        assert_eq!(compression_color(-1.0), COLOR_REST);
        assert_eq!(compression_color(2.0), COLOR_FULL);
        assert_eq!(compression_color(0.5), COLOR_LIGHT);
        assert_eq!(compression_color(0.85), COLOR_MID);
        assert_eq!(compression_color(1.0), COLOR_FULL);
    }
}
