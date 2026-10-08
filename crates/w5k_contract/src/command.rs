//! `Command`: what a driver asks of a vehicle. A human, the AI and scripted scenarios all emit exactly this, nothing else, so the
//! AI can never cheat and the owner can drive any vehicle.
//!
//! A `Command` is `Copy` and fixed-size on purpose (it is built and compared millions of times). Weapon stations are addressed by
//! **aim channel**: channel 0 is the main turret and gun; a cupola, a sight head or a hull-fixed gun's operator use channels 1 to 3. A joint
//! follows the channel named by `JointDef::aim_channel`; a weapon fires when the bit named by `WeaponDef::trigger` is set in `fire`.

use serde::{Deserialize, Serialize};

/// Independent aim channels a command can carry.
pub const AIM_CHANNELS: usize = 4;

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

/// Which frame an aim demand is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AimFrame {
    /// Relative to the joint's parent body (the hull for a turret ring, the turret for a gun cradle): the demand a hand or a stand-in gives.
    #[default]
    Parent,
    /// Relative to the world: azimuth about +Y (left positive, 0 along -Z) and elevation above the horizon. A stabilised joint holds this
    /// direction while the hull moves; an unstabilised one only follows it at the rate the servo allows.
    World,
}

/// One aim channel's demand. `None` = **hold the previous demand** (the model remembers the last demand and its frame).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AimDemand {
    /// Desired yaw, rad (positive = left, as for any yaw).
    pub yaw_rad: Option<f64>,
    /// Desired elevation, rad (positive = up).
    pub pitch_rad: Option<f64>,
    pub frame: AimFrame,
}

impl AimDemand {
    /// Hold everything.
    pub const HOLD: AimDemand = AimDemand { yaw_rad: None, pitch_rad: None, frame: AimFrame::Parent };

    /// A parent-relative yaw and elevation.
    pub fn relative(yaw_rad: f64, pitch_rad: f64) -> AimDemand {
        AimDemand { yaw_rad: Some(yaw_rad), pitch_rad: Some(pitch_rad), frame: AimFrame::Parent }
    }

    /// A world-frame azimuth and elevation (what a stabilised gun is laid onto).
    pub fn world(azimuth_rad: f64, elevation_rad: f64) -> AimDemand {
        AimDemand { yaw_rad: Some(azimuth_rad), pitch_rad: Some(elevation_rad), frame: AimFrame::World }
    }
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
    /// Clutch pedal of a manual gearbox, 0 (released, engaged) to 1 (fully depressed). `None` = the driver model or an automatic clutch decides.
    #[serde(default)]
    pub clutch: Option<f64>,
    /// Driveline state to select (index into `DrivetrainDef::modes`: high or low range, front-axle declutch, diff lock). `None` = keep.
    #[serde(default)]
    pub drive_mode: Option<u8>,
    /// Aim demands per channel (turret and gun, cupola, sight head, ...).
    #[serde(default)]
    pub aim: [AimDemand; AIM_CHANNELS],
    /// Trigger bits held (**level**-triggered): bit `i` set = trigger `i` is held. A single-shot weapon fires once per rising edge when ready; an
    /// automatic weapon keeps firing while the bit is held.
    #[serde(default)]
    pub fire: u8,
}

impl Command {
    /// Everything released, coasting.
    pub const NEUTRAL: Command = Command {
        throttle: 0.0,
        brake: 0.0,
        steer: 0.0,
        gear: GearRequest::Auto,
        parking_brake: false,
        clutch: None,
        drive_mode: None,
        aim: [AimDemand::HOLD; AIM_CHANNELS],
        fire: 0,
    };

    /// The same command with every analogue input forced into range (NaN becomes 0). Models should call this first.
    pub fn sanitized(&self) -> Command {
        let fix = |v: f64, lo: f64, hi: f64| if v.is_nan() { 0.0 } else { v.clamp(lo, hi) };
        let mut c = Command {
            throttle: fix(self.throttle, 0.0, 1.0),
            brake: fix(self.brake, 0.0, 1.0),
            steer: fix(self.steer, -1.0, 1.0),
            clutch: self.clutch.map(|v| fix(v, 0.0, 1.0)),
            ..*self
        };
        for a in c.aim.iter_mut() {
            a.yaw_rad = a.yaw_rad.filter(|v| v.is_finite());
            a.pitch_rad = a.pitch_rad.filter(|v| v.is_finite());
        }
        c
    }

    pub fn throttle(t: f64) -> Command {
        Command { throttle: t, ..Command::NEUTRAL }
    }

    pub fn braking(b: f64) -> Command {
        Command { brake: b, ..Command::NEUTRAL }
    }

    /// The main turret and gun (channel 0) laid on a parent-relative yaw and elevation.
    pub fn aim_main(yaw_rad: f64, pitch_rad: f64) -> Command {
        let mut c = Command::NEUTRAL;
        c.aim[0] = AimDemand::relative(yaw_rad, pitch_rad);
        c
    }

    /// True when trigger bit `i` (0..7) is held.
    pub fn firing(&self, trigger: u8) -> bool {
        trigger < 8 && self.fire & (1 << trigger) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_removes_nan() {
        let mut c = Command { throttle: 3.0, brake: -1.0, steer: f64::NAN, clutch: Some(7.0), ..Command::NEUTRAL };
        c.aim[1].yaw_rad = Some(f64::INFINITY);
        let c = c.sanitized();
        assert_eq!((c.throttle, c.brake, c.steer, c.clutch), (1.0, 0.0, 0.0, Some(1.0)));
        assert_eq!(c.aim[1].yaw_rad, None);
    }

    #[test]
    fn json_round_trip() {
        let mut c =
            Command { throttle: 0.5, steer: -0.25, gear: GearRequest::Gear(2), fire: 0b101, ..Command::NEUTRAL };
        c.aim[0] = AimDemand::relative(0.1, 0.02);
        c.aim[2] = AimDemand::world(-1.0, 0.3);
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Command>(&s).unwrap(), c);
    }

    #[test]
    fn a_command_stays_copy_and_the_trigger_bits_are_levels() {
        fn needs_copy<T: Copy>(_: T) {}
        needs_copy(Command::NEUTRAL);
        let c = Command { fire: 0b0000_0110, ..Command::NEUTRAL };
        assert!(!c.firing(0) && c.firing(1) && c.firing(2) && !c.firing(3));
        assert!(!c.firing(8));
    }

    #[test]
    fn aim_main_addresses_channel_zero_only() {
        let c = Command::aim_main(0.5, 0.1);
        assert_eq!(c.aim[0].yaw_rad, Some(0.5));
        assert!(c.aim[1..].iter().all(|a| *a == AimDemand::HOLD));
    }
}
