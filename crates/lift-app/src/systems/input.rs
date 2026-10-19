use bevy::prelude::*;
use lift_core::Mode;

use crate::resources::InputEvents;

const MODE_KEYS: [KeyCode; 4] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
];

/// Действия фотона (04-light §3.8): L — выстрел, C — очистка,
/// G — ghost path, `,`/`.` — c_vis меньше/больше (пересчёт мгновенный).
#[derive(Resource, Default, Debug)]
pub struct PhotonEvents {
    pub shot: bool,
    pub clear: bool,
    pub toggle_ghost: bool,
    pub c_vis_delta: i32,
}

/// 1/2/3/4 = Planet/Rocket/FreeFall/Inertial + полный сброс.
/// Shift+1..4 = смена режима на лету (позиции и скорости сохраняются).
/// R = сброс, Space = пауза, V = кадр Lift ↔ External.
pub fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut out: ResMut<InputEvents>) {
    let mut events = InputEvents {
        reset_seq: out.reset_seq,
        ..Default::default()
    };
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    for (i, key) in MODE_KEYS.into_iter().enumerate() {
        if keys.just_pressed(key) {
            let mode = [Mode::Planet, Mode::Rocket, Mode::FreeFall, Mode::Inertial][i];
            if shift {
                events.mode_live = Some(mode);
            } else {
                events.mode_reset = Some(mode);
            }
        }
    }

    events.reset = keys.just_pressed(KeyCode::KeyR);
    events.toggle_view = keys.just_pressed(KeyCode::KeyV);
    events.toggle_pause = keys.just_pressed(KeyCode::Space);

    *out = events;
}

pub fn handle_photon_input(keys: Res<ButtonInput<KeyCode>>, mut out: ResMut<PhotonEvents>) {
    *out = PhotonEvents {
        shot: keys.just_pressed(KeyCode::KeyL),
        clear: keys.just_pressed(KeyCode::KeyC),
        toggle_ghost: keys.just_pressed(KeyCode::KeyG),
        c_vis_delta: if keys.just_pressed(KeyCode::Comma) {
            -1
        } else if keys.just_pressed(KeyCode::Period) {
            1
        } else {
            0
        },
    };
}
