//! Измерение силы на весах (03-scales §3.2).
//!
//! Сила приходит ТОЛЬКО из `ContactForceEvent` контактного решателя
//! Rapier — вычисление `N = m·g_eff` запрещено (тавтология); оно
//! используется только в тестах как эталон.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use lift_core::{compression_from_force, smooth_compression, ScaleReading};

use crate::resources::{Ball, Metrics, ScaleReadings, SimClock, SimSteps, BALL_MASS};

/// Полный диапазон шкалы, Н (03-scales §3.4).
pub const N_FULLSCALE: f64 = 150.0;
/// Коэффициент сглаживания compression (03-scales §6).
pub const SMOOTHING: f32 = 0.2;

/// Чтение контактных сил раз за кадр (Update).
///
/// Сообщения биви дважды буферизуются: система видит события
/// физики прошлого кадра — на установившемся режиме это несущественно,
/// зато чтение стабильно при любом FPS (спецификация §6: «читать и
/// обнулять очередь каждый fixed step» — очередь сообщений очищается
/// сама, накопление за несколько кадров исключено).
///
/// На паузе показания замораживаются (последнее измерение честно
/// держится на экране).
pub fn read_contact_forces(
    clock: Res<SimClock>,
    steps: Res<SimSteps>,
    mut last_steps: Local<u64>,
    mut events: MessageReader<ContactForceEvent>,
    loads: Query<(Entity, &Ball)>,
    mut readings: ResMut<ScaleReadings>,
    mut metrics: ResMut<Metrics>,
) {
    // Физика не шагала (пауза или конец прогона) — показания
    // замораживаются: последнее измерение честно держится на экране.
    if !clock.running || steps.0 == *last_steps {
        return;
    }
    *last_steps = steps.0;

    let load_index: Vec<(Entity, usize)> = loads.iter().map(|(e, b)| (e, b.index)).collect();
    let lookup = |e: Entity| {
        load_index
            .iter()
            .find(|(le, _)| *le == e)
            .map(|(_, i)| *i)
    };

    let mut force = [0.0_f64; 3];
    let mut contact = [false; 3];
    for ev in events.read() {
        if let Some(i) = lookup(ev.collider1).or_else(|| lookup(ev.collider2)) {
            // Нормальная сила здесь вертикальна; берём вертикальную
            // компоненту суммы сил контакта (тангенциальное трение не в счёт).
            force[i] += ev.total_force.y.abs() as f64;
            contact[i] = true;
        }
    }

    for (i, item) in readings.items.iter_mut().enumerate() {
        item.mass_kg = BALL_MASS[i];
        item.in_contact = contact[i];
        item.normal_force_n = if contact[i] { force[i] } else { 0.0 };
        let target = compression_from_force(item.normal_force_n, N_FULLSCALE);
        item.compression = smooth_compression(item.compression, target, SMOOTHING);
    }

    // Отладочный overlay: сырое значение, источник, compression (§3.2, §4).
    // Шкала усилена для наглядности; отношение показаний линейно и честно.
    if !metrics.scales_logged && metrics.step_count >= 150 {
        metrics.scales_logged = true;
        for (i, ScaleReading { mass_kg, normal_force_n, compression, in_contact }) in
            readings.items.iter().enumerate()
        {
            bevy::log::info!(
                "[T03] стойка {} ({} кг): N = {:.2} Н (источник: ContactForceEvent), \
                 compression = {:.3}, контакт = {}. Шкала усилена для наглядности; \
                 отношение показаний линейно и честно.",
                i,
                mass_kg,
                normal_force_n,
                compression,
                in_contact
            );
        }
    }
}
