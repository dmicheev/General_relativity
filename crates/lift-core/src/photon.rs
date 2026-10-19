use crate::frame::FrameParams;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotonLaunch {
    pub origin: [f64; 3],
    pub c: f64,
}

pub fn photon_external(t: f64, launch: &PhotonLaunch, external_g: f64) -> [f64; 3] {
    let p = launch.origin;
    [p[0] + launch.c * t, p[1] - 0.5 * external_g * t * t, p[2]]
}

pub fn photon_lift(t: f64, launch: &PhotonLaunch, frame: &FrameParams) -> [f64; 3] {
    let p = launch.origin;
    [
        p[0] + launch.c * t,
        p[1] - 0.5 * (frame.external_g + frame.lift_a) * t * t,
        p[2],
    ]
}

/// Аналитическая точка попадания в правую стену (04-light §3.4):
/// `T = (x_wall − x₀)/c`, без дискретного шага — точка попадания
/// не зависит от частоты кадров.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotonImpact {
    pub t: f64,
    pub external: [f64; 3],
    pub lift: [f64; 3],
}

pub fn photon_impact(
    launch: &PhotonLaunch,
    frame: &FrameParams,
    x_wall: f64,
) -> Option<PhotonImpact> {
    if launch.c <= 0.0 {
        return None;
    }
    let t = (x_wall - launch.origin[0]) / launch.c;
    if t < 0.0 {
        return None;
    }
    Some(PhotonImpact {
        t,
        external: photon_external(t, launch, frame.external_g),
        lift: photon_lift(t, launch, frame),
    })
}

pub fn rindler_null_y(x: f64, a: f64, c: f64) -> Option<f64> {
    if c <= 0.0 {
        return None;
    }
    if a == 0.0 {
        return Some(0.0);
    }
    let u = a * x / (c * c);
    if u.abs() >= 1.0 {
        return None;
    }
    let s = (1.0 - u * u).sqrt();
    Some(-(c * c / a) * (u * u) / (s + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Mode;

    const G: f64 = 9.81;

    fn laser() -> PhotonLaunch {
        PhotonLaunch {
            origin: [0.0, 1.8, 0.0],
            c: 10.0,
        }
    }

    #[test]
    fn t01_09_freefall_photon_is_horizontal() {
        let frame = Mode::FreeFall.params();
        let launch = laser();
        for i in 0..100 {
            let t = i as f64 / 60.0;
            assert_eq!(photon_lift(t, &launch, &frame)[1], 1.8);
        }
    }

    #[test]
    fn t01_10_rocket_photon_matches_parabola() {
        let frame = Mode::Rocket.params();
        let launch = laser();
        for i in 0..100 {
            let t = i as f64 / 60.0;
            let p = photon_lift(t, &launch, &frame);
            let expected_y = 1.8 - 0.5 * frame.g_eff().abs() * t * t;
            assert_eq!(p[1], expected_y);
            assert_eq!(p[0], launch.c * t);
        }
    }

    #[test]
    fn t01_11_rindler_matches_parabola_for_small_argument() {
        let a = 1.0;
        let c = 100.0;
        for &u in &[1e-6, 1e-5, 1e-4, 1e-3] {
            let x = u * c * c / a;
            let exact = rindler_null_y(x, a, c).unwrap();
            let parabola = -a * x * x / (2.0 * c * c);
            let rel_err = ((exact - parabola) / parabola).abs();
            assert!(rel_err < 1e-6, "u = {u}: rel_err = {rel_err}");
        }
    }

    #[test]
    fn t01_12_rindler_none_outside_domain() {
        let a = 1.0;
        let c = 10.0;
        assert_eq!(rindler_null_y(c * c / a, a, c), None);
        assert_eq!(rindler_null_y(-(c * c / a), a, c), None);
        assert_eq!(rindler_null_y(2.0 * c * c / a, a, c), None);
    }

    #[test]
    fn rindler_at_origin_is_zero() {
        assert_eq!(rindler_null_y(0.0, 1.0, 10.0), Some(0.0));
    }

    #[test]
    fn rindler_flat_limit_when_a_is_zero() {
        assert_eq!(rindler_null_y(5.0, 0.0, 10.0), Some(0.0));
    }

    #[test]
    fn rindler_rejects_nonpositive_c() {
        assert_eq!(rindler_null_y(1.0, 1.0, 0.0), None);
        assert_eq!(rindler_null_y(1.0, 1.0, -10.0), None);
    }

    #[test]
    fn photon_external_rocket_is_straight() {
        let launch = laser();
        for i in 0..100 {
            let t = i as f64 / 60.0;
            assert_eq!(photon_external(t, &launch, 0.0)[1], 1.8);
        }
    }

    #[test]
    fn photon_external_planet_falls() {
        let launch = laser();
        let p = photon_external(0.3, &launch, G);
        assert!(p[1] < 1.8);
        assert_eq!(p[1], 1.8 - 0.5 * G * 0.3 * 0.3);
        assert_eq!(p[0], 10.0 * 0.3);
    }

    #[test]
    fn photon_lift_matches_spec_table_for_all_modes() {
        let launch = laser();
        let cases = [
            (Mode::Planet, false),
            (Mode::Rocket, false),
            (Mode::FreeFall, true),
            (Mode::Inertial, true),
        ];
        for (mode, is_straight) in cases {
            let frame = mode.params();
            let p = photon_lift(0.3, &launch, &frame);
            assert_eq!(p[1] == 1.8, is_straight, "mode: {mode:?}");
        }
    }

    // ---- T04 ----

    /// T04.1: photon_lift(Rocket): y(0) == y₀.
    #[test]
    fn t04_1_lift_y_at_zero_is_origin() {
        let frame = Mode::Rocket.params();
        let p = photon_lift(0.0, &laser(), &frame);
        assert_eq!(p[1], 1.8);
        assert_eq!(p[0], 0.0);
    }

    /// T04.2: photon_lift(Rocket): y(x) == y₀ − a·x²/(2c²), rel_err < 1e-12.
    #[test]
    fn t04_2_lift_rocket_matches_parabola_in_x() {
        let frame = Mode::Rocket.params();
        let launch = laser();
        let a = frame.lift_a;
        let c = launch.c;
        for i in 1..=100 {
            let t = i as f64 * 0.003;
            let p = photon_lift(t, &launch, &frame);
            let x = p[0] - launch.origin[0];
            let expected = 1.8 - a * x * x / (2.0 * c * c);
            let rel_err = ((p[1] - expected) / expected).abs();
            assert!(rel_err < 1e-12, "t={t}: {rel_err:e}");
        }
    }

    /// T04.3: photon_lift(FreeFall): y(t) == y₀ для 1000 значений t.
    #[test]
    fn t04_3_lift_freefall_is_straight_1000() {
        let frame = Mode::FreeFall.params();
        let launch = laser();
        for i in 0..1000 {
            let t = i as f64 * 0.001;
            assert_eq!(photon_lift(t, &launch, &frame)[1], 1.8);
        }
    }

    /// T04.4: photon_external(Rocket): y(t) == y₀ всегда.
    #[test]
    fn t04_4_external_rocket_is_straight_1000() {
        let launch = laser();
        for i in 0..1000 {
            let t = i as f64 * 0.001;
            assert_eq!(photon_external(t, &launch, 0.0)[1], 1.8);
        }
    }

    /// T04.5: photon_external(FreeFall): y(t) == y₀ − ½g·t².
    #[test]
    fn t04_5_external_freefall_falls() {
        let launch = laser();
        for i in 0..1000 {
            let t = i as f64 * 0.001;
            let p = photon_external(t, &launch, G);
            assert_eq!(p[1], 1.8 - 0.5 * G * t * t);
        }
    }

    /// T04.6: T_impact = (x_wall − x₀)/c ровно.
    /// Геометрия §3.1: эмиттер x = −1.45, стена x = +1.5 ⇒ W = 2.95 м
    /// (числа «T = 3.0/c» в спеке считали W = 3.0 — следуем формуле §3.4).
    #[test]
    fn t04_6_impact_time_is_width_over_c() {
        let frame = Mode::Planet.params();
        let launch = PhotonLaunch {
            origin: [-1.45, 1.8, 0.0],
            c: 10.0,
        };
        let impact = photon_impact(&launch, &frame, 1.5).unwrap();
        assert_eq!(impact.t.to_bits(), (2.95_f64 / 10.0).to_bits());
    }

    /// T04.7: y_impact_lift совпадает с формулой параболы.
    #[test]
    fn t04_7_impact_y_matches_parabola() {
        let frame = Mode::Planet.params();
        let launch = PhotonLaunch {
            origin: [-1.45, 1.8, 0.0],
            c: 10.0,
        };
        let impact = photon_impact(&launch, &frame, 1.5).unwrap();
        let expected = 1.8 - 0.5 * frame.g_eff().abs() * impact.t * impact.t;
        assert_eq!(impact.lift[1], expected);
        // Δy = ½·9.81·0.295² ≈ 0.42686 м (см. T04.6: W = 2.95 м).
        let dy = 1.8 - impact.lift[1];
        assert!((dy - 0.5 * 9.81 * 0.295 * 0.295).abs() < 1e-12, "{dy}");
        assert_eq!(impact.lift, impact.external);
    }

    /// T04.8: Planet.lift == Rocket.lift побитово при одинаковых c и геометрии.
    #[test]
    fn t04_8_planet_and_rocket_lift_paths_bitwise_equal() {
        let launch = laser();
        for i in 0..100 {
            let t = i as f64 * 0.003;
            let a = photon_lift(t, &launch, &Mode::Planet.params());
            let b = photon_lift(t, &launch, &Mode::Rocket.params());
            assert_eq!(a[1].to_bits(), b[1].to_bits());
            assert_eq!(a[0].to_bits(), b[0].to_bits());
        }
    }

    /// T04.10: инвариантность: p_external − D == p_lift для 1000 «случайных» t
    /// (детерминированный LCG вместо внешнего генератора).
    #[test]
    fn t04_10_external_minus_displacement_equals_lift() {
        let launch = PhotonLaunch {
            origin: [-1.45, 1.8, 0.0],
            c: 7.5,
        };
        let frame = FrameParams {
            external_g: 4.2,
            lift_a: -1.7,
        };
        let mut seed: u64 = 0x243F6A8885A308D3;
        for _ in 0..1000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let t = (seed >> 11) as f64 / (1u64 << 53) as f64 * 0.5;
            let ext = photon_external(t, &launch, frame.external_g);
            let lift = photon_lift(t, &launch, &frame);
            let d = frame.displacement(t);
            assert_eq!(ext[0], lift[0]);
            // Математический инвариант p_ext − D == p_lift; побитового
            // равенства нет из-за разной ассоциации операций (1 ulp).
            let diff = ext[1] - d;
            assert!(
                (diff - lift[1]).abs() <= 1e-12 * lift[1].abs().max(1.0),
                "t = {t}: {} vs {}",
                diff,
                lift[1]
            );
        }
    }

    /// 100 «выстрелов» (повторных вызовов с t от 0) дают идентичные
    /// траектории — накопительного дрейфа нет.
    #[test]
    fn t04_100_shots_no_drift() {
        let frame = Mode::Planet.params();
        let launch = PhotonLaunch {
            origin: [-1.45, 1.8, 0.0],
            c: 10.0,
        };
        let reference = photon_impact(&launch, &frame, 1.5).unwrap();
        for _ in 0..100 {
            let impact = photon_impact(&launch, &frame, 1.5).unwrap();
            for k in 0..3 {
                assert_eq!(impact.lift[k].to_bits(), reference.lift[k].to_bits());
            }
        }
    }
}
