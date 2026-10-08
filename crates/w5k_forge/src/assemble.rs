//! Vehicles: a hull part plus parts attached to sockets, recursively.

use crate::build::Piece;
use crate::geom::{Xform, M3, V3};
use crate::schema::{Attach, DesignDef, Locomotion, Motion, SocketDef};
use crate::{BuiltPart, Forge};

/// A socket placed in vehicle space.
#[derive(Clone, Debug)]
pub struct PlacedSocket {
    pub part_index: usize,
    pub def: SocketDef,
    pub at: V3,
    pub normal: V3,
    pub forward: V3,
}

/// A moving sub-assembly placed in vehicle space.
#[derive(Clone, Debug)]
pub struct JointInst {
    /// Index of the part (in [`Assembly::parts`]) it belongs to.
    pub part: usize,
    pub name: String,
    pub pivot: V3,
    pub axis: V3,
    pub motion: Motion,
    /// The joint it hangs from (an index into [`Assembly::joints`]), if any.
    pub parent: Option<usize>,
}

/// A vehicle's pieces and where each part went.
#[derive(Clone, Debug, Default)]
pub struct Assembly {
    /// (part id, transform from part space to vehicle space) in attachment order; index 0 is the hull.
    pub parts: Vec<(String, Xform)>,
    pub pieces: Vec<Piece>,
    pub sockets: Vec<PlacedSocket>,
    pub errors: Vec<String>,
    /// For each part, which part and socket it hangs from (the hull has none).
    pub placements: Vec<Placement>,
    /// Every moving sub-assembly of every part; `Piece::joint` indexes this (plus one).
    pub joints: Vec<JointInst>,
}

/// Where a part hangs.
#[derive(Clone, Debug)]
pub struct Placement {
    pub part_index: usize,
    pub parent: Option<usize>,
    pub socket: String,
}

fn frame(n: V3, f: V3) -> (V3, V3, V3) {
    let n = n.norm();
    let f = (f - n * f.dot(n)).norm();
    (n, f, n.cross(f))
}

/// Transform that brings a child's mount frame onto a parent's socket frame (mount normal opposing the
/// socket normal, forwards aligned, then spun about the socket normal).
pub fn align(parent_at: V3, parent_n: V3, parent_f: V3, spin_deg: f64, mount: &SocketDef, mirror: bool) -> Xform {
    let pre = if mirror { M3::scale(V3 { x: -1.0, y: 1.0, z: 1.0 }) } else { M3::IDENTITY };
    let (cn, cf, cb) = frame(pre * V3::from_arr(mount.normal), pre * V3::from_arr(mount.forward));
    let spun_f = M3::axis_angle(parent_n, spin_deg) * parent_f;
    let (tn, tf, tb) = frame(-parent_n, spun_f);
    let r = M3::from_cols(tn, tf, tb) * M3::from_cols(cn, cf, cb).transpose();
    let m = r * pre;
    let t = parent_at - m * V3::from_arr(mount.at);
    Xform::new(m, t)
}

impl Forge {
    pub fn assemble(&self, design: &DesignDef) -> Assembly {
        let mut a = Assembly::default();
        let Some(hull) = self.part(&design.hull) else {
            a.errors.push(format!("unknown hull part '{}'", design.hull));
            return a;
        };
        let x = Xform::new(M3::IDENTITY, V3 { x: 0.0, y: design.lift_m, z: 0.0 });
        place(self, hull, x, &mut a);
        a.placements.push(Placement { part_index: 0, parent: None, socket: String::new() });
        attach_children(self, 0, &design.attach, &mut a);
        a
    }
}

fn place(forge: &Forge, part: &BuiltPart, x: Xform, a: &mut Assembly) -> usize {
    let index = a.parts.len();
    a.parts.push((part.def.id.clone(), x));
    let base = a.joints.len();
    for j in &part.joints {
        a.joints.push(JointInst {
            part: index,
            name: j.name.clone(),
            pivot: x.point(j.pivot),
            axis: x.dir(j.axis).norm(),
            motion: j.motion.transformed(&x),
            parent: j.parent.map(|p| base + p as usize),
        });
    }
    for p in &part.pieces {
        let mut q = p.transformed(&x, index as u16);
        if q.joint > 0 {
            q.joint += base as u16;
        }
        a.pieces.push(q);
    }
    for s in &part.def.sockets {
        let at = x.point(V3::from_arr(s.at));
        let normal = x.dir(V3::from_arr(s.normal)).norm();
        let forward = x.dir(V3::from_arr(s.forward)).norm();
        a.sockets.push(PlacedSocket { part_index: index, def: s.clone(), at, normal, forward });
    }
    let _ = forge;
    index
}

fn attach_children(forge: &Forge, parent: usize, list: &[Attach], a: &mut Assembly) {
    for at in list {
        let Some(sock) = a.sockets.iter().find(|s| s.part_index == parent && s.def.name == at.socket).cloned() else {
            a.errors.push(format!("part '{}' has no socket '{}'", a.parts[parent].0, at.socket));
            continue;
        };
        let Some(child) = forge.part(&at.part) else {
            a.errors.push(format!("unknown part '{}'", at.part));
            continue;
        };
        let Some(mount) = child.def.sockets.iter().find(|s| s.name == "mount") else {
            a.errors.push(format!("part '{}' has no 'mount' socket", at.part));
            continue;
        };
        let x = align(sock.at, sock.normal, sock.forward, at.spin, mount, at.mirror);
        let idx = place(forge, child, x, a);
        a.placements.push(Placement { part_index: idx, parent: Some(parent), socket: at.socket.clone() });
        attach_children(forge, idx, &at.children, a);
    }
}

/// One weapon on a vehicle, as the sheet lists it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WeaponLine {
    pub name: String,
    pub kind: String,
    pub energy_mj: f64,
    pub shots_per_min: f64,
    pub penetration_mm: f64,
    pub range_km: f64,
}

/// Totals, budgets and performance of an assembled vehicle (see `Forge::make_sheet`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VehicleSheet {
    pub mass_kg: f64,
    pub power_kw: f64,
    pub draw_kw: f64,
    pub load_kg: f64,
    pub locomotion: Vec<Locomotion>,
    /// Frontal area used for drag (m^2).
    pub frontal_m2: f64,
    /// Top speed on flat ground (km/h); 0 when the vehicle cannot move.
    pub top_speed_kmh: f64,
    /// What sets the top speed: "power" (the power balance) or "running gear" (a locomotion part's limit).
    #[serde(default)]
    pub speed_limited_by: String,
    /// Power spent holding the vehicle up (rotors, air cushion or anti-gravity), kW.
    #[serde(default, alias = "hover_kw")]
    pub lift_kw: f64,
    /// Weight over ground contact area (kPa); 0 when the running gear does not report contact. Soft soil gives way
    /// somewhere around 50-100 kPa; a tank presses about 90.
    #[serde(default)]
    pub ground_pressure_kpa: f64,
    pub power_to_weight_kw_t: f64,
    /// Things that make the design invalid (it cannot be fielded).
    pub problems: Vec<String>,
    /// Things that make it worse but legal.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// "ground", "rail", "hover" or "air".
    #[serde(default)]
    pub movement: String,
    #[serde(default)]
    pub length_m: f64,
    #[serde(default)]
    pub width_m: f64,
    #[serde(default)]
    pub height_m: f64,
    /// How far the hull is raised on its running gear (m).
    #[serde(default)]
    pub lift_height_m: f64,
    /// Typical (median) steel-equivalent armour seen from the front, side, rear and above (mm), and the front's weak spots.
    #[serde(default)]
    pub armour_front_mm: f64,
    #[serde(default)]
    pub armour_side_mm: f64,
    #[serde(default)]
    pub armour_rear_mm: f64,
    #[serde(default)]
    pub armour_top_mm: f64,
    #[serde(default)]
    pub armour_front_weak_mm: f64,
    #[serde(default)]
    pub side_area_m2: f64,
    #[serde(default)]
    pub top_area_m2: f64,
    /// How fast it can turn on the spot or in its tightest turn (degrees per second).
    #[serde(default)]
    pub turn_rate_dps: f64,
    /// Tallest obstacle it crosses (m).
    #[serde(default)]
    pub step_m: f64,
    #[serde(default)]
    pub weapons: Vec<WeaponLine>,
    /// Energy it can deliver per second, all weapons firing (kW); energy weapons are limited by spare power.
    #[serde(default)]
    pub firepower_kw: f64,
    /// Energy of one volley of every weapon (MJ).
    #[serde(default)]
    pub alpha_mj: f64,
    #[serde(default)]
    pub best_pen_mm: f64,
    /// Longest weapon range (km).
    #[serde(default)]
    pub range_km: f64,
    /// Height of its highest sensor above the ground (m) and how far it sees (km), horizon included.
    #[serde(default)]
    pub eye_height_m: f64,
    #[serde(default)]
    pub sight_km: f64,
    /// Speed the biggest gun's recoil gives the whole vehicle (m/s).
    #[serde(default)]
    pub recoil_mps: f64,
    /// Repair rigs: structure restored per second (kg/s) and reach (m).
    #[serde(default)]
    pub repair_kg_s: f64,
    #[serde(default)]
    pub repair_reach_m: f64,
}

/// Fraction of engine power (after what other systems draw) that ends up pushing the vehicle along.
pub fn drive_efficiency(l: Locomotion) -> f64 {
    match l {
        Locomotion::Wheels => 0.9,
        Locomotion::HalfTracks => 0.85,
        Locomotion::Tracks => 0.8,
        Locomotion::Legs => 0.6,
        Locomotion::Hover => 0.7,
        Locomotion::AntiGrav => 0.7,
        Locomotion::Rotor => 0.5,
        Locomotion::Jet => 0.6,
        Locomotion::Rail => 0.92,
    }
}

/// Efficiency of the fans that pump an air cushion.
pub const FAN_EFFICIENCY: f64 = 0.7;

/// Ideal hover power of rotors from actuator-disc (momentum) theory: lifting weight `w` (N) through total
/// disc area `area` (m^2) accelerates air downward, costing w^1.5 / sqrt(2 rho A). Real rotors reach about
/// 70% of that ideal (their "figure of merit").
pub fn hover_power_w(w: f64, area: f64) -> f64 {
    const RHO: f64 = 1.225;
    const FIGURE_OF_MERIT: f64 = 0.7;
    if area <= 0.0 {
        return f64::INFINITY;
    }
    w.powf(1.5) / (2.0 * RHO * area).sqrt() / FIGURE_OF_MERIT
}

/// Solve P = c1 v + c3 v^3 for v >= 0 (Newton's method; the function is monotonic).
pub fn power_balance_speed(power_w: f64, c1: f64, c3: f64) -> f64 {
    if power_w <= 0.0 {
        return 0.0;
    }
    let mut v = if c3 > 0.0 { (power_w / c3).cbrt() } else { power_w / c1.max(1e-9) };
    for _ in 0..50 {
        let f = c3 * v * v * v + c1 * v - power_w;
        let df = 3.0 * c3 * v * v + c1;
        if df <= 0.0 {
            break;
        }
        let nv = (v - f / df).max(0.0);
        if (nv - v).abs() < 1e-9 {
            break;
        }
        v = nv;
    }
    v
}
