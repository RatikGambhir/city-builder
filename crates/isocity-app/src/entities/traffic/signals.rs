//! Traffic light timing.

use isocity_core::Direction;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightState {
    GreenNs,
    YellowNs,
    GreenEw,
    YellowEw,
}

pub fn light_state(t: f32) -> LightState {
    let c = t.rem_euclid(7.6);
    if c < 3.0 {
        LightState::GreenNs
    } else if c < 3.8 {
        LightState::YellowNs
    } else if c < 6.8 {
        LightState::GreenEw
    } else {
        LightState::YellowEw
    }
}

pub(super) fn can_proceed(dir: Direction, s: LightState) -> bool {
    if dir.is_ns() {
        matches!(s, LightState::GreenNs | LightState::YellowNs)
    } else {
        matches!(s, LightState::GreenEw | LightState::YellowEw)
    }
}
