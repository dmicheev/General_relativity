//! Сэмплирование аналитических траекторий фотона (04-light §3.5).
//!
//! Все пути считаются из формул (не интегрированием шагом) — точка
//! попадания и форма траектории не зависят от частоты кадров.

use lift_core::{photon_external, photon_lift, FrameParams, PhotonLaunch};

/// Сегментов в призрачной траектории (04-light §3.5).
pub const GHOST_SEGMENTS: usize = 128;
/// Сегментов в активном следе.
pub const TRAIL_SEGMENTS: usize = 96;

/// В каком кадре сэмплируется путь.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    Lift,
    External,
}

/// `segments + 1` точек пути фотона на `t ∈ [t_from, t_to]`.
pub fn sample_path(
    launch: &PhotonLaunch,
    frame: &FrameParams,
    kind: PathKind,
    t_from: f64,
    t_to: f64,
    segments: usize,
) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    sample_path_into(launch, frame, kind, t_from, t_to, segments, &mut out);
    out
}

/// То же с переиспользованием буфера (T05 §5: без аллокаций в hot path).
pub fn sample_path_into(
    launch: &PhotonLaunch,
    frame: &FrameParams,
    kind: PathKind,
    t_from: f64,
    t_to: f64,
    segments: usize,
    out: &mut Vec<[f64; 3]>,
) {
    let n = segments.max(1);
    out.clear();
    for i in 0..=n {
        let t = t_from + (t_to - t_from) * i as f64 / n as f64;
        out.push(match kind {
            PathKind::Lift => photon_lift(t, launch, frame),
            PathKind::External => photon_external(t, launch, frame.external_g),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lift_core::Mode;

    fn launch() -> PhotonLaunch {
        PhotonLaunch {
            origin: [-1.45, 1.8, 0.0],
            c: 10.0,
        }
    }

    /// Границы сэмпла: первая точка — эмиттер, последняя — на t_to.
    #[test]
    fn sample_endpoints() {
        let frame = Mode::Planet.params();
        let pts = sample_path(&launch(), &frame, PathKind::Lift, 0.0, 0.295, 128);
        assert_eq!(pts.len(), 129);
        assert_eq!(pts[0][1].to_bits(), 1.8_f64.to_bits());
        assert_eq!(pts[0][0].to_bits(), (-1.45_f64).to_bits());
        let last = photon_lift(0.295, &launch(), &frame);
        assert_eq!(pts[128][1].to_bits(), last[1].to_bits());
    }

    /// X монотонно растёт со скоростью c_vis (обоих кадров).
    #[test]
    fn x_monotonic_at_speed_c() {
        let frame = Mode::Rocket.params();
        let l = launch();
        for kind in [PathKind::Lift, PathKind::External] {
            let pts = sample_path(&l, &frame, kind, 0.0, 0.295, 32);
            for w in pts.windows(2) {
                assert!(w[1][0] > w[0][0]);
                assert!((w[1][0] - w[0][0] - l.c * 0.295 / 32.0).abs() < 1e-12);
            }
        }
    }

    /// FreeFall/Lift — строго прямая по y (побитово).
    #[test]
    fn freefall_lift_path_is_straight() {
        let frame = Mode::FreeFall.params();
        let pts = sample_path(&launch(), &frame, PathKind::Lift, 0.0, 0.295, 64);
        for p in &pts {
            assert_eq!(p[1].to_bits(), 1.8_f64.to_bits());
        }
    }
}
