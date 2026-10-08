//! Air-cushion running gear: a skirt under the whole hull, floated on a cushion of fan-blown air.
//!
//! * The cushion carries the weight at a low pressure: p = W / A, a few kilopascals, so a hovercraft crosses mud,
//!   swamp and water that would swallow a tank (a tank presses about 90 kPa; an air cushion 1-5 kPa).
//! * Lift power: the fans must replace the air that leaks under the skirt. Air escapes through the gap at speed
//!   v = sqrt(2 p / rho), so the flow is Q = perimeter x gap x v and the power P = p Q / eta. Because v grows as
//!   sqrt(p) and p as the weight, **lift power grows as W^1.5** at a fixed footprint, like a rotor's.
//! * A bigger footprint spreads the weight over more area (lower p, so less power) but is a bigger, easier target.
//!
//! Part space: the mount is at the middle of the underside of the hull (the socket's normal points down); the skirt
//! hangs from there to just above the ground.

use super::mounts::KIND_BELLY;
use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Motion, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Hover;

const G: f64 = 9.81;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// Footprint of the cushion (length, width) for a hull with the given underbody.
fn footprint(v: &Values) -> (f64, f64) {
    (ctx(v, "ctx.length", 6.0) * 0.98, ctx(v, "ctx.width", 2.4) * 1.06)
}

pub fn cushion_area(v: &Values) -> f64 {
    let (l, w) = footprint(v);
    0.85 * l * w
}

/// Skirt height: at least reaching the ground from the hull's underside.
fn skirt(v: &Values) -> f64 {
    v["skirt_m"].max(ctx(v, "ctx.hip_height", 0.5) + 0.05)
}

impl Family for Hover {
    fn id(&self) -> &'static str {
        "hover"
    }
    fn name(&self) -> &'static str {
        "Air cushion"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("skirt_m", "Skirt height", "m", 0.15, 2.0, 0.6, Scale::Linear, Role::Budgeted, "Taller skirts clear rougher ground and waves, but leak more air and cost more power."),
            param("fans", "Thrust fans", "", 1.0, 4.0, 2.0, Scale::Linear, Role::Budgeted, "Propulsion fans at the stern: more of them push harder (a higher top speed) and weigh more."),
            param("cushion_kpa", "Cushion pressure", "kPa", 0.6, 8.0, 3.0, Scale::Log, Role::Free, "Rated pressure under the hull: the weight it can float is this times the footprint."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Belly]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_skiff", hull_params: &[("length_m", 10.0), ("width_m", 4.4), ("height_m", 1.1), ("front_mm", 30.0), ("side_mm", 15.0)], socket: "belly", kw_per_t: 60.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (l, w) = footprint(v);
        let sk = skirt(v);
        let area = cushion_area(v);
        let fans = v["fans"].round().clamp(1.0, 4.0) as usize;
        let top = ctx(v, "ctx.top", 0.8);
        let mut shapes: Vec<Node> = Vec::new();
        // The skirt: a ring of rubber fingers hanging from just under the hull.
        let t = 0.05 + 0.03 * sk;
        let finger = |len: f64, count_hint: f64| -> usize { ((len / count_hint).ceil() as usize).max(2) };
        for (k, (xs, zs)) in [(0.0, -1.0), (0.0, 1.0)].iter().enumerate() {
            let nf = finger(w, 0.7);
            for i in 0..nf {
                let x = -w / 2.0 + (i as f64 + 0.5) * w / nf as f64;
                let slot = if (i + k) % 2 == 0 { Slot::Secondary } else { Slot::Dark };
                shapes.push(Node::Box { size: [w / nf as f64 * 0.94, sk, t], taper: None, shift: None, at: p(x + xs, -0.02 - sk / 2.0, zs * (l / 2.0)).arr(), rot: [0.0; 3], mat: "rubber".into(), slot, shell: Some(0.006), chamfer: 0.0 });
            }
        }
        for (k, xs) in [-1.0, 1.0].iter().enumerate() {
            let nf = finger(l, 0.7);
            for i in 0..nf {
                let z = -l / 2.0 + (i as f64 + 0.5) * l / nf as f64;
                let slot = if (i + k) % 2 == 0 { Slot::Secondary } else { Slot::Dark };
                shapes.push(Node::Box { size: [t, sk, l / nf as f64 * 0.94], taper: None, shift: None, at: p(xs * (w / 2.0), -0.02 - sk / 2.0, z).arr(), rot: [0.0; 3], mat: "rubber".into(), slot, shell: Some(0.006), chamfer: 0.0 });
            }
        }
        // A glowing seam along the bottom edge of the skirt, on its outer face: the cushion made visible.
        let y_edge = -sk + 0.06;
        let (ox, oz) = (w / 2.0 + 0.5 * t + 0.012, l / 2.0 + 0.5 * t + 0.012);
        for (a, b) in [
            (p(-ox, y_edge, -oz), p(ox, y_edge, -oz)),
            (p(-ox, y_edge, oz), p(ox, y_edge, oz)),
            (p(-ox, y_edge, -oz), p(-ox, y_edge, oz)),
            (p(ox, y_edge, -oz), p(ox, y_edge, oz)),
        ] {
            shapes.push(style::beam(a, b, [0.05, 0.04], p(0.0, 1.0, 0.0), "scenery", Slot::Glow, 0.0));
        }
        // Lift-fan deck under the hull: the fans and their ducting, sized by the power they must deliver.
        let deck = (0.4 * 0.8 * area.sqrt() * 0.1).clamp(0.06, 0.4);
        shapes.push(style::bx([0.7 * w, deck, 0.5 * l], p(0.0, -0.02 - 0.5 * deck, 0.0), "machinery", Slot::Dark, 0.02));
        // Stern thrust fans in ducts: big ring ducts standing on short pylons behind the hull, at deck height.
        let rd = (0.2 * w / fans as f64).clamp(0.12, 1.4);
        let yd = top + 0.15 + 1.1 * rd;
        for i in 0..fans {
            let x = (i as f64 - (fans as f64 - 1.0) / 2.0) * (2.25 * rd + 0.12);
            let z = l / 2.0 + 0.25 * rd + 0.4;
            let wall = 0.02 + 0.03 * rd;
            shapes.push(style::tube_z(rd, 0.5 * rd + 0.12, wall, 16, p(x, yd, z), "steel", Slot::Secondary));
            shapes.push(style::tube_z(1.04 * rd, 0.1, 1.2 * wall, 16, p(x, yd, z - 0.25 * rd - 0.04), "fittings", Slot::Glow));
            // The fan: three blades across the duct and its hub, turning about the duct's axis.
            let mut fan: Vec<Node> = (0..3)
                .map(|k| Node::Group { at: p(x, yd, z).arr(), rot: [0.0, 0.0, (k as f64) * 60.0], scale: 1.0, children: vec![style::bx([1.8 * rd, 0.1 * rd + 0.02, 0.03 + 0.04 * rd], p(0.0, 0.0, 0.0), "composite", Slot::Dark, 0.0)] })
                .collect();
            fan.push(style::cyl(0.22 * rd, 0.25 * rd + 0.1, Axis::Z, 10, 1.0, p(x, yd, z), "machinery", Slot::Dark, None, 0.0));
            shapes.push(Node::Joint { name: "fan".into(), pivot: p(x, yd, z).arr(), axis: [0.0, 0.0, 1.0], motion: Motion::Spin { rps: 3.0 }, children: fan });
            shapes.push(style::beam(p(x, 0.5 * top, l / 2.0 - 0.3), p(x, yd - 0.7 * rd, z - 0.2), [0.1 + 0.12 * rd, 0.1 + 0.12 * rd], p(0.0, 1.0, 0.0), "steel", Slot::Secondary, 0.0));
        }
        PartDef {
            id: format!("cushion_{:.1}kpa_{:.2}m", v["cushion_kpa"], sk),
            name: format!("Air cushion {:.1} kPa", v["cushion_kpa"]),
            category: Category::Locomotion,
            size: SizeClass::Large,
            tags: vec!["hover".into()],
            palette: None,
            voxels: Some(64),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef { name: "mount".into(), kind: SocketKind::Mount, size: SizeClass::Large, at: [0.0; 3], normal: [0.0, 1.0, 0.0], forward: [0.0, 0.0, -1.0], hints: Default::default() }],
            function: Function {
                locomotion: Some(Locomotion::Hover),
                load_kg: v["cushion_kpa"] * 1000.0 * area / G,
                contact_m2: area,
                contact_len_m: l,
                cushion_area_m2: area,
                cushion_perimeter_m: 2.0 * (l + w),
                cushion_gap_m: 0.015 + 0.025 * sk,
                rolling: Some(0.03),
                max_kmh: Some((55.0 + 22.0 * v["fans"].round().clamp(1.0, 4.0)).min(150.0)),
                ride_height_m: Some(sk + 0.03),
                step_m: 0.8 * sk,
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let area = cushion_area(v);
        vec![
            stat("skirt and fans mass", built.mass.mass_kg, "kg"),
            stat("cushion area", area, "m2"),
            stat("rated load", v["cushion_kpa"] * 1000.0 * area / G / 1000.0, "t"),
            stat("steps obstacles", 0.8 * skirt(v), "m"),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, _lib: &MaterialLibrary) {
        // The least cushion pressure that floats the load with a margin on this footprint.
        let kpa = (load_kg * 1.15 * G / cushion_area(v) / 1000.0).clamp(0.6, 8.0);
        v.insert("cushion_kpa".into(), kpa);
    }
}

pub const BELLY: f64 = KIND_BELLY;
