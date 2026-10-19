//! Пружинные весы (T03): педагогическое отображение измеренной силы.
//!
//! Сила приходит из контактного решателя (lift-app), здесь — только
//! чистая математика отображения: сжатие шкалы, масштаб пружины,
//! сглаживание.

/// Полный диапазон шкалы (03-scales §3.4): N = 150 Н → compression = 1.
pub const N_FULLSCALE: f64 = 150.0;

/// Насколько пружина сжимается при compression = 1 (03-scales §3.4).
pub const SQUEEZE: f32 = 0.65;

/// Педагогическое сжатие шкалы: `clamp(N / fullscale, 0, 1)`.
/// Линейно и честно по отношению показаний, усилено по амплитуде.
pub fn compression_from_force(force_n: f64, fullscale: f64) -> f32 {
    if fullscale <= 0.0 {
        return 0.0;
    }
    (force_n / fullscale).clamp(0.0, 1.0) as f32
}

/// Множитель высоты пружины: `1 − squeeze·compression`.
pub fn spring_scale_y(compression: f32, squeeze: f32) -> f32 {
    1.0 - squeeze * compression.clamp(0.0, 1.0)
}

/// Экспоненциальное сглаживание (03-scales §6, ловушка «сжатие прыгает»).
pub fn smooth_compression(old: f32, new: f32, k: f32) -> f32 {
    old + (new - old) * k.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: f64 = 9.81;

    /// Табличные значения 03-scales §3.4.
    #[test]
    fn t03_compression_table() {
        let cases = [
            (9.81, 0.065),
            (49.05, 0.327),
            (98.10, 0.654),
        ];
        for (n, expected) in cases {
            let c = compression_from_force(n, N_FULLSCALE);
            assert!((c - expected).abs() < 1e-3, "N={n}: {c} != {expected}");
        }
    }

    /// T03.6: compression монотонна по N.
    #[test]
    fn t03_6_compression_monotonic() {
        let forces = [0.0, 1.0, 10.0, 50.0, 100.0, 149.0, 150.0];
        let mut prev = -1.0_f32;
        for n in forces {
            let c = compression_from_force(n, N_FULLSCALE);
            assert!(c >= prev, "N={n}: {c} < {prev}");
            prev = c;
        }
    }

    /// T03.7: compression ∈ [0, 1] вплоть до 3× fullscale.
    #[test]
    fn t03_7_compression_clamped() {
        assert_eq!(compression_from_force(-5.0, N_FULLSCALE), 0.0);
        assert_eq!(compression_from_force(0.0, N_FULLSCALE), 0.0);
        assert_eq!(compression_from_force(3.0 * N_FULLSCALE, N_FULLSCALE), 1.0);
        assert_eq!(compression_from_force(10.0 * N_FULLSCALE, N_FULLSCALE), 1.0);
    }

    /// Визуальное сжатие: 10 кг → 57.5% высоты (42.5% сжатия).
    #[test]
    fn spring_height_matches_spec_table() {
        let c10 = compression_from_force(98.10, N_FULLSCALE);
        let scale = spring_scale_y(c10, SQUEEZE);
        assert!((scale - 0.575).abs() < 1e-3, "{scale}");
        assert_eq!(spring_scale_y(0.0, SQUEEZE), 1.0);
    }

    /// Эталон для тестов (разрешён spec §3.2 п.3): N = m·g_eff.
    #[test]
    fn reference_force_is_mass_times_g_eff() {
        for (m, n) in [(1.0, 9.81), (5.0, 49.05), (10.0, 98.10)] {
            assert!((m * G - n).abs() < 1e-9);
        }
    }

    #[test]
    fn smoothing_converges_without_overshoot() {
        let mut c = 0.0_f32;
        for _ in 0..64 {
            c = smooth_compression(c, 1.0, 0.2);
        }
        assert!((c - 1.0).abs() < 1e-3);
        assert!(smooth_compression(0.0, 1.0, 0.2) <= 1.0);
        assert_eq!(smooth_compression(0.5, 0.5, 0.2), 0.5);
    }
}
