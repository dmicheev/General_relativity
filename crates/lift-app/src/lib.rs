pub mod light;
pub mod resources;
pub mod scales;
pub mod scene;
pub mod systems;
pub mod ui;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lift_core::ExperimentClock;
use bevy::gizmos::config::GizmoConfigStore;
use bevy::gizmos::prelude::DefaultGizmoConfigGroup;
use light::PhotonState;
use systems::input::PhotonEvents;
use resources::{
    CustomParams, InputEvents, Metrics, ScaleReadings, SimClock, SimConfig,
};

/// Сборка приложения. Отделена от входных точек, чтобы интеграционные
/// тесты могли собирать её же системы headless (MinimalPlugins).
pub fn app() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Лифт Эйнштейна".into(),
                    ..default()
                }),
                ..default()
            }),
        )
        // UI урока (T06). Автопривязку контекста отключаем: в Bevy 0.20
        // AmbientLight — сущность с обязательным Camera, она создаётся
        // раньше Camera3d, и bevy_egui привязал бы контекст к ней
        // (viewport такой «камеры» не резолвится — egui получает
        // дефолтный rect 10000×10000 и панели рисуются мимо экрана).
        // Контекст привязывается явно к Camera3d в scene::setup_camera.
        .insert_resource(bevy_egui::EguiGlobalSettings {
            auto_create_primary_context: false,
            ..default()
        })
        .add_plugins(bevy_egui::EguiPlugin::default())
        // Физика: детерминированный фиксированный шаг 1/60 (02-lift-physics).
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(TimestepMode::Fixed {
            dt: 1.0 / 60.0,
            substeps: 1,
        })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule())
        .init_resource::<SimConfig>()
        .init_resource::<CustomParams>()
        .init_resource::<InputEvents>()
        .init_resource::<Metrics>()
        .init_resource::<ScaleReadings>()
        .init_resource::<PhotonState>()
        .init_resource::<PhotonEvents>()
        .init_resource::<resources::SimSteps>()
        .init_resource::<scene::OrbitCamera>()
        .init_resource::<ui::UiEvents>()
        .init_resource::<ui::PanelState>()
        .init_resource::<ui::lesson::LessonState>()
        // Шаг 1 урока: сцена ждёт предсказания на паузе (T06 §4:
        // prediction before observation — иначе эксперимент успевает
        // проиграться, пока пользователь читает вопрос).
        .insert_resource(ui::UiEvents {
            reset: true,
            pause: true,
            ..default()
        })
        .insert_resource(SimClock(ExperimentClock::new(
            resources::EXPERIMENT_DURATION,
        )))
        // Линии трейлов и луча заметнее (T02/T04: 1px слишком тонко).
        .add_systems(
            Startup,
            |mut store: ResMut<GizmoConfigStore>| {
                let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
                config.line.width = 2.5;
            },
        )
        .add_systems(
            Startup,
            // .chain() обязателен: setup_scene вставляет SceneRootRef через
            // Commands, а spawn_physics/spawn_scale_visuals его читают —
            // нужен sync point (apply_deferred) между системами.
            (
                scene::setup_scene,
                scene::setup_camera,
                systems::physics::spawn_physics,
                scales::spawn_scale_visuals,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                scene::update_orbit_camera,
                systems::input::handle_input,
                systems::input::handle_photon_input,
                systems::frame_bridge::update_frame_bridge.after(scene::update_orbit_camera),
                systems::frame_bridge::sync_ball_visuals,
                systems::frame_bridge::draw_trails,
                scales::reading::read_contact_forces,
                scales::update_scale_visuals.after(scales::reading::read_contact_forces),
                light::draw_photon,
            ),
        )
        // UI-система запускается bevy_egui из PostUpdate (pass loop);
        // клики пишутся в UiEvents и потребляются sim_pre (см. ui/mod.rs).
        .add_systems(bevy_egui::EguiPrimaryContextPass, ui::ui_system)
        .add_systems(
            FixedUpdate,
            (
                systems::physics::sim_pre.before(PhysicsSet::SyncBackend),
                systems::physics::sim_gate
                    .after(systems::physics::sim_pre)
                    .after(PhysicsSet::SyncBackend)
                    .before(PhysicsSet::StepSimulation),
                systems::physics::sim_post.after(PhysicsSet::Writeback),
                light::update_photon
                    .after(systems::physics::sim_pre)
                    .after(systems::physics::sim_post),
            ),
        )
        .run();
}
