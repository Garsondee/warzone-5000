//! Vehicles: a hull part plus parts attached to sockets, recursively.

use crate::build::Piece;
use crate::geom::{Xform, M3, V3};
use crate::schema::{Attach, DesignDef, Locomotion, SocketDef};
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

/// A vehicle's pieces and where each part went.
#[derive(Clone, Debug, Default)]
pub struct Assembly {
    /// (part id, transform from part space to vehicle space) in attachment order; index 0 is the hull.
    pub parts: Vec<(String, Xform)>,
    pub pieces: Vec<Piece>,
    pub sockets: Vec<PlacedSocket>,
    pub errors: Vec<String>,
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
        place(self, hull, Xform::IDENTITY, &mut a);
        attach_children(self, 0, &design.attach, &mut a);
        a
    }
}

fn place(forge: &Forge, part: &BuiltPart, x: Xform, a: &mut Assembly) -> usize {
    let index = a.parts.len();
    a.parts.push((part.def.id.clone(), x));
    for p in &part.pieces {
        a.pieces.push(p.transformed(&x, index as u16));
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
        attach_children(forge, idx, &at.children, a);
    }
}

/// Totals and budget checks for an assembled vehicle.
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
    /// Power needed just to hover (rotorcraft), kW.
    #[serde(default)]
    pub hover_kw: f64,
    pub power_to_weight_kw_t: f64,
    pub problems: Vec<String>,
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
    }
}

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
