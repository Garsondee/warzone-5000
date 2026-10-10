//! The powertrain: engine, coupling, gearbox, driveline and brakes behind the `DrivePort` contract.
//!
//! Principle: DRIVE never integrates a wheel. Each substep it reflects the wheels' speeds, inertias and loads up the driveline to the
//! engine, lets the coupling decide the torque that flows (an implicit constraint, so a stiff lock cannot blow up), and pushes the result back
//! down to the wheels as torques. CHASSIS integrates the wheel speeds and hands them back next substep.

use w5k_contract::ports::DrivePort;
use w5k_contract::rig::{BrakeSite, DrivetrainDef};
use w5k_contract::{DriveInputs, DriveTelemetry, ShaftState};
use w5k_math::StateHasher;

use crate::brakes::{Brake, BrakedShaft};
use crate::coupling::{Coupling, CouplingTuning, Downstream};
use crate::driveline::{Driveline, DrivelineTuning};
use crate::engine::{Engine, EngineTuning};
use crate::gearbox::{Gearbox, ShiftTuning};

/// Every tuning table the powertrain needs, loaded from `content/physics/drive/`.
#[derive(Clone, Debug)]
pub struct Tunings {
    pub engine: EngineTuning,
    pub coupling: CouplingTuning,
    pub shift: ShiftTuning,
    pub driveline: DrivelineTuning,
}

impl Tunings {
    /// The shipped tuning files, embedded at build time (the simulation does no file I/O).
    pub fn shipped() -> Tunings {
        // the files are checked by the tests below; a parse failure here is a build-time bug, not a runtime condition
        Tunings {
            engine: ron::from_str(include_str!("../../../content/physics/drive/engine_tuning.ron"))
                .expect("engine_tuning.ron"),
            coupling: ron::from_str(include_str!("../../../content/physics/drive/coupling_tuning.ron"))
                .expect("coupling_tuning.ron"),
            shift: ron::from_str(include_str!("../../../content/physics/drive/shift_tuning.ron"))
                .expect("shift_tuning.ron"),
            driveline: ron::from_str(include_str!("../../../content/physics/drive/driveline_tuning.ron"))
                .expect("driveline_tuning.ron"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Powertrain {
    engine: Engine,
    coupling: Coupling,
    gearbox: Gearbox,
    driveline: Driveline,
    brakes: Vec<(Brake, Option<usize>)>,
    final_drive: Vec<f64>,
    ambient_k: f64,
    clutch_heat_j: f64,
    fuel_used_kg: f64,
    tel: DriveTelemetry,
    scratch: Vec<f64>,
}

impl Powertrain {
    pub fn new(def: &DrivetrainDef, tunings: &Tunings) -> Result<Powertrain, String> {
        let engine = Engine::new(&def.engine, &tunings.engine)?;
        let peak = def.engine.torque_curve.iter().fold(0.0_f64, |m, &(_, t)| m.max(t));
        let coupling = Coupling::new(&def.coupling, peak, &tunings.coupling)?;
        let gearbox = Gearbox::new(&def.gearbox, def.engine.redline_rpm, &tunings.shift)?;
        let driveline = Driveline::new(def, &tunings.driveline)?;
        let mut brakes = Vec::new();
        for (i, b) in def.brakes.iter().enumerate() {
            if b.steering {
                return Err(format!("brake {i} is a steering brake: tracked steering units are not implemented yet"));
            }
            let shaft = match b.site {
                BrakeSite::Driveline => None,
                _ => Some(
                    def.outputs
                        .iter()
                        .position(|o| o.station == b.station)
                        .ok_or_else(|| format!("brake {i} sits on station {}, which is not driven", b.station))?,
                ),
            };
            brakes.push((Brake::new(b, crate::AMBIENT_FALLBACK_K)?, shaft));
        }
        Ok(Powertrain {
            engine,
            coupling,
            gearbox,
            driveline,
            brakes,
            final_drive: def.outputs.iter().map(|o| o.final_drive_ratio).collect(),
            ambient_k: crate::AMBIENT_FALLBACK_K,
            clutch_heat_j: 0.0,
            fuel_used_kg: 0.0,
            tel: DriveTelemetry::default(),
            scratch: vec![0.0; def.outputs.len()],
        })
    }
}

impl DrivePort for Powertrain {
    fn output_count(&self) -> usize {
        self.driveline.output_count()
    }

    fn step(&mut self, dt: f64, inputs: &DriveInputs, shafts: &[ShaftState], torque_nm_out: &mut [f64]) {
        let n = self.driveline.output_count();
        self.ambient_k = inputs.ambient_k;
        self.engine.set_running(inputs.engine_on);
        self.driveline.select_mode(inputs.drive_mode);
        let speed =
            shafts.iter().map(|s| s.vehicle_speed_m_s).fold(0.0, |a: f64, v| if v.abs() > a.abs() { v } else { a });

        // wheels -> carrier -> gearbox input
        let outs: Vec<Downstream> = shafts
            .iter()
            .take(n)
            .map(|s| Downstream {
                omega_rad_s: s.omega_rad_s,
                inertia_kg_m2: s.inertia_kg_m2,
                ext_torque_nm: -s.load_torque_nm,
                ext_slope_nm_s_rad: s.load_stiffness_nm_s_rad,
            })
            .collect();
        let carrier = self.driveline.reflect(&outs);
        let input_side = self.gearbox.reflect(&carrier);

        self.gearbox.note_brake(dt, inputs.brake);
        let shift = self.gearbox.update(dt, inputs.gear, inputs.throttle, carrier.omega_rad_s, speed);
        let throttle = if inputs.engine_on { inputs.throttle } else { 0.0 };
        let c = self.coupling.step(dt, &mut self.engine, throttle, inputs.clutch, shift.capacity_scale, &input_side);
        self.clutch_heat_j += c.heat_j;
        self.fuel_used_kg += self.engine.fuel_rate_kg_s() * dt;

        // gearbox output -> driveline input torque; a driveline-site brake acts on this shaft
        let mut t_carrier = self.gearbox.output_torque(c.torque_nm);
        for (brake, shaft) in &mut self.brakes {
            if shaft.is_none() {
                let cmd = command(brake, inputs);
                let sh = BrakedShaft {
                    omega_rad_s: carrier.omega_rad_s,
                    inertia_kg_m2: carrier.inertia_kg_m2,
                    other_torque_nm: t_carrier + carrier.ext_torque_nm,
                    other_slope_nm_s_rad: carrier.ext_slope_nm_s_rad,
                    vehicle_speed_m_s: speed,
                };
                t_carrier += brake.step(dt, cmd, &sh, inputs.ambient_k);
            }
        }
        self.driveline.distribute(dt, t_carrier, &outs, &mut self.scratch);

        // wheel-site brakes
        for (brake, shaft) in &mut self.brakes {
            if let Some(i) = *shaft {
                let k = match brake.def().location {
                    w5k_contract::rig::BrakeLocation::BeforeFinalDrive => self.final_drive[i],
                    _ => 1.0,
                };
                let s = &shafts[i];
                let sh = BrakedShaft {
                    omega_rad_s: k * s.omega_rad_s,
                    inertia_kg_m2: s.inertia_kg_m2 / (k * k),
                    other_torque_nm: (self.scratch[i] - s.load_torque_nm) / k,
                    other_slope_nm_s_rad: s.load_stiffness_nm_s_rad / (k * k),
                    vehicle_speed_m_s: s.vehicle_speed_m_s,
                };
                let cmd = command(brake, inputs);
                self.scratch[i] += k * brake.step(dt, cmd, &sh, inputs.ambient_k);
            }
        }
        torque_nm_out[..n].copy_from_slice(&self.scratch[..n]);

        self.tel = DriveTelemetry {
            engine_rpm: self.engine.rpm(),
            gear: shift.gear,
            engine_torque_nm: self.engine.torque_nm(),
            fuel_rate_kg_s: self.engine.fuel_rate_kg_s(),
            coupling_slip_rad_s: c.slip_rad_s,
            brake_temps_k: self.brakes.iter().map(|(b, _)| b.temperature_k()).collect(),
            shifting: shift.shifting,
            drive_mode: self.driveline.mode(),
            clutch_heat_j: self.clutch_heat_j,
            output_torque_nm: self.scratch[..n].to_vec(),
            fuel_used_kg: self.fuel_used_kg,
        };
    }

    fn telemetry(&self) -> DriveTelemetry {
        self.tel.clone()
    }

    fn hash_state(&self, h: &mut StateHasher) {
        self.engine.hash_state(h);
        self.coupling.hash_state(h);
        self.gearbox.hash_state(h);
        for (b, _) in &self.brakes {
            b.hash_state(h);
        }
        h.write_f64(self.clutch_heat_j);
        h.write_f64(self.fuel_used_kg);
    }
}

/// The demand a brake follows: the pedal for a service brake, the lever for a parking brake (a brake can be both).
fn command(brake: &Brake, inputs: &DriveInputs) -> f64 {
    let d = brake.def();
    let service = if d.service { inputs.brake } else { 0.0 };
    let parking = if d.parking && inputs.parking_brake { 1.0 } else { 0.0 };
    service.max(parking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    #[test]
    fn the_truck_powertrain_builds_and_a_tank_is_refused_with_a_reason() {
        let (rig, _) = box_truck();
        let p = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).unwrap();
        assert_eq!(p.output_count(), rig.drivetrain.outputs.len());
        let tank = w5k_contract::testing::box_tank().0;
        assert!(Powertrain::new(&tank.drivetrain, &Tunings::shipped()).is_err());
    }
}
