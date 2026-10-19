//! Свет: аналитический фотон (04-light). Rapier для фотона НЕ
//! используется — у света нет массы, это геометрия, а не тело.
//!
//! КООРДИНАТНАЯ ДИСЦИПЛИНА (04-light §3.6 — зафиксировано):
//! фотон не является ребёнком `SceneRoot` и рисуется в МИРОВЫХ
//! координатах выбранного кадра ровно один раз:
//!   ViewFrame::Lift     → мир == кадр лифта (SceneRoot на нуле),
//!                         рисуем `photon_lift`.
//!   ViewFrame::External → мир == внешний кадр (SceneRoot сдвинут на
//!                         D), рисуем `photon_external`.
//! Двойное применение D(t) невозможно по построению: точки уже
//! посчитаны в координатах целевого кадра.

pub mod geometry;
pub mod material;

use bevy::prelude::*;
use lift_core::{photon_external, photon_impact, photon_lift, PhotonImpact, PhotonLaunch};

use crate::systems::input::PhotonEvents;
use crate::resources::{InputEvents, SimClock, SimConfig};
use geometry::{sample_path_into, PathKind, GHOST_SEGMENTS, TRAIL_SEGMENTS};
use material::{COLOR_GHOST, COLOR_HEAD, COLOR_IMPACT, COLOR_TRAIL};

/// Правая стена кабины (мишень луча).
pub const X_WALL: f64 = 1.5;
/// Эмиттер на левой стене (04-light §3.1).
pub const EMITTER: [f64; 3] = [-1.45, 1.8, 0.0];
/// Педагогическая скорость света по умолчанию (04-light §3.2).
pub const C_VIS_DEFAULT: f64 = 10.0;
pub const C_VIS_MIN: f64 = 5.0;
pub const C_VIS_MAX: f64 = 30.0;
/// Истинная скорость света — только для подписи коэффициента замедления.
pub const C_REAL: f64 = 299_792_458.0;

/// Состояние луча (04-light §3.3).
#[derive(Resource)]
pub struct PhotonState {
    pub launch: PhotonLaunch,
    pub active: bool,
    pub t_since_shot: f64,
    pub head_external: [f64; 3],
    pub head_lift: [f64; 3],
    pub impact_external: Option<[f64; 3]>,
    pub impact_lift: Option<[f64; 3]>,
    /// Аналитическое попадание (T, точки в обоих кадрах).
    pub impact: Option<PhotonImpact>,
    pub ghost_visible: bool,
    pub impact_logged: bool,
    pub last_reset_seq: u32,
}

impl Default for PhotonState {
    fn default() -> Self {
        Self {
            launch: PhotonLaunch {
                origin: EMITTER,
                c: C_VIS_DEFAULT,
            },
            active: false,
            t_since_shot: 0.0,
            head_external: EMITTER,
            head_lift: EMITTER,
            impact_external: None,
            impact_lift: None,
            impact: None,
            ghost_visible: true,
            impact_logged: false,
            last_reset_seq: 0,
        }
    }
}

fn clear_shot(state: &mut PhotonState) {
    state.active = false;
    state.t_since_shot = 0.0;
    state.head_external = state.launch.origin;
    state.head_lift = state.launch.origin;
    state.impact_external = None;
    state.impact_lift = None;
    state.impact = None;
    state.impact_logged = false;
}

/// Обновление луча: аналитика, без Rapier (04-light §3.4).
pub fn update_photon(
    mut events: ResMut<PhotonEvents>,
    mut state: ResMut<PhotonState>,
    input: Res<InputEvents>,
    sim: Res<SimConfig>,
    clock: Res<SimClock>,
    mut last_reset: Local<u32>,
) {
    // R / смена режима — луч убирается вместе со сбросом сцены.
    if input.reset_seq != *last_reset {
        *last_reset = input.reset_seq;
        clear_shot(&mut state);
    }
    if events.clear {
        clear_shot(&mut state);
    }
    if events.toggle_ghost {
        state.ghost_visible = !state.ghost_visible;
    }
    if events.c_vis_delta != 0 {
        state.launch.c = (state.launch.c + events.c_vis_delta as f64)
            .clamp(C_VIS_MIN, C_VIS_MAX);
        bevy::log::info!(
            "[T04] c_vis = {:.1} м/с (замедление света ×{:.1e} относительно \
             истинной c — педагогический приём)",
            state.launch.c,
            C_REAL / state.launch.c
        );
        // Точка попадания аналитическая — пересчитывается мгновенно.
        state.impact = photon_impact(&state.launch, &sim.frame, X_WALL);
    }
    if events.shot {
        state.active = true;
        state.t_since_shot = 0.0;
        state.impact_external = None;
        state.impact_lift = None;
        state.impact = photon_impact(&state.launch, &sim.frame, X_WALL);
        state.impact_logged = false;
            bevy::log::info!(
                "[T04] выстрел: c_vis = {:.1} м/с (замедление ×{:.1e}), режим {:?}",
                state.launch.c,
                C_REAL / state.launch.c,
                sim.mode
            );
    }
    // События потребляются однократно (в headless-тестах input-системы
    // нет — иначе выстрел срабатывал бы каждый тик).
    *events = PhotonEvents::default();

    // Аналитика не интегрируется шагом: t — чистый счётчик тиков,
    // позиции — формулы (независимость от FPS, §3.4).
    if !state.active || !clock.running {
        return;
    }
    state.t_since_shot += clock.dt;
    state.head_external = photon_external(state.t_since_shot, &state.launch, sim.frame.external_g);
    state.head_lift = photon_lift(state.t_since_shot, &state.launch, &sim.frame);

    if state.head_external[0] >= X_WALL {
        let impact = state
            .impact
            .unwrap_or_else(|| photon_impact(&state.launch, &sim.frame, X_WALL).expect("c > 0"));
        state.impact = Some(impact);
        state.impact_external = Some(impact.external);
        state.impact_lift = Some(impact.lift);
        state.active = false;

        if !state.impact_logged {
            state.impact_logged = true;
            let dy = state.launch.origin[1] - impact.lift[1];
            let note = if dy.abs() < 1e-9 {
                "0.000 м ← свет не искривляется в падающей кабине".to_string()
            } else {
                format!("{dy:.3} м")
            };
            bevy::log::info!(
                "[T04] попадание: Δy = {}, g_eff = {:.2} м/с², c_vis = {:.1} м/с, T = {:.3} с \
                 (ньютонова эвристика; точный Риндлер — lift_core::rindler_null_y)",
                note,
                sim.frame.g_eff(),
                state.launch.c,
                impact.t
            );
        }
    }
}

/// Отрисовка: ghost path, активный след, голова, маркер попадания.
/// См. комментарий координатной дисциплины в начале файла.
pub fn draw_photon(
    mut gizmos: Gizmos,
    state: Res<PhotonState>,
    sim: Res<SimConfig>,
    mut scratch: Local<Vec<[f64; 3]>>,
) {
    let frame = sim.frame;
    let kind = match sim.view {
        lift_core::ViewFrame::Lift => PathKind::Lift,
        lift_core::ViewFrame::External => PathKind::External,
    };
    let to_vec3 = |p: [f64; 3]| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
    let color = |c: (f32, f32, f32)| Color::srgb(c.0, c.1, c.2);

    let Some(impact) = state.impact else {
        return;
    };

    // 1. Ghost path — вся предсказанная траектория, тусклая.
    //    Скрайт-буфер переиспользуется (T05 §5: без аллокаций в hot path).
    if state.ghost_visible {
        sample_path_into(
            &state.launch,
            &frame,
            kind,
            0.0,
            impact.t,
            GHOST_SEGMENTS,
            &mut scratch,
        );
        gizmos.linestrip(scratch.iter().map(|p| to_vec3(*p)), color(COLOR_GHOST));
    }

    // 2. Active trail — от эмиттера до головы (или до стены).
    let t_head = if state.active {
        state.t_since_shot
    } else {
        impact.t
    };
    if t_head > 0.0 {
        sample_path_into(&state.launch, &frame, kind, 0.0, t_head, TRAIL_SEGMENTS, &mut scratch);
        gizmos.linestrip(scratch.iter().map(|p| to_vec3(*p)), color(COLOR_TRAIL));
    }

    // 3. Head — маленькая сфера.
    if state.active {
        let head = match kind {
            PathKind::Lift => state.head_lift,
            PathKind::External => state.head_external,
        };
        gizmos.sphere(to_vec3(head), 0.025, color(COLOR_HEAD));
    }

    // 4. Impact marker — кольцо на правой стене.
    if let Some(p) = match kind {
        PathKind::Lift => state.impact_lift,
        PathKind::External => state.impact_external,
    } {
        let iso = bevy::math::Isometry3d::new(
            to_vec3(p),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        );
        gizmos.circle(iso, 0.05, color(COLOR_IMPACT));
    }
}
