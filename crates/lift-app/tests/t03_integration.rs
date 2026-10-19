//! Интеграционные проверки T03 (нативно, headless) — пружинные весы.
//!
//! Тот же набор продакшн-систем, что и в приложении, плюс стойки весов
//! и `read_contact_forces` (Update). Один `app.update()` = один
//! fixed-тик 1/60 с. Эталон `N = m·g_eff` используется только здесь,
//! в тестах (03-scales §3.2 п. 3) — в измерениях его нет.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_rapier3d::plugin::NoUserData;
use bevy_rapier3d::prelude::*;

use lift_app::resources::{
    Ball, CustomParams, InputEvents, Metrics, ScaleReadings, SimClock, SimConfig, SimSteps,
    BALL_MASS, BALL_Y, PAN_TOP,
};
use lift_app::scales::reading::read_contact_forces;
use lift_app::scales::spawn_scale_platforms;
use lift_app::systems::physics::{sim_gate, sim_post, sim_pre, spawn_balls, spawn_cabin_colliders};
use lift_core::{ExperimentClock, Mode};

fn make_app(mode: Mode) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .init_resource::<Assets<Mesh>>()
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
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
        .init_resource::<lift_app::systems::input::PhotonEvents>()
        .init_resource::<lift_app::ui::UiEvents>()
        .init_resource::<Metrics>()
        .init_resource::<ScaleReadings>()
        .add_systems(
            Startup,
            (
                |mut commands: Commands| spawn_cabin_colliders(&mut commands),
                |mut commands: Commands| spawn_scale_platforms(&mut commands),
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
            ),
        )
        .add_systems(Update, read_contact_forces);
    app
}

fn run_ticks(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.update();
    }
}

fn readings(app: &mut App) -> ScaleReadings {
    let r = *app.world().resource::<ScaleReadings>();
    r
}

const G: f64 = 9.81;

/// Прогон до установления: 3 с (длительность эксперимента) + запас
/// на однотиковое запаздывание очереди сообщений.
fn settle_ticks() -> usize {
    200
}

/// T03.1 + точность: Planet — показания ≈ m·g с погрешностью < 2%,
/// Rocket — совпадают с Planet с погрешностью < 1%.
#[test]
fn t03_1_planet_rocket_match_within_1_percent() {
    let mut planet = make_app(Mode::Planet);
    run_ticks(&mut planet, settle_ticks());
    let planet = readings(&mut planet);

    let mut rocket = make_app(Mode::Rocket);
    run_ticks(&mut rocket, settle_ticks());
    let rocket = readings(&mut rocket);

    for i in 0..3 {
        let expected = BALL_MASS[i] * G;
        let n_planet = planet.items[i].normal_force_n;

        assert!(
            planet.items[i].in_contact,
            "груз {i} должен лежать на чаше (Planet): N={:?} c={:?}",
            [planet.items[0].normal_force_n, planet.items[1].normal_force_n, planet.items[2].normal_force_n],
            [planet.items[0].in_contact, planet.items[1].in_contact, planet.items[2].in_contact],
        );
        assert!(
            (n_planet - expected).abs() / expected < 0.02,
            "Planet, груз {i} ({} кг): N = {n_planet} vs {expected}",
            BALL_MASS[i]
        );

        let n_rocket = rocket.items[i].normal_force_n;
        let rel = (n_planet - n_rocket).abs() / n_planet;
        assert!(rel < 0.01, "Planet {n_planet} vs Rocket {n_rocket}: {rel}");
    }
}

/// T03.2/T03.3: FreeFall и Inertial — |N| < 0.05 Н после 1 с
/// (грузы висят, чаши не касаются).
#[test]
fn t03_2_3_weightless_modes_read_zero() {
    for mode in [Mode::FreeFall, Mode::Inertial] {
        let mut app = make_app(mode);
        run_ticks(&mut app, 120);
        let r = readings(&mut app);
        for item in &r.items {
            assert!(
                item.normal_force_n < 0.05,
                "{mode:?}: N = {}",
                item.normal_force_n
            );
            assert!(!item.in_contact, "{mode:?}: ложный контакт");
        }
    }
}

/// T03.4/T03.5: показания линейны по массе —
/// N(5)/N(1) ≈ 5 ± 0.1, N(10)/N(1) ≈ 10 ± 0.1.
#[test]
fn t03_4_5_readings_linear_in_mass() {
    let mut app = make_app(Mode::Planet);
    run_ticks(&mut app, settle_ticks());
    let r = readings(&mut app);

    let (n1, n5, n10) = (
        r.items[0].normal_force_n,
        r.items[1].normal_force_n,
        r.items[2].normal_force_n,
    );
    assert!(
        (n5 / n1 - 5.0).abs() < 0.1,
        "N5/N1 = {}",
        n5 / n1
    );
    assert!(
        (n10 / n1 - 10.0).abs() < 0.1,
        "N10/N1 = {}",
        n10 / n1
    );
}

/// T03.8: выключенная гравитация + нулевая скорость — контакт
/// не создаёт силу.
#[test]
fn t03_8_contact_without_gravity_makes_no_force() {
    let mut app = make_app(Mode::Inertial);
    // Груз 1 кладём точно на чашу: y = PAN_TOP + радиус.
    let mut query = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Ball>>();
    for (i, mut transform) in query.iter_mut(app.world_mut()).enumerate() {
        if i == 1 {
            transform.translation =
                Vec3::new(0.0, (PAN_TOP + 0.15) as f32, 0.0);
        }
    }
    run_ticks(&mut app, 90);
    let r = readings(&mut app);
    for (i, item) in r.items.iter().enumerate() {
        assert!(
            item.normal_force_n < 0.05,
            "груз {i}: N = {}",
            item.normal_force_n
        );
    }
}

/// Грузы стартуют над чашами (y = 1.8) и в Planet опускаются на них.
#[test]
fn t03_loads_drop_from_release_height() {
    let mut app = make_app(Mode::Planet);
    run_ticks(&mut app, 200);
    let mut query = app.world_mut().query::<(&Ball, &Transform)>();
    let ys: Vec<f32> = query
        .iter(app.world())
        .map(|(_, tr)| tr.translation.y)
        .collect();
    for y in ys {
        assert!(y < BALL_Y as f32, "груз должен опуститься: y = {y}");
        assert!(
            (y - (PAN_TOP + 0.15) as f32).abs() < 0.05,
            "груз должен лежать на чаше: y = {y}"
        );
    }
}
