use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::resources::*;
use crate::ui::UiEvents;
use lift_core::{FrameParams, Mode, ViewFrame};

const CABIN_HALF: f32 = 1.5;
const CABIN_H: f32 = 3.0;

/// Статические коллайдеры кабины: пол, три стены (левая, правая,
/// задняя), потолок. Передняя стена визуально отсутствует — коллайдер
/// тоже не ставим (02-lift-physics §3.2).
pub fn spawn_cabin_colliders(commands: &mut Commands) {
    let collider = RigidBody::Fixed;

    // Пол: верх на y = 0.
    commands.spawn((
        collider,
        Collider::cuboid(CABIN_HALF, 0.05, CABIN_HALF),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    // Боковые стены.
    for x in [-1.0_f32, 1.0] {
        commands.spawn((
            collider,
            Collider::cuboid(0.05, CABIN_H / 2.0, CABIN_HALF),
            Transform::from_xyz(x * (CABIN_HALF + 0.05), CABIN_H / 2.0, 0.0),
        ));
    }
    // Задняя стена.
    commands.spawn((
        collider,
        Collider::cuboid(CABIN_HALF, CABIN_H / 2.0, 0.05),
        Transform::from_xyz(0.0, CABIN_H / 2.0, -(CABIN_HALF + 0.05)),
    ));
    // Потолок: низ на y = 3.
    commands.spawn((
        collider,
        Collider::cuboid(CABIN_HALF, 0.05, CABIN_HALF),
        Transform::from_xyz(0.0, CABIN_H + 0.05, 0.0),
    ));
}

/// Три динамических мяча (T02 §3.2): массы 1/5/10 кг, одинаковый
/// радиус 0.15, x ∈ {−0.8, 0, 0.8}, y = 2.2, нулевые скорости,
/// damping = 0, ccd = true. Живут в корне мира (не дети SceneRoot —
/// см. комментарий на SceneRootMarker).
pub fn spawn_balls(commands: &mut Commands) {
    for i in 0..3 {
        commands.spawn((
            Ball { index: i },
            RigidBody::Dynamic,
            Collider::ball(BALL_RADIUS),
            ColliderMassProperties::Mass(BALL_MASS[i] as f32),
            ActiveEvents::CONTACT_FORCE_EVENTS,
            ContactForceEventThreshold(0.0),
            Velocity::zero(),
            Damping::default(),
            Sleeping::disabled(),
            Ccd::enabled(),
            Restitution::new(0.5),
            Transform::from_xyz(BALL_X[i] as f32, BALL_Y as f32, 0.0),
            BallTrace::default(),
        ));
    }

    // Фантом кабины: масса 1 кг, ни с чем не сталкивается; sim_gate
    // выдаёт ему силу (lift_a − g_eff), так что он движется ровно так,
    // как двигалась бы кабина в этом же численном интеграторе.
    commands.spawn((
        CabinGhost,
        RigidBody::Dynamic,
        Collider::ball(0.05),
        ColliderMassProperties::Mass(1.0),
        CollisionGroups::new(Group::NONE, Group::NONE),
        ExternalForce::default(),
        Velocity::zero(),
        Damping::default(),
        Sleeping::disabled(),
        Transform::from_xyz(1_000.0, 0.0, 0.0),
    ));
}

/// Спавн физики и визуальных мешей мячей (дети `SceneRoot`).
pub fn spawn_physics(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene_root: Res<SceneRootRef>,
) {
    spawn_cabin_colliders(&mut commands);
    spawn_balls(&mut commands);

    // Один общий Mesh-handle на все три груза (T05 §5: share meshes —
    // одна геометрия, три Transform); материалы различаются по цвету.
    let shared_sphere = meshes.add(Sphere::new(BALL_RADIUS));
    for i in 0..3 {
        let (r, g, b) = BALL_COLORS[i];
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            ..default()
        });
        commands.entity(scene_root.0).with_children(|parent| {
            parent.spawn((
                Mesh3d(shared_sphere.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(BALL_X[i] as f32, BALL_Y as f32, 0.0),
                BallVisual { index: i },
            ));
        });
    }
}

/// Действия ввода применяются ДО синхронизации Rapier (до
/// `PhysicsSet::SyncBackend`), чтобы телепорты и обнуление скоростей
/// попали в тела тем же тиком.
#[allow(clippy::too_many_arguments)] // SystemParam-набор Bevy — норма
pub fn sim_pre(
    mut input: ResMut<InputEvents>,
    mut ui_events: ResMut<UiEvents>,
    mut photon_events: ResMut<crate::systems::input::PhotonEvents>,
    mut sim: ResMut<SimConfig>,
    custom: Res<CustomParams>,
    mut clock: ResMut<SimClock>,
    mut metrics: ResMut<Metrics>,
    mut balls: Query<(&Ball, &mut Transform, &mut Velocity, &mut BallTrace)>,
    mut ghost: Query<
        (&CabinGhost, &mut Transform, &mut Velocity, &mut ExternalForce),
        Without<Ball>,
    >,
) {
    // Кадр считается здесь, чтобы сила фантома была применена
    // синхронизацией Rapier тем же тиком (apply_rigid_body_user_changes
    // выполняется в PhysicsSet::SyncBackend — после sim_pre).
    let mut need_reset = input.reset || ui_events.reset;
    if let Some(mode) = input.mode_reset.or(ui_events.mode_reset) {
        sim.mode = mode;
        need_reset = true;
    }
    if let Some(mode) = input.mode_live.or(ui_events.mode_live) {
        sim.mode = mode;
    }
    // На финише (t = duration) Space не «снимает с паузы», а перезапускает
    // эксперимент: полный сброс — грузы снова падают с 1.8 м.
    if (input.toggle_pause || ui_events.toggle_pause)
        && !clock.running
        && clock.t >= clock.duration
    {
        need_reset = true;
        input.toggle_pause = false;
        ui_events.toggle_pause = false;
    }
    sim.frame = match sim.mode {
        Mode::Custom => FrameParams {
            external_g: custom.external_g,
            lift_a: custom.lift_a,
        },
        mode => mode.params(),
    };
    let frame = sim.frame;

    if need_reset {
        for (ball, mut transform, mut velocity, mut trace) in &mut balls {
            transform.translation =
                Vec3::new(BALL_X[ball.index] as f32, BALL_Y as f32, 0.0);
            transform.rotation = Quat::IDENTITY;
            *velocity = Velocity::zero();
            trace.samples_lift.clear();
            trace.samples_external.clear();
        }
        if let Ok((_, mut transform, mut velocity, _)) = ghost.single_mut() {
            transform.translation = Vec3::new(1_000.0, 0.0, 0.0);
            *velocity = Velocity::zero();
        }
        clock.reset();
        metrics.reset();
        input.reset_seq = input.reset_seq.wrapping_add(1);
    }
    if input.toggle_view {
        sim.view = match sim.view {
            ViewFrame::Lift => ViewFrame::External,
            ViewFrame::External => ViewFrame::Lift,
        };
    }
    // Прямая установка кадра (кнопки панели, шаг урока).
    if let Some(view) = ui_events.set_view {
        sim.view = view;
    }
    if input.toggle_pause || ui_events.toggle_pause {
        clock.running = !clock.running;
    }
    // Пауза на входе в шаг урока: предсказание до наблюдения.
    if ui_events.pause {
        clock.running = false;
    }
    // Выстрел UI → PhotonEvents: update_photon потребит его тем же
    // тиком (идёт после sim_pre в расписании FixedUpdate). Через
    // UiEvents обязательно: прямая запись из PostUpdate могла бы быть
    // стёрта handle_photon_input до ближайшего fixed-тика.
    if ui_events.shot {
        photon_events.shot = true;
    }

    // Фантом кабины: g_eff + F/m = lift_a ⇒ F = lift_a − g_eff (масса 1 кг).
    if let Ok((_, _, _, mut force)) = ghost.single_mut() {
        force.force = Vec3::new(0.0, (frame.lift_a - frame.g_eff()) as f32, 0.0);
    }

    ui_events.reset();
    *input = InputEvents::default();
}

/// Синхронизация гравитации и тактирование — строго до шага физики
/// (02-lift-physics §3.3, ловушка №1).
///
/// Работает после `PhysicsSet::SyncBackend` (где появляется
/// `RapierConfiguration`) и до `PhysicsSet::StepSimulation`, поэтому
/// ни один шаг не выполняется с несинхронизированной гравитацией.
pub fn sim_gate(
    mut config: Query<&mut RapierConfiguration, With<DefaultRapierContext>>,
    sim: Res<SimConfig>,
    mut clock: ResMut<SimClock>,
    mut steps: ResMut<SimSteps>,
) {
    // Контекст создаётся системой `setup_physics` в Update первого кадра;
    // до этого физика ещё не существует — ничего не делаем.
    let Ok(mut config) = config.single_mut() else {
        return;
    };

    if sim.view == ViewFrame::External && clock.t >= EXTERNAL_PAUSE_T {
        clock.running = false;
    }
    if clock.running {
        let was_running = clock.running;
        clock.step();
        // Конец эксперимента — подсказка (ExperimentClock сам ставит running=false).
        if was_running && !clock.running {
            bevy::log::info!(
                "[подсказка] эксперимент завершён (t = {:.1} c). Space — рестарт, R — сброс, V — кадр, L — луч",
                clock.t
            );
        }
        steps.0 += 1;
    }

    config.gravity = Vec3::new(0.0, sim.frame.g_eff() as f32, 0.0);
    config.physics_pipeline_active = clock.running;
}

/// Сбор трейлов и отладочных метрик — после записи результатов шага
/// (после `PhysicsSet::Writeback`).
pub fn sim_post(
    sim: Res<SimConfig>,
    mut metrics: ResMut<Metrics>,
    mut balls: Query<(&Ball, &Transform, &mut BallTrace)>,
    ghost: Query<&Transform, (With<CabinGhost>, Without<Ball>)>,
) {
    // Дискретное D — фактическая позиция фантома кабины, которую
    // интегрирует тот же решатель (см. CabinGhost). До первого шага — 0.
    let d = ghost
        .single()
        .map(|t| t.translation.y as f64)
        .unwrap_or(0.0);

    let mut y = [0.0_f64; 3];
    let mut seen = [false; 3];
    for (ball, transform, mut trace) in &mut balls {
        let p = [
            transform.translation.x as f64,
            transform.translation.y as f64,
            transform.translation.z as f64,
        ];
        trace.samples_lift.push_back(p);
        trace.samples_external.push_back([p[0], p[1] + d, p[2]]);
        if trace.samples_lift.len() > TRACE_LEN {
            trace.samples_lift.pop_front();
            trace.samples_external.pop_front();
        }
        y[ball.index] = p[1];
        seen[ball.index] = true;
    }
    if !seen.iter().all(|s| *s) {
        return;
    }

    metrics.step_count += 1;

    let dy_ab = (y[0] - y[1]).abs();
    let dy_ac = (y[0] - y[2]).abs();

    // Сначала фиксируем касание (его собственный скачок не должен
    // портить метрики до касания).
    let radius = BALL_RADIUS as f64;
    if !metrics.contact_seen && y.iter().any(|&yi| yi <= PAN_TOP + radius + 1e-3) {
        metrics.contact_seen = true;
        bevy::log::info!(
            "T02.5 [{:?}] до касания: max|yA-yB| = {:.3e}, max|yA-yC| = {:.3e} (порог 1e-4)",
            sim.mode,
            metrics.max_dy_ab,
            metrics.max_dy_ac
        );
        if sim.mode == Mode::Rocket {
            metrics.rocket_logged = true;
            bevy::log::info!(
                "T02.6 [Rocket] max|y_external - y0| = {:.3e} (порог 1e-3)",
                metrics.rocket_y_dev
            );
        }
    }

    if !metrics.contact_seen {
        metrics.max_dy_ab = metrics.max_dy_ab.max(dy_ab);
        metrics.max_dy_ac = metrics.max_dy_ac.max(dy_ac);
        if sim.mode == Mode::Rocket {
            metrics.rocket_y_dev = metrics
                .rocket_y_dev
                .max(((y[0] + d) - BALL_Y).abs());
        }
    }

    if metrics.step_count == 60 {
        metrics.run_hash = hash_positions(&y);
        let drift = metrics.last_run_hash != 0 && metrics.last_run_hash != metrics.run_hash;
        bevy::log::info!(
            "T02.7 [{:?}] шаг 60: hash = {:016x}{}",
            sim.mode,
            metrics.run_hash,
            if drift { " !! ДРЕЙФ ДЕТЕРМИНИЗМА" } else { "" }
        );
        metrics.last_run_hash = metrics.run_hash;
        metrics.hash_taken = true;
    }
}
