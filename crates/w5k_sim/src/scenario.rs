//! The scenario runner: step a vehicle through a world under a scripted driver and record what happened.

use serde::Serialize;
use w5k_contract::frame::{Event, Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::testing::{BumpStrip, RigidBoxVehicle, ScriptedCommands};
use w5k_contract::{ForceLedger, VehicleModel, WorldQuery};
use w5k_math::StateHasher;

/// The fixed outer tick of the simulation (Hz). Vehicles split it into their own substeps.
pub const TICK_HZ: f64 = 60.0; // const-ok: the fixed outer tick, a design decision (ADR-0002)

/// What a run did, in a few numbers a human (and a golden test) can read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Summary {
    pub frames: usize,
    pub duration_s: f64,
    /// Path length driven, m.
    pub distance_m: f64,
    pub max_speed_m_s: f64,
    pub final_speed_m_s: f64,
    pub max_abs_pitch_deg: f64,
    pub max_abs_roll_deg: f64,
    /// Largest suspension travel seen (m), over every station.
    pub max_abs_travel_m: f64,
    /// False if any frame contained a NaN or an infinity (the run is invalid then).
    pub all_finite: bool,
}

pub struct ScenarioResult {
    pub header: ReplayHeader,
    pub frames: Vec<Frame>,
    /// Hash of the final simulation state.
    pub final_hash: u64,
    pub summary: Summary,
}

/// Everything a first-light run needs. Today every part is a stand-in; ARCH replaces them one at a time as the lanes land.
pub struct FirstLightParts {
    pub name: String,
    pub course: String,
    pub world: Box<dyn WorldQuery>,
    pub vehicle: Box<dyn VehicleModel>,
    pub driver: ScriptedCommands,
    pub duration_s: f64,
}

impl FirstLightParts {
    /// The stand-in configuration: a kinematic box truck on the standard bump strip.
    pub fn stand_ins() -> FirstLightParts {
        let (rig, _) = w5k_contract::testing::box_truck();
        FirstLightParts {
            name: "first_light".into(),
            course: "bump_strip".into(),
            world: Box::new(BumpStrip::standard()),
            vehicle: Box::new(RigidBoxVehicle::new(rig, 0.0, 0.0, 0.0)),
            driver: ScriptedCommands::first_light(),
            duration_s: 40.0, // const-ok: length of the scripted first-light drive
        }
    }
}

/// Run the first-light scenario with the given parts.
pub fn first_light(mut p: FirstLightParts) -> ScenarioResult {
    run(
        &mut *p.vehicle,
        &*p.world,
        &p.driver,
        p.duration_s,
        30.0, /* const-ok: replay frame rate (Hz) */
        &p.name,
        &p.course,
    )
}

/// Step `model` for `duration_s` seconds at [`TICK_HZ`] under `script`, recording `record_hz` frames per second.
pub fn run(
    model: &mut dyn VehicleModel,
    world: &dyn WorldQuery,
    script: &ScriptedCommands,
    duration_s: f64,
    record_hz: f64,
    scenario: &str,
    course: &str,
) -> ScenarioResult {
    let dt = 1.0 / TICK_HZ;
    let ticks = (duration_s * TICK_HZ).round() as usize;
    let every = (TICK_HZ / record_hz).round().max(1.0) as usize;
    let mut ledger = ForceLedger::off();
    let mut frames = Vec::new();
    let mut state_hashes = Vec::new();
    let mut chain = StateHasher::new();
    let mut s = Summary {
        frames: 0,
        duration_s,
        distance_m: 0.0,
        max_speed_m_s: 0.0,
        final_speed_m_s: 0.0,
        max_abs_pitch_deg: 0.0,
        max_abs_roll_deg: 0.0,
        max_abs_travel_m: 0.0,
        all_finite: true,
    };
    let names = model.rig().joint_names();
    let n_st = model.rig().stations.len();
    let n_steer = model.rig().stations.iter().filter(|st| st.steer.is_some()).count();
    let travel_range = (n_st + n_steer)..(2 * n_st + n_steer);
    let mut last = model.frame().pos_m;
    let mut pending: Vec<Event> = Vec::new();
    for k in 0..ticks {
        let cmd = script.at(k as f64 * dt);
        let rep = model.step(dt, &cmd, world, &mut ledger);
        model.drain_events(&mut pending);
        let mut f = model.frame();
        f.vehicle = 0;
        s.distance_m += (f.pos_m - last).length();
        last = f.pos_m;
        s.max_speed_m_s = s.max_speed_m_s.max(rep.speed_m_s);
        s.final_speed_m_s = rep.speed_m_s;
        if !f.is_finite() {
            s.all_finite = false;
        }
        if (k + 1) % every == 0 {
            let (_, pitch, roll) = f.rot.to_ypr();
            s.max_abs_pitch_deg = s.max_abs_pitch_deg.max(w5k_math::scalar::rad_to_deg(pitch.abs()));
            s.max_abs_roll_deg = s.max_abs_roll_deg.max(w5k_math::scalar::rad_to_deg(roll.abs()));
            for t in &f.joints[travel_range.clone()] {
                s.max_abs_travel_m = s.max_abs_travel_m.max(t.abs() as f64);
            }
            frames.push(Frame {
                t_s: (k + 1) as f64 * dt,
                vehicles: vec![f],
                events: std::mem::take(&mut pending),
                projectiles: vec![],
            });
        }
        if (k + 1) % (TICK_HZ as usize) == 0 {
            model.hash_state(&mut chain);
            state_hashes.push(chain.finish());
        }
    }
    s.frames = frames.len();
    let rig = model.rig();
    let header = ReplayHeader {
        version: REPLAY_VERSION,
        scenario: scenario.into(),
        frame_dt_s: 1.0 / record_hz,
        vehicles: vec![VehicleHeader {
            name: rig.id.clone(),
            rig_id: rig.id.clone(),
            joint_names: names,
            contact_names: rig.stations.iter().map(|st| st.name.clone()).collect(),
            livery: None,
        }],
        world: WorldHeader { course: course.into(), seed: 0, terrain: None },
        state_hashes,
    };
    let mut h = StateHasher::new();
    model.hash_state(&mut h);
    ScenarioResult { header, frames, final_hash: h.finish(), summary: s }
}
