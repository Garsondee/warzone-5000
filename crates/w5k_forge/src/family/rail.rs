//! Rail bogies: running gear that rides on rails.
//!
//! * Steel wheels on steel rails roll with almost no resistance (C_rr about 0.0015, a twentieth of a tyre) and
//!   carry great loads: a bogie's rating is simply its axles times the axle load. The price is **adhesion**: steel
//!   on steel grips with mu of about 0.25, and the vehicle can only go where the rails go.
//! * Heavier axle loads need stronger, heavier bogie frames; wider gauge is more stable and faster.
//!
//! Part space: the mount is under the hull on the centre line (the socket's normal points down); the bogie hangs
//! below it and rests on a short length of rail and sleepers (`scenery`: no mass, no armour) so a train reads as
//! riding on a track.

use super::mounts::KIND_KEEL;
use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Rail;

/// Height of the rail head above the ground (sleeper plus rail).
const RAIL_H: f64 = 0.18;

fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// Axles that fit in the slot the hull offers, and their spacing.
fn layout(v: &Values) -> (usize, f64) {
    let r = v["wheel_d_m"] / 2.0;
    let slot = ctx(v, "ctx.length", 6.0);
    let spacing_pref = 2.4 * r + 0.3;
    let spacing_min = 2.05 * r;
    let mut n = v["axles"].round().clamp(2.0, 6.0) as usize;
    while n > 2 && (n as f64 - 1.0) * spacing_min + 2.2 * r > slot {
        n -= 1;
    }
    let spacing = if n > 1 { spacing_pref.min(((slot - 2.2 * r) / (n as f64 - 1.0)).max(spacing_min)) } else { spacing_pref };
    (n, spacing)
}

/// Height of the hull's mount above the ground when riding (m).
fn ride(v: &Values) -> f64 {
    let hip = ctx(v, "ctx.hip_height", 0.8);
    hip.max(RAIL_H + v["wheel_d_m"] + 0.32)
}

pub fn rated_load_kg(v: &Values) -> f64 {
    layout(v).0 as f64 * v["axle_load_t"] * 1000.0
}

impl Family for Rail {
    fn id(&self) -> &'static str {
        "rail"
    }
    fn name(&self) -> &'static str {
        "Rail bogie"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("wheel_d_m", "Wheel diameter", "m", 0.5, 1.6, 0.9, Scale::Linear, Role::Budgeted, "Bigger wheels ride better and lift the hull."),
            param("axle_load_t", "Axle load", "t", 8.0, 45.0, 25.0, Scale::Linear, Role::Budgeted, "Rated load per axle: heavier axles need heavier frames and rails."),
            param("axles", "Axles", "", 2.0, 6.0, 4.0, Scale::Linear, Role::Free, "Axles per bogie (as many as fit under the hull's slot)."),
            param("gauge_m", "Gauge", "m", 0.8, 6.0, 1.9, Scale::Linear, Role::Free, "Distance between the rails: wide track is stable and fast, but the vehicle must be at least that wide."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Keel]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_dreadnought", hull_params: &[("length_m", 24.0), ("stations", 4.0), ("turrets", 2.0)], socket: "keel_*", kw_per_t: 12.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (n, a) = layout(v);
        let rw = v["wheel_d_m"] / 2.0;
        let gauge = v["gauge_m"];
        let ride = ride(v);
        let wheel_y = -(ride - RAIL_H - rw);
        let wheelbase = (n as f64 - 1.0) * a;
        let slot = ctx(v, "ctx.spacing", ctx(v, "ctx.length", 6.0) / 0.94);
        let load_kg = rated_load_kg(v);
        let mut shapes: Vec<Node> = Vec::new();
        // Frame: a box between the wheels, sized so the bogie weighs about 3% of what it carries.
        let frame_w = gauge + 0.3;
        let frame_l = wheelbase + 1.4 * rw;
        let frame_vol = 0.03 * load_kg / 900.0;
        let frame_h = (frame_vol / (frame_w * frame_l)).clamp(0.12, 0.7 * (ride - RAIL_H - 2.0 * rw).max(0.12) + 0.4);
        let frame_y = wheel_y + rw + 0.5 * frame_h + 0.05;
        shapes.push(style::bx([frame_w, frame_h, frame_l], p(0.0, frame_y, 0.0), "running_gear", Slot::Dark, 0.02));
        // Wheelsets: two flanged wheels on an axle per axle position, outside side frames with glowing axle boxes.
        let wl = 0.12 + 0.1 * rw;
        let fx = gauge / 2.0 + 0.5 * wl + 0.09;
        for k in 0..n {
            let z = (k as f64 - (n as f64 - 1.0) / 2.0) * a;
            for s in [-1.0, 1.0] {
                let x = s * gauge / 2.0;
                shapes.push(style::cyl(rw, wl, Axis::X, 20, 1.0, p(x, wheel_y, z), "steel", Slot::Dark, None, 0.02 * rw));
                shapes.push(style::cyl(1.1 * rw, 0.03, Axis::X, 20, 1.0, p(x - s * (0.5 * wl), wheel_y, z), "steel", Slot::Secondary, None, 0.0));
                shapes.push(style::bx([0.2, 0.34 * rw, 0.5 * rw], p(s * (fx + 0.02), wheel_y, z), "machinery", Slot::Trim, 0.015));
                shapes.push(style::cyl(0.16 * rw, 0.05, Axis::X, 8, 1.0, p(s * (fx + 0.14), wheel_y, z), "fittings", Slot::Glow, None, 0.0));
            }
            shapes.push(style::cyl(0.07 * rw, gauge, Axis::X, 8, 1.0, p(0.0, wheel_y, z), "steel", Slot::Dark, None, 0.0));
        }
        for s in [-1.0, 1.0] {
            shapes.push(style::bx([0.16, 0.5 * rw, wheelbase + 1.7 * rw], p(s * fx, wheel_y + 0.62 * rw, 0.0), "running_gear", Slot::Secondary, 0.03));
        }
        // Rail, ballast and sleepers under the wheels: scenery, continuous from one bogie to the next, and running on
        // past the ends of the vehicle (the first and last bogie extend the track).
        let idx = ctx(v, "ctx.index", -1.0);
        let total = ctx(v, "ctx.stations", 0.0);
        let ext_f = if idx >= -0.5 && idx < 0.5 { 4.0 } else { 0.0 };
        let ext_r = if total > 0.0 && (idx - (total - 1.0)).abs() < 0.5 { 4.0 } else { 0.0 };
        let rail_len = slot * 1.02 + ext_f + ext_r;
        let rail_z = 0.5 * (ext_r - ext_f);
        shapes.push(style::bx([gauge + 1.5, 0.05, rail_len], p(0.0, -ride + 0.025, rail_z), "scenery", Slot::Dark, 0.0));
        for s in [-1.0, 1.0] {
            shapes.push(style::bx([0.1 + 0.02 * rw, 0.08, rail_len], p(s * gauge / 2.0, -ride + RAIL_H - 0.04, rail_z), "scenery", Slot::Metal, 0.0));
        }
        let sleepers = (rail_len / 0.7).floor().max(2.0) as usize;
        for k in 0..sleepers {
            let z = (k as f64 + 0.5) / sleepers as f64 * rail_len - rail_len / 2.0 + rail_z;
            shapes.push(style::bx([gauge + 0.9, 0.1, 0.26], p(0.0, -ride + 0.1, z), "scenery", Slot::Secondary, 0.0));
        }
        let contact = (wheelbase + 1.2) * (gauge + 0.9) * 0.5;
        PartDef {
            id: format!("bogie_{n}x{:.0}t_{gauge:.2}m", v["axle_load_t"]),
            name: format!("{n}-axle bogie"),
            category: Category::Locomotion,
            size: SizeClass::Large,
            tags: vec!["rail".into()],
            palette: None,
            voxels: Some(64),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Large,
                at: [0.0; 3],
                normal: [0.0, 1.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::Rail),
                rail_bound: true,
                load_kg,
                contact_m2: contact,
                contact_len_m: wheelbase + 1.2,
                rolling: Some(0.0015),
                traction: Some(0.25),
                max_kmh: Some((100.0 + 50.0 * (gauge - 1.0)).clamp(80.0, 250.0)),
                ride_height_m: Some(ride),
                step_m: 0.0,
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let (n, _) = layout(v);
        vec![
            stat("bogie mass", built.mass.mass_kg / 1000.0, "t"),
            stat("axles", n as f64, ""),
            stat("rated load", rated_load_kg(v) / 1000.0, "t"),
            stat("rolling resistance", 0.0015, ""),
            stat("adhesion", 0.25, ""),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, _lib: &MaterialLibrary) {
        // The least axle load that carries the load on the axles that fit, with a margin.
        let n = layout(v).0 as f64;
        let t = (load_kg * 1.15 / 1000.0 / n).clamp(8.0, 45.0);
        v.insert("axle_load_t".into(), t);
    }
}

/// The hint code of the keel socket (for tests).
pub const KEEL: f64 = KIND_KEEL;
