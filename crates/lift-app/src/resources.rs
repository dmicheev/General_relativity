use std::collections::VecDeque;

use bevy::prelude::*;
use lift_core::{ExperimentClock, FrameParams, Mode, ScaleReading, ViewFrame};

/// Длительность эксперимента (см. 00-physics-spec, авто-пауза).
pub const EXPERIMENT_DURATION: f64 = 3.0;
/// Авто-пауза во внешнем кадре (02-lift-physics §3.6).
pub const EXTERNAL_PAUSE_T: f64 = 1.2;

/// Начальные позиции и массы грузов (02-lift-physics §3.2, 03-scales §3.1).
/// Радиус одинаковый намеренно: различие поведения нельзя списать на размер.
pub const BALL_X: [f64; 3] = [-0.8, 0.0, 0.8];
pub const BALL_MASS: [f64; 3] = [1.0, 5.0, 10.0];
/// Высота сброса грузов (03-scales §3.7: R поднимает на 1.8 и отпускает).
pub const BALL_Y: f64 = 1.8;
pub const BALL_RADIUS: f32 = 0.15;

/// Верх чаши весов (03-scales): панель на колонне, см. scales::mod.
pub const PAN_TOP: f64 = 0.324;
/// Верх колонны основания — точка крепления пружины.
pub const BASE_TOP: f32 = 0.10;
/// Естественная высота пружины (visual scale.y при compression = 0).
pub const SPRING_H0: f32 = 0.20;

/// Цвета мячей A/B/C (голубой / зелёный / оранжевый, 02-lift-physics §3.2).
pub const BALL_COLORS: [(f32, f32, f32); 3] = [
    (0.35, 0.55, 0.95),
    (0.30, 0.75, 0.30),
    (0.95, 0.55, 0.20),
];

/// Ресурс-обёртка над `ExperimentClock` (lift-core остаётся чистым Rust
/// без Bevy — обёртка даёт ему свойство `Resource`).
#[derive(Resource, Default)]
pub struct SimClock(pub ExperimentClock);

impl std::ops::Deref for SimClock {
    type Target = ExperimentClock;

    fn deref(&self) -> &ExperimentClock {
        &self.0
    }
}

impl std::ops::DerefMut for SimClock {
    fn deref_mut(&mut self) -> &mut ExperimentClock {
        &mut self.0
    }
}

/// Число фактически выполненных шагов физики (по одному за fixed-тик
/// при работающем часах). Индекс для `displacement_discrete`.
#[derive(Resource, Default, Debug)]
pub struct SimSteps(pub u64);

#[derive(Resource)]
pub struct SimConfig {
    pub mode: Mode,
    pub frame: FrameParams,
    pub view: ViewFrame,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Planet,
            frame: Mode::Planet.params(),
            view: ViewFrame::Lift,
        }
    }
}

/// Параметры режима `Custom` (UI-слайдеры появятся на этапе 6).
#[derive(Resource)]
pub struct CustomParams {
    pub external_g: f64,
    pub lift_a: f64,
}

impl Default for CustomParams {
    fn default() -> Self {
        Self {
            external_g: 5.0,
            lift_a: 0.0,
        }
    }
}

/// Действия, накопленные системой ввода за кадр и потребляемые `sim_pre`
/// в ближайший fixed-тик. `handle_input` перезаписывает их каждый кадр,
/// поэтому устаревшие флаги не накапливаются.
#[derive(Resource, Default)]
pub struct InputEvents {
    pub mode_reset: Option<Mode>,
    pub mode_live: Option<Mode>,
    pub reset: bool,
    pub toggle_view: bool,
    pub toggle_pause: bool,
    /// Номер сброса сцены (R / смена режима). Наращивается в `sim_pre`;
    /// потребители (например, фотон) сбрасываются по изменению.
    pub reset_seq: u32,
}

/// Маркер физического тела мяча (индекс 0..3 = A/B/C).
#[derive(Component, Clone, Copy, Debug)]
pub struct Ball {
    pub index: usize,
}

/// История позиций мяча в обоих кадрах (02-lift-physics §3.7).
/// Хранится на сущности мяча.
#[derive(Component, Default)]
pub struct BallTrace {
    pub samples_lift: VecDeque<[f64; 3]>,
    pub samples_external: VecDeque<[f64; 3]>,
}

pub const TRACE_LEN: usize = 120;

/// Маркер визуального меша мяча (ребёнок `SceneRoot`).
#[derive(Component, Clone, Copy, Debug)]
pub struct BallVisual {
    pub index: usize,
}

/// Визуальная пружина стойки весов (ребёнок `SceneRoot`).
#[derive(Component, Clone, Copy, Debug)]
pub struct SpringVisual {
    pub stand: usize,
}

/// Визуальный корень сцены. Двигается во внешнем кадре на `D(t)`.
///
/// ОТКЛОНЕНИЕ ОТ §3.5 (зафиксировано): физические сущности НЕ дети
/// `SceneRoot`. В bevy_rapier3d 0.37 смена `GlobalTransform` родителя
/// считается телепортом тела (`apply_rigid_body_user_changes` делает
/// `set_position` по `Changed<GlobalTransform>`), поэтому двигается
/// только визуальный корень, а симуляция всегда в координатах лифта.
#[derive(Component, Debug)]
pub struct SceneRootMarker;

/// Инерциальная референс-сетка на y = 0. Не является ребёнком `SceneRoot`.
#[derive(Component, Debug)]
pub struct InertialGrid;

/// Фантом кабины — невидимое динамическое тело, которое Rapier
/// интегрирует с ускорением `lift_a` (внешняя сила компенсирует `g_eff`).
/// Его позиция и есть дискретное `D(t)`: внешний кадр восстанавливается
/// тем же интегратором, что и мячи, поэтому `p_lift + D` остаётся
/// константой до машинной точности (T02.6).
#[derive(Component, Debug)]
pub struct CabinGhost;

/// Сущность визуального корня, найденная на Startup.
#[derive(Resource, Debug)]
pub struct SceneRootRef(pub Entity);

/// Показания трёх весов (03-scales §3.3). Заполняется системой
/// `read_contact_forces` из ContactForceEvent — не из формулы m·g_eff.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct ScaleReadings {
    pub items: [ScaleReading; 3],
}

/// Отладочные метрики T02.5–T02.7 и T03 (вывод в консоль).
#[derive(Resource, Default)]
pub struct Metrics {
    pub step_count: u64,
    pub max_dy_ab: f64,
    pub max_dy_ac: f64,
    pub contact_seen: bool,
    pub rocket_y_dev: f64,
    pub rocket_logged: bool,
    pub run_hash: u64,
    pub hash_taken: bool,
    pub last_run_hash: u64,
    pub scales_logged: bool,
}

impl Metrics {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// FNV-1a по битам f64 — хеш позиций для проверки детерминизма (T02.7).
pub fn hash_positions(values: &[f64]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for v in values {
        for byte in v.to_bits().to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ball_layout_matches_spec() {
        assert_eq!(BALL_X, [-0.8, 0.0, 0.8]);
        assert_eq!(BALL_MASS, [1.0, 5.0, 10.0]);
        // 03-scales §3.7: сброс груза над чашей с высоты 1.8 м.
        assert_eq!(BALL_Y, 1.8);
        assert_eq!(BALL_RADIUS, 0.15);
    }

    #[test]
    fn fnv1a_reference_vector() {
        assert_eq!(hash_positions(&[]), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash_positions(&[0.0]), hash_positions(&[0.0]));
        assert_ne!(hash_positions(&[0.0]), hash_positions(&[1.0]));
    }

    #[test]
    fn metrics_reset_clears_state() {
        let mut m = Metrics {
            step_count: 10,
            max_dy_ab: 1.0,
            ..Default::default()
        };
        m.reset();
        assert_eq!(m.step_count, 0);
        assert_eq!(m.max_dy_ab, 0.0);
    }
}
