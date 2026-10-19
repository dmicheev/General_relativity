use bevy::prelude::*;
use lift_core::ViewFrame;

use crate::resources::*;

type RootQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<SceneRootMarker>,
        Without<Camera3d>,
        Without<InertialGrid>,
    ),
>;
type CameraQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<Camera3d>,
        Without<SceneRootMarker>,
        Without<InertialGrid>,
    ),
>;
type GhostQuery<'w, 's> = Query<
    'w,
    's,
    &'static Transform,
    (
        With<CabinGhost>,
        Without<SceneRootMarker>,
        Without<Camera3d>,
        Without<InertialGrid>,
    ),
>;

/// Фиксированная внешняя камера (02-lift-physics §3.6):
/// удерживает в кадре область y ∈ [-1, 5] при паузе на t = 1.2 с.
pub const EXTERNAL_CAM_POS: Vec3 = Vec3::new(7.2, 2.6, 7.2);
pub const EXTERNAL_CAM_TARGET: Vec3 = Vec3::new(0.0, 2.1, 0.0);

/// FrameBridge: восстановление внешнего кадра как ОТОБРАЖЕНИЯ.
///
/// Симуляция всегда в системе лифта; во внешнем виде визуальный корень
/// смещается на `D`, физические сущности не трогаются. Инерциальная
/// сетка не является ребёнком корня и потому остаётся на месте,
/// подчёркивая неподвижность внешнего наблюдателя.
pub fn update_frame_bridge(
    sim: Res<SimConfig>,
    // Без Without<…> запросы Transform в одной системе конфликтуют (B0001).
    mut root: RootQuery,
    mut grid: Query<&mut Visibility, With<InertialGrid>>,
    mut camera: CameraQuery,
    ghost: GhostQuery,
) {
    // Дискретное D — позиция фантома кабины, интегрируемого тем же
    // решателем, что и мячи (см. CabinGhost). До первого шага — 0.
    let d = ghost
        .single()
        .map(|t| t.translation.y)
        .unwrap_or(0.0);

    if let Ok(mut transform) = root.single_mut() {
        transform.translation = match sim.view {
            ViewFrame::Lift => Vec3::ZERO,
            ViewFrame::External => Vec3::new(0.0, d, 0.0),
        };
    }

    if let Ok(mut visibility) = grid.single_mut() {
        *visibility = match sim.view {
            ViewFrame::Lift => Visibility::Hidden,
            ViewFrame::External => Visibility::Visible,
        };
    }

    if let Ok(mut transform) = camera.single_mut() {
        if sim.view == ViewFrame::External {
            transform.translation = EXTERNAL_CAM_POS;
            transform.look_at(EXTERNAL_CAM_TARGET, Vec3::Y);
        }
    }
}

/// Копирование поз лифта в визуальные меши мячей (f64-симуляция
/// остаётся в Rapier; меши — только отображение, дети `SceneRoot`).
pub fn sync_ball_visuals(
    balls: Query<(&Ball, &Transform), Without<BallVisual>>,
    mut visuals: Query<(&mut Transform, &BallVisual)>,
) {
    let mut translation = [Vec3::ZERO; 3];
    let mut rotation = [Quat::IDENTITY; 3];
    for (ball, transform) in &balls {
        translation[ball.index] = transform.translation;
        rotation[ball.index] = transform.rotation;
    }
    for (mut transform, visual) in &mut visuals {
        transform.translation = translation[visual.index];
        transform.rotation = rotation[visual.index];
    }
}

/// Трейлы: последние 120 позиций каждого мяча в текущем кадре
/// (02-lift-physics §3.7). Тонкие линии поверх сцены.
/// Скрайт-буферы переиспользуются (T05 §5: без аллокаций в hot path).
pub fn draw_trails(
    mut gizmos: Gizmos,
    sim: Res<SimConfig>,
    balls: Query<(&Ball, &BallTrace)>,
    mut scratch: Local<[Vec<Vec3>; 3]>,
) {
    for (buffer, trace_of) in scratch.iter_mut().zip(&balls) {
        let (ball, trace) = trace_of;
        buffer.clear();
        let samples = match sim.view {
            ViewFrame::Lift => &trace.samples_lift,
            ViewFrame::External => &trace.samples_external,
        };
        buffer.extend(samples.iter().map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)));
        if buffer.len() >= 2 {
            let (r, g, b) = BALL_COLORS[ball.index];
            gizmos.linestrip(buffer.iter().copied(), Color::srgb(r, g, b));
        }
    }
}
