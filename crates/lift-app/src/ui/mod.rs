//! Учебный интерфейс (T06): панели egui, слайдеры, урок с
//! предсказаниями, живые показания.
//!
//! ПОРЯДОК СОБЫТИЙ (важно): UI-системы выполняются в
//! `EguiPrimaryContextPass` (bevy_egui запускает их из PostUpdate), а
//! `handle_input` (Update) перезаписывает `InputEvents` каждый кадр.
//! Поэтому клики UI пишутся в ОТДЕЛЬНЫЙ ресурс [`UiEvents`]: он не
//! трогается `handle_input`, потребляется `sim_pre` в ближайший
//! fixed-тик и там же очищается — клик не теряется и не срабатывает
//! дважды при любом соотношении FPS/60 Гц.

pub mod lesson;
pub mod strings;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use lift_core::{photon_impact, Mode, ViewFrame};

use crate::light::{PhotonState, X_WALL};
use crate::resources::{CabinGhost, CustomParams, ScaleReadings, SimClock, SimConfig};
use lesson::LessonState;
use strings as s;

/// Действия UI, накопленные за кадр (см. модульный комментарий).
/// Поля складываются («merge»), очищаются только в `sim_pre`.
#[derive(Resource, Default, Debug)]
pub struct UiEvents {
    pub mode_reset: Option<Mode>,
    pub mode_live: Option<Mode>,
    pub reset: bool,
    /// Прямая установка кадра (кнопки панели/шаг урока), не тумблер.
    pub set_view: Option<ViewFrame>,
    /// Пауза ПОСЛЕ применения остальных действий (шаг урока ждёт
    /// предсказание — эксперимент стартует по кнопке «Запустить»).
    pub pause: bool,
    pub toggle_pause: bool,
    pub shot: bool,
}

impl UiEvents {
    /// Очистка после потребления в `sim_pre` (публично: потребляет
    /// система физики).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Состояние панелей между кадрами (сворачивание, узкий экран).
#[derive(Resource, Debug)]
pub struct PanelState {
    pub left_open: bool,
    pub right_open: bool,
    pub theory_open: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            left_open: true,
            right_open: true,
            theory_open: false,
        }
    }
}

/// Порог «мобильной» вёрстки (06-ui-pedagogy §2): панели становятся
/// раскрывающимися окнами.
pub const NARROW_WIDTH: f32 = 700.0;

const LEFT_PANEL_W: f32 = 216.0;
const RIGHT_PANEL_W: f32 = 312.0;

/// Индекс режима для строк/радиокнопок: Planet=0 .. Custom=4.
fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::Planet => 0,
        Mode::Rocket => 1,
        Mode::FreeFall => 2,
        Mode::Inertial => 3,
        Mode::Custom => 4,
    }
}

fn index_mode(i: usize) -> Mode {
    [
        Mode::Planet,
        Mode::Rocket,
        Mode::FreeFall,
        Mode::Inertial,
        Mode::Custom,
    ][i]
}

/// Главное UI-приложение: раскладка панелей + делегирование блоков.
/// Выполняется в `EguiPrimaryContextPass` (см. lib.rs).
#[allow(clippy::too_many_arguments)] // SystemParam-набор Bevy — норма
pub fn ui_system(
    mut contexts: EguiContexts,
    sim: Res<SimConfig>,
    mut clock: ResMut<SimClock>,
    mut custom: ResMut<CustomParams>,
    mut photon: ResMut<PhotonState>,
    readings: Res<ScaleReadings>,
    mut ui_events: ResMut<UiEvents>,
    mut lesson: ResMut<LessonState>,
    mut panels: ResMut<PanelState>,
    ghost: Query<&Transform, With<CabinGhost>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let screen_w = ctx.viewport_rect().width();
    // egui 0.36: панели строятся на корневом Ui вьюпорта
    // (паттерн из examples/ui.rs bevy_egui 0.43).
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::top("top_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(s::TITLE);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut panels.theory_open, s::BTN_THEORY);
            });
        });
    });

    egui::Panel::bottom("timeline_bar").show(&mut viewport_ui, |ui| {
        timeline_ui(ui, &sim, &clock, &mut ui_events);
    });

    // На узких экранах боковые панели — раскрывающиеся окна
    // (bottom sheets из спеки в egui выражаются плавающими окнами).
    let narrow = screen_w < NARROW_WIDTH;
    if narrow {
        egui::Window::new(format!("☰ {}", s::SECTION_MODE))
            .open(&mut panels.left_open)
            .default_width(LEFT_PANEL_W)
            .show(ctx, |ui| {
                left_panel(
                    ui,
                    &sim,
                    &mut clock,
                    &mut custom,
                    &mut photon,
                    &mut ui_events,
                    &mut lesson,
                );
            });
        egui::Window::new(format!("ℹ {}", s::SECTION_STATUS))
            .open(&mut panels.right_open)
            .default_width(RIGHT_PANEL_W)
            .show(ctx, |ui| {
                right_panel(
                    ui,
                    &sim,
                    &clock,
                    &readings,
                    &photon,
                    &mut lesson,
                    &mut ui_events,
                    &ghost,
                );
            });
    } else {
        egui::Panel::left("left_panel")
            .resizable(false)
            .exact_size(LEFT_PANEL_W)
            .show(&mut viewport_ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    left_panel(
                        ui,
                        &sim,
                        &mut clock,
                        &mut custom,
                        &mut photon,
                        &mut ui_events,
                        &mut lesson,
                    );
                });
            });
        egui::Panel::right("right_panel")
            .resizable(false)
            .exact_size(RIGHT_PANEL_W)
            .show(&mut viewport_ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    right_panel(
                        ui,
                        &sim,
                        &clock,
                        &readings,
                        &photon,
                        &mut lesson,
                        &mut ui_events,
                        &ghost,
                    );
                });
            });
    }

    if panels.theory_open {
        egui::Window::new(s::BTN_THEORY)
            .open(&mut panels.theory_open)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(s::THEORY_TEXT);
            });
    }

    Ok(())
}

// ---- Левая панель: режимы, кадр, параметры, управление, урок ----

fn left_panel(
    ui: &mut egui::Ui,
    sim: &SimConfig,
    clock: &mut SimClock,
    custom: &mut CustomParams,
    photon: &mut PhotonState,
    events: &mut UiEvents,
    lesson: &mut LessonState,
) {
    ui.heading(s::SECTION_MODE);
    for (i, name) in s::MODE_NAMES.iter().enumerate() {
        let mode = index_mode(i);
        if ui
            .add_enabled(true, egui::RadioButton::new(mode_index(sim.mode) == i, *name))
            .clicked()
        {
            events.mode_reset = Some(mode); // смена со сбросом
        }
    }
    ui.separator();

    ui.heading(s::SECTION_FRAME);
    ui.horizontal(|ui| {
        if ui
            .selectable_label(sim.view == ViewFrame::Lift, s::FRAME_LIFT)
            .clicked()
        {
            events.set_view = Some(ViewFrame::Lift);
        }
        if ui
            .selectable_label(sim.view == ViewFrame::External, s::FRAME_EXTERNAL)
            .clicked()
        {
            events.set_view = Some(ViewFrame::External);
        }
    });
    ui.separator();

    ui.heading(s::SECTION_PARAMS);
    // Слайдеры меняют физику немедленно: пишут CustomParams (считывается
    // sim_pre каждый тик) и переключают в Custom на лету. Подписи —
    // внутри слайдера: панель должна влезать по высоте без прокрутки.
    let mut slider_moved = false;
    if ui
        .add(
            egui::Slider::new(&mut custom.external_g, 0.0..=30.0)
                .step_by(0.01)
                .text(s::LABEL_G_EXT),
        )
        .changed()
    {
        slider_moved = true;
    }
    if ui
        .add(
            egui::Slider::new(&mut custom.lift_a, -20.0..=20.0)
                .step_by(0.01)
                .text(s::LABEL_A_LIFT),
        )
        .changed()
    {
        slider_moved = true;
    }
    if slider_moved {
        events.mode_live = Some(Mode::Custom);
    }
    ui.label(egui::RichText::new(s::HINT_HOMOGENEOUS).small());
    ui.horizontal_wrapped(|ui| {
        for (label, g) in [
            (s::PRESET_EARTH, 9.81),
            (s::PRESET_MOON, 1.62),
            (s::PRESET_MARS, 3.71),
            (s::PRESET_JUPITER, 24.79),
        ] {
            if ui.small_button(label).clicked() {
                custom.external_g = g;
                events.mode_live = Some(Mode::Custom);
            }
        }
    });
    // c_vis применяется мгновенно (аналитический пересчёт попадания).
    let c_vis_changed = ui
        .add(
            egui::Slider::new(&mut photon.launch.c, crate::light::C_VIS_MIN..=crate::light::C_VIS_MAX)
                .step_by(0.5)
                .text(s::LABEL_C_VIS),
        )
        .changed();
    if c_vis_changed && photon.impact.is_some() {
        photon.impact = photon_impact(&photon.launch, &sim.frame, X_WALL);
    }
    // Если эксперимент «заехал» за новую длительность — ничего не делаем:
    // следующий шаг/Space приведут к честному состоянию.
    ui.add(
        egui::Slider::new(&mut clock.duration, 0.5..=5.0)
            .step_by(0.1)
            .text(s::LABEL_DURATION),
    );
    ui.separator();

    ui.heading(s::SECTION_CONTROL);
    ui.horizontal_wrapped(|ui| {
        if ui.button(if clock.running { s::BTN_PAUSE } else { s::BTN_RESUME }).clicked() {
            events.toggle_pause = true;
        }
        if ui.button(s::BTN_RESTART).clicked() {
            events.reset = true;
        }
        if ui.button(s::BTN_FIRE).clicked() {
            events.shot = true;
        }
        if ui.button(s::BTN_CLEAR_BEAM).clicked() {
            photon.impact = None;
            photon.impact_lift = None;
            photon.impact_external = None;
            photon.active = false;
        }
        if ui.button(s::BTN_GHOST).clicked() {
            photon.ghost_visible = !photon.ghost_visible;
        }
    });
    ui.add_space(4.0);
    ui.label(egui::RichText::new(s::LABEL_MODE_LIVE).small());
    ui.horizontal_wrapped(|ui| {
        for (i, short) in ["П", "Р", "Пд", "И", "С"].iter().enumerate() {
            let mode = index_mode(i);
            if ui
                .small_button(*short)
                .on_hover_text(s::TOOLTIP_MODE_LIVE)
                .clicked()
            {
                events.mode_live = Some(mode);
            }
        }
    });
    ui.separator();

    ui.heading(s::SECTION_LESSON);
    ui.horizontal_wrapped(|ui| {
        if lesson.active {
            if ui.small_button(s::BTN_LESSON_RESTART).clicked() {
                *lesson = LessonState::new();
                apply_step_entry(events, lesson.step_def());
            }
            let sandbox = ui.small_button(s::BTN_SANDBOX);
            if sandbox.clicked() {
                *lesson = LessonState::sandbox();
            }
        } else if ui.small_button(s::BTN_LESSON_RESTART).clicked() {
            *lesson = LessonState::new();
            apply_step_entry(events, lesson.step_def());
        }
    });
}

/// Вход в шаг урока: поставить режим/кадр, сбросить сцену и СРАЗУ
/// поставить на паузу — предсказание до наблюдения (T06 §4).
fn apply_step_entry(events: &mut UiEvents, step: &lesson::LessonStep) {
    events.mode_reset = Some(step.mode);
    events.set_view = Some(step.view);
    events.reset = true;
    events.pause = true;
    if step.fire_beam {
        events.shot = true;
    }
}

// ---- Правая панель: урок/объяснение + живые показания ----

#[allow(clippy::too_many_arguments)]
fn right_panel(
    ui: &mut egui::Ui,
    sim: &SimConfig,
    clock: &SimClock,
    readings: &ScaleReadings,
    photon: &PhotonState,
    lesson: &mut LessonState,
    events: &mut UiEvents,
    ghost: &Query<&Transform, With<CabinGhost>>,
) {
    if lesson.active {
        lesson_block(ui, sim, clock, photon, lesson, events);
        ui.separator();
    } else {
        ui.heading(s::SANDBOX_TITLE);
        ui.label(s::MODE_DESCRIPTIONS[mode_index(sim.mode)]);
        ui.add_space(4.0);
        ui.label(s::MODE_PHYSICS[mode_index(sim.mode)]);
        ui.separator();
    }
    status_block(ui, sim, clock, readings, photon, ghost);
}

/// Блок урока: шаг, вопрос, варианты, раскрытие, «Далее».
fn lesson_block(
    ui: &mut egui::Ui,
    sim: &SimConfig,
    clock: &SimClock,
    photon: &PhotonState,
    lesson: &mut LessonState,
    events: &mut UiEvents,
) {
    let step = lesson.step_def();
    ui.horizontal(|ui| {
        ui.strong(format!(
            "{} {} {} «{}»",
            s::LESSON_LABEL,
            lesson.step + 1,
            s::LESSON_OF,
            step.title
        ));
    });
    ui.label(egui::RichText::new(s::LESSON_HINT_PREDICT).small());

    // Вопрос подсвечен, пока предсказания нет (T06 §4).
    if !lesson.revealed {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(step.question)
                .strong()
                .color(egui::Color32::from_rgb(255, 210, 80)),
        );
        ui.add_space(2.0);
        for (i, option) in step.options.iter().enumerate() {
            if ui.add(egui::Button::new(*option).wrap()).clicked() {
                lesson.answer(i);
            }
        }
        // Кнопка «Запустить» доступна и без ответа (вход в шаг ставит паузу).
        if ui.button(s::BTN_RUN).clicked() {
            events.toggle_pause = true;
        }
        return;
    }

    // Раскрытие.
    let chosen = lesson.predicted.unwrap_or(0);
    let correct = lesson
        .history
        .iter()
        .any(|r| r.step == lesson.step && r.correct);
    let (verdict, verdict_color) = if correct {
        (s::ANSWER_CORRECT, egui::Color32::from_rgb(120, 220, 120))
    } else {
        (s::ANSWER_WRONG, egui::Color32::from_rgb(255, 160, 120))
    };
    ui.colored_label(verdict_color, verdict);
    ui.label(format!(
        "{} — выбрано «{}», верно: «{}».",
        step.question,
        step.options[chosen.min(step.options.len() - 1)],
        step.options[step.correct]
    ));
    ui.add_space(2.0);
    ui.label(step.reveal);
    if ui.button(s::BTN_RUN).clicked() {
        events.toggle_pause = true;
    }

    // Дополнительный вопрос (шаг 4).
    if let Some(follow) = &step.follow_up {
        ui.separator();
        ui.strong(s::LESSON_FOLLOW_UP);
        if !lesson.follow_revealed {
            ui.label(egui::RichText::new(follow.question).strong());
            for (i, option) in follow.options.iter().enumerate() {
                if ui.add(egui::Button::new(*option).wrap()).clicked() {
                    lesson.answer_follow_up(i);
                }
            }
        } else {
            let fchosen = lesson.follow_predicted.unwrap_or(0);
            let fok = fchosen == follow.correct;
            ui.colored_label(
                if fok {
                    egui::Color32::from_rgb(120, 220, 120)
                } else {
                    egui::Color32::from_rgb(255, 160, 120)
                },
                if fok { s::ANSWER_CORRECT } else { s::ANSWER_WRONG },
            );
            ui.label(format!(
                "Верно: «{}».",
                follow.options[follow.correct]
            ));
            ui.label(follow.reveal);
        }
    }

    // Шаг 5: дополнительное действие «Rocket → парабола».
    if lesson.step == 4 && !lesson.rocket_parabola_done {
        ui.separator();
        if ui.button(s::BTN_SWITCH_ROCKET).clicked() {
            lesson.rocket_parabola_done = true;
            events.mode_reset = Some(Mode::Rocket);
            events.set_view = Some(ViewFrame::Lift);
            events.reset = true;
            events.shot = true;
        }
    }

    // Финальный вывод — после шага 5 и переключения в ракету.
    if lesson.step == 4 && lesson.rocket_parabola_done {
        ui.separator();
        ui.strong(s::LESSON_COMPLETE);
        ui.label(lesson::FINAL_CONCLUSION);
        // Наблюдение: в ракете луч парабола (g_eff = −9.81), факт виден на сцене.
        ui.label(egui::RichText::new(format!(
            "Сейчас: g_ext = {}, a_lift = {} — парабола в кадре кабины.",
            s::fmt_signed(sim.frame.external_g, 2),
            s::fmt_signed(sim.frame.lift_a, 2)
        )).small());
        if clock.t >= clock.duration && !clock.running {
            ui.label(egui::RichText::new(format!(
                "{}: t = {} {} / {} {}",
                s::LABEL_TIME,
                s::fmt_fixed(clock.t, 2),
                s::LABEL_UNIT_S,
                s::fmt_fixed(clock.duration, 2),
                s::LABEL_UNIT_S
            )).small());
        }
        if photon.impact.is_some() {
            if let Some(impact) = photon.impact {
                ui.label(egui::RichText::new(format!(
                    "Δy = {} {}",
                    s::fmt_signed(photon.launch.origin[1] - impact.lift[1], 3),
                    s::LABEL_UNIT_M
                )).small());
            }
        }
    }

    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(lesson.can_retreat(), egui::Button::new(s::BTN_PREV))
            .clicked()
        {
            lesson.retreat();
            apply_step_entry(events, lesson.step_def());
        }
        if lesson.has_next()
            && ui
                .add_enabled(lesson.can_advance(), egui::Button::new(s::BTN_NEXT))
                .clicked()
        {
            lesson.advance();
            apply_step_entry(events, lesson.step_def());
        }
        let (good, total) = lesson.score();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{good}/{total}")).monospace());
        });
    });
}

/// Живые показания (T06 §6): режим, g_ext/a_lift/g_eff/D(t), весы,
/// свет, кадр. Все числа monospace с фиксированной шириной.
fn status_block(
    ui: &mut egui::Ui,
    sim: &SimConfig,
    clock: &SimClock,
    readings: &ScaleReadings,
    photon: &PhotonState,
    ghost: &Query<&Transform, With<CabinGhost>>,
) {
    ui.strong(s::SECTION_STATUS);
    ui.label(s::MODE_DESCRIPTIONS[mode_index(sim.mode)]);

    let d = ghost.iter().next().map(|t| t.translation.y as f64).unwrap_or(0.0);
    let frame = sim.frame;
    let rows = [
        (
            s::LABEL_G_EXT,
            s::fmt_signed(frame.external_g, 2),
            s::LABEL_UNIT_G,
        ),
        (
            s::LABEL_A_LIFT,
            s::fmt_signed(frame.lift_a, 2),
            s::LABEL_UNIT_G,
        ),
        (
            "g_eff",
            s::fmt_signed(frame.g_eff(), 2),
            s::LABEL_UNIT_G,
        ),
        (s::LABEL_DT, s::fmt_signed(d, 3), s::LABEL_UNIT_M),
    ];
    egui::Grid::new("frame_grid")
        .num_columns(3)
        .spacing([8.0, 1.0])
        .show(ui, |ui| {
            for (label, value, unit) in rows {
                ui.monospace(label);
                ui.monospace(value);
                ui.label(egui::RichText::new(unit).small());
                ui.end_row();
            }
        });
    ui.add_space(4.0);

    ui.strong(s::SECTION_SCALES);
    egui::Grid::new("scales_grid")
        .num_columns(3)
        .spacing([8.0, 1.0])
        .show(ui, |ui| {
            for item in &readings.items {
                ui.monospace(format!("{:>4.0} {}", item.mass_kg, s::LABEL_KG));
                ui.monospace(format!(
                    "→ {:>6.2} {}",
                    item.normal_force_n, s::LABEL_UNIT_N
                ));
                // Бар дублирует число цветом: значение читается и без
                // цвета (T06 §7 — не полагаться только на цвет).
                ui.add(
                    egui::ProgressBar::new(item.compression.clamp(0.0, 1.0))
                        .desired_height(10.0)
                        .desired_width(80.0),
                );
                ui.end_row();
            }
        });
    ui.add_space(4.0);

    ui.strong(s::SECTION_LIGHT);
    match photon.impact {
        Some(impact) => {
            let dy = photon.launch.origin[1] - impact.lift[1];
            egui::Grid::new("light_grid")
                .num_columns(3)
                .spacing([8.0, 1.0])
                .show(ui, |ui| {
                    ui.monospace("Δy");
                    ui.monospace(s::fmt_signed(dy, 3));
                    ui.label(egui::RichText::new(s::LABEL_UNIT_M).small());
                    ui.end_row();
                    ui.monospace("T");
                    ui.monospace(s::fmt_fixed(impact.t, 3));
                    ui.label(egui::RichText::new(s::LABEL_UNIT_S).small());
                    ui.end_row();
                });
        }
        None => {
            ui.label(egui::RichText::new(s::LABEL_BEAM_OFF).small());
        }
    }
    // Подпись о замедлении видна всегда при активном луче (критерий).
    ui.label(egui::RichText::new(format!(
        "c_vis = {} {} ({} ×{})",
        s::fmt_fixed(photon.launch.c, 1),
        s::LABEL_UNIT_G,
        s::LABEL_SLOWED,
        s::format_slowdown(crate::light::C_REAL / photon.launch.c),
    )).monospace().small());
    ui.label(egui::RichText::new(s::beam_slowdown_note()).small());
    ui.add_space(4.0);

    ui.label(egui::RichText::new(format!(
        "{}: {}",
        s::SECTION_FRAME,
        match sim.view {
            ViewFrame::Lift => s::FRAME_STATUS_LIFT,
            ViewFrame::External => s::FRAME_STATUS_EXTERNAL,
        }
    )));
    ui.label(egui::RichText::new(format!(
        "{} = {} {} / {} {}",
        s::LABEL_TIME,
        s::fmt_fixed(clock.t, 2),
        s::LABEL_UNIT_S,
        s::fmt_fixed(clock.duration, 2),
        s::LABEL_UNIT_S
    )).monospace());
}

/// Нижняя панель: таймлайн t/duration + пауза/сброс.
fn timeline_ui(ui: &mut egui::Ui, sim: &SimConfig, clock: &SimClock, events: &mut UiEvents) {
    ui.horizontal(|ui| {
        ui.monospace(format!(
            "{} {} / {} {}",
            s::LABEL_TIME,
            s::fmt_fixed(clock.t, 2),
            s::fmt_fixed(clock.duration, 2),
            s::LABEL_UNIT_S
        ));
        let frac = if clock.duration > 0.0 {
            (clock.t / clock.duration) as f32
        } else {
            0.0
        };
        ui.add(
            egui::ProgressBar::new(frac.clamp(0.0, 1.0))
                .show_percentage()
                .desired_height(14.0),
        );
        if ui.button(s::BTN_RESTART).clicked() {
            events.reset = true;
        }
        if ui
            .button(if clock.running { s::BTN_PAUSE } else { s::BTN_RESUME })
            .clicked()
        {
            events.toggle_pause = true;
        }
        ui.label(egui::RichText::new(match sim.view {
            ViewFrame::Lift => s::FRAME_LIFT,
            ViewFrame::External => s::FRAME_EXTERNAL,
        }).small());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UiEvents по умолчанию — никаких действий.
    #[test]
    fn ui_events_default_is_noop() {
        let e = UiEvents::default();
        assert!(e.mode_reset.is_none() && e.mode_live.is_none());
        assert!(!e.reset && !e.pause && !e.toggle_pause && !e.shot);
        assert!(e.set_view.is_none());
    }

    /// Сброс очищает накопленные действия (после потребления sim_pre).
    #[test]
    fn ui_events_reset_clears() {
        let mut e = UiEvents::default();
        e.mode_reset = Some(Mode::Rocket);
        e.reset = true;
        e.pause = true;
        e.reset();
        assert_eq!(e.mode_reset, None);
        assert!(!e.reset && !e.pause);
    }

    /// Панели по умолчанию раскрыты, теория закрыта.
    #[test]
    fn panel_state_defaults() {
        let p = PanelState::default();
        assert!(p.left_open && p.right_open && !p.theory_open);
    }

    /// Соответствие индексов и режимов (для строк и радиокнопок).
    #[test]
    fn mode_index_roundtrip() {
        for i in 0..5 {
            assert_eq!(mode_index(index_mode(i)), i);
        }
        assert_eq!(index_mode(0), Mode::Planet);
        assert_eq!(index_mode(4), Mode::Custom);
    }
}
