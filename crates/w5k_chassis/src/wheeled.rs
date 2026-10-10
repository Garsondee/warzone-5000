//! The wheeled chassis: one 6-DoF hull plus one station per wheel (suspension travel, unsprung mass, wheel spin, steer, tyre).
//!
//! Reduced coordinates, no constraint solver. Each station's wheel centre moves along its `bump_dir` relative to the hull; its travel `c`
//! is a coordinate with its own mass (the unsprung mass). Along the strut the wheel feels the tyre, the strut and gravity; across the strut
//! it is carried by the hull, so the hull receives the tyre's cross-strut force at the wheel centre (less the wheel's own inertia, using the
//! hull acceleration of the previous substep). Each substep follows the reference order of `CONTRACTS.md`.

use w5k_contract::rig::{AeroDef, PhysRig, StationDef, WheelKind};
use w5k_contract::{
    ContactElement, ContactInput, ContactOutput, DriveInputs, DrivePort, MaterialId, ShaftState, SuspensionElement,
};
use w5k_contract::{ForceLedger, ForceTerm, SuspensionOut, WorldQuery, AIR_DENSITY_KG_M3};
use w5k_math::{scalar, Quat, StateHasher, Vec3};

use crate::hull::{Hull, HullError};
use crate::steering::{wheel_angle_rad, SteerGeometry};
use crate::suspension::Suspension;
use crate::tuning::ChassisTuning;
use crate::tyre::Tyre;

/// Why a rig was refused (the solver never silently ignores a field it does not implement).
#[derive(Clone, Debug, PartialEq)]
pub enum ChassisRefusal {
    /// An optional rig feature (`PhysRig::required_features`) this solver does not implement yet.
    Feature(String),
    /// A part of the rig outside this solver's scope (tracks, swinging arms, a station without a tyre).
    Unsupported(String),
    Hull(HullError),
    /// The stiffest mode needs more substeps than the contract allows (`MAX_SUBSTEPS`).
    TooStiff {
        f_max_hz: f64,
        required_substeps: u32,
    },
}

/// What one station did in the last substep (for frames, plots and the ledger).
#[derive(Clone, Copy, Debug, Default)]
pub struct StationReport {
    pub contact: ContactOutput,
    pub suspension: SuspensionOut,
    pub contact_point_m: Vec3,
    pub normal: Vec3,
    pub material: MaterialId,
    /// Force the station puts on the hull, world frame, N.
    pub force_on_hull_n: Vec3,
    /// The strut (travel) direction this substep, world frame.
    pub strut_dir: Vec3,
}

#[derive(Clone, Debug)]
pub struct Station {
    pub name: String,
    susp: Suspension,
    tyre: Tyre,
    /// Wheel-centre rest position relative to the hull centre of mass, hull axes, m.
    rest_body_m: Vec3,
    bump_dir: Vec3,
    droop_travel_m: f64,
    unsprung_mass_kg: f64,
    inertia_kg_m2: f64,
    radius_m: f64,
    drive_output: Option<usize>,
    steer: Option<(w5k_contract::rig::SteerDef, SteerGeometry)>,
    /// Travel (m, positive = compressed) and its rate.
    pub travel_m: f64,
    pub travel_rate_m_s: f64,
    /// Wheel spin (rad/s, positive = rolling forward) and the accumulated spin angle (continuous, rad).
    pub omega_rad_s: f64,
    pub spin_angle_rad: f64,
    pub steer_rad: f64,
    pub report: StationReport,
    anti_roll_n: f64,
}

pub struct WheeledChassis {
    pub hull: Hull,
    pub stations: Vec<Station>,
    com_m: Vec3,
    aero: AeroDef,
    anti_roll: Vec<(usize, usize, f64)>,
    substeps: u32,
    shafts: Vec<ShaftState>,
    torque_out: Vec<f64>,
    pub time_s: f64,
    /// Every force of the last substep, by term and body (0 = hull, 1 + i = station i). Off unless switched on.
    pub ledger: ForceLedger,
    /// Each station's travel acceleration in the last substep, m/s^2 (the ledger's station bodies are booked in the hull's frame).
    pub travel_acc_m_s2: Vec<f64>,
    /// Gravity, world frame, m/s^2. Straight down by default; a tilt-table bench rotates it instead of tilting the ground.
    pub gravity_m_s2: Vec3,
}

impl WheeledChassis {
    /// Build the chassis for `rig`, standing at plan position `(x, z)` with heading `yaw_rad`, hull datum at the design ride height.
    pub fn new(
        rig: &PhysRig,
        tuning: &ChassisTuning,
        world: &dyn WorldQuery,
        x_m: f64,
        z_m: f64,
        yaw_rad: f64,
    ) -> Result<Self, ChassisRefusal> {
        if let Some(f) = rig.required_features().first() {
            return Err(ChassisRefusal::Feature((*f).to_string()));
        }
        if !rig.tracks.is_empty() {
            return Err(ChassisRefusal::Unsupported("tracks (lane TRACKS)".into()));
        }
        let com = rig.hull.com_m;
        let mut stations = Vec::new();
        let unsteered: Vec<&StationDef> = rig.stations.iter().filter(|s| s.steer.is_none()).collect();
        let z_ref = if unsteered.is_empty() {
            0.0
        } else {
            unsteered.iter().map(|s| s.rest_pos_m.z).sum::<f64>() / unsteered.len() as f64
        };
        for s in &rig.stations {
            let tyre_def = match (&s.wheel.kind, &s.wheel.tyre) {
                (WheelKind::Tyre, Some(t)) => t,
                _ => return Err(ChassisRefusal::Unsupported(format!("station {}: not a tyred wheel", s.name))),
            };
            if s.arm_pivot_m.is_some() {
                return Err(ChassisRefusal::Unsupported(format!("station {}: swinging-arm geometry", s.name)));
            }
            stations.push(Station {
                name: s.name.clone(),
                susp: Suspension::new(&s.suspension, s.bump_travel_m, tuning.suspension()),
                tyre: Tyre::new(tyre_def, s.wheel.radius_m, tuning.tyre()),
                rest_body_m: s.rest_pos_m - com,
                bump_dir: s.bump_dir.normalized_or_zero(),
                droop_travel_m: s.droop_travel_m,
                unsprung_mass_kg: s.unsprung_mass_kg,
                inertia_kg_m2: s.wheel.inertia_kg_m2,
                radius_m: s.wheel.radius_m,
                drive_output: s.drive_output,
                steer: s
                    .steer
                    .clone()
                    .map(|d| (d, SteerGeometry { ahead_of_ref_m: z_ref - s.rest_pos_m.z, lateral_m: s.rest_pos_m.x })),
                travel_m: 0.0,
                travel_rate_m_s: 0.0,
                omega_rad_s: 0.0,
                spin_angle_rad: 0.0,
                steer_rad: 0.0,
                report: StationReport::default(),
                anti_roll_n: 0.0,
            });
        }
        // substeps: what the rig declares, raised to what its stiffest mode needs (never fewer than the rule)
        let report = crate::modes::modes(rig, tuning);
        if report.required_substeps > w5k_contract::rig::MAX_SUBSTEPS {
            return Err(ChassisRefusal::TooStiff {
                f_max_hz: report.f_max_hz,
                required_substeps: report.required_substeps,
            });
        }
        let substeps = rig.integration.substeps.max(report.required_substeps).max(1);
        let rot = Quat::from_yaw(yaw_rad);
        let datum = Vec3::new(x_m, world.height_m(x_m, z_m) + rig.ride_height_m, z_m);
        let hull = Hull::new(rig.hull.mass_kg, rig.hull.inertia_kg_m2, datum + rot.rotate(com), rot)
            .map_err(ChassisRefusal::Hull)?;
        let outputs = rig.drivetrain.outputs.len();
        Ok(WheeledChassis {
            hull,
            stations,
            com_m: com,
            aero: rig.aero.clone(),
            anti_roll: rig.anti_roll.iter().map(|a| (a.left_station, a.right_station, a.rate_n_m)).collect(),
            substeps,
            shafts: vec![ShaftState::default(); outputs],
            torque_out: vec![0.0; outputs],
            time_s: 0.0,
            ledger: ForceLedger::off(),
            travel_acc_m_s2: vec![0.0; rig.stations.len()],
            gravity_m_s2: Vec3::new(0.0, -scalar::G, 0.0),
        })
    }

    pub fn substeps(&self) -> u32 {
        self.substeps
    }

    /// Hull datum position (the frame origin of the rig), world frame.
    pub fn datum_m(&self) -> Vec3 {
        self.hull.pos_m - self.hull.rot.rotate(self.com_m)
    }

    /// Forward speed of the hull (along its -Z axis), m/s.
    pub fn forward_speed_m_s(&self) -> f64 {
        self.hull.vel_m_s.dot(self.hull.rot.rotate(Vec3::FORWARD))
    }

    /// Start rolling: hull moving forward at `speed_m_s` with every wheel spinning to match (no slip).
    pub fn set_forward_speed(&mut self, speed_m_s: f64) {
        self.hull.vel_m_s = self.hull.rot.rotate(Vec3::FORWARD) * speed_m_s;
        for st in self.stations.iter_mut() {
            st.omega_rad_s = speed_m_s / st.radius_m;
        }
    }

    /// Yaw rate (about world +Y, positive = nose left), rad/s.
    pub fn yaw_rate_rad_s(&self) -> f64 {
        self.hull.omega_rad_s().y
    }

    /// One 60 Hz tick: `substeps` substeps of `dt / substeps`.
    pub fn tick(&mut self, dt_s: f64, inputs: &DriveInputs, world: &dyn WorldQuery, drive: &mut dyn DrivePort) {
        let h = dt_s / f64::from(self.substeps);
        for _ in 0..self.substeps {
            self.substep(h, inputs, world, drive);
        }
    }

    /// One substep, in the reference order of `CONTRACTS.md`.
    pub fn substep(&mut self, dt_s: f64, inputs: &DriveInputs, world: &dyn WorldQuery, drive: &mut dyn DrivePort) {
        let g = self.gravity_m_s2;
        let a_prev = self.hull.acc_m_s2;
        self.ledger.clear();
        let speed = self.forward_speed_m_s();

        // 4 (first half): the powertrain sees the shafts as they are at the start of the substep.
        for s in self.shafts.iter_mut() {
            *s = ShaftState { vehicle_speed_m_s: speed, ..ShaftState::default() };
        }
        let mut members = vec![0usize; self.shafts.len()];
        for st in &self.stations {
            if let Some(k) = st.drive_output {
                members[k] += 1;
                let sh = &mut self.shafts[k];
                sh.omega_rad_s += st.omega_rad_s;
                sh.inertia_kg_m2 += st.inertia_kg_m2;
                sh.load_torque_nm += st.report.contact.shaft_reaction_nm;
            }
        }
        for (sh, n) in self.shafts.iter_mut().zip(&members) {
            if *n > 0 {
                sh.omega_rad_s /= *n as f64;
            }
        }
        drive.step(dt_s, inputs, &self.shafts, &mut self.torque_out);

        // anti-roll bars: force on each end from the difference of the two travels
        for st in self.stations.iter_mut() {
            st.anti_roll_n = 0.0;
        }
        for &(l, r, rate) in &self.anti_roll {
            let f = rate * (self.stations[l].travel_m - self.stations[r].travel_m);
            self.stations[l].anti_roll_n += f;
            self.stations[r].anti_roll_n -= f;
        }

        let rot = self.hull.rot;
        let mut travel_acc = vec![0.0; self.stations.len()];
        for (i, st) in self.stations.iter_mut().enumerate() {
            // 1. wheel centre and the ground under it
            let d = rot.rotate(st.bump_dir);
            let wc = self.hull.point_world(st.rest_body_m + st.bump_dir * st.travel_m);
            let v_wc = self.hull.point_velocity(wc) + d * st.travel_rate_m_s;
            let n = world.normal(wc.x, wc.z);
            let ground = Vec3::new(wc.x, world.height_m(wc.x, wc.z), wc.z);
            let dist = (wc - ground).dot(n);
            let pen = st.radius_m - dist;

            st.steer_rad = st.steer.as_ref().map_or(0.0, |(sd, sg)| wheel_angle_rad(sd, sg, inputs.steer));
            let heading = rot.rotate(Quat::from_axis_angle(st.bump_dir, st.steer_rad).rotate(Vec3::FORWARD));
            let x_c = heading.reject_from(n).normalized_or_zero();
            let y_c = n.cross(x_c);

            // 3. contact
            let mat_id = world.material_id_at(wc.x, wc.z);
            let surface = st.omega_rad_s * (st.radius_m - pen.max(0.0));
            let out = st.tyre.step(&ContactInput {
                penetration_m: pen,
                penetration_rate_m_s: -v_wc.dot(n),
                vel_long_m_s: v_wc.dot(x_c),
                vel_lat_m_s: v_wc.dot(y_c),
                surface_speed_m_s: surface,
                ground: world.materials().get(mat_id),
                dt_s,
            });
            let f_tyre = x_c * out.fx_n + y_c * out.fy_n + n * out.fz_n;

            // 2. suspension (force along the strut, pushing hull and wheel apart)
            let so = st.susp.step(st.travel_m, st.travel_rate_m_s, dt_s);
            let strut_n = so.force_n + st.anti_roll_n;

            // unsprung mass along the strut; the cross-strut part is carried by the hull
            let f_ext = f_tyre + g * st.unsprung_mass_kg;
            let along = f_ext.dot(d);
            travel_acc[i] =
                if st.unsprung_mass_kg > 0.0 { (along - strut_n) / st.unsprung_mass_kg - a_prev.dot(d) } else { 0.0 };
            let across = (f_ext - d * along) - (a_prev - d * a_prev.dot(d)) * st.unsprung_mass_kg;
            let on_hull = d * strut_n + across;
            self.hull.add_force_at(on_hull, wc);
            self.hull.add_torque(n * out.mz_nm);
            // the tyre's forces act at the contact patch, not the wheel centre: moving them up to the hub leaves the moment
            // (patch - hub) x F. Its part about the spin axis is the shaft reaction (the wheel takes it); the rest, the lateral
            // force's roll/pitch lever `dist * fy` about the wheel's forward axis, goes through the upright into the hull.
            let patch_moment = x_c * (dist * out.fy_n);
            self.hull.add_torque(patch_moment);

            // 4 (second half): wheel spin; the drive torque's reaction goes into the hull about the wheel's axle (positive spin axis = y_c)
            let t_shaft = st.drive_output.map_or(0.0, |k| self.torque_out[k] / members[k].max(1) as f64);
            if st.inertia_kg_m2 > 0.0 {
                st.omega_rad_s =
                    scalar::flush_tiny(st.omega_rad_s + (t_shaft - out.shaft_reaction_nm) / st.inertia_kg_m2 * dt_s);
            }
            st.spin_angle_rad += st.omega_rad_s * dt_s;
            self.hull.add_torque(y_c * -t_shaft);

            if self.ledger.is_on() {
                let parts = [
                    (ForceTerm::TyreLongitudinal, x_c * (out.fx_n + st.tyre.last_rolling_n())),
                    (ForceTerm::RollingResistance, x_c * -st.tyre.last_rolling_n()),
                    (ForceTerm::TyreLateral, y_c * out.fy_n),
                    (ForceTerm::TyreNormal, n * out.fz_n),
                    (ForceTerm::Gravity, g * st.unsprung_mass_kg),
                ];
                let struts = [
                    (ForceTerm::SuspensionSpring, so.spring_n),
                    (ForceTerm::SuspensionDamper, so.damper_n),
                    (ForceTerm::BumpStop, so.bump_stop_n),
                    (ForceTerm::AntiRoll, st.anti_roll_n),
                ];
                let book = BookCtx { d, wc, com: self.hull.pos_m, body: 1 + i as u16 };
                book.station(
                    &mut self.ledger,
                    &parts,
                    &struts,
                    a_prev * st.unsprung_mass_kg,
                    t_shaft * st.omega_rad_s >= 0.0,
                    y_c * -t_shaft,
                    n * out.mz_nm + patch_moment,
                );
            }
            st.report = StationReport {
                contact: out,
                suspension: so,
                contact_point_m: wc - n * dist,
                normal: n,
                material: mat_id,
                force_on_hull_n: on_hull,
                strut_dir: d,
            };
        }

        // 5. gravity and aero drag (at the centre of pressure)
        self.hull.add_force(g * self.hull.mass_kg());
        let v = self.hull.vel_m_s;
        let drag = v * (-0.5 * AIR_DENSITY_KG_M3 * self.aero.drag_coeff * self.aero.frontal_area_m2 * v.length());
        let cp = self.hull.point_world(self.aero.centre_of_pressure_m - self.com_m);
        self.hull.add_force_at(drag, cp);
        if self.ledger.is_on() {
            self.ledger.add(ForceTerm::Gravity, 0, g * self.hull.mass_kg(), Vec3::ZERO);
            self.ledger.add(ForceTerm::Aero, 0, drag, (cp - self.hull.pos_m).cross(drag));
        }

        // 6. integrate the hull and each station's travel (semi-implicit: rate first), with the travel limits
        self.hull.integrate(dt_s);
        self.travel_acc_m_s2.clone_from(&travel_acc);
        for (st, acc) in self.stations.iter_mut().zip(travel_acc) {
            st.travel_rate_m_s += acc * dt_s;
            st.travel_m += st.travel_rate_m_s * dt_s;
            if st.travel_m < -st.droop_travel_m {
                st.travel_m = -st.droop_travel_m;
                st.travel_rate_m_s = st.travel_rate_m_s.max(0.0);
            }
            let (c, r) = st.susp.apply_hard_limit(st.travel_m, st.travel_rate_m_s);
            st.travel_m = c;
            st.travel_rate_m_s = scalar::flush_tiny(r);
        }
        self.time_s += dt_s;
    }

    pub fn is_finite(&self) -> bool {
        self.hull.is_finite() && self.stations.iter().all(|s| s.travel_m.is_finite() && s.omega_rad_s.is_finite())
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        self.hull.hash_state(h);
        for s in &self.stations {
            for v in [s.travel_m, s.travel_rate_m_s, s.omega_rad_s, s.spin_angle_rad] {
                h.write_f64(v);
            }
            s.tyre.hash_state(h);
        }
    }
}

/// Books one station's forces in the ledger. The hull (body 0) receives every force's **cross-strut** part at the wheel centre and the
/// strut's force along the strut; the station body (1 + i) keeps every force's **along-strut** part, minus the strut, minus the inertia of
/// riding on an accelerating hull (`ForceTerm::Other`, a frame force), so that its net force is `m_u * travel_acc * d`. Summed over bodies
/// each term then appears exactly once.
struct BookCtx {
    d: Vec3,
    wc: Vec3,
    com: Vec3,
    body: u16,
}

impl BookCtx {
    #[allow(clippy::too_many_arguments)]
    fn station(
        &self,
        l: &mut ForceLedger,
        parts: &[(ForceTerm, Vec3)],
        struts: &[(ForceTerm, f64)],
        m_a_prev: Vec3,
        driving: bool,
        drive_reaction_nm: Vec3,
        aligning_nm: Vec3,
    ) {
        let r = self.wc - self.com;
        let hull = |l: &mut ForceLedger, t: ForceTerm, f: Vec3| l.add(t, 0, f, r.cross(f));
        for &(t, f) in parts {
            let along = self.d * f.dot(self.d);
            hull(l, t, f - along);
            l.add(t, self.body, along, Vec3::ZERO);
        }
        for &(t, f) in struts {
            hull(l, t, self.d * f);
            l.add(t, self.body, self.d * -f, Vec3::ZERO);
        }
        // the hull carries the wheel across the strut: it pays for the wheel's cross-strut inertia
        hull(l, ForceTerm::Other, -(m_a_prev - self.d * m_a_prev.dot(self.d)));
        l.add(ForceTerm::Other, self.body, -self.d * m_a_prev.dot(self.d), Vec3::ZERO);
        l.add(if driving { ForceTerm::EngineDrive } else { ForceTerm::Brake }, 0, Vec3::ZERO, drive_reaction_nm);
        l.add(ForceTerm::TyreLateral, 0, Vec3::ZERO, aligning_nm);
    }
}
