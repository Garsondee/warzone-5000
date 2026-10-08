//! Canned replays for the viewers: run a stand-in vehicle on a stand-in world with a scripted driver and record frames.
//! `w5k_sim` has the real scenario runner; this is the same loop in miniature so the contract crate needs nothing above it.

use w5k_math::StateHasher;

use crate::command::Command;
use crate::frame::{ContactFrame, Event, Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use crate::ledger::ForceLedger;
use crate::render::RenderRig;
use crate::rig::PhysRig;
use crate::testing::rigs::{box_tank, box_truck};
use crate::testing::script::ScriptedCommands;
use crate::testing::vehicle::RigidBoxVehicle;
use crate::testing::world::{BumpStrip, FlatPlane};
use crate::vehicle::VehicleModel;
use crate::world::WorldQuery;

/// Run `rig` as a [`RigidBoxVehicle`] from `(x, z)` facing -Z for `duration_s` seconds at 60 Hz, recording `record_hz` frames per
/// second. Returns the header, the frames and the final state hash.
#[allow(clippy::too_many_arguments)]
pub fn run_stand_in(
    scenario: &str,
    rig: &PhysRig,
    world: &dyn WorldQuery,
    course: &str,
    start: (f64, f64),
    script: &ScriptedCommands,
    duration_s: f64,
    record_hz: f64,
) -> (ReplayHeader, Vec<Frame>, u64) {
    let tick_hz = 60.0;
    let dt = 1.0 / tick_hz;
    let mut v = RigidBoxVehicle::new(rig.clone(), start.0, start.1, 0.0);
    let mut ledger = ForceLedger::off();
    let ticks = (duration_s * tick_hz).round() as usize;
    let every = (tick_hz / record_hz).round().max(1.0) as usize;
    let mut frames = Vec::new();
    let mut state_hashes = Vec::new();
    let mut chain = StateHasher::new();
    let mut pending: Vec<Event> = Vec::new();
    for k in 0..ticks {
        let t = k as f64 * dt;
        let cmd: Command = script.at(t);
        v.step(dt, &cmd, world, &mut ledger);
        v.drain_events(&mut pending);
        if (k + 1) % every == 0 {
            let mut f = v.frame();
            f.vehicle = 0;
            frames.push(Frame {
                t_s: (k + 1) as f64 * dt,
                vehicles: vec![f],
                events: std::mem::take(&mut pending),
                projectiles: vec![],
            });
        }
        if (k + 1) % (tick_hz as usize) == 0 {
            v.hash_state(&mut chain);
            state_hashes.push(chain.finish());
        }
    }
    let n_contacts = rig.stations.len();
    for f in &mut frames {
        for vf in &mut f.vehicles {
            if vf.contacts.len() != n_contacts {
                vf.contacts = vec![ContactFrame::default(); n_contacts];
            }
        }
    }
    let header = ReplayHeader {
        version: REPLAY_VERSION,
        scenario: scenario.into(),
        frame_dt_s: 1.0 / record_hz,
        vehicles: vec![VehicleHeader {
            name: rig.id.clone(),
            rig_id: rig.id.clone(),
            joint_names: rig.joint_names(),
            contact_names: rig.stations.iter().map(|s| s.name.clone()).collect(),
            livery: None,
        }],
        world: WorldHeader { course: course.into(), seed: 0, terrain: None },
        state_hashes,
    };
    let mut h = StateHasher::new();
    v.hash_state(&mut h);
    (header, frames, h.finish())
}

/// The truck driving the bump strip: launch, hump, washboard, plateau, pothole, mud patch, turn, brake. 40 s at 30 Hz.
pub fn truck_over_bumps() -> (ReplayHeader, Vec<Frame>, RenderRig) {
    let (rig, render) = box_truck();
    let (h, f, _) = run_stand_in(
        "truck_over_bumps",
        &rig,
        &BumpStrip::standard(),
        "bump_strip",
        (0.0, 0.0),
        &ScriptedCommands::first_light(),
        40.0,
        30.0,
    );
    (h, f, render)
}

/// The tank creeping along flat ground while the turret slews through a full circle, the gun elevates and fires once. 36 s at 30 Hz.
pub fn tank_slew_and_pitch() -> (ReplayHeader, Vec<Frame>, RenderRig) {
    let (rig, render) = box_tank();
    let (h, f, _) = run_stand_in(
        "tank_slew_and_pitch",
        &rig,
        &FlatPlane::new(),
        "flat_plane",
        (0.0, 0.0),
        &ScriptedCommands::tank_demo(),
        36.0,
        30.0,
    );
    (h, f, render)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canned_replays_are_complete_finite_and_consistent_with_their_rigs() {
        for (h, frames, render) in [truck_over_bumps(), tank_slew_and_pitch()] {
            assert!(frames.len() > 900, "{} frames", frames.len());
            assert_eq!(h.vehicles[0].joint_names.len(), render.joint_count);
            assert!(!h.state_hashes.is_empty());
            for f in &frames {
                assert_eq!(f.vehicles[0].joints.len(), render.joint_count);
                assert!(f.vehicles[0].is_finite());
            }
        }
    }

    #[test]
    fn the_truck_covers_the_whole_strip_and_the_tank_fires() {
        let (_, frames, _) = truck_over_bumps();
        let end_z = frames.last().unwrap().vehicles[0].pos_m.z;
        assert!(end_z < -150.0, "the truck should have driven well past the pothole, ended at z = {end_z}");
        let (_, tank, _) = tank_slew_and_pitch();
        let n = tank[0].vehicles[0].joints.len();
        let max_recoil = tank.iter().map(|f| f.vehicles[0].joints[n - 1]).fold(0.0f32, f32::max);
        let max_yaw = tank.iter().map(|f| f.vehicles[0].joints[n - 3].abs()).fold(0.0f32, f32::max);
        assert!(max_recoil > 0.05, "the gun fired");
        assert!(max_yaw > 3.0, "the turret swung round: {max_yaw}");
    }
}
