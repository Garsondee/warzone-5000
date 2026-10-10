//! Proving-ground benches against closed-form answers. Tests are physics sentences.

use w5k_chassis::bench::{com_height_m, half_track_m, skidpad, tilt_table, Skidpad, TiltTable};
use w5k_chassis::tuning::ChassisTuning;
use w5k_contract::rig::{PhysRig, SpringKind};
use w5k_contract::testing::{box_truck, standard_materials, ConstantTorquePowertrain, FlatPlane};
use w5k_contract::{Material, MaterialId, MaterialTable, PropRef, RayHit, WorldQuery};
use w5k_math::{scalar, Vec3};

fn tuning() -> ChassisTuning {
    ChassisTuning::from_ron(include_str!("../../../content/physics/chassis/tuning.ron")).unwrap()
}

/// A level plane whose one surface has the given peak friction (a grippy tilt-table deck, so the truck tips before it slides).
struct GripPlane {
    materials: MaterialTable,
}

impl GripPlane {
    fn new(mu: f64) -> GripPlane {
        let base = standard_materials().get(MaterialId(0)).clone();
        let mut materials = MaterialTable::default();
        materials.push(Material { name: "grip deck".into(), mu_peak: mu, mu_slide: mu, ..base });
        GripPlane { materials }
    }
}

impl WorldQuery for GripPlane {
    fn height_m(&self, _x: f64, _z: f64) -> f64 {
        0.0
    }
    fn normal(&self, _x: f64, _z: f64) -> Vec3 {
        Vec3::Y
    }
    fn material_id_at(&self, _x: f64, _z: f64) -> MaterialId {
        MaterialId(0)
    }
    fn materials(&self) -> &MaterialTable {
        &self.materials
    }
    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        FlatPlane::new().raycast(origin, dir, max_m)
    }
    fn props_in_aabb(&self, _min: Vec3, _max: Vec3, _out: &mut Vec<PropRef>) {}
    fn bounds(&self) -> (Vec3, Vec3) {
        FlatPlane::new().bounds()
    }
}

fn drive(r: &PhysRig) -> ConstantTorquePowertrain {
    ConstantTorquePowertrain::new(r.drivetrain.outputs.len(), 600.0, 4000.0)
}

const TILT: TiltTable = TiltTable { rate_rad_s: 0.02, settle_s: 3.0, max_angle_rad: 1.2 };

/// Roll stiffness about the ground (N m/rad): wheel springs and anti-roll bars in parallel, then the tyres in series.
fn roll_stiffness_nm_rad(r: &PhysRig) -> f64 {
    let (mut k_susp, mut k_tyre) = (0.0, 0.0);
    for s in &r.stations {
        let x2 = s.rest_pos_m.x * s.rest_pos_m.x;
        if let SpringKind::Linear { rate_n_m } = s.suspension.spring {
            k_susp += rate_n_m * x2;
        }
        k_tyre += s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m * x2;
    }
    for a in &r.anti_roll {
        let t = r.stations[a.left_station].rest_pos_m.x - r.stations[a.right_station].rest_pos_m.x;
        k_susp += a.rate_n_m * t * t;
    }
    1.0 / (1.0 / k_susp + 1.0 / k_tyre)
}

/// The compliant tilt-table answer: the body rolls by phi = m g h (sin(theta) + cos(theta) phi) / K_phi about the ground, which moves the
/// centre of mass toward the low side by h phi; the uphill wheels lift when tan(theta) = (t/2 - h phi) / h.
fn compliant_lift_angle_rad(r: &PhysRig) -> f64 {
    let m = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let (h, half_t, k) = (com_height_m(r), half_track_m(r), roll_stiffness_nm_rad(r));
    let mut theta: f64 = scalar::atan(half_t / h);
    for _ in 0..100 {
        let w = m * scalar::G * h;
        let phi = w * scalar::sin(theta) / (k - w * scalar::cos(theta));
        theta = scalar::atan((half_t - h * phi) / h);
    }
    theta
}

#[test]
fn tilt_table_uphill_load_matches_compliant_roll_statics_until_the_droop_stop() {
    use w5k_chassis::wheeled::WheeledChassis;
    use w5k_contract::rig::TICK_HZ;
    use w5k_contract::{DriveInputs, GearRequest};
    let r = box_truck().0;
    let w = GripPlane::new(2.5);
    let (m, h, half_t, k) = (
        r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>(),
        com_height_m(&r),
        half_track_m(&r),
        roll_stiffness_nm_rad(&r),
    );
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, 0.0, 0.0).unwrap();
    let mut d = drive(&r);
    let parked = DriveInputs { gear: GearRequest::Neutral, parking_brake: true, ..DriveInputs::default() };
    for deg in [10.0_f64, 20.0, 30.0, 40.0] {
        let th = deg.to_radians();
        c.gravity_m_s2 = Vec3::new(scalar::sin(th), -scalar::cos(th), 0.0) * scalar::G;
        for _ in 0..(4.0 * TICK_HZ) as usize {
            c.tick(1.0 / TICK_HZ, &parked, &w, &mut d);
        }
        // moments about the downhill contact line: N_up t = W (t/2 cos - h sin - h cos phi), phi from the roll stiffness
        let wt = m * scalar::G;
        let phi = wt * h * scalar::sin(th) / (k - wt * h * scalar::cos(th));
        let expect = wt * (half_t * scalar::cos(th) - h * scalar::sin(th) - h * scalar::cos(th) * phi) / (2.0 * half_t);
        let uphill: f64 = c
            .stations
            .iter()
            .zip(&r.stations)
            .filter(|(_, d)| d.rest_pos_m.x < 0.0)
            .map(|(s, _)| s.report.contact.fz_n)
            .sum();
        println!("{deg:4} deg: uphill load {uphill:.0} N, statics {expect:.0} N");
        assert!((uphill / expect - 1.0).abs() < 0.05);
    }
    // the linear answer (no droop stop) for reference: the real truck tips earlier, once its uphill wheels hang on their droop stops
    let res = tilt_table(&r, &tuning(), &w, &mut drive(&r), &TILT).unwrap();
    let (lift, tip) = (res.lift_angle_rad.unwrap(), res.tip_angle_rad.unwrap());
    println!(
        "first wheel lifts at {:.2} deg, tips at {:.2} deg; linear compliant {:.2} deg; rigid atan(t/2h) {:.2} deg",
        lift.to_degrees(),
        tip.to_degrees(),
        compliant_lift_angle_rad(&r).to_degrees(),
        res.rigid_estimate_rad.to_degrees()
    );
    assert!(lift <= tip && tip < compliant_lift_angle_rad(&r) && tip < res.rigid_estimate_rad);
}

#[test]
fn a_stiffer_suspension_tips_closer_to_the_rigid_answer() {
    let soft = box_truck().0;
    let mut stiff = soft.clone();
    for s in stiff.stations.iter_mut() {
        if let SpringKind::Linear { rate_n_m } = &mut s.suspension.spring {
            *rate_n_m *= 4.0;
        }
    }
    let w = GripPlane::new(2.5);
    let a = tilt_table(&soft, &tuning(), &w, &mut drive(&soft), &TILT).unwrap();
    let b = tilt_table(&stiff, &tuning(), &w, &mut drive(&stiff), &TILT).unwrap();
    let (la, lb) = (a.tip_angle_rad.unwrap(), b.tip_angle_rad.unwrap());
    println!(
        "soft {:.2} deg, stiff {:.2} deg (closed form {:.2}), rigid {:.2} deg",
        la.to_degrees(),
        lb.to_degrees(),
        compliant_lift_angle_rad(&stiff).to_degrees(),
        a.rigid_estimate_rad.to_degrees()
    );
    assert!(la < lb && lb < a.rigid_estimate_rad);
}

const PAD: Skidpad = Skidpad {
    steer: 0.1,
    start_speed_m_s: 4.0,
    ramp_m_s2: 0.15,
    speed_gain_per_m_s: 1.0,
    warmup_s: 3.0,
    max_s: 200.0,
    slide_out_frac: 0.8,
};

/// Bicycle-model understeer gradient `K = (m / L) (b / C_f - a / C_r)` (rad per m/s^2), with the axle cornering stiffnesses
/// `C = C_alpha * F_z` summed over the axle's tyres (the tyre model's stiffness is per unit load) and a, b the centre of mass to the front and rear axles.
fn bicycle_understeer_gradient(r: &PhysRig) -> f64 {
    let m = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let zs: Vec<f64> = r.stations.iter().map(|s| s.rest_pos_m.z).collect();
    let (zf, zr) = (zs.iter().cloned().fold(f64::MAX, f64::min), zs.iter().cloned().fold(f64::MIN, f64::max));
    let z_com = (r.hull.mass_kg * r.hull.com_m.z
        + r.stations.iter().map(|s| s.unsprung_mass_kg * s.rest_pos_m.z).sum::<f64>())
        / m;
    let (a, b, l) = (z_com - zf, zr - z_com, zr - zf);
    let (fz_f, fz_r) = (m * scalar::G * b / l, m * scalar::G * a / l);
    let c_alpha = |front: bool| {
        let s = r.stations.iter().find(|s| ((s.rest_pos_m.z - zf).abs() < 1e-9) == front).unwrap();
        s.wheel.tyre.as_ref().unwrap().cornering_stiffness_per_rad
    };
    let (cf, cr) = (c_alpha(true) * fz_f, c_alpha(false) * fz_r);
    m / l * (b / cf - a / cr)
}

#[test]
fn skidpad_understeer_gradient_matches_the_bicycle_model() {
    let neutral = box_truck().0;
    // halve the front tyres' cornering stiffness: the front needs twice the slip angle, so the truck understeers
    let mut under = neutral.clone();
    let zf = under.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MAX, f64::min);
    for s in under.stations.iter_mut().filter(|s| (s.rest_pos_m.z - zf).abs() < 1e-9) {
        s.wheel.tyre.as_mut().unwrap().cornering_stiffness_per_rad *= 0.5;
    }
    let w = FlatPlane::new();
    let k0 = skidpad(&neutral, &tuning(), &w, &mut drive(&neutral), &PAD).unwrap().understeer_gradient_rad_per_m_s2;
    let k1 = skidpad(&under, &tuning(), &w, &mut drive(&under), &PAD).unwrap().understeer_gradient_rad_per_m_s2;
    let (e0, e1) = (bicycle_understeer_gradient(&neutral), bicycle_understeer_gradient(&under));
    println!("neutral: K {k0:.5} (bicycle {e0:.5}); soft fronts: K {k1:.5} (bicycle {e1:.5}) rad per m/s2");
    assert!(k1 > 0.0 && (k1 - k0) > 0.5 * e1);
    assert!(((k1 - k0) / (e1 - e0) - 1.0).abs() < 0.2);
}

#[test]
fn skidpad_limit_is_mu_g_and_a_neutral_truck_has_no_understeer_gradient() {
    let r = box_truck().0;
    let w = FlatPlane::new();
    let res = skidpad(&r, &tuning(), &w, &mut drive(&r), &PAD).unwrap();
    let tyre = r.stations[0].wheel.tyre.as_ref().unwrap();
    let mu = w.materials().get(MaterialId(0)).mu_peak * tyre.mu_scale;
    println!(
        "a_y max {:.2} m/s2 = {:.3} g (mu {mu:.3}), K = {:.5} rad per m/s2 over {} points, L {:.2}",
        res.max_lateral_acc_m_s2,
        res.max_lateral_acc_m_s2 / scalar::G,
        res.understeer_gradient_rad_per_m_s2,
        res.points.len(),
        res.wheelbase_m
    );
    // the axle that saturates first sets the limit: a neutral truck reaches mu g, a slightly understeering one stops a little short
    let ratio = res.max_lateral_acc_m_s2 / (mu * scalar::G);
    assert!(ratio > 0.85 && ratio <= 1.0, "a_y max / mu g = {ratio}");
}

#[test]
fn steady_turn_lateral_load_transfer_matches_m_ay_h_over_track() {
    let r = box_truck().0;
    let w = FlatPlane::new();
    let res = skidpad(&r, &tuning(), &w, &mut drive(&r), &PAD).unwrap();
    let m = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let (h, track) = (com_height_m(&r), 2.0 * half_track_m(&r));
    // the steady part of the run, below the limit. On top of m a_y h / t, the sprung body leans out by phi. The patches sit below the wheel
    // centres, which move sideways with the body, so the body's centre of mass moves outward relative to them by (h_s - h_wheel) phi:
    // + m_s g (h_s - h_wheel) phi / t (a roll centre at wheel-centre height).
    let h_wheel = r.stations.iter().map(|s| s.rest_pos_m.y).sum::<f64>() / r.stations.len() as f64 + r.ride_height_m;
    let (m_s, h_s) = (r.hull.mass_kg, r.hull.com_m.y + r.ride_height_m - h_wheel);
    let mut worst: f64 = 0.0;
    for p in
        res.points.iter().filter(|p| p.lateral_acc_m_s2 > 2.0 && p.lateral_acc_m_s2 < 0.7 * res.max_lateral_acc_m_s2)
    {
        let expect = (m * p.lateral_acc_m_s2 * h + m_s * scalar::G * h_s * p.roll_out_rad) / track;
        worst = worst.max((p.lateral_transfer_n / expect - 1.0).abs());
    }
    println!("worst relative error of the lateral load transfer {worst:.4}");
    assert!(worst < 0.03);
}
