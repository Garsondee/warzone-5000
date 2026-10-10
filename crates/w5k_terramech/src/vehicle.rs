//! The running gear of a whole tracked vehicle: one [`TrackedRunningGear`] per track and the hull's [`Belly`], behind one `step` per substep.
//! This is the object the glue holds (see `docs/lanes/tracks/glue-note.md`); it adds no physics, only the bookkeeping of calling the parts in order.

use w5k_contract::rig::PhysRig;
use w5k_contract::{ContactOutput, Material};

use crate::belly::{Belly, BellyGeom};
use crate::gear::{GearConfig, GearInput, GearTotals, TrackedRunningGear};
use crate::tuning::Tuning;

/// One track's inputs for a substep (see [`GearInput`]; the time step is the vehicle's).
pub struct TrackStepInput<'a> {
    pub wheel_penetration_m: &'a [f64],
    pub wheel_penetration_rate_m_s: &'a [f64],
    pub vel_long_m_s: f64,
    pub vel_lat_m_s: f64,
    pub yaw_rate_rad_s: f64,
    pub sprocket_omega_rad_s: f64,
    pub ground: &'a [&'a Material],
}

/// The hull belly's inputs: the belt-bottom penetration (mean of the road wheels), the hull velocity over the ground, the surface under it.
pub struct BellyStepInput<'a> {
    pub belt_penetration_m: f64,
    pub vel_long_m_s: f64,
    pub vel_lat_m_s: f64,
    pub ground: &'a Material,
}

pub struct TrackedVehicleGear {
    tracks: Vec<TrackedRunningGear>,
    belly: Option<Belly>,
    totals: Vec<GearTotals>,
    belly_out: ContactOutput,
}

impl TrackedVehicleGear {
    /// One gear per `PhysRig::tracks` entry, in rig order; the belly from `belly` (`BellyGeom::from_rig` or the reference tank's file).
    pub fn new(rig: &PhysRig, tuning: Tuning, belly: Option<BellyGeom>) -> TrackedVehicleGear {
        let tracks: Vec<TrackedRunningGear> =
            (0..rig.tracks.len()).map(|i| TrackedRunningGear::new(GearConfig::from_rig(rig, i), tuning)).collect();
        TrackedVehicleGear {
            totals: vec![GearTotals::default(); tracks.len()],
            tracks,
            belly: belly.map(|g| Belly::new(g, tuning)),
            belly_out: ContactOutput::default(),
        }
    }

    pub fn track(&self, i: usize) -> &TrackedRunningGear {
        &self.tracks[i]
    }

    /// Per-track sums of the last step: `shear_thrust_n` ("track shear"), `compaction_n` ("soil compaction"), `shaft_reaction_nm` (to DRIVE).
    pub fn totals(&self) -> &[GearTotals] {
        &self.totals
    }

    /// The belly's output of the last step ("belly drag" in the ledger; `fx_n` includes the bulldozing of the plate).
    pub fn belly_output(&self) -> &ContactOutput {
        &self.belly_out
    }

    /// One substep: `tracks` has one entry per track in rig order; `belly` is `None` for a rig without one.
    pub fn step(&mut self, tracks: &[TrackStepInput], belly: Option<&BellyStepInput>, dt_s: f64) {
        assert_eq!(tracks.len(), self.tracks.len(), "one input per track, in rig order");
        for (i, t) in tracks.iter().enumerate() {
            self.totals[i] = self.tracks[i].step(&GearInput {
                wheel_penetration_m: t.wheel_penetration_m,
                wheel_penetration_rate_m_s: t.wheel_penetration_rate_m_s,
                vel_long_m_s: t.vel_long_m_s,
                vel_lat_m_s: t.vel_lat_m_s,
                yaw_rate_rad_s: t.yaw_rate_rad_s,
                sprocket_omega_rad_s: t.sprocket_omega_rad_s,
                ground: t.ground,
                dt_s,
            });
        }
        self.belly_out = match (&mut self.belly, belly) {
            (Some(b), Some(i)) => b.step(i.belt_penetration_m, i.vel_long_m_s, i.vel_lat_m_s, i.ground, dt_s),
            _ => ContactOutput::default(),
        };
    }

    pub fn reset(&mut self) {
        self.tracks.iter_mut().for_each(TrackedRunningGear::reset);
        if let Some(b) = &mut self.belly {
            b.reset();
        }
        self.totals.iter_mut().for_each(|t| *t = GearTotals::default());
        self.belly_out = ContactOutput::default();
    }
}
