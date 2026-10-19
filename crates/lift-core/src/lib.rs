pub mod clock;
pub mod frame;
pub mod photon;
pub mod scales;

pub use clock::ExperimentClock;
pub use frame::{FrameParams, Mode, ViewFrame};
pub use photon::{photon_external, photon_impact, photon_lift, PhotonImpact, PhotonLaunch};
pub use scales::{compression_from_force, smooth_compression, spring_scale_y};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScaleReading {
    pub mass_kg: f64,
    pub normal_force_n: f64,
    pub compression: f32,
    pub in_contact: bool,
}
