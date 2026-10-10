//! Spike S5: design-to-physics coherence (`docs/lanes/forge/spike-s5.md`). THROWAWAY: the real compile (M1) replaces this module.
//!
//! Compiles the hull, tyre, spring, damper and engine of a two-axle wheeled `VehicleDef` and grafts them onto the stand-in truck's
//! drivetrain, so that `PhysRig::validate()` judges the result. The `*Extras` structs are the `VehicleDef` fields the spike needed and
//! did not find: they are the evidence for the CCR in `docs/swarm/requests/forge-ccr-vehicledef.md`.

use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::param::Param;
use w5k_contract::render::RenderRig;
use w5k_contract::rig::*;
use w5k_contract::testing::rigs::{box_truck, cylinder_mesh};
use w5k_math::{scalar, Mat3, Transform, Vec3};

/// Tyre fields the rig needs that `TyreSliders` cannot state (CCR item W1).
pub struct TyreExtras {
    pub vertical_stiffness_n_m: Param,
    pub vertical_damping_ns_m: Param,
    pub slip_stiffness: Param,
    pub relaxation_length_m: Param,
    /// Wheel, tyre and brake disc spin inertia about the axle, kg m^2.
    pub wheel_inertia_kg_m2: Param,
}

/// Suspension fields the rig needs that `SuspensionSliders` cannot state (CCR item W2).
pub struct SuspensionExtras {
    /// Rebound damping over bump damping (about 2 to 3 on a road vehicle).
    pub rebound_to_bump: Param,
    /// Where the bump stop engages, as a fraction of `bump_travel_m`.
    pub bump_stop_engage_frac: Param,
    pub bump_stop_rate_n_m: Param,
}

/// Engine fields the rig needs that `EngineSliders` cannot state (CCR item W3).
pub struct EngineExtras {
    /// Template shape: full-load torque at idle as a fraction of the torque peak (diesel about 0.5 to 0.7).
    pub idle_torque_frac: Param,
    /// Closed-throttle engine drag, constant part, N m.
    pub drag_const_nm: Param,
    pub drag_per_rpm_nm: Param,
}

pub struct Extras {
    pub tyre: TyreExtras,
    pub susp: SuspensionExtras,
    pub engine: EngineExtras,
    /// Peak friction of the table's dry hard reference surface (the denominator of the `mu_scale` rule).
    pub ref_surface_mu_peak: f64,
}

fn rpm_to_rad_s(rpm: f64) -> f64 {
    rpm * scalar::TAU / 60.0 // const-ok: unit conversion, 60 s per minute
}

/// Sprung weight share of the front axle for a COM `com_z` and axles at `z_front`, `z_rear` (statics, moment about the rear axle).
fn front_share(z_front: f64, z_rear: f64, com_z: f64) -> f64 {
    (z_rear - com_z) / (z_rear - z_front)
}

/// Full-load torque curve through the def's two peaks, exact at both: above the torque peak `T(n) = T_pk + a d^2 + b d^3`, `d = n - n_T`. The vertex sits at the
/// torque peak by construction; `a` and `b` solve `T(n_P) = P/omega_P` and `dP/dn = 0` at `n_P`. Rejects a design whose peaks do not
/// admit that shape (torque negative inside the working range, or a higher peak elsewhere).
pub fn torque_curve_through_peaks(
    t_pk: f64,
    n_t: f64,
    p_pk_w: f64,
    n_p: f64,
    idle: f64,
    redline: f64,
    idle_torque_frac: f64,
) -> Result<Vec<(f64, f64)>, String> {
    let t_p = p_pk_w / rpm_to_rad_s(n_p);
    let d_p = n_p - n_t;
    if d_p < 1.0 || t_p >= t_pk {
        return Err(format!("peak power ({t_p:.0} N m at {n_p} rpm) must lie past the torque peak ({t_pk} N m at {n_t} rpm) and be strictly below it"));
    }
    let x = t_p - t_pk;
    let b = (-t_p / n_p - 2.0 * x / d_p) / (d_p * d_p);
    let a = (x - b * d_p * d_p * d_p) / (d_p * d_p);
    // Above the peak the cubic; below it a parabola with the same vertex that reaches `idle_torque_frac` of the peak at idle (a cubic
    // blows up on the low side, which the first spike run showed).
    let q_low = (1.0 - idle_torque_frac) * t_pk / ((n_t - idle) * (n_t - idle));
    let t = |n: f64| {
        let d = n - n_t;
        if d < 0.0 {
            t_pk - q_low * d * d
        } else {
            t_pk + a * d * d + b * d * d * d
        }
    };
    let steps = 40; // const-ok: sampling resolution of the stored curve
    let mut pts: Vec<(f64, f64)> =
        (0..=steps).map(|k| idle + (redline - idle) * k as f64 / steps as f64).map(|n| (n, t(n))).collect();
    pts.push((n_t, t_pk));
    pts.push((n_p, t_p));
    pts.sort_by(|p, q| p.0.total_cmp(&q.0));
    pts.dedup_by(|p, q| (p.0 - q.0).abs() < 1e-6); // const-ok: rpm values closer than this are the same sample
    let tol = 1e-9; // const-ok: float noise allowance on exact-by-construction peaks
    if let Some(bad) = pts.iter().find(|p| p.1 <= 0.0) {
        return Err(format!("the peaks give a torque of {:.0} N m at {:.0} rpm: not a drivable engine", bad.1, bad.0));
    }
    if let Some(bad) = pts.iter().find(|p| p.1 > t_pk * (1.0 + tol) || p.1 * rpm_to_rad_s(p.0) > p_pk_w * (1.0 + tol)) {
        return Err(format!("the peaks give a higher torque or power at {:.0} rpm than the stated peaks", bad.0));
    }
    Ok(pts)
}

pub struct Compiled {
    pub rig: PhysRig,
    pub render: RenderRig,
    pub report: Vec<String>,
}

pub fn compile(def: &VehicleDef, ex: &Extras) -> Result<Compiled, String> {
    def.check().map_err(|e| e.join("; "))?;
    let RunningGearDef::Wheeled(w) = &def.running_gear else { return Err("S5 compiles wheeled defs only".into()) };
    if w.axles.len() != 2 {
        return Err("S5 compiles two axles".into());
    }
    let (h, ty, su) = (&def.hull, &w.tyre, &def.suspension);
    let g = scalar::G;
    let mut report = Vec::new();

    // Hull frame: datum at the hull box centre, +Z back (the front of the hull is at z = -L/2).
    let z_of = |from_front: f64| from_front - 0.5 * h.length_m.v;
    let ride_height = h.ground_clearance_m.v + 0.5 * h.height_m.v;
    let com = Vec3::new(0.0, h.com_height_m.v - ride_height, z_of(h.com_from_front_m.v));
    let (zf, zr) = (z_of(w.axles[0].from_front_m.v), z_of(w.axles[1].from_front_m.v));
    let share_f = front_share(zf, zr, com.z);
    if !(0.0..=1.0).contains(&share_f) {
        return Err(format!(
            "the COM at z = {:.2} m lies outside the axles ({zf:.2} to {zr:.2} m): statics cannot carry it",
            com.z
        ));
    }

    let r = 0.5 * ty.outer_diameter_m.v;
    let k_t = ex.tyre.vertical_stiffness_n_m.v;
    let mut stations = Vec::new();
    let mut proto = box_truck().0;
    for (idx, s) in proto.stations.iter_mut().enumerate() {
        let front = s.axle == 0;
        let share = if front { share_f } else { 1.0 - share_f };
        let m_corner = 0.5 * share * h.mass_kg.v; // sprung mass on one wheel
        let f_hz = if front { su.front_ride_frequency_hz.v } else { su.rear_ride_frequency_hz.v };
        // The slider is the RIDE frequency: sprung mass on the suspension in series with the tyre.
        let k_ride = m_corner * (scalar::TAU * f_hz) * (scalar::TAU * f_hz);
        if k_ride >= k_t {
            return Err(format!(
                "station {idx}: the ride rate {k_ride:.0} N/m is not below the tyre stiffness {k_t:.0} N/m"
            ));
        }
        let k_spring = k_ride * k_t / (k_t - k_ride);
        let c_mean = 2.0 * su.damping_ratio.v * scalar::sqrt(k_ride * m_corner);
        let rr = ex.susp.rebound_to_bump.v;
        let (c_bump, c_reb) = (2.0 * c_mean / (1.0 + rr), 2.0 * c_mean * rr / (1.0 + rr));
        // Tyre load = sprung share + unsprung weight; the free wheel sits that much into the ground.
        let load = (m_corner + ty.unsprung_mass_kg.v) * g;
        let deflection = load / k_t;
        let wheel_centre_above_ground = r - deflection;
        let x = 0.5 * w.axles[usize::from(!front)].track_width_m.v;
        let sx = if s.side == Side::Left { -x } else { x };
        s.rest_pos_m = Vec3::new(sx, wheel_centre_above_ground - ride_height, if front { zf } else { zr });
        s.unsprung_mass_kg = ty.unsprung_mass_kg.v;
        s.bump_travel_m = su.bump_travel_m.v;
        s.droop_travel_m = su.droop_travel_m.v;
        s.suspension.spring = SpringKind::Linear { rate_n_m: k_spring };
        s.suspension.preload_n = m_corner * g;
        s.suspension.damper.bump_ns_m = c_bump;
        s.suspension.damper.rebound_ns_m = c_reb;
        s.suspension.bump_stop.engage_m = ex.susp.bump_stop_engage_frac.v * su.bump_travel_m.v;
        s.suspension.bump_stop.rate_n_m = ex.susp.bump_stop_rate_n_m.v;
        s.wheel.radius_m = r;
        s.wheel.width_m = ty.section_width_m.v;
        s.wheel.inertia_kg_m2 = ex.tyre.wheel_inertia_kg_m2.v;
        let patch = load / (ty.inflation_pa.v * ty.section_width_m.v); // contact area = load / pressure
        s.wheel.tyre = Some(TyreDef {
            vertical_stiffness_n_m: k_t,
            vertical_damping_ns_m: ex.tyre.vertical_damping_ns_m.v,
            mu_scale: ty.mu_peak_ref.v / ex.ref_surface_mu_peak,
            slip_stiffness: ex.tyre.slip_stiffness.v,
            cornering_stiffness_per_rad: ty.cornering_stiffness_per_rad.v,
            relaxation_length_m: ex.tyre.relaxation_length_m.v,
            rolling_coeff: ty.rolling_coeff.v,
            inflation_pa: ty.inflation_pa.v,
            patch_length_m: patch,
            speed_floor_m_s: 0.0,
            aligning_trail_frac: 0.0,
            kappa_peak: 0.0,
            alpha_peak_rad: 0.0,
        });
        report.push(format!(
            "{}: sprung {m_corner:.1} kg, f {f_hz} Hz -> ride rate {k_ride:.0}, spring {k_spring:.0} N/m, zeta {} -> {c_bump:.0}/{c_reb:.0} N s/m, tyre deflection {:.1} mm, patch {:.0} mm",
            s.name,
            su.damping_ratio.v,
            deflection * 1e3, // const-ok: m to mm for display
            patch * 1e3       // const-ok: m to mm for display
        ));
        stations.push(s.clone());
    }

    // Hull mass properties: a uniform box, moved to the COM with the parallel-axis theorem.
    let size = Vec3::new(h.width_m.v, h.height_m.v, h.length_m.v);
    let k = h.mass_kg.v / 12.0; // const-ok: solid box inertia 1/12
    let box_i = Mat3::diagonal(
        k * (size.y * size.y + size.z * size.z),
        k * (size.x * size.x + size.z * size.z),
        k * (size.x * size.x + size.y * size.y),
    );
    let d = com; // the box centre is the datum, so the offset of the COM from it is `com`
    let shift = Mat3::diagonal(1.0, 1.0, 1.0).scaled(d.dot(d)).add(&Mat3::outer(d, d).scaled(-1.0)).scaled(h.mass_kg.v);
    proto.hull = BodyDef { name: "hull".into(), mass_kg: h.mass_kg.v, com_m: com, inertia_kg_m2: box_i.add(&shift) };
    proto.stations = stations;
    proto.ride_height_m = ride_height;
    proto.id = def.id.clone();
    proto.proxies.truncate(1);
    proto.proxies[0].shape = ProxyShape::Box { half_m: 0.5 * size };

    let e = &def.powertrain.engine;
    proto.drivetrain.engine.torque_curve = torque_curve_through_peaks(
        e.peak_torque_nm.v,
        e.peak_torque_rpm.v,
        e.peak_power_w.v,
        e.peak_power_rpm.v,
        e.idle_rpm.v,
        e.redline_rpm.v,
        ex.engine.idle_torque_frac.v,
    )?;
    proto.drivetrain.engine.idle_rpm = e.idle_rpm.v;
    proto.drivetrain.engine.redline_rpm = e.redline_rpm.v;
    proto.drivetrain.engine.inertia_kg_m2 = e.inertia_kg_m2.v;
    proto.drivetrain.engine.drag_const_nm = ex.engine.drag_const_nm.v;
    proto.drivetrain.engine.drag_per_rpm_nm = ex.engine.drag_per_rpm_nm.v;

    // Render rig: same source for the radius as the physics (the wheel meshes are rebuilt from the compiled station).
    let mut render = box_truck().1;
    for n in render.nodes.iter_mut() {
        if let Some(i) = n.name.strip_suffix(".travel").and_then(|nm| proto.stations.iter().position(|s| s.name == nm))
        {
            n.rest = Transform::from_pos(proto.stations[i].rest_pos_m);
        }
    }
    for m in render.meshes.iter_mut() {
        if let Some(s) = m.name.strip_suffix(".tyre").and_then(|nm| proto.stations.iter().find(|s| s.name == nm)) {
            let (node, slot) = (m.node, m.material_slot);
            *m = cylinder_mesh(&m.name, node, slot, Vec3::ZERO, s.wheel.radius_m, 0.5 * s.wheel.width_m, 0, 20);
            // const-ok: mesh segments
        }
    }
    Ok(Compiled { rig: proto, render, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::rigs::dummy_vehicle_def;

    fn est(v: f64) -> Param {
        Param::estimate(v, 0.5 * v, 2.0 * v, "spike stand-in")
    }

    fn extras() -> Extras {
        Extras {
            tyre: TyreExtras {
                vertical_stiffness_n_m: est(250_000.0),
                vertical_damping_ns_m: est(1_500.0),
                slip_stiffness: est(12.0),
                relaxation_length_m: est(0.3),
                wheel_inertia_kg_m2: est(1.4),
            },
            susp: SuspensionExtras {
                rebound_to_bump: est(1.45),
                bump_stop_engage_frac: Param::estimate(0.75, 0.5, 0.9, "spike"),
                bump_stop_rate_n_m: est(120_000.0),
            },
            engine: EngineExtras {
                idle_torque_frac: Param::estimate(0.6, 0.4, 0.8, "spike"),
                drag_const_nm: est(12.0),
                drag_per_rpm_nm: est(0.006),
            },
            ref_surface_mu_peak: 0.9, // const-ok: spike stand-in for the table's dry hard surface
        }
    }

    fn built() -> (VehicleDef, Compiled) {
        let mut def = dummy_vehicle_def();
        // The stand-in's 110 kW at 3500 rpm is 300 N m: as much as its torque peak, so power would still be rising there.
        def.powertrain.engine.peak_power_w = est(100_000.0);
        let c = compile(&def, &extras()).expect("compiles");
        (def, c)
    }

    #[test]
    fn the_compiled_rig_validates_and_masses_sum_to_the_def() {
        let (def, c) = built();
        c.rig.validate().unwrap_or_else(|e| panic!("{e:?}"));
        c.render.validate().unwrap_or_else(|e| panic!("{e:?}"));
        let RunningGearDef::Wheeled(w) = &def.running_gear else { unreachable!() };
        let want = def.hull.mass_kg.v + 4.0 * w.tyre.unsprung_mass_kg.v;
        assert!((c.rig.total_mass_kg() - want).abs() < 1e-9);
    }

    #[test]
    fn wheel_radius_in_rig_and_mesh_agree_to_a_millimetre() {
        let (def, c) = built();
        let RunningGearDef::Wheeled(w) = &def.running_gear else { unreachable!() };
        for s in &c.rig.stations {
            assert!((s.wheel.radius_m - 0.5 * w.tyre.outer_diameter_m.v).abs() < 1e-12);
            let m = c.render.meshes.iter().find(|m| m.name == format!("{}.tyre", s.name)).unwrap();
            let r = m.positions.iter().map(|p| scalar::hypot(f64::from(p[1]), f64::from(p[2]))).fold(0.0, f64::max);
            assert!((r - s.wheel.radius_m).abs() < 1e-3, "{}: mesh {r} vs rig {}", s.name, s.wheel.radius_m);
        }
    }

    #[test]
    fn static_equilibrium_sits_at_the_design_ride_height() {
        let (def, c) = built();
        let rig = &c.rig;
        // Independent of the compile: the tyre deflection the rig implies must carry the load the springs leave to the ground.
        for s in &rig.stations {
            let t = s.wheel.tyre.as_ref().unwrap();
            let deflection = s.wheel.radius_m - rig.ride_height_m - s.rest_pos_m.y;
            let ground_load = t.vertical_stiffness_n_m * deflection;
            let want = s.suspension.preload_n + s.unsprung_mass_kg * scalar::G;
            assert!((ground_load - want).abs() < 0.01 * want, "{}: ground {ground_load} vs {want}", s.name);
        }
        let clearance = rig.ride_height_m - 0.5 * def.hull.height_m.v;
        assert!((clearance - def.hull.ground_clearance_m.v).abs() < 1e-3);
        assert!((rig.hull.com_m.y + rig.ride_height_m - def.hull.com_height_m.v).abs() < 1e-3);
        let z_com = rig.hull.com_m.z + 0.5 * def.hull.length_m.v;
        assert!((z_com - def.hull.com_from_front_m.v).abs() < 1e-3);
    }

    #[test]
    fn ride_frequency_slider_gives_the_stated_natural_frequency() {
        let (def, c) = built();
        for s in &c.rig.stations {
            let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!() };
            let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
            let k_ride = rate_n_m * k_t / (rate_n_m + k_t);
            let m = s.suspension.preload_n / scalar::G;
            let f = scalar::sqrt(k_ride / m) / scalar::TAU;
            let want = if s.axle == 0 {
                def.suspension.front_ride_frequency_hz.v
            } else {
                def.suspension.rear_ride_frequency_hz.v
            };
            assert!((f - want).abs() < 0.005 * want, "{}: {f} vs {want}", s.name);
        }
    }

    #[test]
    fn damping_ratio_slider_gives_the_stated_zeta() {
        let (def, c) = built();
        for s in &c.rig.stations {
            let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!() };
            let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
            let k_ride = rate_n_m * k_t / (rate_n_m + k_t);
            let m = s.suspension.preload_n / scalar::G;
            let c_mean = 0.5 * (s.suspension.damper.bump_ns_m + s.suspension.damper.rebound_ns_m);
            let zeta = c_mean / (2.0 * scalar::sqrt(k_ride * m));
            assert!((zeta - def.suspension.damping_ratio.v).abs() < 0.005 * zeta);
        }
    }

    #[test]
    fn torque_curve_peaks_match_the_def_peaks() {
        let (def, c) = built();
        let e = &def.powertrain.engine;
        let curve = &c.rig.drivetrain.engine.torque_curve;
        let (n_t, t_max) = curve.iter().fold((0.0, 0.0), |a, &(n, t)| if t > a.1 { (n, t) } else { a });
        let (n_p, p_max) =
            curve.iter().map(|&(n, t)| (n, t * rpm_to_rad_s(n))).fold((0.0, 0.0), |a, x| if x.1 > a.1 { x } else { a });
        assert!((t_max - e.peak_torque_nm.v).abs() < 0.005 * t_max && (n_t - e.peak_torque_rpm.v).abs() < 1.0);
        assert!((p_max - e.peak_power_w.v).abs() < 0.005 * p_max && (n_p - e.peak_power_rpm.v).abs() < 1.0);
    }

    #[test]
    fn the_contract_stand_in_def_compiles_now_that_its_engine_peaks_are_consistent() {
        // Contract 0.2 fixed the stand-in (95 kW at 3500 rpm with a 300 N m peak at 2500 rpm); 0.1 had 110 kW and was rejected here.
        compile(&dummy_vehicle_def(), &extras()).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn peaks_that_admit_no_drivable_curve_are_rejected_with_a_reason() {
        // Power peak at the torque-peak speed with more torque than the torque peak: impossible.
        assert!(torque_curve_through_peaks(300.0, 2_500.0, 300_000.0, 3_500.0, 800.0, 4_500.0, 0.6).is_err());
    }
}
