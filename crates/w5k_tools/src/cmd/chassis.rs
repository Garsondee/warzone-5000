//! `w5k chassis`: the command line of lane CHASSIS (only that lane edits this file).
//!
//!   w5k chassis strip [--scenario content/physics/chassis/scenarios/mule_strip.ron] --out DIR
//!
//! Compiles the scenario's vehicle with FORGE, drives it on WORLD's standard data strip with the stand-in powertrain and writes
//! `replay.json`, `replay.w5kr`, `rig.json` (the render rig, for `w5k viewer render`) and CSV traces (`heave_pitch.csv`, `speed.csv`,
//! `travel.csv`, `loads.csv`) for `w5k viewer plot`.

use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;
use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::frame::{Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::rig::PhysRig;
use w5k_contract::rig::TICK_HZ;
use w5k_contract::testing::ConstantTorquePowertrain;
use w5k_contract::{
    ContactFrame, DriveInputs, DrivePort, DriveTelemetry, GearRequest, LimitingFactor, Param, VehicleFrame,
};
use w5k_math::{scalar, StateHasher};
use w5k_replay::ReplayFile;
use w5k_world::strip::DataStrip;

const USAGE: &str = "usage: w5k chassis strip [--scenario FILE.ron] --out DIR";
const DEFAULT_SCENARIO: &str = "content/physics/chassis/scenarios/mule_strip.ron";
const TUNING: &str = "content/physics/chassis/tuning.ron";
const MM_PER_M: f64 = 1e3; // const-ok: unit conversion for the CSV traces
const KN_PER_N: f64 = 1e-3; // const-ok: unit conversion for the CSV traces
const PEDAL_PLOT_SCALE: f64 = 10.0; // const-ok: draws a 0..1 pedal on the km/h axis as 0..10

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StripRun {
    vehicle: String,
    extras: String,
    target_speed_m_s: Param,
    speed_gain_per_m_s: Param,
    brake_at_z_m: Param,
    max_seconds: Param,
    wheel_torque_nm: Param,
    brake_torque_nm: Param,
    stopped_below_m_s: Param,
    frame_every_ticks: u32,
}

/// Entry point for `w5k chassis <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("strip") => strip(&args[1..]),
        _ => Err(USAGE.to_string()),
    }
}

fn opt<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn strip(args: &[String]) -> Result<(), String> {
    let out = Path::new(opt(args, "--out").ok_or(USAGE)?);
    let sc: StripRun = ron::from_str(&read(opt(args, "--scenario").unwrap_or(DEFAULT_SCENARIO))?)
        .map_err(|e| format!("scenario: {e}"))?;
    let tuning = ChassisTuning::from_ron(&read(TUNING)?).map_err(|e| format!("{TUNING}: {e}"))?;
    let def = w5k_forge::compile::parse_def(&read(&sc.vehicle)?)?;
    let extras = w5k_forge::compile::parse_extras(&read(&sc.extras)?)?;
    let compiled =
        w5k_forge::compile::compile(&def, &extras).map_err(|e| format!("FORGE refused {}: {e:?}", sc.vehicle))?;
    let rig = &compiled.rig;
    let world = DataStrip::standard();
    let mut chassis = WheeledChassis::new(rig, &tuning, &world, 0.0, 0.0, 0.0)
        .map_err(|e| format!("CHASSIS refused {}: {e:?}", rig.id))?;
    let mut drive =
        ConstantTorquePowertrain::new(rig.drivetrain.outputs.len(), sc.wheel_torque_nm.v, sc.brake_torque_nm.v);

    let dt = 1.0 / TICK_HZ;
    let (mut frames, mut hashes) = (Vec::new(), Vec::new());
    let mut heave = String::from("t_s,heave_mm,pitch_mrad,roll_mrad\n");
    let mut speed = String::from("t_s,speed_kmh,throttle_x10,brake_x10\n");
    let mut travel = String::from("t_s");
    let mut loads = String::from("t_s");
    for s in &chassis.stations {
        let _ = write!(travel, ",{}_mm", s.name);
        let _ = write!(loads, ",{}_kn", s.name);
    }
    travel.push('\n');
    loads.push('\n');
    let y0 = chassis.datum_m().y;
    let mut braking = false;
    let mut bottomed = 0u32;
    let ticks = (sc.max_seconds.v * TICK_HZ) as u64;
    let mut stop_reason = format!("ran the full {} s", sc.max_seconds.v);
    for n in 0..ticks {
        let v = chassis.forward_speed_m_s();
        braking |= chassis.datum_m().z < sc.brake_at_z_m.v;
        let throttle =
            if braking { 0.0 } else { scalar::clamp((sc.target_speed_m_s.v - v) * sc.speed_gain_per_m_s.v, 0.0, 1.0) };
        let inputs = DriveInputs {
            throttle,
            brake: if braking { 1.0 } else { 0.0 },
            gear: GearRequest::Auto,
            ..DriveInputs::default()
        };
        chassis.tick(dt, &inputs, &world, &mut drive);
        if !chassis.is_finite() {
            return Err(format!("non-finite state at t = {:.3} s", chassis.time_s));
        }
        let t = chassis.time_s;
        bottomed += chassis.stations.iter().map(|s| u32::from(s.report.suspension.at_limit)).sum::<u32>();
        if n % u64::from(sc.frame_every_ticks.max(1)) == 0 {
            frames.push(Frame {
                t_s: t,
                vehicles: vec![vehicle_frame(&chassis, rig, &drive.telemetry())],
                events: Vec::new(),
                projectiles: Vec::new(),
            });
            let (_yaw, pitch, roll) = chassis.hull.rot.to_ypr();
            let _ = writeln!(
                heave,
                "{t:.4},{:.3},{:.3},{:.3}",
                (chassis.datum_m().y - y0) * MM_PER_M,
                pitch * MM_PER_M,
                roll * MM_PER_M
            );
            let _ = writeln!(
                speed,
                "{t:.4},{:.3},{:.2},{:.2}",
                scalar::ms_to_kmh(v),
                throttle * PEDAL_PLOT_SCALE,
                inputs.brake * PEDAL_PLOT_SCALE
            );
            let _ = write!(travel, "{t:.4}");
            let _ = write!(loads, "{t:.4}");
            for s in &chassis.stations {
                let _ = write!(travel, ",{:.2}", s.travel_m * MM_PER_M);
                let _ = write!(loads, ",{:.3}", s.report.contact.fz_n * KN_PER_N);
            }
            travel.push('\n');
            loads.push('\n');
        }
        if n % TICK_HZ as u64 == 0 {
            let mut h = StateHasher::new();
            chassis.hash_state(&mut h);
            hashes.push(h.finish());
        }
        if braking && v.abs() < sc.stopped_below_m_s.v {
            stop_reason =
                format!("stopped at z = {:.1} m after braking from z = {} m", chassis.datum_m().z, sc.brake_at_z_m.v);
            break;
        }
    }

    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let replay = ReplayFile {
        header: ReplayHeader {
            version: REPLAY_VERSION,
            scenario: format!("chassis strip: {} ({stop_reason})", rig.id),
            frame_dt_s: dt * f64::from(sc.frame_every_ticks.max(1)),
            vehicles: vec![VehicleHeader {
                name: def.id.clone(),
                rig_id: rig.id.clone(),
                joint_names: rig.joint_names(),
                contact_names: rig.contact_names(),
                livery: None,
            }],
            world: WorldHeader { course: "standard data strip".into(), seed: 0, terrain: None },
            state_hashes: hashes,
        },
        frames,
    };
    w5k_replay::write_json(&out.join("replay.json"), &replay)?;
    w5k_replay::write_bin(&out.join("replay.w5kr"), &replay)?;
    let render = w5k_forge::render::render_rig(rig, compiled.hull_size_m);
    std::fs::write(out.join("rig.json"), serde_json::to_string(&render).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for (name, text) in
        [("heave_pitch.csv", &heave), ("speed.csv", &speed), ("travel.csv", &travel), ("loads.csv", &loads)]
    {
        std::fs::write(out.join(name), text).map_err(|e| format!("cannot write {name}: {e}"))?;
    }
    println!(
        "{}: {stop_reason}; {} frames, {} substeps per tick, travel limit reached in {bottomed} station-ticks",
        rig.id,
        replay.frames.len(),
        chassis.substeps()
    );
    Ok(())
}

/// The replay frame: hull datum pose, joints in `PhysRig::joint_names()` order (spin, steer of steered stations, travel), one contact per station.
fn vehicle_frame(c: &WheeledChassis, rig: &PhysRig, telemetry: &DriveTelemetry) -> VehicleFrame {
    let mut joints: Vec<f32> = c.stations.iter().map(|s| s.spin_angle_rad as f32).collect();
    joints.extend(
        c.stations.iter().zip(&rig.stations).filter(|(_, d)| d.steer.is_some()).map(|(s, _)| s.steer_rad as f32),
    );
    joints.extend(c.stations.iter().map(|s| s.travel_m as f32));
    let contacts = c
        .stations
        .iter()
        .map(|s| {
            let k = &s.report.contact;
            let flags =
                u8::from(k.fz_n > 0.0) | (u8::from(k.saturated) << 1) | (u8::from(s.report.suspension.at_limit) << 3);
            ContactFrame {
                flags,
                normal_force_n: k.fz_n as f32,
                sinkage_m: k.sinkage_m as f32,
                slip: k.slip_ratio as f32,
                material: s.report.material.0,
            }
        })
        .collect();
    VehicleFrame {
        vehicle: 0,
        pos_m: c.datum_m(),
        rot: c.hull.rot,
        lin_vel_m_s: c.hull.vel_m_s,
        ang_vel_rad_s: c.hull.omega_rad_s(),
        joints,
        engine_rpm: telemetry.engine_rpm as f32,
        gear: telemetry.gear,
        contacts,
        ledger_n: Vec::new(),
        limiting: LimitingFactor::None,
        weapons: Vec::new(),
    }
}
