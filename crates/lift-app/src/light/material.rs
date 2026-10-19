//! Цвета луча (04-light §3.5). UI-материалы этапа 6 возьмут их же.

/// Призрачная траектория (вся форма сразу) — тусклая.
pub const COLOR_GHOST: (f32, f32, f32) = (0.55, 0.52, 0.15);
/// Активный след — яркий.
pub const COLOR_TRAIL: (f32, f32, f32) = (1.0, 0.85, 0.10);
/// Голова фотона.
pub const COLOR_HEAD: (f32, f32, f32) = (1.0, 1.0, 0.75);
/// Маркер попадания на стене.
pub const COLOR_IMPACT: (f32, f32, f32) = (1.0, 0.40, 0.10);

#[cfg(test)]
mod tests {
    use super::*;

    /// Все цвета различимы и лежат в [0, 1].
    #[test]
    fn colors_distinct_and_valid() {
        let all = [COLOR_GHOST, COLOR_TRAIL, COLOR_HEAD, COLOR_IMPACT];
        for &(r, g, b) in &all {
            for v in [r, g, b] {
                assert!((0.0..=1.0).contains(&v));
            }
        }
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                assert_ne!(all[i], all[j], "{i} == {j}");
            }
        }
    }
}
