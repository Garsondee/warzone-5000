//! One chassis for ARCH's runners: a rig with wheels steps on `WheeledChassis`, a rig with tracks on `TrackedChassis`.
//!
//! The scripted drivers (`arch_course`) only need a pose, a speed, a tick, a hash and a replay frame, so they see this enum and never
//! which one they are driving. Nothing here adds physics: every number is read from the chassis it wraps.

use w5k_chassis::hull::Hull;
use w5k_chassis::tracked::TrackedChassis;
use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::rig::PhysRig;
use w5k_contract::{ContactFrame, DriveInputs, DriveTelemetry, LimitingFactor, VehicleFrame, WorldQuery};
use w5k_drive::powertrain::Powertrain;
use w5k_math::{StateHasher, Vec3};
use w5k_terramech::belly::BellyGeom;
use w5k_terramech::Tuning as TracksTuning;

pub(crate) enum AnyChassis {
    Wheeled(Box<WheeledChassis>),
    Tracked(Box<TrackedChassis>),
}

impl AnyChassis {
    /// Build the chassis the rig asks for, standing at plan position `(x_m, z_m)` with the heading `yaw_rad`.
    pub(crate) fn new(
        rig: &PhysRig,
        tuning: &ChassisTuning,
        world: &dyn WorldQuery,
        x_m: f64,
        z_m: f64,
        yaw_rad: f64,
    ) -> Result<AnyChassis, String> {
        if rig.tracks.is_empty() {
            let c = WheeledChassis::new(rig, tuning, world, x_m, z_m, yaw_rad)
                .map_err(|e| format!("CHASSIS refused {}: {e:?}", rig.id))?;
            return Ok(AnyChassis::Wheeled(Box::new(c)));
        }
        let tracks = TracksTuning::shipped();
        let belly = BellyGeom::from_rig(rig, &tracks);
        let c = TrackedChassis::new(rig, tuning, tracks, belly, world, x_m, z_m, yaw_rad)
            .map_err(|e| format!("CHASSIS refused {}: {e:?}", rig.id))?;
        Ok(AnyChassis::Tracked(Box::new(c)))
    }

    pub(crate) fn is_tracked(&self) -> bool {
        matches!(self, AnyChassis::Tracked(_))
    }

    pub(crate) fn hull(&self) -> &Hull {
        match self {
            AnyChassis::Wheeled(c) => &c.hull,
            AnyChassis::Tracked(c) => &c.hull,
        }
    }

    pub(crate) fn time_s(&self) -> f64 {
        match self {
            AnyChassis::Wheeled(c) => c.time_s,
            AnyChassis::Tracked(c) => c.time_s,
        }
    }

    pub(crate) fn datum_m(&self) -> Vec3 {
        match self {
            AnyChassis::Wheeled(c) => c.datum_m(),
            AnyChassis::Tracked(c) => c.datum_m(),
        }
    }

    pub(crate) fn forward_speed_m_s(&self) -> f64 {
        match self {
            AnyChassis::Wheeled(c) => c.forward_speed_m_s(),
            AnyChassis::Tracked(c) => c.forward_speed_m_s(),
        }
    }

    pub(crate) fn tick(&mut self, dt_s: f64, inputs: &DriveInputs, world: &dyn WorldQuery, drive: &mut Powertrain) {
        match self {
            AnyChassis::Wheeled(c) => c.tick(dt_s, inputs, world, drive),
            AnyChassis::Tracked(c) => c.tick(dt_s, inputs, world, drive),
        }
    }

    pub(crate) fn is_finite(&self) -> bool {
        match self {
            AnyChassis::Wheeled(c) => c.is_finite(),
            AnyChassis::Tracked(c) => c.is_finite(),
        }
    }

    pub(crate) fn hash_state(&self, h: &mut StateHasher) {
        match self {
            AnyChassis::Wheeled(c) => c.hash_state(h),
            AnyChassis::Tracked(c) => c.hash_state(h),
        }
    }

    /// Largest steered-wheel angle, rad (0 for a tracked rig: it steers with its sprocket speeds).
    pub(crate) fn steer_angle_rad(&self) -> f64 {
        match self {
            AnyChassis::Wheeled(c) => c.stations.iter().map(|s| s.steer_rad.abs()).fold(0.0, f64::max),
            AnyChassis::Tracked(_) => 0.0,
        }
    }

    /// Some contact patch is at its friction (or soil shear) limit.
    pub(crate) fn at_traction_limit(&self) -> bool {
        match self {
            AnyChassis::Wheeled(c) => c.stations.iter().any(|s| s.report.contact.saturated),
            AnyChassis::Tracked(c) => {
                (0..c.tracks.len()).any(|k| c.gear.track(k).outputs().iter().any(|o| o.saturated))
            }
        }
    }

    /// The load carried by each wheel (tyre normal force, or the soil's reaction on a road wheel), N, with its name.
    pub(crate) fn wheel_loads(&self) -> Vec<(&str, f64)> {
        match self {
            AnyChassis::Wheeled(c) => c.stations.iter().map(|s| (s.name.as_str(), s.report.contact.fz_n)).collect(),
            AnyChassis::Tracked(c) => c.wheels.iter().map(|w| (w.name.as_str(), w.contact_force_n)).collect(),
        }
    }

    /// The replay frame: hull datum pose, joints in `PhysRig::joint_names()` order, contacts in `PhysRig::contact_names()` order.
    pub(crate) fn frame(&self, rig: &PhysRig, telemetry: &DriveTelemetry) -> VehicleFrame {
        match self {
            AnyChassis::Wheeled(c) => crate::cmd::arch_course::vehicle_frame(c, rig, telemetry),
            AnyChassis::Tracked(c) => tracked_frame(c, rig, telemetry),
        }
    }
}

/// A tracked rig's frame. The belt links every station of a track to its sprocket, so a road wheel or idler turns by the sprocket's angle times
/// the ratio of the radii (exact kinematics, no extra state); a road wheel's travel is its own; one contact per belt sample.
fn tracked_frame(c: &TrackedChassis, rig: &PhysRig, telemetry: &DriveTelemetry) -> VehicleFrame {
    let mut spin = vec![0.0_f32; rig.stations.len()];
    for (k, def) in rig.tracks.iter().enumerate() {
        let sprocket_r = rig.stations[def.sprocket].wheel.radius_m;
        let angle = c.tracks[k].sprocket_angle_rad;
        for &i in def.stations.iter().chain([&def.sprocket, &def.idler]) {
            spin[i] = (angle * sprocket_r / rig.stations[i].wheel.radius_m) as f32;
        }
    }
    let mut joints = spin;
    joints.extend(rig.stations.iter().filter(|s| s.steer.is_some()).map(|_| 0.0_f32));
    joints.extend(
        rig.stations.iter().map(|s| c.wheels.iter().find(|w| w.name == s.name).map_or(0.0, |w| w.travel_m as f32)),
    );
    let mut contacts = Vec::new();
    for k in 0..rig.tracks.len() {
        for o in c.gear.track(k).outputs() {
            let flags = u8::from(o.fz_n > 0.0) | (u8::from(o.saturated) << 1) | (u8::from(o.sinkage_m > 0.0) << 2);
            contacts.push(ContactFrame {
                flags,
                normal_force_n: o.fz_n as f32,
                sinkage_m: o.sinkage_m as f32,
                slip: o.slip_ratio as f32,
                material: 0,
            });
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::FlatPlane;
    use w5k_contract::GearRequest;
    use w5k_drive::powertrain::Tunings;
    use w5k_forge::compile::{compile, parse_def, parse_extras};

    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn carrier() -> PhysRig {
        let dir = root().join("content/vehicles/game");
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("file");
        let def = parse_def(&read("carrier_tracked.ron")).expect("def");
        let extras = parse_extras(&read("carrier_tracked.extras.ron")).expect("extras");
        compile(&def, &extras).expect("compiles").rig
    }

    /// Drive the carrier on flat ground: `settle_s` straight at the given throttle, then `steer` for `turn_s`. Returns (yaw rate, speed) samples.
    fn turn(steer: f64, throttle: f64, settle_s: f64, turn_s: f64) -> Vec<(f64, f64, f64)> {
        let rig = carrier();
        let tuning = ChassisTuning::from_ron(
            &std::fs::read_to_string(root().join("content/physics/chassis/tuning.ron")).expect("tuning"),
        )
        .expect("chassis tuning");
        let world = FlatPlane::new();
        let mut c = AnyChassis::new(&rig, &tuning, &world, 0.0, 0.0, 0.0).expect("chassis");
        let mut drive = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).expect("drive");
        let dt = 1.0 / 60.0; // const-ok: the fixed tick of the test
        let mut out = Vec::new();
        let n = ((settle_s + turn_s) / dt) as usize;
        for k in 0..n {
            let t = k as f64 * dt;
            let st = if t >= settle_s { steer } else { 0.0 };
            c.tick(
                dt,
                &DriveInputs { throttle, steer: st, gear: GearRequest::Auto, ..DriveInputs::default() },
                &world,
                &mut drive,
            );
            if k % 30 == 0 {
                out.push((t, c.hull().omega_rad_s().y, c.forward_speed_m_s()));
            }
        }
        out
    }

    /// A tracked rig turns by running its tracks at different speeds: at full steer demand it must turn at a useful rate and keep rolling.
    /// Was red until DRIVE modelled the controlled differential as a regenerative servo (it used to brake the inner track dead and stall):
    /// `docs/swarm/requests/arch-carrier-steering.md`.
    #[test]
    fn a_full_steer_demand_turns_the_carrier_at_a_useful_rate_and_keeps_it_rolling() {
        let o = turn(1.0, 0.4, 6.0, 8.0);
        let (w, v) =
            o.iter().filter(|x| x.0 > 9.0).fold((0.0_f64, f64::MAX), |a, x| (a.0.max(x.1.abs()), a.1.min(x.2.abs())));
        assert!(w > 0.2, "peak yaw rate {w} rad/s"); // const-ok: a 20 t carrier pivots at several tenths of a rad/s
        assert!(v > 0.5, "the carrier stopped (slowest {v} m/s) instead of turning");
        // const-ok: it must keep rolling through the turn
    }
}
