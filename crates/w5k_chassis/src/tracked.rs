//! The tracked hull on the chassis integrator (design: `docs/lanes/chassis/tracked-design-delta.md`). The same 6-DoF `Hull` and ledger as the
//! wheeled chassis; road wheels on torsion arms, each with its own travel coordinate; one spinning sprocket per track carrying the inertia of everything
//! the belt links to it; the belt and the soil are TRACKS' `TrackedVehicleGear`.
//!
use w5k_contract::rig::{feature, AeroDef, PhysRig, TrackDef, WheelKind, MAX_SUBSTEPS, SAMPLES_PER_PERIOD, TICK_HZ};
use w5k_contract::{
    DriveInputs, DrivePort, ForceLedger, ForceTerm, Material, ShaftState, SuspensionElement, SuspensionOut, WorldQuery,
    AIR_DENSITY_KG_M3,
};
use w5k_math::{scalar, Quat, StateHasher, Vec3};
use w5k_terramech::belly::BellyGeom;
use w5k_terramech::vehicle::{BellyStepInput, TrackStepInput, TrackedVehicleGear};

use crate::hull::Hull;
use crate::suspension::Suspension;
use crate::tuning::ChassisTuning;
use crate::wheeled::ChassisRefusal;

/// A road wheel's swinging arm, in the hull frame: the wheel centre moves on a circle about `pivot_m` in the hull's y-z plane.
/// The travel coordinate `c` is the centre's vertical rise from the rest position (the contract's convention for every station).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TorsionArm {
    pub pivot_m: Vec3,
    /// Arm length, m.
    pub length_m: f64,
    /// Arm angle below the horizontal at rest, rad.
    pub rest_angle_rad: f64,
    /// +1 when the wheel trails the pivot toward +Z (rearward), -1 when it leads.
    pub fore_aft_sign: f64,
}

impl TorsionArm {
    /// The arm from a station's rest position and its pivot.
    pub fn new(rest_pos_m: Vec3, pivot_m: Vec3) -> TorsionArm {
        let (dy, dz) = (rest_pos_m.y - pivot_m.y, rest_pos_m.z - pivot_m.z);
        let length_m = scalar::hypot(dy, dz);
        TorsionArm { pivot_m, length_m, rest_angle_rad: scalar::atan2(-dy, dz.abs()), fore_aft_sign: scalar::sign(dz) }
    }

    /// Arm angle below the horizontal for travel `c`: `sin(phi) = sin(phi0) - c / L` (clamped short of vertical).
    pub fn angle_rad(&self, c_m: f64) -> f64 {
        let s = scalar::clamp(scalar::sin(self.rest_angle_rad) - c_m / self.length_m, -0.999, 0.999); // const-ok: keeps cos(phi) off zero
        scalar::asin(s)
    }

    /// Wheel centre in the hull frame for travel `c`.
    pub fn centre_m(&self, c_m: f64) -> Vec3 {
        let phi = self.angle_rad(c_m);
        self.pivot_m
            + Vec3::new(0.0, -self.length_m * scalar::sin(phi), self.fore_aft_sign * self.length_m * scalar::cos(phi))
    }

    /// `d centre / dc` in the hull frame: one up, and `tan(phi)` fore-aft (the wheel swings as it rises). A force `F` on the wheel does the
    /// work `F . d` per unit of travel (the generalized force of `c`).
    pub fn centre_rate(&self, c_m: f64) -> Vec3 {
        let phi = self.angle_rad(c_m);
        Vec3::new(0.0, 1.0, self.fore_aft_sign * scalar::tan(phi))
    }

    /// The generalized mass of `c` for an unsprung mass `m` at the wheel centre: `m |d centre / dc|^2 = m / cos^2(phi)`.
    pub fn generalized_mass_kg(&self, unsprung_mass_kg: f64, c_m: f64) -> f64 {
        unsprung_mass_kg * self.centre_rate(c_m).length_sq()
    }
}

/// The inertia every part the belt links rigidly to the sprocket adds to its spin, kg m^2: each other station's `J (r_s / r_i)^2` (it turns
/// `r_s / r_i` times faster) and the belt's mass at the pitch radius, `m_belt r_s^2`.
pub fn reflected_sprocket_inertia_kg_m2(rig: &PhysRig, track: &TrackDef) -> f64 {
    let sprocket = &rig.stations[track.sprocket];
    let r_s = sprocket.wheel.radius_m;
    let linked: f64 = track
        .stations
        .iter()
        .filter(|&&i| i != track.sprocket)
        .map(|&i| &rig.stations[i].wheel)
        .filter(|w| w.radius_m > 0.0)
        .map(|w| w.inertia_kg_m2 * (r_s / w.radius_m) * (r_s / w.radius_m))
        .sum();
    sprocket.wheel.inertia_kg_m2 + linked + track.mass_per_m_kg * track.belt_length_m * r_s * r_s
}

/// How fast the belt's reaction on the sprocket grows with the sprocket's spin, `dR/dw`, N m s/rad: per sample the shear stress responds to the
/// slip speed through TRACKS' damping `kappa = 2 zeta sqrt(mu / (g K))` and, over one step, through the shear spring `mu dt / K`, so
/// `dR/dw = r^2 sum fz (kappa + mu dt / K)`. Saturated samples respond less; the estimate is then an upper bound, which only damps the spin a
/// little more. PROVISIONAL: TRACKS asked to report `dR/dw` from the gear itself (`requests/chassis-tracks-shaft-reaction-rate.md`).
fn belt_reaction_rate(
    t: &w5k_terramech::Tuning,
    outputs: &[w5k_contract::ContactOutput],
    grounds: &[&Material],
    cfg: &w5k_terramech::gear::GearConfig,
    dt_s: f64,
) -> f64 {
    let r = cfg.sprocket_radius_m;
    let per_load: f64 = outputs
        .iter()
        .zip(grounds)
        .map(|(o, g)| {
            let (mu, k) = match &g.soil {
                Some(s) => (scalar::tan(s.friction_angle_rad) * cfg.shoe_mu_scale_soft.unwrap_or(1.0), s.shear_k_m),
                None => (g.mu_peak * cfg.shoe_mu_scale, t.firm_shear_k_m),
            };
            let kappa = 2.0 * t.damping_ratio * scalar::sqrt(mu / (t.gravity_m_s2 * k));
            o.fz_n * (kappa + mu * dt_s / k)
        })
        .sum();
    r * r * per_load
}

/// The optional rig features a tracked vehicle may use: the ones this chassis implements (swinging arms, strut dry friction, the hard bump
/// limit; track running resistance is the gear's) and the ones DRIVE consumes behind the `DrivePort` (steering unit and brakes, brake lag,
/// driveline brake, band-brake reverse factor, differential split, drive modes, free turbine). Anything else is refused, never ignored.
const TRACKED_SUPPORTED: [&str; 12] = [
    feature::SWING_ARM,
    feature::DRY_FRICTION,
    feature::HARD_BUMP_LIMIT,
    feature::TRACK_RESISTANCE,
    feature::STEER_LAW,
    feature::BRAKE_STEERING,
    feature::BRAKE_LAG,
    feature::BRAKE_DRIVELINE_SITE,
    feature::BRAKE_REVERSE_FACTOR,
    feature::DIFF_SPLIT_OR_EFFICIENCY,
    feature::DRIVE_MODES,
    feature::FREE_TURBINE,
];

/// One road wheel: a travel coordinate on a straight strut (`bump_dir`) or a torsion arm, its own unsprung mass, its suspension.
#[derive(Clone, Debug)]
pub struct RoadWheel {
    pub name: String,
    susp: Suspension,
    /// Rest centre relative to the hull's centre of mass (hull axes) and the datum-frame rest position (for the arm).
    rest_body_m: Vec3,
    rest_datum_m: Vec3,
    bump_dir: Vec3,
    arm: Option<TorsionArm>,
    droop_travel_m: f64,
    unsprung_mass_kg: f64,
    /// Wheel radius plus belt thickness: the belt bottom sits this far below the centre, m.
    reach_m: f64,
    pub travel_m: f64,
    pub travel_rate_m_s: f64,
    /// Belt penetration under this wheel and the soil's vertical reaction on it, last substep.
    pub penetration_m: f64,
    pub contact_force_n: f64,
    pub suspension: SuspensionOut,
    /// `d centre / dc` in the world frame used in the last substep (the travel direction the ledger books against).
    pub strut_dir: Vec3,
}

impl RoadWheel {
    /// Wheel centre in the hull frame (relative to the centre of mass) and `d centre / dc`, for the current travel.
    fn centre_and_rate(&self, com: Vec3) -> (Vec3, Vec3) {
        match &self.arm {
            Some(a) => (a.centre_m(self.travel_m) - com, a.centre_rate(self.travel_m)),
            None => (self.rest_body_m + self.bump_dir * self.travel_m, self.bump_dir),
        }
    }
}

/// One track run: its road wheels (indices into `TrackedChassis::wheels`, ascending forward position, as the gear wants them) and its sprocket.
#[derive(Clone, Debug)]
pub struct TrackRun {
    wheels: Vec<usize>,
    /// Contact centre of the ground run, hull frame relative to the centre of mass (x = the track's lateral position).
    centre_body_m: Vec3,
    pub sprocket_omega_rad_s: f64,
    pub sprocket_angle_rad: f64,
    sprocket_j_kg_m2: f64,
    drive_output: Option<usize>,
    pub shaft_reaction_nm: f64,
}

/// The tracked vehicle: hull, road wheels, sprockets, and TRACKS' gear (both belts and the belly).
pub struct TrackedChassis {
    pub hull: Hull,
    com_m: Vec3,
    pub wheels: Vec<RoadWheel>,
    pub tracks: Vec<TrackRun>,
    pub gear: TrackedVehicleGear,
    aero: AeroDef,
    substeps: u32,
    shafts: Vec<ShaftState>,
    torque_out: Vec<f64>,
    pub time_s: f64,
    pub ledger: ForceLedger,
    pub gravity_m_s2: Vec3,
    tracks_tuning: w5k_terramech::Tuning,
    /// Each road wheel's travel acceleration in the last substep, m/s^2.
    pub travel_acc_m_s2: Vec<f64>,
}

impl TrackedChassis {
    /// Build the tracked chassis for `rig` standing at plan position `(x, z)`, heading `yaw_rad`, hull datum at the design ride height.
    #[allow(clippy::too_many_arguments)] // rig, both tunings, belly, world and the starting pose: all distinct inputs
    pub fn new(
        rig: &PhysRig,
        tuning: &ChassisTuning,
        tracks_tuning: w5k_terramech::Tuning,
        belly: Option<BellyGeom>,
        world: &dyn WorldQuery,
        x_m: f64,
        z_m: f64,
        yaw_rad: f64,
    ) -> Result<Self, ChassisRefusal> {
        if let Some(f) = rig.required_features().into_iter().find(|f| !TRACKED_SUPPORTED.contains(f)) {
            return Err(ChassisRefusal::Feature(f.to_string()));
        }
        if rig.tracks.is_empty() {
            return Err(ChassisRefusal::Unsupported("no tracks (use WheeledChassis)".into()));
        }
        let com = rig.hull.com_m;
        let mut wheels = Vec::new();
        let mut runs = Vec::new();
        let mut f_max_hz: f64 = 0.0;
        for t in &rig.tracks {
            let mut idx: Vec<usize> =
                t.stations.iter().copied().filter(|&i| rig.stations[i].wheel.kind == WheelKind::RoadWheel).collect();
            idx.sort_by(|&a, &b| rig.stations[b].rest_pos_m.z.total_cmp(&rig.stations[a].rest_pos_m.z)); // ascending forward (x = -z)
            let mut mine = Vec::new();
            for &i in &idx {
                let s = &rig.stations[i];
                let susp = Suspension::new(&s.suspension, s.bump_travel_m, tuning.suspension());
                let h = 1e-6; // const-ok: finite-difference step for the spring's local rate
                let k_s = (susp.spring_force_n(h) - susp.spring_force_n(-h)) / (2.0 * h);
                if s.unsprung_mass_kg > 0.0 {
                    let k = k_s + t.wheel_contact.vertical_stiffness_n_m;
                    f_max_hz = f_max_hz.max(scalar::sqrt(k / s.unsprung_mass_kg) / (2.0 * core::f64::consts::PI));
                }
                mine.push(wheels.len());
                wheels.push(RoadWheel {
                    name: s.name.clone(),
                    susp,
                    rest_body_m: s.rest_pos_m - com,
                    rest_datum_m: s.rest_pos_m,
                    bump_dir: s.bump_dir.normalized_or_zero(),
                    arm: s.arm_pivot_m.map(|p| TorsionArm::new(s.rest_pos_m, p)),
                    droop_travel_m: s.droop_travel_m,
                    unsprung_mass_kg: s.unsprung_mass_kg,
                    reach_m: s.wheel.radius_m + t.thickness_m,
                    travel_m: 0.0,
                    travel_rate_m_s: 0.0,
                    penetration_m: 0.0,
                    contact_force_n: 0.0,
                    suspension: SuspensionOut::default(),
                    strut_dir: Vec3::ZERO,
                });
            }
            let n = idx.len().max(1) as f64;
            let mean = idx.iter().fold(Vec3::ZERO, |a, &i| a + rig.stations[i].rest_pos_m) / n;
            let reach = idx.iter().map(|&i| rig.stations[i].wheel.radius_m).sum::<f64>() / n + t.thickness_m;
            runs.push(TrackRun {
                wheels: mine,
                centre_body_m: Vec3::new(mean.x, mean.y - reach, mean.z) - com,
                sprocket_omega_rad_s: 0.0,
                sprocket_angle_rad: 0.0,
                sprocket_j_kg_m2: reflected_sprocket_inertia_kg_m2(rig, t),
                drive_output: rig.stations[t.sprocket].drive_output,
                shaft_reaction_nm: 0.0,
            });
        }
        let required = (SAMPLES_PER_PERIOD * f_max_hz / TICK_HZ).ceil() as u32;
        if required > MAX_SUBSTEPS {
            return Err(ChassisRefusal::TooStiff { f_max_hz, required_substeps: required });
        }
        let rot = Quat::from_yaw(yaw_rad);
        let datum = Vec3::new(x_m, world.height_m(x_m, z_m) + rig.ride_height_m, z_m);
        let hull = Hull::new(rig.hull.mass_kg, rig.hull.inertia_kg_m2, datum + rot.rotate(com), rot)
            .map_err(ChassisRefusal::Hull)?;
        let outputs = rig.drivetrain.outputs.len();
        Ok(TrackedChassis {
            hull,
            com_m: com,
            travel_acc_m_s2: vec![0.0; wheels.len()],
            wheels,
            tracks: runs,
            gear: TrackedVehicleGear::new(rig, tracks_tuning, belly),
            tracks_tuning,
            aero: rig.aero.clone(),
            substeps: rig.integration.substeps.max(required).max(1),
            shafts: vec![ShaftState::default(); outputs],
            torque_out: vec![0.0; outputs],
            time_s: 0.0,
            ledger: ForceLedger::off(),
            gravity_m_s2: Vec3::new(0.0, -scalar::G, 0.0),
        })
    }

    pub fn substeps(&self) -> u32 {
        self.substeps
    }

    /// Hull datum position, world frame.
    pub fn datum_m(&self) -> Vec3 {
        self.hull.pos_m - self.hull.rot.rotate(self.com_m)
    }

    pub fn forward_speed_m_s(&self) -> f64 {
        self.hull.vel_m_s.dot(self.hull.rot.rotate(Vec3::FORWARD))
    }

    /// One 60 Hz tick of `substeps` substeps.
    pub fn tick(&mut self, dt_s: f64, inputs: &DriveInputs, world: &dyn WorldQuery, drive: &mut dyn DrivePort) {
        let h = dt_s / f64::from(self.substeps);
        for _ in 0..self.substeps {
            self.substep(h, inputs, world, drive);
        }
    }

    /// One substep in the order of `docs/lanes/chassis/tracked-design-delta.md`.
    pub fn substep(&mut self, dt_s: f64, inputs: &DriveInputs, world: &dyn WorldQuery, drive: &mut dyn DrivePort) {
        let g = self.gravity_m_s2;
        let a_prev = self.hull.acc_m_s2;
        let rot = self.hull.rot;
        // The soil acts in the ground's frame: up along the ground normal, forward along the hull's heading laid onto the ground (not the hull's
        // own axes: a hull pitched by squat would otherwise tilt the belts' support into a fore-aft force).
        let heading = rot.rotate(Vec3::FORWARD);
        let ground_frame = |p: Vec3| {
            let n = world.normal(p.x, p.z);
            let x = heading.reject_from(n).normalized_or_zero();
            (x, n.cross(x), n)
        };
        let omega = self.hull.omega_rad_s();
        self.ledger.clear();

        // 1. belt penetration under each road wheel, against the undeformed ground
        let mut centres = Vec::with_capacity(self.wheels.len());
        for w in self.wheels.iter_mut() {
            let (c_body, rate_dir) = w.centre_and_rate(self.com_m);
            let wc = self.hull.point_world(c_body);
            let v = self.hull.point_velocity(wc) + rot.rotate(rate_dir) * w.travel_rate_m_s;
            w.penetration_m = world.height_m(wc.x, wc.z) - (wc.y - w.reach_m);
            centres.push((wc, rot.rotate(rate_dir), -v.y));
        }

        // 2-3. track-frame velocity at each contact centre and the ground under every sample
        let mut pens: Vec<Vec<f64>> = Vec::new();
        let mut rates: Vec<Vec<f64>> = Vec::new();
        let mut grounds: Vec<Vec<&Material>> = Vec::new();
        let mut vels = Vec::new();
        let mut frames = Vec::new();
        let mut wheel_up = vec![Vec3::Y; self.wheels.len()];
        for (k, t) in self.tracks.iter().enumerate() {
            pens.push(t.wheels.iter().map(|&i| self.wheels[i].penetration_m).collect());
            rates.push(t.wheels.iter().map(|&i| centres[i].2).collect());
            let cp = self.hull.point_world(t.centre_body_m);
            let v = self.hull.point_velocity(cp);
            let (fwd, left, up) = ground_frame(cp);
            vels.push((v.dot(fwd), v.dot(left)));
            frames.push((fwd, left, up));
            for &i in &t.wheels {
                wheel_up[i] = up;
            }
            grounds.push(
                self.gear
                    .track(k)
                    .sample_x_m()
                    .iter()
                    .map(|x| {
                        let p = cp + frames[k].0 * *x;
                        world.materials().get(world.material_id_at(p.x, p.z))
                    })
                    .collect(),
            );
        }

        // DRIVE: one shaft per sprocket output
        let speed = self.forward_speed_m_s();
        for s in self.shafts.iter_mut() {
            *s = ShaftState { vehicle_speed_m_s: speed, ..ShaftState::default() };
        }
        for t in &self.tracks {
            if let Some(k) = t.drive_output {
                self.shafts[k] = ShaftState {
                    omega_rad_s: t.sprocket_omega_rad_s,
                    inertia_kg_m2: t.sprocket_j_kg_m2,
                    vehicle_speed_m_s: speed,
                    load_torque_nm: t.shaft_reaction_nm,
                    ..ShaftState::default()
                };
            }
        }
        drive.step(dt_s, inputs, &self.shafts, &mut self.torque_out);

        // 4. the gear: both belts and the belly
        let steps: Vec<TrackStepInput> = (0..self.tracks.len())
            .map(|k| TrackStepInput {
                wheel_penetration_m: &pens[k],
                wheel_penetration_rate_m_s: &rates[k],
                vel_long_m_s: vels[k].0,
                vel_lat_m_s: vels[k].1,
                yaw_rate_rad_s: omega.dot(frames[k].2),
                sprocket_omega_rad_s: self.tracks[k].sprocket_omega_rad_s,
                ground: &grounds[k],
            })
            .collect();
        let belly_pen = self.wheels.iter().map(|w| w.penetration_m).sum::<f64>() / self.wheels.len().max(1) as f64;
        let hc = self.hull.pos_m;
        let belly_ground = world.materials().get(world.material_id_at(hc.x, hc.z));
        let bv = self.hull.vel_m_s;
        let (fwd, left, up) = ground_frame(hc);
        let belly_in = BellyStepInput {
            belt_penetration_m: belly_pen,
            vel_long_m_s: bv.dot(fwd),
            vel_lat_m_s: bv.dot(left),
            ground: belly_ground,
        };
        self.gear.step(&steps, Some(&belly_in), dt_s);

        // 5. forces: belt shear at every sample onto the hull; the soil's vertical reaction through each road wheel's suspension
        let mut belt_rate = vec![0.0; self.tracks.len()];
        for (k, t) in self.tracks.iter_mut().enumerate() {
            let cfg = self.gear.track(k).config();
            belt_rate[k] =
                belt_reaction_rate(&self.tracks_tuning, self.gear.track(k).outputs(), &grounds[k], cfg, dt_s);
            let cp = self.hull.point_world(t.centre_body_m);
            let (fwd, left, _) = frames[k];
            let totals = self.gear.totals()[k];
            for (o, x) in self.gear.track(k).outputs().iter().zip(self.gear.track(k).sample_x_m()) {
                let (p, f) = (cp + fwd * *x, fwd * o.fx_n + left * o.fy_n);
                self.hull.add_force_at(f, p);
                let r = p - self.hull.pos_m;
                // fx carries the compaction drag; book it apart (at the same point, so the torques still add up)
                let share = if totals.fz_n > 0.0 { o.fz_n / totals.fz_n } else { 0.0 };
                let comp = fwd * (-totals.compaction_n * share);
                self.ledger.add(ForceTerm::TrackShear, 0, f - comp, r.cross(f - comp));
                self.ledger.add(ForceTerm::SoilCompaction, 0, comp, r.cross(comp));
            }
            for (j, &i) in t.wheels.iter().enumerate() {
                self.wheels[i].contact_force_n = self.gear.track(k).wheel_force_n()[j];
            }
            t.shaft_reaction_nm = totals.shaft_reaction_nm;
        }
        let belly = *self.gear.belly_output();
        let belly_f = fwd * belly.fx_n + left * belly.fy_n + up * belly.fz_n;
        self.hull.add_force(belly_f);
        self.ledger.add(ForceTerm::BellyDrag, 0, belly_f, Vec3::ZERO);

        let mut travel_acc = vec![0.0; self.wheels.len()];
        for (i, w) in self.wheels.iter_mut().enumerate() {
            let (wc, d, _) = centres[i];
            let so = w.susp.step(w.travel_m, w.travel_rate_m_s, dt_s);
            w.suspension = so;
            w.strut_dir = d;
            let parts = [
                (ForceTerm::TrackNormal, wheel_up[i] * w.contact_force_n),
                (ForceTerm::Gravity, g * w.unsprung_mass_kg),
            ];
            let struts = [
                (ForceTerm::SuspensionSpring, so.spring_n),
                (ForceTerm::SuspensionDamper, so.damper_n),
                (ForceTerm::BumpStop, so.bump_stop_n),
            ];
            let dd = d.length_sq();
            let q: f64 = parts.iter().map(|(_, f)| f.dot(d)).sum();
            let m_a = a_prev * w.unsprung_mass_kg;
            travel_acc[i] =
                if w.unsprung_mass_kg > 0.0 { (q - so.force_n - m_a.dot(d)) / (w.unsprung_mass_kg * dd) } else { 0.0 };
            // Hull: every force's part across the travel direction (projector I - d d^T / |d|^2), the struts along it, less the wheel's inertia.
            let r = wc - self.hull.pos_m;
            let body = 1 + i as u16;
            let mut on_hull = Vec3::ZERO;
            let mut book = |l: &mut ForceLedger, t: ForceTerm, hull_f: Vec3, wheel_f: Vec3| {
                on_hull += hull_f;
                l.add(t, 0, hull_f, r.cross(hull_f));
                l.add(t, body, wheel_f, Vec3::ZERO);
            };
            for &(t, f) in &parts {
                let along = d * (f.dot(d) / dd);
                book(&mut self.ledger, t, f - along, along);
            }
            for &(t, s) in &struts {
                book(&mut self.ledger, t, d * (s / dd), d * (-s / dd));
            }
            let along = d * (m_a.dot(d) / dd);
            book(&mut self.ledger, ForceTerm::Other, -(m_a - along), -along);
            self.hull.add_force_at(on_hull, wc);
        }

        // gravity and aero drag
        self.hull.add_force(g * self.hull.mass_kg());
        self.ledger.add(ForceTerm::Gravity, 0, g * self.hull.mass_kg(), Vec3::ZERO);
        let v = self.hull.vel_m_s;
        let drag = v * (-0.5 * AIR_DENSITY_KG_M3 * self.aero.drag_coeff * self.aero.frontal_area_m2 * v.length());
        let cp = self.hull.point_world(self.aero.centre_of_pressure_m - self.com_m);
        self.hull.add_force_at(drag, cp);
        self.ledger.add(ForceTerm::Aero, 0, drag, (cp - self.hull.pos_m).cross(drag));

        // sprockets: the drive torque spins the belt; its reaction goes into the hull about the sprocket axle (positive spin axis = left).
        // The belt's shear damping is sized for the tank's mass, far stiffer than the sprocket's own inertia can take explicitly (`c dt / J`
        // near 2 on box_tank), so the spin is implicit in the reaction. The reaction answers the *slip* (belt speed against ground speed), so
        // the prediction is `R' = R + c (dw - dv / r)` with `dv` the track's ground speed change this substep (the hull's acceleration, known now
        // that every force is in): `dw = dt (T - R + c dv / (r dt)) / (J + c dt)`. With hull and belt accelerating together the damping adds
        // nothing (no phantom inertia); a sudden change of slip is damped.
        let axle = rot.rotate(-Vec3::X); // the sprocket axle is fixed in the hull: positive spin about the hull's left
        let a_hull = self.hull.wrench().0 / self.hull.mass_kg();
        for (k, (t, rate)) in self.tracks.iter_mut().zip(&belt_rate).enumerate() {
            let torque = t.drive_output.map_or(0.0, |o| self.torque_out[o]);
            let r_s = self.gear.track(k).config().sprocket_radius_m;
            let dv = a_hull.dot(frames[k].0) * dt_s;
            let dw =
                (dt_s * (torque - t.shaft_reaction_nm) + rate * dv / r_s * dt_s) / (t.sprocket_j_kg_m2 + rate * dt_s);
            t.sprocket_omega_rad_s = scalar::flush_tiny(t.sprocket_omega_rad_s + dw);
            t.sprocket_angle_rad += t.sprocket_omega_rad_s * dt_s;
            self.hull.add_torque(axle * -torque);
            let term = if torque * t.sprocket_omega_rad_s >= 0.0 { ForceTerm::EngineDrive } else { ForceTerm::Brake };
            self.ledger.add(term, 0, Vec3::ZERO, axle * -torque);
        }

        // 6. integrate
        self.hull.integrate(dt_s);
        self.travel_acc_m_s2.clone_from(&travel_acc);
        for (w, acc) in self.wheels.iter_mut().zip(travel_acc) {
            w.travel_rate_m_s += acc * dt_s;
            w.travel_m += w.travel_rate_m_s * dt_s;
            if w.travel_m < -w.droop_travel_m {
                w.travel_m = -w.droop_travel_m;
                w.travel_rate_m_s = w.travel_rate_m_s.max(0.0);
            }
            let (c, r) = w.susp.apply_hard_limit(w.travel_m, w.travel_rate_m_s);
            w.travel_m = c;
            w.travel_rate_m_s = scalar::flush_tiny(r);
        }
        self.time_s += dt_s;
    }

    pub fn is_finite(&self) -> bool {
        self.hull.is_finite()
            && self.wheels.iter().all(|w| w.travel_m.is_finite())
            && self.tracks.iter().all(|t| t.sprocket_omega_rad_s.is_finite())
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        self.hull.hash_state(h);
        for w in &self.wheels {
            h.write_f64(w.travel_m);
            h.write_f64(w.travel_rate_m_s);
        }
        for t in &self.tracks {
            h.write_f64(t.sprocket_omega_rad_s);
            h.write_f64(t.sprocket_angle_rad);
        }
    }
}

impl RoadWheel {
    /// The rest centre in the datum frame (for replay frames).
    pub fn rest_datum_m(&self) -> Vec3 {
        self.rest_datum_m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_tank;

    #[test]
    fn torsion_arm_wheel_centre_follows_the_circle() {
        // a trailing arm 0.45 m long, 20 degrees below horizontal at rest
        let pivot = Vec3::new(1.2, -0.2, -1.0);
        let phi0: f64 = 0.35;
        let rest = pivot + Vec3::new(0.0, -0.45 * scalar::sin(phi0), 0.45 * scalar::cos(phi0));
        let arm = TorsionArm::new(rest, pivot);
        assert!((arm.length_m - 0.45).abs() < 1e-12 && (arm.rest_angle_rad - phi0).abs() < 1e-12);
        assert!((arm.centre_m(0.0) - rest).length() < 1e-12);
        for c in [-0.1, -0.03, 0.0, 0.05, 0.12] {
            let p = arm.centre_m(c);
            // on the circle, and risen by exactly c
            assert!(((p - pivot).length() - 0.45).abs() < 1e-12);
            assert!((p.y - rest.y - c).abs() < 1e-12, "c {c}");
            // the rate is the derivative of the position (central difference)
            let h = 1e-6;
            let fd = (arm.centre_m(c + h) - arm.centre_m(c - h)) * (0.5 / h);
            assert!((fd - arm.centre_rate(c)).length() < 1e-6, "c {c}: {fd:?} vs {:?}", arm.centre_rate(c));
        }
        // at rest the generalized mass is m / cos^2(phi0)
        let c2 = scalar::cos(phi0) * scalar::cos(phi0);
        assert!((arm.generalized_mass_kg(100.0, 0.0) - 100.0 / c2).abs() < 1e-9);
    }

    #[test]
    fn reflected_inertia_spins_up_at_t_over_j_eff() {
        // Integrate the sprocket alone under a constant torque: after t seconds the spin is T t / J_eff, and J_eff carries every linked
        // wheel at its speed ratio squared plus the belt at the pitch radius.
        let (rig, _) = box_tank();
        let track = &rig.tracks[0];
        let j = reflected_sprocket_inertia_kg_m2(&rig, track);
        let r_s = rig.stations[track.sprocket].wheel.radius_m;
        let by_hand = rig.stations[track.sprocket].wheel.inertia_kg_m2
            + track
                .stations
                .iter()
                .filter(|&&i| i != track.sprocket)
                .map(|&i| rig.stations[i].wheel.inertia_kg_m2 * scalar::powi(r_s / rig.stations[i].wheel.radius_m, 2))
                .sum::<f64>()
            + track.mass_per_m_kg * track.belt_length_m * r_s * r_s;
        assert!((j / by_hand - 1.0).abs() < 1e-12);
        assert!(j > rig.stations[track.sprocket].wheel.inertia_kg_m2, "the belt and wheels add inertia");
        let (torque, dt) = (2000.0, 1.0 / 300.0);
        let mut omega = 0.0;
        for _ in 0..300 {
            omega += torque / j * dt;
        }
        assert!((omega - torque * 1.0 / j).abs() < 1e-9);
        // the kinetic energy the torque put in is shared exactly as J_eff says: sprocket, every wheel at its own speed, the belt
        let belt_speed = omega * r_s;
        let parts = 0.5 * rig.stations[track.sprocket].wheel.inertia_kg_m2 * omega * omega
            + track
                .stations
                .iter()
                .filter(|&&i| i != track.sprocket)
                .map(|&i| {
                    let w = belt_speed / rig.stations[i].wheel.radius_m;
                    0.5 * rig.stations[i].wheel.inertia_kg_m2 * w * w
                })
                .sum::<f64>()
            + 0.5 * track.mass_per_m_kg * track.belt_length_m * belt_speed * belt_speed;
        assert!((parts / (0.5 * j * omega * omega) - 1.0).abs() < 1e-12);
    }
}
