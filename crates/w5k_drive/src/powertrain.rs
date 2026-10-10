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
    /// For each brake, the side (0 left, 1 right) of the steering demand it follows, if it is a steering brake.
    steer_side: Vec<Option<usize>>,
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
        let peak_rpm =
            def.engine.torque_curve.iter().fold((0.0_f64, 0.0_f64), |m, &(r, t)| if t > m.1 { (r, t) } else { m }).0;
        let coupling = Coupling::new(&def.coupling, peak, peak_rpm, &tunings.coupling)?;
        let gearbox = Gearbox::new(&def.gearbox, def.engine.redline_rpm, &tunings.shift)?
            .with_engine_curve(&def.engine.torque_curve);
        let driveline = Driveline::new(def, &tunings.driveline)?;
        let mut brakes = Vec::new();
        for (i, b) in def.brakes.iter().enumerate() {
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
        // which brakes steer: the ones the rig's law names, else the ones flagged `steering`, else the brakes on the steering unit's own outputs
        let mut steer_side = vec![None; brakes.len()];
        if let Some(outs) = driveline.steer_outputs() {
            let side_of = |shaft: Option<usize>| shaft.and_then(|o| outs.iter().position(|&x| x == o));
            if let Some(named) = driveline.steer_brakes() {
                for (side, &i) in named.iter().enumerate() {
                    steer_side[i] = Some(side);
                }
            } else if def.brakes.iter().any(|b| b.steering) {
                for (i, (b, shaft)) in brakes.iter().enumerate() {
                    steer_side[i] = if b.def().steering { side_of(*shaft) } else { None };
                }
            } else {
                for (i, (_, shaft)) in brakes.iter().enumerate() {
                    steer_side[i] = side_of(*shaft);
                }
            }
            if steer_side.iter().zip(&brakes).any(|(s, (b, _))| s.is_none() && b.def().steering) {
                return Err("a steering brake must sit on one of the steering unit's outputs".into());
            }
        } else if def.brakes.iter().any(|b| b.steering) {
            return Err("a steering brake needs a steering unit in the driveline".into());
        }
        Ok(Powertrain {
            engine,
            coupling,
            gearbox,
            driveline,
            brakes,
            steer_side,
            final_drive: def.outputs.iter().map(|o| o.final_drive_ratio).collect(),
            ambient_k: crate::AMBIENT_FALLBACK_K,
            clutch_heat_j: 0.0,
            fuel_used_kg: 0.0,
            tel: DriveTelemetry::default(),
            scratch: vec![0.0; def.outputs.len()],
        })
    }
}

impl Powertrain {
    /// Reduction from the gearbox output to the first driven shaft (transfer case, differentials, final drive).
    pub fn driveline_ratio(&self) -> f64 {
        self.driveline.overall_ratio()
    }

    /// Ratio of forward gear `g` (1-based), engine speed over gearbox output speed.
    pub fn gear_ratio(&self, g: usize) -> Option<f64> {
        self.gearbox.forward_ratio(g)
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
        let mut carrier = self.driveline.reflect(&outs);
        // a steering unit that moves torque between the tracks adds power to the wheels; that power is a load on the engine (applied one step late)
        let carrier_speed = carrier.omega_rad_s.abs();
        if carrier_speed > 1.0 {
            // const-ok: below one rad/s the carrier is effectively stopped and the division is meaningless (a neutral pivot is PROVISIONAL)
            carrier.ext_torque_nm -= self.driveline.steer_power_w().max(0.0) / carrier_speed;
        }
        let input_side = self.gearbox.reflect(&carrier);

        self.gearbox.note_brake(dt, inputs.brake);
        self.gearbox.note_load(-carrier.ext_torque_nm);
        let shift =
            self.gearbox.update(dt, inputs.gear, inputs.throttle, self.driveline.slowest_input_speed(&outs), speed);
        let throttle = if inputs.engine_on { inputs.throttle } else { 0.0 };
        let c = self.coupling.step(dt, &mut self.engine, throttle, inputs.clutch, shift.capacity_scale, &input_side);
        self.clutch_heat_j += c.heat_j;
        self.fuel_used_kg += self.engine.fuel_rate_kg_s() * dt;

        // gearbox output -> driveline input torque; a driveline-site brake acts on this shaft
        let mut t_carrier = self.gearbox.output_torque(c.torque_nm);
        let steer_demand = self.driveline.steer_brake_demand(inputs.steer);
        for (n, (brake, shaft)) in self.brakes.iter_mut().enumerate() {
            if shaft.is_none() {
                let cmd = command(brake, inputs, self.steer_side[n].zip(steer_demand).map(|(s, d)| d[s]));
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
        self.driveline.distribute(dt, t_carrier, &outs, inputs.steer, shift.gear, &mut self.scratch);

        // wheel-site brakes
        for (n, (brake, shaft)) in self.brakes.iter_mut().enumerate() {
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
                let cmd = command(brake, inputs, self.steer_side[n].zip(steer_demand).map(|(s, d)| d[s]));
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

/// The demand a brake follows: the pedal for a service brake, the lever for a parking brake (a brake can be both), and the steering
/// demand of its side if it is a steering brake (a dedicated steering brake, `steering: true`, follows nothing else).
fn command(brake: &Brake, inputs: &DriveInputs, steering: Option<f64>) -> f64 {
    let d = brake.def();
    let service = if d.service && !d.steering { inputs.brake } else { 0.0 };
    let parking = if d.parking && inputs.parking_brake && !d.steering { 1.0 } else { 0.0 };
    service.max(parking).max(steering.unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    #[test]
    fn the_truck_and_the_tank_powertrains_build() {
        let (rig, _) = box_truck();
        let p = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).unwrap();
        assert_eq!(p.output_count(), rig.drivetrain.outputs.len());
        let tank = w5k_contract::testing::box_tank().0;
        let t = Powertrain::new(&tank.drivetrain, &Tunings::shipped()).unwrap();
        assert_eq!(t.output_count(), 2);
    }

    // ------------------------------------------------------------------------------------------------ tracked steering units

    use w5k_contract::rig::{DriveNode, SteerLaw, SteerUnitKind};
    use w5k_contract::testing::box_tank;
    use w5k_contract::GearRequest;

    /// The box tank with its steering unit changed to `kind` and `law`.
    fn tank_with(kind: SteerUnitKind, law: SteerLaw) -> w5k_contract::rig::DrivetrainDef {
        let mut def = box_tank().0.drivetrain;
        if let DriveNode::SteerUnit { kind: k, law: l, .. } = &mut def.driveline {
            *k = kind;
            *l = law;
        }
        def
    }

    /// Run the two sprockets of a tank for `secs` (the wheels are integrated here, as CHASSIS would): each has the vehicle's mass reflected
    /// onto it and a rolling resistance that opposes its own direction of rotation. Returns the final speeds and the powertrain.
    fn drive_tank(def: &w5k_contract::rig::DrivetrainDef, inputs: &DriveInputs, secs: f64) -> ([f64; 2], Powertrain) {
        drive_tank_with(def, |_| *inputs, secs)
    }

    fn drive_tank_with(
        def: &w5k_contract::rig::DrivetrainDef,
        inputs: impl Fn(f64) -> DriveInputs,
        secs: f64,
    ) -> ([f64; 2], Powertrain) {
        let mut pt = Powertrain::new(def, &Tunings::shipped()).unwrap();
        let (j, load, dt) = (1800.0, 2500.0, 1.0 / 240.0);
        let mut w = [0.0_f64; 2];
        let mut out = [0.0; 2];
        for k in 0..((secs / dt) as usize) {
            let inputs = inputs(k as f64 * dt);
            let resist = |w: f64| load * (w / 0.05).clamp(-1.0, 1.0);
            let sh = [0, 1].map(|i| ShaftState {
                omega_rad_s: w[i],
                inertia_kg_m2: j,
                vehicle_speed_m_s: 0.35 * 0.5 * (w[0] + w[1]),
                load_torque_nm: resist(w[i]),
                load_stiffness_nm_s_rad: 0.0,
            });
            pt.step(dt, &inputs, &sh, &mut out);
            for i in 0..2 {
                w[i] += dt * (out[i] - resist(w[i])) / j;
            }
        }
        (w, pt)
    }

    fn go(steer: f64) -> DriveInputs {
        DriveInputs { throttle: 0.6, steer, gear: GearRequest::Gear(2), ..Default::default() }
    }

    const KINDS: [SteerUnitKind; 4] = [
        SteerUnitKind::ControlledDifferential,
        SteerUnitKind::ClutchBrake,
        SteerUnitKind::DoubleDifferential,
        SteerUnitKind::Hydrostatic,
    ];

    #[test]
    fn steering_unit_demand_turns_the_two_outputs_in_opposite_senses() {
        for kind in KINDS {
            let law = SteerLaw { diff_speed_rad_s: Some(10.0), ..Default::default() };
            let def = tank_with(kind, law);
            let ([l0, r0], _) = drive_tank(&def, &go(0.0), 15.0);
            let ([lr, rr], _) = drive_tank(&def, &go(0.8), 15.0);
            let ([ll, rl], _) = drive_tank(&def, &go(-0.8), 15.0);
            let mean = 0.5 * (l0 + r0);
            assert!(mean > 5.0, "{kind:?} does not move: {l0} {r0}");
            assert!((l0 - r0).abs() < 0.02 * mean, "{kind:?} wanders when going straight: {l0} {r0}");
            assert!(lr > rr + 0.05 * mean, "{kind:?}: a right turn must slow the right track: {lr} {rr}");
            assert!(rl > ll + 0.05 * mean, "{kind:?}: a left turn must slow the left track: {ll} {rl}");
        }
    }

    #[test]
    fn a_hydrostatic_unit_holds_a_speed_difference_whatever_the_speed_and_pivots_in_neutral() {
        let law = SteerLaw {
            diff_speed_rad_s: Some(4.0),
            works_in_neutral: true,
            max_steer_torque_nm: 30_000.0,
            ..Default::default()
        };
        let def = tank_with(SteerUnitKind::Hydrostatic, law);
        for throttle in [0.4, 0.9] {
            let ([l, r], _) = drive_tank(&def, &DriveInputs { throttle, ..go(0.5) }, 20.0);
            assert!(((l - r) - 2.0).abs() < 0.1, "throttle {throttle}: difference {} rad/s, wanted 4 x 0.5", l - r);
        }
        let neutral = DriveInputs { gear: GearRequest::Neutral, ..go(1.0) };
        let ([l, r], _) = drive_tank(&def, &neutral, 10.0);
        assert!(l > 1.0 && r < -1.0 && ((l - r) - 4.0).abs() < 0.2, "pivot turn: {l} {r}");
        // a kinematic unit cannot do that
        let kin = tank_with(SteerUnitKind::DoubleDifferential, SteerLaw::default());
        let ([l, r], _) = drive_tank(&kin, &neutral, 10.0);
        assert!(l.abs() < 0.1 && r.abs() < 0.1, "{l} {r}");
    }

    #[test]
    fn a_double_differential_gives_the_stated_fractional_speed_difference() {
        let law = SteerLaw { diff_ratio_by_gear: vec![0.3], max_steer_torque_nm: 60_000.0, ..Default::default() };
        let def = tank_with(SteerUnitKind::DoubleDifferential, law);
        let ([l, r], _) = drive_tank(&def, &go(0.5), 20.0);
        let frac = (l - r) / (l + r);
        assert!((frac - 0.15).abs() < 0.015, "fractional difference {frac}, wanted 0.3 x 0.5");
    }

    #[test]
    fn detents_snap_the_demand_and_the_dead_band_goes_straight() {
        let law = SteerLaw {
            diff_ratio_by_gear: vec![0.3],
            detents: vec![0.5, 1.0],
            max_steer_torque_nm: 60_000.0,
            ..Default::default()
        };
        let def = tank_with(SteerUnitKind::DoubleDifferential, law);
        let at = |s: f64| {
            let ([l, r], _) = drive_tank(&def, &go(s), 20.0);
            (l - r) / (l + r)
        };
        assert!((at(0.4) - at(0.5)).abs() < 1e-3, "0.4 should snap to the 0.5 detent");
        assert!((at(0.9) - at(1.0)).abs() < 1e-3, "0.9 should snap to the full detent");
        assert!(at(0.02).abs() < 1e-3, "inside the dead band the tank runs straight");
    }

    #[test]
    fn a_clutch_brake_unit_heats_the_inner_brake_only() {
        let def = tank_with(SteerUnitKind::ClutchBrake, SteerLaw::default());
        // up to speed in a straight line, then a hard right turn
        let (_, pt) = drive_tank_with(&def, |t| go(if t < 12.0 { 0.0 } else { 0.9 }), 18.0);
        let t = pt.telemetry().brake_temps_k;
        assert!(t[1] > t[0] + 5.0, "a right turn must heat the right brake only: {t:?}");
    }

    #[test]
    fn a_controlled_differential_turns_at_its_stated_speed_ratio_and_keeps_rolling() {
        // the unit's law says d = 0.3: at full stick the outer track runs (1 + d) / (1 - d) times the inner
        let law = SteerLaw { diff_ratio_by_gear: vec![0.3], max_steer_torque_nm: 60_000.0, ..Default::default() };
        let def = tank_with(SteerUnitKind::ControlledDifferential, law);
        let ([l, r], _) = drive_tank(&def, &go(1.0), 20.0);
        assert!(l > 3.0 && r > 0.0, "it must keep rolling: {l} {r}");
        let frac = (l - r) / (l + r);
        assert!((frac - 0.3).abs() < 0.02, "fractional difference {frac}, wanted 0.3");
    }
}
