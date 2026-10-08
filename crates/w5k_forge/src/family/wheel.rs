//! Wheeled running gear: one wheel station per hull station.
//!
//! * A tyre's **load rating** is its contact patch times its inflation pressure: F = p w (0.35 D). Wide, tall,
//!   well-inflated tyres carry more.
//! * The same pressure is the **ground pressure** the tyre puts on the soil, so a low-pressure tyre floats over
//!   soft ground that a hard one sinks into, and pays for it with rolling resistance on a hard road.
//! * Rolling resistance falls with wheel diameter (big wheels roll over irregularities instead of climbing them):
//!   C_rr = (0.010 + 0.015 / sqrt(D)) (300 / p)^0.25.
//!
//! Part space: the mount is on the hull side at the origin, the wheel hangs outward (+x) and down so that its
//! lowest point is exactly `ctx.hip_height` below the mount (the ground).

use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Wheel;

const G: f64 = 9.81;

/// Length of the contact patch as a fraction of the diameter at rated load.
const PATCH: f64 = 0.35;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// Wheel diameter after clamping to the room between stations.
fn diameter(v: &Values) -> f64 {
    let spacing = ctx(v, "ctx.spacing", 100.0);
    v["diameter_m"].min(0.96 * spacing).max(0.2)
}

/// Rated load (kg) of one wheel.
pub fn rated_load_kg(v: &Values) -> f64 {
    let d = diameter(v);
    v["pressure_kpa"] * 1000.0 * v["width_m"] * PATCH * d / G
}

impl Family for Wheel {
    fn id(&self) -> &'static str {
        "wheel"
    }
    fn name(&self) -> &'static str {
        "Wheel station"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("diameter_m", "Diameter", "m", 0.4, 5.0, 1.1, Scale::Log, Role::Budgeted, "Big wheels roll over obstacles and soft ground more easily and carry more, but weigh more."),
            param("width_m", "Width", "m", 0.15, 1.8, 0.4, Scale::Linear, Role::Budgeted, "Wider tyres carry more and spread the weight."),
            param("pressure_kpa", "Tyre pressure", "kPa", 50.0, 700.0, 300.0, Scale::Log, Role::Free, "Low pressure floats over soft ground but rolls badly on hard roads; high pressure carries more per tyre."),
            param("tread", "Tread", "", 0.0, 1.0, 0.6, Scale::Linear, Role::Free, "Smooth to lugged: grip on loose ground."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Station]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_bastion", hull_params: &[("stations", 3.0)], socket: "station_*", kw_per_t: 18.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let d = diameter(v);
        let r = d / 2.0;
        let w = v["width_m"].min(1.2 * d);
        let hip = ctx(v, "ctx.hip_height", 0.6);
        let tread = v["tread"];
        // Suspension: the wheel hangs out from the hull side on an arm.
        let gap = 0.1 + 0.1 * r;
        let cx = gap + w / 2.0;
        let cy = r - hip; // wheel centre relative to the mount (the lowest point of the tyre is at -hip)
        let c = p(cx, cy, 0.0);
        let mut shapes: Vec<Node> = Vec::new();
        // Tyre: a hollow rubber carcass.
        let seg = if r > 1.0 { 28 } else if r > 0.4 { 22 } else { 16 };
        shapes.push(style::cyl(r, w, Axis::X, seg, 1.0, c, "rubber", Slot::Rubber, Some((0.06 * r).max(0.008)), (0.12 * w.min(r)).min(0.1)));
        // Tread lugs around the circumference.
        if tread > 0.25 {
            let n = ((6.0 + 7.0 * d.max(0.3).ln().max(-0.5) + 8.0 * d.min(2.0)).round().clamp(8.0, 30.0)) as u32;
            let h = (0.03 + 0.07 * tread) * r;
            let chord = std::f64::consts::TAU * r / n as f64 * 0.45;
            shapes.push(Node::Group {
                at: c.arr(),
                rot: [0.0; 3],
                scale: 1.0,
                children: vec![Node::Radial {
                    count: n,
                    axis: Axis::X,
                    phase: 0.0,
                    children: vec![style::bx([w * 0.96, h, chord], p(0.0, r + 0.2 * h, 0.0), "rubber", Slot::Rubber, 0.01 * h)],
                }],
            });
        }
        // Rim and hub cap with a glowing centre.
        shapes.push(style::cyl(0.62 * r, w * 0.92, Axis::X, seg.min(18), 1.0, c, "steel", Slot::Secondary, Some(0.012 + 0.01 * r), 0.04 * r));
        shapes.push(style::cyl(0.24 * r, 0.05 + 0.1 * r, Axis::X, 10, 1.0, c + p(w / 2.0 + 0.01, 0.0, 0.0), "steel", Slot::Metal, None, 0.0));
        shapes.push(style::cyl(0.09 * r, 0.03 + 0.05 * r, Axis::X, 8, 1.0, c + p(w / 2.0 + 0.04 + 0.05 * r, 0.0, 0.0), "fittings", Slot::Glow, None, 0.0));
        // Suspension arm and a damper from the hull to the hub.
        let aw = (0.1 + 0.12 * r).min(0.5);
        shapes.push(style::beam(p(0.0, -0.04 * r, 0.0), p(gap + 0.02, cy, 0.0), [aw, aw * 0.8], p(0.0, 0.0, 1.0), "steel", Slot::Dark, 0.1 * aw));
        shapes.push(style::beam(p(0.0, 0.18 * r + 0.05, 0.0), p(gap * 0.8, cy + 0.5 * r, 0.0), [0.6 * aw, 0.6 * aw], p(0.0, 0.0, 1.0), "machinery", Slot::Metal, 0.0));
        let patch = PATCH * d;
        let contact = w * patch;
        PartDef {
            id: format!("wheel_{d:.2}m_{w:.2}m"),
            name: format!("Wheel {d:.2} m"),
            category: Category::Locomotion,
            size: SizeClass::Medium,
            tags: vec!["wheels".into()],
            palette: None,
            voxels: Some(64),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [0.0; 3],
                normal: [-1.0, 0.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::Wheels),
                load_kg: v["pressure_kpa"] * 1000.0 * contact / G,
                contact_m2: contact,
                contact_len_m: patch,
                contact_w_m: w,
                rolling: Some(((0.010 + 0.015 / d.sqrt()) * (300.0 / v["pressure_kpa"]).powf(0.25)).max(0.008)),
                traction: Some(0.5 + 0.3 * tread),
                max_kmh: Some((160.0 - 18.0 * d).clamp(35.0, 140.0)),
                ride_height_m: Some(hip),
                step_m: 0.6 * r,
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let d = diameter(v);
        vec![
            stat("wheel mass", built.mass.mass_kg, "kg"),
            stat("rated load", rated_load_kg(v) / 1000.0, "t"),
            stat("ground pressure", v["pressure_kpa"], "kPa"),
            stat("rolling resistance", (0.010 + 0.015 / d.sqrt()) * (300.0 / v["pressure_kpa"]).powf(0.25), ""),
            stat("steps obstacles", 0.6 * d / 2.0, "m"),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, _lib: &MaterialLibrary) {
        // Carry the load with the pressure the design asked for: widen the tyre first (up to as wide as it is
        // tall), then make the wheel bigger (up to the room between stations).
        let need = load_kg * 1.2;
        let spacing = ctx(v, "ctx.spacing", 100.0);
        let dmax = (0.96 * spacing).min(5.0);
        let mut d = v["diameter_m"].min(dmax);
        loop {
            let w = need * G / (v["pressure_kpa"] * 1000.0 * PATCH * d);
            if w <= (0.5 * d).clamp(0.15, 1.8) || d >= dmax - 1e-9 {
                v.insert("diameter_m".into(), d);
                v.insert("width_m".into(), w.clamp(0.15, 1.8));
                return;
            }
            d = (d * 1.08).min(dmax);
        }
    }
}

/// Convenience for tests: the point where the tyre touches the ground, relative to the mount.
pub fn contact_point(v: &Values) -> V3 {
    let d = diameter(v);
    let hip = ctx(v, "ctx.hip_height", 0.6);
    let gap = 0.1 + 0.1 * d / 2.0;
    p(gap + v["width_m"].min(1.2 * d) / 2.0, -hip, 0.0)
}
