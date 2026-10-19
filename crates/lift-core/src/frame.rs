#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameParams {
    pub external_g: f64,
    pub lift_a: f64,
}

impl FrameParams {
    pub fn g_eff(&self) -> f64 {
        -self.external_g - self.lift_a
    }

    pub fn displacement(&self, t: f64) -> f64 {
        0.5 * self.lift_a * t * t
    }

    /// Аналитическая траектория свободного тела в кадре лифта (T02).
    /// Тело стартует из `y0` с нулевой скоростью и движется с `g_eff`.
    pub fn lift_y(&self, y0: f64, t: f64) -> f64 {
        y0 + 0.5 * self.g_eff() * t * t
    }

    /// Та же траектория глазами внешнего инерциального наблюдателя:
    /// `p_external = p_lift + D(t)` (T02).
    pub fn external_y(&self, y0: f64, t: f64) -> f64 {
        self.lift_y(y0, t) + self.displacement(t)
    }

    /// Дискретное смещение кабины для численного интегратора
    /// semi-implicit Euler (как в Rapier): `v_k = v_{k-1} + a·dt`,
    /// `x_k = x_{k-1} + v_k·dt` ⇒ `x_n = a·dt²·n(n+1)/2`.
    ///
    /// Совпадает с `displacement` в пределе `dt → 0`, но отличается от
    /// него на `O(a·dt·t)` — ровно на «лид» интегратора. Использование
    /// дискретной версии во FrameBridge делает `p_lift + D` константой
    /// в симуляции (критерий T02.6: < 1e-3, фактически ~1e-15).
    pub fn displacement_discrete(&self, steps: u64, dt: f64) -> f64 {
        let n = steps as f64;
        self.lift_a * dt * dt * n * (n + 1.0) / 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Planet,
    Rocket,
    FreeFall,
    Inertial,
    Custom,
}

impl Mode {
    pub fn params(self) -> FrameParams {
        match self {
            Mode::Planet => FrameParams {
                external_g: 9.81,
                lift_a: 0.0,
            },
            Mode::Rocket => FrameParams {
                external_g: 0.0,
                lift_a: 9.81,
            },
            Mode::FreeFall => FrameParams {
                external_g: 9.81,
                lift_a: -9.81,
            },
            Mode::Inertial => FrameParams {
                external_g: 0.0,
                lift_a: 0.0,
            },
            Mode::Custom => FrameParams {
                external_g: 0.0,
                lift_a: 0.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFrame {
    Lift,
    External,
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: f64 = 9.81;

    #[test]
    fn t01_01_g_eff_planet() {
        assert_eq!(Mode::Planet.params().g_eff(), -G);
    }

    #[test]
    fn t01_02_g_eff_rocket() {
        assert_eq!(Mode::Rocket.params().g_eff(), -G);
    }

    #[test]
    fn t01_03_g_eff_freefall_is_zero() {
        assert_eq!(Mode::FreeFall.params().g_eff(), 0.0);
    }

    #[test]
    fn t01_04_g_eff_inertial_is_zero() {
        assert_eq!(Mode::Inertial.params().g_eff(), 0.0);
    }

    #[test]
    fn t01_05_planet_and_rocket_g_eff_bitwise_equal() {
        let planet = Mode::Planet.params().g_eff();
        let rocket = Mode::Rocket.params().g_eff();
        assert_eq!(planet.to_bits(), rocket.to_bits());
    }

    #[test]
    fn t01_06_displacement_at_zero_is_zero() {
        for mode in [Mode::Planet, Mode::Rocket, Mode::FreeFall, Mode::Inertial] {
            assert_eq!(mode.params().displacement(0.0), 0.0);
        }
    }

    #[test]
    fn t01_07_displacement_rocket_positive() {
        let d = Mode::Rocket.params().displacement(1.0);
        assert!(d > 0.0);
        assert_eq!(d, 0.5 * G);
    }

    #[test]
    fn t01_08_displacement_freefall_negative() {
        let d = Mode::FreeFall.params().displacement(1.0);
        assert!(d < 0.0);
        assert_eq!(d, -0.5 * G);
    }

    #[test]
    fn displacement_matches_formula() {
        let frame = FrameParams {
            external_g: 3.3,
            lift_a: 2.0,
        };
        assert_eq!(frame.displacement(2.0), 0.5 * 2.0 * 4.0);
    }

    #[test]
    fn g_eff_custom_mode() {
        let frame = FrameParams {
            external_g: 5.0,
            lift_a: 1.5,
        };
        assert_eq!(frame.g_eff(), -6.5);
    }

    // ---- T02 ----

    const Y0: f64 = 2.2;
    const DT: f64 = 1.0 / 60.0;

    /// T02.1: p_lift для Planet == Rocket (побитово) на t ∈ [0, 0.5]
    #[test]
    fn t02_1_lift_y_planet_equals_rocket() {
        let planet = Mode::Planet.params();
        let rocket = Mode::Rocket.params();
        let steps = (0.5 / DT).round() as usize;
        for k in 0..=steps {
            let t = k as f64 * DT;
            let a = planet.lift_y(Y0, t);
            let b = rocket.lift_y(Y0, t);
            assert_eq!(a.to_bits(), b.to_bits(), "t = {t}");
        }
    }

    /// T02.2: p_external для Rocket = константа y0 (с точностью до fp-округления:
    /// `(y0 − A) + A` может отличаться на 1–2 ulp)
    #[test]
    fn t02_2_external_y_rocket_is_constant() {
        let rocket = Mode::Rocket.params();
        let steps = (1.5 / DT).round() as usize;
        for k in 0..=steps {
            let t = k as f64 * DT;
            let y = rocket.external_y(Y0, t);
            assert!((y - Y0).abs() < 1e-12, "t = {t}, y = {y}");
        }
    }

    /// T02.3: p_external для FreeFall = y0 − ½·g·t²
    #[test]
    fn t02_3_external_y_freefall_matches_analytics() {
        let freefall = Mode::FreeFall.params();
        let steps = (1.5 / DT).round() as usize;
        for k in 0..=steps {
            let t = k as f64 * DT;
            let expected = Y0 - 0.5 * G * t * t;
            let y = freefall.external_y(Y0, t);
            assert_eq!(y.to_bits(), expected.to_bits(), "t = {t}");
        }
    }

    /// T02.4: g_eff не зависит от массы (фиксация контракта)
    #[test]
    fn t02_4_g_eff_independent_of_mass() {
        let g = Mode::Planet.params().g_eff();
        for _mass in [1.0_f64, 5.0, 10.0] {
            assert_eq!(Mode::Planet.params().g_eff().to_bits(), g.to_bits());
        }
        assert_eq!(g.to_bits(), (-G).to_bits());
    }

    /// Доп. проверка: Inertial — тело покоится в обоих кадрах.
    #[test]
    fn t02_inertial_body_is_rest_in_both_frames() {
        let inertial = Mode::Inertial.params();
        for k in 0..=60 {
            let t = k as f64 * DT;
            assert_eq!(inertial.lift_y(Y0, t).to_bits(), Y0.to_bits());
            assert_eq!(inertial.external_y(Y0, t).to_bits(), Y0.to_bits());
        }
    }

    /// Дискретное D сходится к аналитическому и компенсирует лид
    /// интегратора: p_lift(дискретный) + D_discrete == y0.
    #[test]
    fn t02_discrete_displacement_compensates_integrator_lead() {
        let rocket = Mode::Rocket.params();
        let freefall = Mode::FreeFall.params();
        for n in [0_u64, 1, 30, 60, 72] {
            let nf = n as f64;
            let t = nf * DT;
            let analytic = rocket.displacement(t);
            let discrete = rocket.displacement_discrete(n, DT);
            assert!((analytic - discrete).abs() < G * DT * t + 1e-12);

            // Дискретная траектория тела в кадре лифта (semi-implicit):
            // y_n = y0 - g_eff·dt²·n(n+1)/2.  С D_discrete сумма = y0.
            let lift_discrete = Y0 + rocket.g_eff() * DT * DT * nf * (nf + 1.0) / 2.0;
            let y_ext = lift_discrete + rocket.displacement_discrete(n, DT);
            assert!((y_ext - Y0).abs() < 1e-12, "n = {n}, y = {y_ext}");

            // FreeFall: тело покоится в кадре лифта, кабина уходит вниз.
            let y_ext_ff = Y0 + freefall.displacement_discrete(n, DT);
            assert!((y_ext_ff - (Y0 - 0.5 * G * t * t)).abs() < G * DT * t + 1e-12);
        }
    }
}
