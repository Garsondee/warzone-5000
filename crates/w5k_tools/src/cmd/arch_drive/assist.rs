//! Between the player's pedals and the powertrain. Raw controls pass straight through (the adult mode); the kid assists arrive in a
//! later change of this series, behind the same `apply`.

use w5k_contract::{DriveInputs, GearRequest};
use w5k_math::scalar;

/// What the player asks for (the body of `POST /api/input`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Raw {
    pub throttle: f64,
    pub brake: f64,
    /// -1 left .. +1 right.
    pub steer: f64,
    pub reverse: bool,
}

impl Raw {
    /// Everything clamped to its range (a page may send 1.2 or -0.1); non-finite numbers count as zero.
    pub(crate) fn clamped(self) -> Raw {
        let c = |v: f64, lo: f64| if v.is_finite() { scalar::clamp(v, lo, 1.0) } else { 0.0 };
        Raw {
            throttle: c(self.throttle, 0.0),
            brake: c(self.brake, 0.0),
            steer: c(self.steer, -1.0),
            reverse: self.reverse,
        }
    }
}

pub(crate) struct Assist {
    pub on: bool,
    /// Short status for the page: `""`, or which assist is acting.
    pub message: &'static str,
}

impl Assist {
    pub(crate) fn new(on: bool) -> Assist {
        Assist { on, message: "" }
    }

    /// Forget all state (after a recovery).
    pub(crate) fn reset(&mut self) {
        self.message = "";
    }

    /// The powertrain inputs for this tick.
    pub(crate) fn apply(&mut self, raw: &Raw, _dt_s: f64) -> DriveInputs {
        let raw = raw.clamped();
        let gear = if raw.reverse { GearRequest::Reverse } else { GearRequest::Auto };
        DriveInputs { throttle: raw.throttle, brake: raw.brake, steer: raw.steer, gear, ..DriveInputs::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedals_are_clamped_to_their_ranges_and_garbage_counts_as_released() {
        let raw = Raw { throttle: 1.7, brake: -0.2, steer: -4.0, reverse: false }.clamped();
        assert_eq!((raw.throttle, raw.brake, raw.steer), (1.0, 0.0, -1.0));
        let raw = Raw { throttle: f64::NAN, brake: f64::INFINITY, steer: f64::NAN, reverse: false }.clamped();
        assert_eq!((raw.throttle, raw.brake, raw.steer), (0.0, 0.0, 0.0));
    }

    #[test]
    fn the_gearbox_is_automatic_unless_reverse_is_asked_for() {
        let mut a = Assist::new(true);
        assert_eq!(a.apply(&Raw::default(), 0.016).gear, GearRequest::Auto);
        assert_eq!(a.apply(&Raw { reverse: true, ..Raw::default() }, 0.016).gear, GearRequest::Reverse);
    }
}
