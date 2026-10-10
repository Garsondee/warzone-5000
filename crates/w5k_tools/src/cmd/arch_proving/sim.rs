//! One truck on one world with DRIVE's real powertrain: the bench every proving-ground test runs on. It ticks the fixed step, keeps the
//! replay frames and ends the run early (saying why) on a non-finite state or a rollover.

use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::frame::Frame;
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::{DriveInputs, DrivePort, GearRequest, WorldQuery};
use w5k_drive::powertrain::{Powertrain, Tunings};
use w5k_math::{StateHasher, Vec3};

use crate::cmd::arch_course::vehicle_frame;

pub(crate) struct Sim<'a> {
    pub rig: &'a PhysRig,
    pub world: Box<dyn WorldQuery>,
    pub chassis: WheeledChassis,
    pub drive: Powertrain,
    pub frames: Vec<Frame>,
    pub ticks: u64,
    every: u64,
    /// Why the run ended early (rolled over, non-finite state); once set, `tick` does nothing.
    pub early: Option<String>,
}

impl<'a> Sim<'a> {
    pub fn new(
        rig: &'a PhysRig,
        tuning: &ChassisTuning,
        world: Box<dyn WorldQuery>,
        frame_every_ticks: u32,
    ) -> Result<Sim<'a>, String> {
        let chassis = WheeledChassis::new(rig, tuning, world.as_ref(), 0.0, 0.0, 0.0)
            .map_err(|e| format!("CHASSIS refused {}: {e:?}", rig.id))?;
        let drive = Powertrain::new(&rig.drivetrain, &Tunings::shipped())
            .map_err(|e| format!("DRIVE refused the drivetrain of {}: {e}", rig.id))?;
        Ok(Sim {
            rig,
            world,
            chassis,
            drive,
            frames: Vec::new(),
            ticks: 0,
            every: u64::from(frame_every_ticks.max(1)),
            early: None,
        })
    }

    /// One 60 Hz tick. Returns false (and steps nothing) once the run has ended early.
    pub fn tick(&mut self, inputs: &DriveInputs) -> bool {
        if self.early.is_some() {
            return false;
        }
        self.chassis.tick(1.0 / TICK_HZ, inputs, self.world.as_ref(), &mut self.drive);
        if !self.chassis.is_finite() {
            self.early = Some(format!("non-finite state at t = {:.3} s", self.chassis.time_s));
        } else if self.chassis.hull.rot.rotate(Vec3::Y).y <= 0.0 {
            self.early = Some(format!("rolled over at t = {:.3} s", self.chassis.time_s));
        }
        if self.ticks.is_multiple_of(self.every) {
            let tel = self.drive.telemetry();
            self.frames.push(Frame {
                t_s: self.chassis.time_s,
                vehicles: vec![vehicle_frame(&self.chassis, self.rig, &tel)],
                events: Vec::new(),
                projectiles: Vec::new(),
            });
        }
        self.ticks += 1;
        self.early.is_none()
    }

    /// Hold the truck parked (drive selected, parking brake on) for `seconds`.
    pub fn park(&mut self, seconds: f64) {
        let parked = DriveInputs { gear: GearRequest::Auto, parking_brake: true, ..DriveInputs::default() };
        for _ in 0..(seconds * TICK_HZ).round() as u64 {
            self.tick(&parked);
        }
    }

    pub fn speed_m_s(&self) -> f64 {
        self.chassis.forward_speed_m_s()
    }

    /// Hash of the whole simulation state (chassis and powertrain): two runs of a test must agree on it.
    pub fn hash(&self) -> u64 {
        let mut h = StateHasher::new();
        self.chassis.hash_state(&mut h);
        self.drive.hash_state(&mut h);
        h.finish()
    }
}
