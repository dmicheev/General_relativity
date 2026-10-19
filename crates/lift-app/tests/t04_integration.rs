//! Интеграционные проверки T04 (нативно, headless) — фотон.
//!
//! Аналитика не зависит от шага, поэтому T04.11 гоняет один и тот же
//! выстрел при «60 Гц» и «120 Гц» (два fixed-тика на update) и требует
//! побитово одинаковой точки попадания. T04.12 стреляет 100 раз.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_rapier3d::plugin::NoUserData;
use bevy_rapier3d::prelude::*;

use lift_app::light::{update_photon, PhotonState, X_WALL};
use lift_app::systems::input::PhotonEvents;
use lift_app::resources::{
    CustomParams, InputEvents, Metrics, SimClock, SimConfig, SimSteps,
};
use lift_app::systems::physics::{sim_gate, sim_post, sim_pre, spawn_balls, spawn_cabin_colliders};
use lift_core::{photon_impact, ExperimentClock, Mode};

fn make_app(mode: Mode, display_hz: f64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .init_resource::<Assets<Mesh>>()
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / display_hz,
        )))
        .insert_resource(TimestepMode::Fixed {
            dt: 1.0 / 60.0,
            substeps: 1,
        })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule())
        .insert_resource(SimClock(ExperimentClock::new(3.0)))
        .init_resource::<SimSteps>()
        .insert_resource(SimConfig {
            mode,
            ..Default::default()
        })
        .insert_resource(CustomParams::default())
        .insert_resource(InputEvents::default())
        .init_resource::<Metrics>()
        .init_resource::<PhotonState>()
        .init_resource::<PhotonEvents>()
        .init_resource::<lift_app::ui::UiEvents>()
        .add_systems(
            Startup,
            (
                |mut commands: Commands| spawn_cabin_colliders(&mut commands),
                |mut commands: Commands| spawn_balls(&mut commands),
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                sim_pre.before(PhysicsSet::SyncBackend),
                sim_gate
                    .after(sim_pre)
                    .after(PhysicsSet::SyncBackend)
                    .before(PhysicsSet::StepSimulation),
                sim_post.after(PhysicsSet::Writeback),
                update_photon
                    .after(sim_pre)
                    .after(sim_post),
            ),
        );
    app
}

fn shoot(app: &mut App) {
    app.world_mut().resource_mut::<PhotonEvents>().shot = true;
}

/// Выстрелить и дождаться попадания; вернуть аналитическую точку.
fn shoot_and_wait(app: &mut App, updates: usize) -> [f64; 3] {
    shoot(app);
    for _ in 0..updates {
        app.update();
    }
    let state = app.world().resource::<PhotonState>();
    assert!(!state.active, "луч должен был долететь");
    state
        .impact_lift
        .expect("точка попадания должна существовать")
}

/// Аналитическое попадание из lift-core — эталон для всего ниже.
fn expected_impact(mode: Mode) -> [f64; 3] {
    photon_impact(
        &lift_core::PhotonLaunch {
            origin: lift_app::light::EMITTER,
            c: lift_app::light::C_VIS_DEFAULT,
        },
        &mode.params(),
        X_WALL,
    )
    .unwrap()
    .lift
}

/// T04.11: точка попадания не зависит от частоты кадров (60 vs 120 Hz)
/// — побитовое равенство и совпадение с аналитикой.
#[test]
fn t04_11_impact_independent_of_frame_rate() {
    let mut at_60 = make_app(Mode::Planet, 60.0);
    let impact_60 = shoot_and_wait(&mut at_60, 40);

    let mut at_120 = make_app(Mode::Planet, 120.0);
    let impact_120 = shoot_and_wait(&mut at_120, 40);

    let expected = expected_impact(Mode::Planet);
    for k in 0..3 {
        assert_eq!(impact_60[k].to_bits(), impact_120[k].to_bits());
        assert_eq!(impact_60[k].to_bits(), expected[k].to_bits());
    }
}

/// Все четыре режима: внешняя/кабинная формы луча (§1) — через точки
/// попадания: Planet/Rocket имеют провал по y, FreeFall/Inertial — нет.
#[test]
fn t04_mode_table_impact_heights() {
    for (mode, bends) in [
        (Mode::Planet, true),
        (Mode::Rocket, true),
        (Mode::FreeFall, false),
        (Mode::Inertial, false),
    ] {
        let mut app = make_app(mode, 60.0);
        let impact = shoot_and_wait(&mut app, 40);
        let expected = expected_impact(mode);
        for k in 0..3 {
            assert_eq!(impact[k].to_bits(), expected[k].to_bits());
        }
        let dy = 1.8 - impact[1];
        assert_eq!(dy > 1e-9, bends, "{mode:?}: Δy = {dy}");
        // FreeFall/Inertial: Δy = 0.000 побитово.
        if !bends {
            assert_eq!(impact[1].to_bits(), 1.8_f64.to_bits());
        }
    }
}

/// T04.12: 100 последовательных выстрелов без накопительного дрейфа
/// (точка попадания побитово та же).
#[test]
fn t04_12_100_shots_no_drift() {
    let mut app = make_app(Mode::Planet, 60.0);
    let expected = expected_impact(Mode::Planet);
    for shot in 0..100 {
        // Часы эксперимента 3 с = 180 тиков — короче 100 выстрелов;
        // перед каждым выстрелом перезапускаем (как R в приложении).
        app.world_mut().resource_mut::<SimClock>().reset();
        shoot(&mut app);
        for _ in 0..30 {
            app.update();
        }
        let state = app.world().resource::<PhotonState>();
        if shot < 99 {
            // Новый выстрел на следующей итерации сбрасывает луч.
            continue;
        }
        let impact = state.impact_lift.expect("попадание");
        for k in 0..3 {
            assert_eq!(impact[k].to_bits(), expected[k].to_bits(), "shot {shot}");
        }
    }
}

/// c_vis меняется на лету: ghost/попадание пересчитываются
/// аналитически (T = W/c).
#[test]
fn t04_c_vis_change_recomputes_impact() {
    let mut app = make_app(Mode::Planet, 60.0);
    app.world_mut().resource_mut::<PhotonEvents>().c_vis_delta = 10; // 10 → 20
    // Первый update не даёт fixed-тика (нулевая дельта времени) — два.
    app.update();
    app.update();

    let state = app.world().resource::<PhotonState>();
    assert!((state.launch.c - 20.0).abs() < 1e-12);
    let impact = state.impact.expect("попадание пересчитано сразу");
    assert_eq!(impact.t.to_bits(), (2.95_f64 / 20.0).to_bits());
}
