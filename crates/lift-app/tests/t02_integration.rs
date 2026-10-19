//! Интеграционные проверки T02 (нативно, headless, без окна и рендера).
//!
//! Собирается тот же набор продакшн-систем, что и в приложении
//! (`sim_pre` → Rapier → `sim_gate`/`sim_post`), но с MinimalPlugins
//! и `TimeUpdateStrategy::ManualDuration` — ровно как в тестах самого
//! bevy_rapier3d. Один `app.update()` = один fixed-тик 1/60 с.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_rapier3d::plugin::NoUserData;
use bevy_rapier3d::prelude::*;

use lift_app::resources::{
    Ball, CustomParams, InputEvents, Metrics, SimClock, SimConfig, SimSteps, BALL_Y,
    EXTERNAL_PAUSE_T,
};
use lift_app::systems::physics::{sim_gate, sim_post, sim_pre, spawn_balls, spawn_cabin_colliders};
use lift_core::{ExperimentClock, Mode, ViewFrame};

/// Биты f64 начальных x-координат (мячи не должны смещаться по x).
/// Спавн идёт через f32, поэтому ожидание — `f32 as f64`.
const BALL_X_BITS: [u64; 3] = [
    ((-0.8_f32) as f64).to_bits(),
    0.0_f64.to_bits(),
    ((0.8_f32) as f64).to_bits(),
];

fn make_app(mode: Mode) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // Нужен bevy_rapier: его SyncBackend вызывает системы
        // bevy_transform, а init_async_colliders читает Assets<Mesh>.
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
            ),
        );
    app
}

fn run_ticks(app: &mut App, ticks: usize) {
    for _ in 0..ticks {
        app.update();
    }
}

/// (y, x, z) мячей после прогонки, в порядке индексов A/B/C.
fn ball_positions(app: &mut App) -> Vec<[f64; 3]> {
    let mut pos: Vec<Option<[f64; 3]>> = vec![None; 3];
    let mut query = app.world_mut().query::<(&Ball, &Transform)>();
    for (ball, transform) in query.iter(app.world()) {
        pos[ball.index] = Some([
            transform.translation.x as f64,
            transform.translation.y as f64,
            transform.translation.z as f64,
        ]);
    }
    pos.into_iter()
        .map(|p| p.expect("все три мяча на месте"))
        .collect()
}

/// T02.5: три массы дают одинаковую траекторию (различие < 1e-4),
/// мячи реально падают и касаются пола.
#[test]
fn t02_5_planet_masses_identical_until_contact() {
    let mut app = make_app(Mode::Planet);
    run_ticks(&mut app, 45); // 0.75 c > времени падения с 2.2 м (~0.646 c)

    let metrics = app.world().resource::<Metrics>();
    assert!(metrics.contact_seen, "мячи должны были коснуться пола");
    assert!(
        metrics.max_dy_ab < 1e-4,
        "max|yA-yB| = {:e}",
        metrics.max_dy_ab
    );
    assert!(
        metrics.max_dy_ac < 1e-4,
        "max|yA-yC| = {:e}",
        metrics.max_dy_ac
    );

    let pos = ball_positions(&mut app);
    for p in &pos {
        assert!(p[1] < BALL_Y, "мяч должен опуститься");
    }
}

/// T02.1 (динамическая версия): Planet и Rocket неразличимы внутри
/// кабины — позиции совпадают побитово на всём интервале до касания.
#[test]
fn t02_planet_and_rocket_identical_in_lift_frame() {
    let mut planet = make_app(Mode::Planet);
    let mut rocket = make_app(Mode::Rocket);
    run_ticks(&mut planet, 38); // 0.633 c — непосредственно перед касанием
    run_ticks(&mut rocket, 38);

    let a = ball_positions(&mut planet);
    let b = ball_positions(&mut rocket);
    for (pa, pb) in a.iter().zip(b.iter()) {
        for k in 0..3 {
            assert_eq!(
                pa[k].to_bits(),
                pb[k].to_bits(),
                "Planet {pa:?} vs Rocket {pb:?}"
            );
        }
    }
}

/// Критерий: FreeFall и Inertial — мячи не меняют p_lift (< 1e-4),
/// x-координаты не меняются вовсе.
#[test]
fn t02_freefall_and_inertial_are_weightless() {
    for mode in [Mode::FreeFall, Mode::Inertial] {
        let mut app = make_app(mode);
        run_ticks(&mut app, 60);
        for (i, p) in ball_positions(&mut app).into_iter().enumerate() {
            assert!((p[1] - BALL_Y).abs() < 1e-4, "{mode:?}: y = {}", p[1]);
            assert_eq!(
                p[0].to_bits(),
                crate::BALL_X_BITS[i],
                "{mode:?}: мяч {i} сместился по x"
            );
        }
    }
}

/// T02.6: Rocket + External — мяч сохраняет y_external (дискретный D),
/// отклонение < 1e-3 (фактически ~1e-15).
#[test]
fn t02_6_rocket_external_y_is_constant() {
    let mut app = make_app(Mode::Rocket);
    app.world_mut().resource_mut::<SimConfig>().view = ViewFrame::External;
    // Прогоняем до авто-паузы внешнего кадра (t = EXTERNAL_PAUSE_T).
    let ticks = (EXTERNAL_PAUSE_T / (1.0 / 60.0)).ceil() as usize + 2;
    run_ticks(&mut app, ticks);

    let clock = app.world().resource::<SimClock>();
    assert!(!clock.running, "авто-пауза на t = 1.2 c");
    assert!((clock.t - EXTERNAL_PAUSE_T).abs() < 1e-9);

    let metrics = app.world().resource::<Metrics>();
    assert!(
        metrics.rocket_y_dev < 1e-3,
        "max|y_external - y0| = {:e}",
        metrics.rocket_y_dev
    );
}

/// T02.7: детерминизм — 10 прогонов дают одинаковый хеш позиций
/// после 60 шагов (первый update не даёт шага — контекст создаётся
/// в Update того же кадра, поэтому 62 тика).
#[test]
fn t02_7_determinism_10_runs() {
    let mut reference = (0_u64, Vec::new());
    for run in 0..10 {
        let mut app = make_app(Mode::Planet);
        run_ticks(&mut app, 62);

        let metrics = app.world().resource::<Metrics>();
        assert!(metrics.hash_taken);
        if run == 0 {
            reference = (metrics.run_hash, ball_positions(&mut app));
        } else {
            assert_eq!(metrics.run_hash, reference.0, "прогон {run}: дрейф хеша");
            let pos = ball_positions(&mut app);
            for (p, r) in pos.iter().zip(reference.1.iter()) {
                assert_eq!(p[1].to_bits(), r[1].to_bits(), "прогон {run}: дрейф y");
            }
        }
    }
}

/// Внешний кадр FreeFall: кабина и мячи падают вместе — мяч покоится
/// в кадре лифта, фантом кабины уводит внешний кадр вниз.
#[test]
fn t02_freefall_external_falls_together() {
    let mut app = make_app(Mode::FreeFall);
    app.world_mut().resource_mut::<SimConfig>().view = ViewFrame::External;
    run_ticks(&mut app, 40);

    let (t, d) = {
        use lift_app::resources::CabinGhost;
        let t = app.world().resource::<SimClock>().t;
        let mut q = app
            .world_mut()
            .query::<(&CabinGhost, &Transform)>();
        let (_, tr) = q.single(app.world()).unwrap();
        (t, tr.translation.y as f64)
    };

    assert!(d < 0.0, "кабина в FreeFall уходит вниз: D = {d}");
    for p in ball_positions(&mut app) {
        // Внешняя позиция мяча = p_lift + D — падает вместе с кабиной.
        let y_ext = p[1] + d;
        assert!(
            (y_ext - (BALL_Y + 0.5 * (-9.81) * t * t)).abs() < 0.05,
            "y_external = {y_ext} при t = {t}"
        );
    }
}
