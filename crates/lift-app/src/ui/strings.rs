//! Все строки интерфейса (T06 §7). Русский язык, единый модуль вместо
//! JSON-ассетов: compile-time, без serde и загрузки файлов в wasm
//! (отклонение от списка артефактов зафиксировано в 00-physics-spec).
//!
//! Форматирование чисел — фиксированная точность и явный знак:
//! ширина текста не «прыгает» (T06 §6, критерий приёмки).

/// Названия режимов (00-physics-spec §2).
pub const MODE_NAMES: [&str; 5] = [
    "На планете",
    "В ракете",
    "Свободное падение",
    "Инерция",
    "Свой",
];

/// Полные описания режима — первая строка живой панели.
pub const MODE_DESCRIPTIONS: [&str; 5] = [
    "Кабина стоит на планете, g = 9.81 м/с²",
    "Ракета ускоряется в пустоте, a = +9.81 м/с²",
    "Кабина свободно падает в поле g = 9.81",
    "Ни поля, ни ускорения — инерциальный покой",
    "Эксперимент: слайдеры g_ext и a_lift",
];

/// Объяснение физики режима — после вопроса/в песочнице.
pub const MODE_PHYSICS: [&str; 5] = [
    "На тело действует поле: свободное падение с g_eff = −g_ext. \
     Весы показывают N = m·g.",
    "Внешнего поля нет, но кабина ускоряется вверх. В её системе \
     всё падает вниз с тем же g_eff — ускорение неотличимо от гравитации.",
    "Кабина и всё внутри падают вместе: g_eff = 0, весы молчат. \
     Это весомость уходит, а не гравитация.",
    "Контрольный режим: нет ни гравитации, ни ускорения. \
     Тела висят, весы молчат — как и в свободном падении.",
    "Поле остаётся однородным — меняются только величины \
     g_ext и a_lift (не геометрия поля).",
];

pub const TITLE: &str = "Лифт Эйнштейна — принцип эквивалентности";

pub const SECTION_MODE: &str = "РЕЖИМ";
pub const SECTION_FRAME: &str = "КАДР";
pub const SECTION_PARAMS: &str = "ПАРАМЕТРЫ";
pub const SECTION_CONTROL: &str = "УПРАВЛЕНИЕ";
pub const SECTION_LESSON: &str = "УРОК";
pub const SECTION_STATUS: &str = "ЧТО ПРОИСХОДИТ";
pub const SECTION_SCALES: &str = "ВЕСЫ";
pub const SECTION_LIGHT: &str = "СВЕТ";
pub const SECTION_PREDICT: &str = "ПРЕДСКАЗАНИЕ";

pub const FRAME_LIFT: &str = "Внутри";
pub const FRAME_EXTERNAL: &str = "Снаружи";
pub const FRAME_STATUS_LIFT: &str = "Внутри кабины";
pub const FRAME_STATUS_EXTERNAL: &str = "Внешний наблюдатель";

pub const BTN_PAUSE: &str = "⏸ Пауза";
pub const BTN_RESUME: &str = "▶ Пуск";
pub const BTN_RESTART: &str = "↺ Сброс";
pub const BTN_FIRE: &str = "L — Луч";
pub const BTN_CLEAR_BEAM: &str = "C — Стереть луч";
pub const BTN_GHOST: &str = "G — Пунктир";
pub const BTN_LESSON_RESTART: &str = "Пройти урок заново";
pub const BTN_SANDBOX: &str = "Песочница (без урока)";
pub const BTN_NEXT: &str = "Далее →";
pub const BTN_PREV: &str = "← Назад";
pub const BTN_RUN: &str = "▶ Запустить эксперимент";
pub const BTN_THEORY: &str = "Теория";
pub const BTN_SWITCH_ROCKET: &str = "Переключить в ракету (сброс)";

pub const LABEL_MODE_LIVE: &str = "На лету (контрфактический):";
pub const HINT_HOMOGENEOUS: &str =
    "Поле остаётся однородным: мы меняем только его величину, не геометрию.";

pub const LABEL_G_EXT: &str = "g_ext";
pub const LABEL_A_LIFT: &str = "a_lift";
pub const LABEL_C_VIS: &str = "c_vis";
pub const LABEL_DURATION: &str = "Длительность";

pub const PRESET_EARTH: &str = "Земля 9.81";
pub const PRESET_MOON: &str = "Луна 1.62";
pub const PRESET_MARS: &str = "Марс 3.71";
pub const PRESET_JUPITER: &str = "Юпитер 24.79";

pub const LABEL_UNIT_G: &str = "м/с²";
pub const LABEL_UNIT_N: &str = "Н";
pub const LABEL_UNIT_M: &str = "м";
pub const LABEL_UNIT_S: &str = "с";
pub const LABEL_DT: &str = "D(t)";
pub const LABEL_KG: &str = "кг";
pub const LABEL_SLOWED: &str = "замедлено";
pub const LABEL_BEAM_OFF: &str = "Луч не выпущен (L или кнопка «Луч»)";
pub const LABEL_TIME: &str = "t";

pub const SANDBOX_TITLE: &str = "Свободный режим";

pub const LESSON_LABEL: &str = "Шаг";
pub const LESSON_OF: &str = "из";
pub const LESSON_COMPLETE: &str = "Урок пройден!";
pub const LESSON_HINT_PREDICT: &str = "Сначала предскажите результат, потом проверьте экспериментом.";
pub const LESSON_FOLLOW_UP: &str = "Дополнительный вопрос";

pub const ANSWER_CORRECT: &str = "✓ Верно!";
/// Текст на неверный ответ (T06 §4: неправильный ответ не наказывается).
pub const ANSWER_WRONG: &str = "✗ Это интуитивно ожидаемый ответ. Давайте посмотрим, что говорит эксперимент.";

pub const TOOLTIP_MODE_RESET: &str = "Сменить режим со сбросом эксперимента";
pub const TOOLTIP_MODE_LIVE: &str = "Контрфактический эксперимент: смена режима на лету, без сброса";

pub const THEORY_TEXT: &str = "\
Принцип эквивалентности: в достаточно малой области пространства-времени \
результаты всех локальных экспериментов в однородном гравитационном поле \
совпадают с результатами тех же экспериментов в равноускоренной системе \
отсчёта без гравитации.\n\n\
Пары режимов: «Планета» ↔ «Ракета» — ускорение имитирует гравитацию; \
«Падение» ↔ «Инерция» — свободное падение имитирует отсутствие гравитации.\n\n\
Границы модели: поле однородное (Δg/g ≈ 2h/R ≈ 1e-6 для кабины 3 м — \
приливные силы невидимы); траектория света — ньютонова эвристика, \
точный релятивистский результат (Риндлер) отличается при больших a·x/c².";

/// Подпись у замедленного света — обязана быть видна при активном луче
/// (критерий приёмки T06).
pub fn beam_slowdown_note() -> String {
    format!(
        "c_vis — педагогический приём: свет замедлен в ≈ {} раз. \
         Отношение форм траекторий и смысл принципа эквивалентности не меняются.",
        format_slowdown(crate::light::C_REAL / crate::light::C_VIS_DEFAULT)
    )
}

/// Фиксированная точность, без знака: `9.81`, `0.00`.
pub fn fmt_fixed(v: f64, digits: usize) -> String {
    format!("{:.*}", digits, v)
}

/// Фиксированная точность с явным знаком и типографским минусом:
/// `+9.81`, `−9.81`, `+0.00`. Ширина постоянна — вёрстка не прыгает.
pub fn fmt_signed(v: f64, digits: usize) -> String {
    let s = format!("{:.*}", digits, v.abs());
    if v < 0.0 {
        format!("−{}", s)
    } else {
        format!("+{}", s)
    }
}

/// Коэффициент замедления света: `3.0·10⁷` (мантисса 1 цифра после
/// точки, показатель — юникод-надстрочные цифры).
pub fn format_slowdown(factor: f64) -> String {
    if !(factor.is_finite() && factor > 0.0) {
        return "—".to_string();
    }
    let exp = factor.log10().floor() as i32;
    let mantissa = factor / 10f64.powi(exp);
    format!("{:.1}·10{}", mantissa, superscript(exp))
}

/// `7` → `⁷`; отрицательные: `−3` → `⁻³`.
fn superscript(exp: i32) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let mut out = String::new();
    for ch in exp.abs().to_string().chars() {
        out.push(DIGITS[ch as usize - '0' as usize]);
    }
    if exp < 0 {
        format!("⁻{}", out)
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_format_has_constant_width() {
        assert_eq!(fmt_fixed(9.81, 2), "9.81");
        assert_eq!(fmt_fixed(0.0, 2), "0.00");
        assert_eq!(fmt_fixed(98.10, 2), "98.10");
        // 0..1000 с двумя знаками: ширина растёт монотонно, без «скачков»
        // формата (разделителей тысяч и т.п. нет).
        for v in [0.0, 0.5, 9.81, 99.99, 999.99] {
            assert!(fmt_fixed(v, 2).chars().all(|c| c.is_ascii_digit() || c == '.'));
        }
    }

    #[test]
    fn signed_format_uses_explicit_sign_and_unicode_minus() {
        assert_eq!(fmt_signed(9.81, 2), "+9.81");
        assert_eq!(fmt_signed(-9.81, 2), "−9.81");
        assert_eq!(fmt_signed(0.0, 2), "+0.00");
        assert_eq!(fmt_signed(-0.0, 2), "+0.00");
        // Одинаковая ВИДИМАЯ ширина для одинаковой точности
        // (− — многобайтовый символ, сравниваем число литер).
        assert_eq!(
            fmt_signed(1.5, 3).chars().count(),
            fmt_signed(-1.5, 3).chars().count()
        );
    }

    #[test]
    fn slowdown_formats_scientific_with_superscript() {
        assert_eq!(format_slowdown(3.0e7), "3.0·10⁷");
        assert_eq!(format_slowdown(1.62e1), "1.6·10¹");
        assert_eq!(format_slowdown(5.0), "5.0·10⁰");
        assert_eq!(format_slowdown(0.5), "5.0·10⁻¹");
        assert_eq!(format_slowdown(f64::NAN), "—");
    }

    #[test]
    fn slowdown_note_mentions_real_and_visual_speed() {
        let note = beam_slowdown_note();
        assert!(note.contains("замедлен"));
        assert!(note.contains("3.0·10⁷"));
    }
}
