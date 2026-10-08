//! A scripted driver and a constant-torque powertrain.

use w5k_math::{scalar, StateHasher};

use crate::command::{Command, GearRequest};
use crate::ports::{DriveInputs, DrivePort, DriveTelemetry, ShaftState};

/// A driver that replays keyframes: each `(t_s, Command)` holds from `t_s` until the next key.
#[derive(Clone, Debug, Default)]
pub struct ScriptedCommands {
    keys: Vec<(f64, Command)>,
}

impl ScriptedCommands {
    pub fn new(mut keys: Vec<(f64, Command)>) -> ScriptedCommands {
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        ScriptedCommands { keys }
    }

    /// The command in force at `t_s` (neutral before the first key).
    pub fn at(&self, t_s: f64) -> Command {
        self.keys.iter().rev().find(|(t, _)| *t <= t_s).map(|(_, c)| *c).unwrap_or(Command::NEUTRAL)
    }

    /// The standard first-light drive for a road vehicle: accelerate, cruise over the bump strip, steer left and right, brake to a
    /// stop. 40 s long.
    pub fn first_light() -> ScriptedCommands {
        let go = |t: f64, th: f64, st: f64| (t, Command { throttle: th, steer: st, ..Command::NEUTRAL });
        ScriptedCommands::new(vec![
            go(0.0, 0.0, 0.0),
            go(1.0, 0.9, 0.0),  // launch
            go(5.0, 0.30, 0.0), // cruise toward the speed hump
            go(14.0, 0.30, 0.0),
            go(20.0, 0.30, 0.25),  // gentle right (steer is +1 = full right)
            go(23.0, 0.30, -0.25), // gentle left
            go(26.0, 0.30, 0.0),
            (30.0, Command { brake: 0.8, ..Command::NEUTRAL }), // brake to a stop
            (40.0, Command { brake: 1.0, parking_brake: true, ..Command::NEUTRAL }),
        ])
    }

    /// A slow drive on flat ground while the turret slews through a full circle and the gun elevates and fires. 40 s long.
    pub fn tank_demo() -> ScriptedCommands {
        let aim = |t: f64, yaw: f64, pitch: f64, fire: bool| {
            (
                t,
                Command {
                    throttle: 0.2,
                    turret_yaw_rad: Some(yaw),
                    gun_pitch_rad: Some(pitch),
                    fire,
                    ..Command::NEUTRAL
                },
            )
        };
        ScriptedCommands::new(vec![
            aim(0.0, 0.0, 0.0, false),
            aim(2.0, scalar::PI / 2.0, 0.1, false),
            aim(8.0, scalar::PI, 0.2, false),
            aim(14.0, -scalar::PI / 2.0, 0.0, false),
            aim(20.0, 0.0, 0.0, false),
            aim(24.0, 0.0, 0.15, true), // fire
            aim(24.1, 0.0, 0.15, false),
            aim(30.0, 0.3, -0.05, false),
        ])
    }
}

/// A powertrain that gives every output shaft `throttle * max_torque_nm` and brakes with `brake * brake_torque_nm`, never
/// reversing a stopped shaft. Telemetry is fake but plausible.
#[derive(Clone, Debug)]
pub struct ConstantTorquePowertrain {
    pub outputs: usize,
    pub max_torque_nm: f64,
    pub brake_torque_nm: f64,
    last_rpm: f64,
    last_gear: i8,
}

impl ConstantTorquePowertrain {
    pub fn new(outputs: usize, max_torque_nm: f64, brake_torque_nm: f64) -> ConstantTorquePowertrain {
        ConstantTorquePowertrain { outputs, max_torque_nm, brake_torque_nm, last_rpm: 800.0, last_gear: 0 }
    }
}

impl DrivePort for ConstantTorquePowertrain {
    fn output_count(&self) -> usize {
        self.outputs
    }

    fn step(&mut self, dt_s: f64, inputs: &DriveInputs, shafts: &[ShaftState], torque_nm_out: &mut [f64]) {
        let drive = match inputs.gear {
            GearRequest::Neutral => 0.0,
            GearRequest::Reverse => -inputs.throttle * self.max_torque_nm,
            _ => inputs.throttle * self.max_torque_nm,
        };
        let brake_cap = if inputs.parking_brake { self.brake_torque_nm } else { inputs.brake * self.brake_torque_nm };
        let mut mean_omega = 0.0;
        for (i, s) in shafts.iter().enumerate().take(self.outputs) {
            // The torque that would bring this shaft to rest in one step, capped by what the brake can give.
            let stop = -s.omega_rad_s * s.inertia_kg_m2 / dt_s.max(1e-9);
            torque_nm_out[i] = drive + scalar::clamp(stop, -brake_cap, brake_cap);
            mean_omega += s.omega_rad_s;
        }
        mean_omega /= self.outputs.max(1) as f64;
        self.last_rpm = 800.0 + scalar::rad_s_to_rpm(mean_omega.abs()) * 6.0;
        self.last_gear = if matches!(inputs.gear, GearRequest::Reverse) {
            -1
        } else if matches!(inputs.gear, GearRequest::Neutral) {
            0
        } else {
            1
        };
    }

    fn telemetry(&self) -> DriveTelemetry {
        DriveTelemetry { engine_rpm: self.last_rpm, gear: self.last_gear, ..DriveTelemetry::default() }
    }

    fn hash_state(&self, h: &mut StateHasher) {
        h.write_f64(self.last_rpm);
        h.write_u8(self.last_gear as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_holds_each_key_until_the_next() {
        let s = ScriptedCommands::first_light();
        assert_eq!(s.at(-1.0), Command::NEUTRAL);
        assert!((s.at(2.0).throttle - 0.9).abs() < 1e-12);
        assert!((s.at(6.0).throttle - 0.30).abs() < 1e-12);
        assert!(s.at(35.0).brake > 0.5);
    }

    #[test]
    fn the_brake_stops_a_shaft_without_reversing_it() {
        let mut p = ConstantTorquePowertrain::new(1, 100.0, 500.0);
        let mut out = [0.0];
        let inputs = DriveInputs { brake: 1.0, ..DriveInputs::default() };
        let mut omega = 20.0;
        let j = 2.0;
        let dt = 1.0 / 240.0;
        for _ in 0..2000 {
            p.step(
                dt,
                &inputs,
                &[ShaftState { omega_rad_s: omega, inertia_kg_m2: j, vehicle_speed_m_s: 0.0 }],
                &mut out,
            );
            omega += out[0] / j * dt;
            assert!(omega > -1e-9, "reversed: {omega}");
        }
        assert!(omega.abs() < 1e-6);
    }
}
