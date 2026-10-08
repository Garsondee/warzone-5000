//! Walking legs: a two-link leg (thigh and shin) with a hip actuator, a knee and a foot pad.
//!
//! The physics comes from the part's own geometry (docs/notes/m1b-physics-of-movement.md):
//! * **Column strength.** Each segment is a hollow box tube. It fails by crushing (A sigma) or, if slender, by
//!   buckling (Euler: P = pi^2 E I / L^2); the weaker governs. Because I grows as the fourth power of the section
//!   but the load as the cube of the vehicle's size, **giants need disproportionately fat legs**.
//! * **Actuators.** The hip must hold the leg's rated load at the foot's reach: torque = F x reach, and a
//!   hydraulic actuator delivers about 2,000 N m per kg, so long reach and heavy loads cost actuator mass.
//! * **Gait.** Walking speed follows the Froude number Fr = v^2 / (g h): v_max = sqrt(Fr g h) with h the hip
//!   height (a long-legged walker is faster), and every step spends energy: cost of transport about 0.6, ten times a
//!   track's rolling resistance.
//! * **Feet.** Only half the feet are down at once, so ground pressure is the weight over half the pad area.
//!
//! Part space: the hip is the mount at the origin on the hull side; the leg reaches out (+x) and down so that the
//! foot touches the ground `stance_m` below the hip.

use super::style::{self, p};
use super::{stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Legs;

pub const MATERIALS: [&str; 4] = ["steel", "titanium", "aluminium", "composite"];
const SAFETY: f64 = 2.5;
/// A two-link leg bends as well as compresses; its axial capacity is knocked down by this factor.
const BENDING: f64 = 0.5;
/// Landing impacts in a gait load a leg harder than standing still.
const DYNAMIC: f64 = 2.0;
/// Torque a hydraulic actuator delivers per kilogram (N m / kg).
const TORQUE_DENSITY: f64 = 2000.0;
/// Highest Froude number a legged gait reaches (a jog).
const FROUDE_MAX: f64 = 1.0;
/// Energy per weight per distance travelled: legs are costly.
pub const COST_OF_TRANSPORT: f64 = 0.6;
const G: f64 = 9.81;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// The leg's layout in its own plane (x out, y up), solved by two-link inverse kinematics.
#[derive(Clone, Copy, Debug)]
pub struct Geo {
    pub x_out: f64,
    pub stance: f64,
    pub pad_h: f64,
    /// Height of the frustum that blends the shin into the pad.
    pub cone_h: f64,
    pub knee: (f64, f64),
    pub ankle: (f64, f64),
    pub l1: f64,
    pub l2: f64,
    /// Section (out-of-plane width b, in-plane depth h, wall t) of the thigh and shin.
    pub s1: (f64, f64, f64),
    pub s2: (f64, f64, f64),
}

fn section(b: f64) -> (f64, f64, f64) {
    let b = b.max(0.03);
    (b, 1.12 * b, (0.1 * b).clamp(0.003, 0.45 * b))
}

pub fn geometry(v: &Values) -> Geo {
    let stance = v["stance_m"];
    let x_out = (v["splay"] * stance).max(0.15);
    let foot = v["foot_m"];
    let pad_h = 0.06 + 0.07 * foot;
    let cone_h = (0.55 * foot).min(0.3 * stance);
    let ankle = (x_out, -stance + pad_h + cone_h);
    let dist = (ankle.0 * ankle.0 + ankle.1 * ankle.1).sqrt();
    let total = dist * (1.0 + v["bend"]);
    let (l1, l2) = (0.46 * total, 0.54 * total);
    // Knee: the intersection of a circle of radius l1 about the hip and l2 about the ankle (the upper one).
    let a = (l1 * l1 - l2 * l2 + dist * dist) / (2.0 * dist);
    let hh = (l1 * l1 - a * a).max(0.0).sqrt();
    let (ux, uy) = (ankle.0 / dist, ankle.1 / dist);
    let knee = (a * ux - hh * uy, a * uy + hh * ux);
    Geo { x_out, stance, pad_h, cone_h, knee, ankle, l1, l2, s1: section(v["thickness"] * l1), s2: section(0.9 * v["thickness"] * l2) }
}

/// Area and weak-axis second moment of a rectangular tube of out-of-plane width `b`, depth `h` and wall `t`.
fn tube(b: f64, h: f64, t: f64) -> (f64, f64) {
    let area = b * h - (b - 2.0 * t) * (h - 2.0 * t);
    let i_out = (h * b.powi(3) - (h - 2.0 * t) * (b - 2.0 * t).powi(3)) / 12.0;
    let i_in = (b * h.powi(3) - (b - 2.0 * t) * (h - 2.0 * t).powi(3)) / 12.0;
    (area, i_out.min(i_in))
}

fn material_of(v: &Values) -> &'static str {
    MATERIALS[v["material"].round().clamp(0.0, 3.0) as usize]
}

/// How a leg fails first, and the axial force it can carry (N), before the safety factor.
pub fn capacity(v: &Values, lib: &MaterialLibrary) -> (f64, &'static str) {
    let g = geometry(v);
    let m = lib.materials.get(material_of(v));
    let e = m.map(|m| m.modulus_gpa).filter(|x| *x > 0.0).unwrap_or(10.0) * 1e9;
    let sy = m.map(|m| m.strength_mpa).filter(|x| *x > 0.0).unwrap_or(100.0) * 1e6;
    let mut best = (f64::MAX, "crushing");
    for (l, (b, h, t)) in [(g.l1, g.s1), (g.l2, g.s2)] {
        let (area, i) = tube(b, h, t);
        let buckle = std::f64::consts::PI.powi(2) * e * i / (l * l);
        let crush = sy * area;
        let (f, how) = if buckle < crush { (buckle, "buckling") } else { (crush, "crushing") };
        if f < best.0 {
            best = (f, how);
        }
    }
    best
}

/// Rated axial force per leg (N): the weaker failure, knocked down for bending and divided by the safety factor.
pub fn rated_force_n(v: &Values, lib: &MaterialLibrary) -> f64 {
    BENDING * capacity(v, lib).0 / SAFETY
}

/// Rated load (kg) one leg contributes: half the legs are down at once, and landings load them DYNAMIC times.
pub fn rated_load_kg(v: &Values, lib: &MaterialLibrary) -> f64 {
    0.5 * rated_force_n(v, lib) / (DYNAMIC * G)
}

/// Mass of the hip actuator (kg): torque (rated force at the foot's reach) over torque density.
pub fn hip_actuator_kg(v: &Values, lib: &MaterialLibrary) -> f64 {
    let g = geometry(v);
    rated_force_n(v, lib) * g.x_out.max(0.25 * g.stance) / TORQUE_DENSITY
}

impl Family for Legs {
    fn id(&self) -> &'static str {
        "legs"
    }
    fn name(&self) -> &'static str {
        "Walker leg"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("stance_m", "Stance height", "m", 0.5, 16.0, 2.4, Scale::Log, Role::Budgeted, "Hip height above the ground. Taller legs are faster (Froude law) and step over more, and cost more in mass."),
            param("thickness", "Leg strength", "", 0.03, 0.18, 0.06, Scale::Linear, Role::Budgeted, "Section of the load-bearing tube as a fraction of segment length. Strength grows with the fourth power; mass with the square, plus the hip actuator it needs."),
            param("foot_m", "Foot radius", "m", 0.1, 4.0, 0.3, Scale::Log, Role::Budgeted, "Big feet spread the weight: a giant on small feet sinks."),
            param("splay", "Splay", "", 0.2, 1.4, 0.75, Scale::Linear, Role::Free, "How far out the foot plants, as a fraction of stance: a wide stance is stable but needs hip torque."),
            param("bend", "Knee bend", "", 0.05, 0.7, 0.5, Scale::Linear, Role::Free, "How high the knee rises: a spider's high knee or a straighter strut."),
            param("material", "Material", "", 0.0, 3.0, 0.0, Scale::Linear, Role::Free, "0 steel, 1 titanium, 2 aluminium, 3 composite: strength, stiffness and weight."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Station, SocketKind::Hip]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_strider", hull_params: &[("radius_m", 2.4), ("height_m", 1.4)], socket: "station_*", kw_per_t: 30.0 })
    }

    fn generate(&self, v: &Values, lib: &MaterialLibrary) -> PartDef {
        let g = geometry(v);
        let mat = material_of(v);
        let foot = v["foot_m"];
        let rho = lib.materials.get("gun_steel").map(|m| m.density).unwrap_or(7850.0);
        let act = hip_actuator_kg(v, lib);
        let (b1, h1, t1) = g.s1;
        let (b2, h2, t2) = g.s2;
        let knee = p(g.knee.0, g.knee.1, 0.0);
        let ankle = p(g.ankle.0, g.ankle.1, 0.0);
        let z = p(0.0, 0.0, 1.0);
        let mut shapes: Vec<Node> = Vec::new();
        // Hip joint: a drum across the leg's plane, sized from the actuator's mass.
        let len_j = 1.35 * b1;
        let r_j = (0.2 * act / rho / (std::f64::consts::PI * len_j)).sqrt().max(0.6 * h1);
        let hip = p(0.35 * r_j, 0.0, 0.0);
        shapes.push(style::cyl(r_j, len_j, Axis::Z, 12, 1.0, hip, "gun_steel", Slot::Dark, None, 0.1 * r_j));
        // The load-bearing tubes (hidden inside the fairings) and the thin composite fairings that give the leg its look.
        let fair = |from: V3, to: V3, base: (f64, f64, f64), l: f64, min_rel: f64, slot: Slot| {
            let fb = (1.25 * base.0).max(min_rel * l);
            Node::Beam { from: from.arr(), to: to.arr(), size: [1.15 * fb, fb], end: Some([0.78 * 1.15 * fb, 0.78 * fb]), up: z.arr(), mat: "composite".into(), slot, shell: Some(0.008), chamfer: 0.01 + 0.04 * fb }
        };
        shapes.push(Node::Beam { from: hip.arr(), to: knee.arr(), size: [h1, b1], end: Some([0.8 * h1, 0.8 * b1]), up: z.arr(), mat: mat.into(), slot: Slot::Dark, shell: Some(t1), chamfer: 0.3 * t1 });
        shapes.push(fair(hip, knee, g.s1, g.l1, 0.11, Slot::Primary));
        // Knee: a smaller drum with glowing caps on both faces.
        let fb1 = (1.25 * b1).max(0.11 * g.l1);
        let r_k = (0.1 * act / rho / (std::f64::consts::PI * 1.3 * fb1)).sqrt().max(0.6 * fb1);
        shapes.push(style::cyl(r_k, 1.25 * fb1, Axis::Z, 12, 1.0, knee, "gun_steel", Slot::Dark, None, 0.1 * r_k));
        for s in [-1.0, 1.0] {
            shapes.push(style::cyl(0.5 * r_k, 0.03 + 0.08 * r_k, Axis::Z, 8, 1.0, knee + p(0.0, 0.0, s * (0.625 * fb1 + 0.01)), "fittings", Slot::Glow, None, 0.0));
        }
        // Hydraulic ram inside the thigh, carrying the rest of the actuator's mass.
        let d1 = p(knee.x - hip.x, knee.y - hip.y, 0.0).norm();
        let n1 = p(-d1.y, d1.x, 0.0);
        let ram_v = 0.7 * act / rho;
        let side0 = (0.8 * fb1).max(0.04 * h1);
        let len_r = (ram_v / (side0 * side0)).clamp(0.2 * g.l1, 0.85 * g.l1);
        let side = (ram_v / len_r).sqrt().max(0.04 * h1);
        let off = n1 * (0.1 * fb1);
        shapes.push(style::beam(hip + d1 * (0.12 * g.l1) + off, hip + d1 * (0.12 * g.l1 + len_r) + off, [side, side], z, "gun_steel", Slot::Dark, 0.0));
        shapes.push(Node::Beam { from: knee.arr(), to: ankle.arr(), size: [h2, b2], end: Some([0.7 * h2, 0.7 * b2]), up: z.arr(), mat: mat.into(), slot: Slot::Dark, shell: Some(t2), chamfer: 0.3 * t2 });
        shapes.push(fair(knee, ankle, g.s2, g.l2, 0.09, Slot::Secondary));
        // Warning band near the knee: a thin shell, so it merges with the hollow shin.
        let d2 = p(ankle.x - knee.x, ankle.y - knee.y, 0.0).norm();
        shapes.push(Node::Beam {
            from: (knee + d2 * (0.16 * g.l2)).arr(),
            to: (knee + d2 * (0.24 * g.l2)).arr(),
            size: [1.04 * 1.15 * ((1.25 * b2).max(0.09 * g.l2)) * 0.97, 1.04 * ((1.25 * b2).max(0.09 * g.l2)) * 0.97],
            end: None,
            up: z.arr(),
            mat: "fittings".into(),
            slot: Slot::Trim,
            shell: Some(0.004),
            chamfer: 0.0,
        });
        // Foot: a frustum blending the shin into a wide pad, with a trim band.
        shapes.push(style::cyl(0.72 * foot, g.cone_h, Axis::Y, if foot > 0.5 { 14 } else { 10 }, 0.3, p(g.ankle.0, -g.stance + g.pad_h + g.cone_h / 2.0, 0.0), "rubber", Slot::Secondary, None, 0.05 * foot));
        shapes.push(style::cyl(foot, g.pad_h, Axis::Y, if foot > 0.5 { 14 } else { 10 }, 0.85, p(g.ankle.0, -g.stance + g.pad_h / 2.0, 0.0), "rubber", Slot::Rubber, None, (0.12 * foot).min(0.06)));
        shapes.push(style::cyl(0.86 * foot, 0.03 + 0.015 * foot, Axis::Y, 14, 1.0, p(g.ankle.0, -g.stance + g.pad_h + 0.01, 0.0), "fittings", Slot::Trim, None, 0.0));

        let contact = 0.5 * std::f64::consts::PI * foot * foot;
        PartDef {
            id: format!("leg_{:.1}m_{:.3}", v["stance_m"], v["thickness"]),
            name: format!("Leg {:.1} m", v["stance_m"]),
            category: Category::Leg,
            size: SizeClass::Large,
            tags: vec!["legs".into(), "walker".into()],
            palette: None,
            voxels: Some(64),
            // A hollow leg tube is air: a shot through it hits two walls and nothing vital.
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Large,
                at: [0.0; 3],
                normal: [-1.0, 0.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::Legs),
                load_kg: rated_load_kg(v, lib),
                contact_m2: contact,
                contact_len_m: 2.0 * foot,
                rolling: Some(COST_OF_TRANSPORT),
                traction: Some(0.8),
                max_kmh: Some((3.6 * (FROUDE_MAX * G * g.stance).sqrt()).min(60.0)),
                ride_height_m: Some(g.stance),
                step_m: 0.35 * g.stance,
                draw_kw: 0.002 * act,
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let lib = MaterialLibrary { materials: Default::default(), palettes: Default::default() };
        let _ = lib;
        let g = geometry(v);
        vec![
            stat("leg mass", built.mass.mass_kg, "kg"),
            stat("stance", g.stance, "m"),
            stat("gait speed limit", (3.6 * (FROUDE_MAX * G * g.stance).sqrt()).min(60.0), "km/h"),
            stat("foot reach", g.x_out, "m"),
            stat("slenderness (shin)", g.l2 / g.s2.0, ""),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, lib: &MaterialLibrary) {
        // Ground pressure: pads large enough to keep about 100 kPa under the half of the feet that are down.
        let r = (2.0 * load_kg * G / (std::f64::consts::PI * 100e3)).sqrt();
        v.insert("foot_m".into(), r.clamp(0.1, 4.0));
        // Thickness: the thinnest leg that carries the load with a margin.
        let need = load_kg * 1.25;
        let (mut lo, mut hi) = (0.03, 0.18);
        v.insert("thickness".into(), hi);
        if rated_load_kg(v, lib) < need {
            return; // even the fattest leg is not enough: the design will report it
        }
        for _ in 0..30 {
            let m = 0.5 * (lo + hi);
            v.insert("thickness".into(), m);
            if rated_load_kg(v, lib) >= need {
                hi = m;
            } else {
                lo = m;
            }
        }
        v.insert("thickness".into(), hi);
    }
}

/// Convenience for tests: the foot position in part space.
pub fn foot_position(v: &Values) -> V3 {
    let g = geometry(v);
    p(g.x_out, -g.stance, 0.0)
}
