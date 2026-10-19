//! Состояние учебного сценария (T06 §3–4): prediction before
//! observation. Пользователь сначала отвечает, потом видит эксперимент.
//! Неправильный ответ не наказывается и не блокирует прогресс.

use bevy::prelude::Resource;
use lift_core::{Mode, ViewFrame};

/// Один шаг урока: вопрос, варианты, правильный ответ, объяснение.
pub struct LessonStep {
    pub title: &'static str,
    /// Режим, в который шаг ставит эксперимент при входе.
    pub mode: Mode,
    /// Кадр, в который шаг переключает вид.
    pub view: ViewFrame,
    /// Шаг начинает с выстрела луча (шаг 5).
    pub fire_beam: bool,
    pub question: &'static str,
    pub options: &'static [&'static str],
    pub correct: usize,
    /// Пояснение после раскрытия (показывается и при верном, и при
    /// неверном ответе).
    pub reveal: &'static str,
    /// Дополнительный вопрос (шаг 4: «А гравитация исчезла?»).
    pub follow_up: Option<FollowUp>,
}

pub struct FollowUp {
    pub question: &'static str,
    pub options: &'static [&'static str],
    pub correct: usize,
    pub reveal: &'static str,
}

/// Ответ пользователя: индекс варианта + верность.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepResult {
    pub step: usize,
    pub chosen: usize,
    pub correct: bool,
}

/// Правильный/неправильный — инвариант: `chosen == correct` ⇔ correct.
/// Оценка считается здесь, чтобы UI не дублировал логику.
pub fn is_correct(step: &LessonStep, chosen: usize) -> bool {
    chosen == step.correct
}

/// Состояние урока (T06 §4).
#[derive(Resource)]
pub struct LessonState {
    /// false — песочница (свободный режим, без вопросов).
    pub active: bool,
    pub step: usize,
    pub predicted: Option<usize>,
    pub revealed: bool,
    pub follow_predicted: Option<usize>,
    pub follow_revealed: bool,
    pub history: Vec<StepResult>,
    /// Шаг 5: выполнено дополнительное действие «Rocket → парабола».
    pub rocket_parabola_done: bool,
}

impl Default for LessonState {
    /// Приложение стартует с урока (шаг 1); песочница — по явному выбору
    /// (критерий T06: «Песочница доступна без прохождения урока»).
    fn default() -> Self {
        Self::new()
    }
}

impl LessonState {
    /// Урок с первого шага: режим/кадр шага применит `sim_pre`.
    pub fn new() -> Self {
        Self {
            active: true,
            step: 0,
            predicted: None,
            revealed: false,
            follow_predicted: None,
            follow_revealed: false,
            history: Vec::new(),
            rocket_parabola_done: false,
        }
    }

    /// Песочница: урок пропущен, вопросы скрыты.
    pub fn sandbox() -> Self {
        Self {
            active: false,
            ..Self::default()
        }
    }

    pub fn step_def(&self) -> &'static LessonStep {
        &LESSON_STEPS[self.step]
    }

    /// Ответ на основной вопрос. Повторный ответ не перезаписывает
    /// первый (fixация истории) — «Далее» уже разблокирован.
    /// В песочнице вопросы не задаются — ответ игнорируется.
    pub fn answer(&mut self, chosen: usize) {
        if self.revealed || !self.active {
            return;
        }
        let step = self.step_def();
        let result = StepResult {
            step: self.step,
            chosen,
            correct: is_correct(step, chosen),
        };
        self.history.retain(|r| r.step != self.step);
        self.history.push(result);
        self.predicted = Some(chosen);
        self.revealed = true;
    }

    /// Ответ на дополнительный вопрос (если он есть и ещё не дан).
    /// В песочнице игнорируется.
    pub fn answer_follow_up(&mut self, chosen: usize) {
        if !self.active
            || self.follow_revealed
            || self.step_def().follow_up.is_none()
        {
            return;
        }
        self.follow_predicted = Some(chosen);
        self.follow_revealed = true;
    }

    /// «Далее» доступен только после раскрытия основного вопроса
    /// (критерий: предсказание обязательно, но не блокирует).
    pub fn can_advance(&self) -> bool {
        self.revealed
    }

    pub fn has_next(&self) -> bool {
        self.step + 1 < LESSON_STEPS.len()
    }

    /// Переход к следующему шагу. Возвращает false, если урок завершён.
    /// Режим/кадр нового шага применит `sim_pre` через `UiEvents`.
    pub fn advance(&mut self) -> bool {
        if !self.can_advance() || !self.has_next() {
            return false;
        }
        self.step += 1;
        self.predicted = None;
        self.revealed = false;
        self.follow_predicted = None;
        self.follow_revealed = false;
        self.rocket_parabola_done = false;
        true
    }

    pub fn can_retreat(&self) -> bool {
        self.step > 0
    }

    pub fn retreat(&mut self) -> bool {
        if !self.can_retreat() {
            return false;
        }
        self.step -= 1;
        self.predicted = None;
        self.revealed = false;
        self.follow_predicted = None;
        self.follow_revealed = false;
        self.rocket_parabola_done = false;
        true
    }

    /// Статистика прохождения: (верных, всего отвеченных).
    pub fn score(&self) -> (usize, usize) {
        (
            self.history.iter().filter(|r| r.correct).count(),
            self.history.len(),
        )
    }
}

// ---- Сценарий из 5 шагов (06-ui-pedagogy §3) ----

pub static LESSON_STEPS: [LessonStep; 5] = [
    // Шаг 1: «Что чувствует наблюдатель?»
    LessonStep {
        title: "Что чувствует наблюдатель?",
        mode: Mode::Planet,
        view: ViewFrame::Lift,
        fire_beam: false,
        question: "Что покажут весы под грузом 1 кг?",
        options: &["0 Н", "9.81 Н", "1.962 Н", "Зависит от материала"],
        correct: 1,
        reveal: "N = m·g. Для 1 кг — 9.81 Н. Проверьте: три груза покажут \
                 9.81 / 49.05 / 98.10 Н — сила растёт пропорционально массе, \
                 движение всех трёх при этом одинаковое.",
        follow_up: None,
    },
    // Шаг 2: «Ракета вместо планеты»
    LessonStep {
        title: "Ракета вместо планеты",
        mode: Mode::Rocket,
        view: ViewFrame::Lift,
        fire_beam: false,
        question: "Покажание весов изменится?",
        options: &["Станет 0 Н", "Останется 9.81 Н", "Станет 19.62 Н"],
        correct: 1,
        reveal: "Показание то же самое: в системе ракеты всё падает вниз с \
                 g_eff = −a_lift, весы дают N = m·a = 9.81 Н. Локально \
                 ускорение и гравитация неразличимы.",
        follow_up: None,
    },
    // Шаг 3: «Взгляд снаружи»
    LessonStep {
        title: "Взгляд снаружи",
        mode: Mode::Rocket,
        view: ViewFrame::External,
        fire_beam: false,
        question: "Что делает мяч во внешнем кадре?",
        options: &["Падает вниз", "Стоит на месте", "Ускоряется вверх"],
        correct: 1,
        reveal: "Во внешнем инерциальном кадре на мяч не действует ничто: он \
                 сохраняет скорость и висит на месте, а пол кабины ускоряется \
                 ему навстречу. Одно и то же событие — два разных описания: \
                 снаружи «пол догоняет мяч», внутри «мяч падает на пол».",
        follow_up: None,
    },
    // Шаг 4: «Свободное падение»
    LessonStep {
        title: "Свободное падение",
        mode: Mode::FreeFall,
        view: ViewFrame::Lift,
        fire_beam: false,
        question: "Что покажут весы?",
        options: &["9.81 Н", "0 Н", "19.62 Н"],
        correct: 1,
        reveal: "0 Н — невесомость: кабина и грузы падают с одинаковым \
                 ускорением, груз ничего не «давит» на чашу.",
        follow_up: Some(FollowUp {
            question: "А гравитация исчезла?",
            options: &[
                "Да — в кабине поля больше нет",
                "Нет — кабина и всё внутри падают вместе",
            ],
            correct: 1,
            reveal: "Внешний наблюдатель видит: поле действует, все тела \
                     ускоряются вниз на 9.81. Просто между грузом и чашей \
                     больше нет взаимного давления. Невесомость ≠ отсутствие \
                     гравитации (режим «Инерция» — контроль без поля).",
        }),
    },
    // Шаг 5: «Свет в падающей кабине»
    LessonStep {
        title: "Свет в падающей кабине",
        mode: Mode::FreeFall,
        view: ViewFrame::Lift,
        fire_beam: true,
        question: "Изогнётся ли луч внутри кабины?",
        options: &["Да, вниз", "Да, вверх", "Нет, будет прямым"],
        correct: 2,
        reveal: "Луч идёт прямо: кабина падает вместе с источником и стеной, \
                 g_eff = 0. Теперь переключитесь в ракету — там луч изогнётся \
                 параболой при той же невесомости экипажа.",
        follow_up: None,
    },
];

/// Финальный вывод урока (шаг 5, после «Rocket → парабола»).
pub const FINAL_CONCLUSION: &str = "Если в свободно падающей системе свет идёт \
    прямо, а относительно Земли — нет, то гравитацию можно описывать как \
    геометрию пространства-времени. Это и есть путь к общей теории \
    относительности.";

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> LessonState {
        LessonState::new()
    }

    /// Инварианты сценария: у каждого шага корректный индекс ответа,
    /// у шага 4 — дополнительный вопрос, у шага 5 — выстрел луча.
    #[test]
    fn scenario_invariants() {
        for (i, step) in LESSON_STEPS.iter().enumerate() {
            assert!(step.correct < step.options.len(), "шаг {i}");
            assert!(!step.question.is_empty() && !step.reveal.is_empty());
        }
        assert!(LESSON_STEPS[0].follow_up.is_none());
        assert!(LESSON_STEPS[3].follow_up.is_some());
        // Шаг 5 начинает с выстрела луча (FreeFall → прямая).
        assert!(LESSON_STEPS[4].fire_beam);
        // Шаги 1→2: Planet → Rocket со сбросом; шаг 3 — внешний кадр.
        assert_eq!(LESSON_STEPS[0].mode, Mode::Planet);
        assert_eq!(LESSON_STEPS[1].mode, Mode::Rocket);
        assert_eq!(LESSON_STEPS[2].view, ViewFrame::External);
        assert_eq!(LESSON_STEPS[3].mode, Mode::FreeFall);
        assert_eq!(LESSON_STEPS[4].mode, Mode::FreeFall);
    }

    /// Правильные ответы сценария (контракт T06 §3).
    #[test]
    fn correct_answers_match_spec() {
        assert_eq!(LESSON_STEPS[0].correct, 1); // 9.81 Н
        assert_eq!(LESSON_STEPS[1].correct, 1); // останется 9.81
        assert_eq!(LESSON_STEPS[2].correct, 1); // стоит на месте
        assert_eq!(LESSON_STEPS[3].correct, 1); // 0 Н
        assert_eq!(LESSON_STEPS[3].follow_up.as_ref().unwrap().correct, 1); // не исчезла
        assert_eq!(LESSON_STEPS[4].correct, 2); // прямой
    }

    /// T06: на каждом шаге сначала запрашивается предсказание —
    /// «Далее» заблокирован до ответа.
    #[test]
    fn advance_blocked_before_prediction() {
        let mut s = fresh();
        assert!(!s.can_advance());
        assert!(!s.advance());
        assert_eq!(s.step, 0);
    }

    /// T06: неправильный ответ не блокирует прогресс.
    #[test]
    fn wrong_answer_does_not_block() {
        let mut s = fresh();
        s.answer(0); // неверно (правильно 1)
        assert!(s.revealed);
        assert!(s.can_advance());
        assert_eq!(s.score(), (0, 1));
        s.answer_follow_up(0);
        // follow_up без дополнительного вопроса — игнорируется.
        assert!(!s.follow_revealed);
        assert!(s.advance());
        assert_eq!(s.step, 1);
    }

    /// Повторный ответ не перезаписывает историю.
    #[test]
    fn repeated_answer_keeps_first_result() {
        let mut s = fresh();
        s.answer(1);
        s.answer(0);
        assert_eq!(s.predicted, Some(1));
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.score(), (1, 1));
    }

    /// Полное прохождение: 5 шагов от начала до конца.
    #[test]
    fn full_run_completes() {
        let mut s = fresh();
        for i in 0..LESSON_STEPS.len() {
            assert_eq!(s.step, i);
            s.answer(LESSON_STEPS[s.step].correct);
            if i == 3 {
                s.answer_follow_up(LESSON_STEPS[3].follow_up.as_ref().unwrap().correct);
            }
            if i < LESSON_STEPS.len() - 1 {
                assert!(s.advance());
            }
        }
        assert!(!s.has_next());
        assert!(!s.advance());
        assert_eq!(s.score(), (5, 5));
    }

    /// Возврат назад сбрасывает состояние шага.
    #[test]
    fn retreat_resets_step_state() {
        let mut s = fresh();
        s.answer(1);
        assert!(s.advance());
        assert!(s.retreat());
        assert_eq!(s.step, 0);
        assert!(!s.revealed);
        assert_eq!(s.predicted, None);
        // История ответов сохраняется.
        assert_eq!(s.score(), (1, 1));
    }

    /// Песочница не задаёт вопросов и не накапливает историю.
    #[test]
    fn sandbox_has_no_questions() {
        let mut s = LessonState::sandbox();
        assert!(!s.active);
        s.answer(1);
        assert!(!s.revealed);
        assert!(s.history.is_empty());
    }

    /// Шаг 5: флаг «Rocket → парабола» сбрасывается при навигации.
    #[test]
    fn rocket_flag_resets_on_navigation() {
        let mut s = fresh();
        s.answer(LESSON_STEPS[0].correct);
        assert!(s.advance()); // шаг 1
        s.rocket_parabola_done = true;
        s.retreat(); // шаг 0: флаг сброшен
        assert!(!s.rocket_parabola_done);
        s.rocket_parabola_done = true;
        s.answer(LESSON_STEPS[s.step].correct);
        s.advance();
        assert!(!s.rocket_parabola_done);
    }
}
