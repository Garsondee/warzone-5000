//! `Command`: what a driver asks of a vehicle. A human, the AI and scripted scenarios all emit exactly this, nothing else, so the
//! AI can never cheat and the owner can drive any vehicle.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GearRequest {
    /// The gearbox chooses (automatic shift logic, or the driver model for a manual).
    #[default]
    Auto,
    Neutral,
    Reverse,
    /// Stay in the current gear.
    Hold,
    Up,
    Down,
    /// A specific forward gear, 1-based.
    Gear(u8),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Command {
    /// Accelerator, 0 (released) to 1 (floored).
    pub throttle: f64,
    /// Service brake, 0 to 1.
    pub brake: f64,
    /// Steering, -1 (full left) to +1 (full right). For tracked vehicles this is the steering-unit demand.
    pub steer: f64,
    pub gear: GearRequest,
    pub parking_brake: bool,
    /// Desired turret yaw relative to the hull, rad (positive = left, as for any yaw). `None` = hold.
    pub turret_yaw_rad: Option<f64>,
    /// Desired gun elevation relative to the turret, rad (positive = up). `None` = hold.
    pub gun_pitch_rad: Option<f64>,
    pub fire: bool,
}

impl Command {
    /// Everything released, coasting.
    pub const NEUTRAL: Command = Command {
        throttle: 0.0,
        brake: 0.0,
        steer: 0.0,
        gear: GearRequest::Auto,
        parking_brake: false,
        turret_yaw_rad: None,
        gun_pitch_rad: None,
        fire: false,
    };

    /// The same command with every analogue input forced into range (NaN becomes 0). Models should call this first.
    pub fn sanitized(&self) -> Command {
        let fix = |v: f64, lo: f64, hi: f64| if v.is_nan() { 0.0 } else { v.clamp(lo, hi) };
        Command { throttle: fix(self.throttle, 0.0, 1.0), brake: fix(self.brake, 0.0, 1.0), steer: fix(self.steer, -1.0, 1.0), ..*self }
    }

    pub fn throttle(t: f64) -> Command {
        Command { throttle: t, ..Command::NEUTRAL }
    }

    pub fn braking(b: f64) -> Command {
        Command { brake: b, ..Command::NEUTRAL }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_removes_nan() {
        let c = Command { throttle: 3.0, brake: -1.0, steer: f64::NAN, ..Command::NEUTRAL }.sanitized();
        assert_eq!((c.throttle, c.brake, c.steer), (1.0, 0.0, 0.0));
    }

    #[test]
    fn json_round_trip() {
        let c = Command { throttle: 0.5, steer: -0.25, gear: GearRequest::Gear(2), turret_yaw_rad: Some(0.1), ..Command::NEUTRAL };
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Command>(&s).unwrap(), c);
    }
}
