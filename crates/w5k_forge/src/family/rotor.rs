//! Rotors: lift from spinning blades, on arms at the side stations or as a coaxial pair above a turret ring.
//!
//! * **Hover power** is set by the disc. Momentum theory gives the ideal power to hold a weight W on a disc of
//!   area A as P = W^1.5 / sqrt(2 rho A), divided by a figure of merit of about 0.7 for real blades. Doubling the
//!   disc radius quadruples the area (the disc loading W/A falls to a quarter) and halves the hover power; the sheet
//!   sums the discs of every rotor on the vehicle.
//! * **Rated lift** is what the blades can bite: thrust T = C_T rho A V_tip^2 where the blades stall beyond
//!   C_T = 0.12 sigma (sigma, the solidity, is the blade area over the disc area). More blades, bigger discs and
//!   faster tips lift more.
//! * Blades are slender composite beams: their mass grows with R^3, so a big rotor is far from free.
//!
//! Part space: the mount is on the hull; the rotor sits above it on a pylon (stations, hips) or on a mast (ring).

use super::mounts::{KIND_MAST, KIND_RING, KIND_STATION};
use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Rotor;

const G: f64 = 9.81;
const RHO: f64 = 1.225;
/// Chord of a blade as a fraction of the rotor radius.
const CHORD: f64 = 0.08;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// A coaxial pair on a turret ring.
fn is_ring(v: &Values) -> bool {
    (ctx(v, "ctx.kind", KIND_STATION) - KIND_RING).abs() < 0.5
}

/// A single rotor on a tall mast (no neighbours to clear).
fn is_mast(v: &Values) -> bool {
    (ctx(v, "ctx.kind", KIND_STATION) - KIND_MAST).abs() < 0.5
}

/// Mounted on its own mast, with no neighbouring discs to avoid.
fn is_free(v: &Values) -> bool {
    is_ring(v) || is_mast(v)
}

/// Rotor radius after clamping to the room between stations (arm rotors may not overlap their neighbours).
fn radius(v: &Values) -> f64 {
    let r = v["radius_m"];
    if is_free(v) {
        r
    } else {
        r.min(0.49 * ctx(v, "ctx.spacing", 100.0)).max(0.2)
    }
}

/// Rated lift (kg) of one rotor: the thrust at the blade-stall limit.
pub fn rated_lift_kg(v: &Values) -> f64 {
    let r = radius(v);
    let nb = v["blades"].round().clamp(2.0, 6.0);
    let sigma = nb * CHORD / std::f64::consts::PI;
    let area = std::f64::consts::PI * r * r;
    let pair = if is_ring(v) { 2.0 } else { 1.0 };
    pair * 0.12 * sigma * RHO * area * v["tip_ms"] * v["tip_ms"] / G
}

impl Family for Rotor {
    fn id(&self) -> &'static str {
        "rotor"
    }
    fn name(&self) -> &'static str {
        "Rotor"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("radius_m", "Rotor radius", "m", 0.25, 14.0, 1.4, Scale::Log, Role::Budgeted, "Bigger discs hover on less power (it falls with the square root of the disc area), but weigh more and need room."),
            param("blades", "Blades", "", 2.0, 6.0, 3.0, Scale::Linear, Role::Budgeted, "More blades can bite more air (rated lift), and weigh more."),
            param("tip_ms", "Tip speed", "m/s", 120.0, 260.0, 200.0, Scale::Linear, Role::Free, "Faster tips lift more (thrust grows with the square), at the cost of noise."),
            param("altitude_m", "Altitude", "m", 1.5, 60.0, 6.0, Scale::Log, Role::Free, "Cruise height above the ground: the horizon (what it can see, and be seen from) grows with its square root."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Station, SocketKind::Hip, SocketKind::TurretRing, SocketKind::Mast]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_skiff", hull_params: &[("length_m", 9.0), ("width_m", 3.0), ("height_m", 1.0), ("front_mm", 20.0), ("side_mm", 10.0)], socket: "hub", kw_per_t: 300.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let r = radius(v);
        let nb = v["blades"].round().clamp(2.0, 6.0) as u32;
        let ring = is_ring(v);
        let top = ctx(v, "ctx.top", 0.5);
        let chord = CHORD * r;
        let mut shapes: Vec<Node> = Vec::new();
        let hub_r = 0.07 * r + 0.05;
        // One set of blades about the hub centre: slim composite beams with a glowing tip, and a faint ring that
        // marks the disc the blades sweep.
        let tb = 0.14 * chord + 0.012;
        let blades = |phase: f64, slot: Slot| -> Node {
            Node::Radial {
                count: nb,
                axis: Axis::Y,
                phase,
                children: vec![
                    style::bx([r - hub_r, tb, chord], p(0.5 * (r + hub_r), 0.0, 0.0), "composite", slot, 0.0),
                    style::bx([0.06 * r + 0.03, tb + 0.006, chord * 1.04], p(r - 0.03 * r - 0.015, 0.0, 0.0), "fittings", Slot::Glow, 0.0),
                ],
            }
        };
        let tip_ring = |at: crate::geom::V3, shapes: &mut Vec<Node>| {
            shapes.push(style::flat_ring(r, 0.012 + 0.006 * r, 0.006, (24.0 + 6.0 * r).min(72.0) as u32, at, "scenery", Slot::Glow));
        };
        let hub_disc = |at: crate::geom::V3, shapes: &mut Vec<Node>| {
            shapes.push(style::cyl(hub_r, 0.12 * hub_r + 0.05, Axis::Y, 12, 1.0, at, "machinery", Slot::Dark, None, 0.0));
            shapes.push(style::cyl(0.45 * hub_r, 0.12 * hub_r + 0.08, Axis::Y, 8, 1.0, at + p(0.0, 0.02, 0.0), "fittings", Slot::Glow, None, 0.0));
        };
        let alt = v["altitude_m"];
        let (hub_x, hub_y);
        if is_free(v) {
            // A mast carrying one disc, or two counter-rotating discs on a turret ring (no tail rotor needed).
            let coax = ring;
            let mh = (if coax { 0.5 } else { 0.9 }) + 0.22 * r;
            let gap = 0.25 + 0.05 * r;
            shapes.push(style::cyl(0.55 * hub_r + 0.02, mh, Axis::Y, 12, 1.0, p(0.0, mh / 2.0, 0.0), "machinery", Slot::Secondary, None, 0.0));
            shapes.push(style::cyl(1.1 * hub_r + 0.04, 0.15 * mh, Axis::Y, 12, 1.0, p(0.0, 0.07 * mh, 0.0), "machinery", Slot::Dark, None, 0.0));
            let levels: Vec<f64> = if coax { vec![mh - gap, mh] } else { vec![mh] };
            for (k, y) in levels.iter().enumerate() {
                let at = p(0.0, *y, 0.0);
                hub_disc(at, &mut shapes);
                shapes.push(Node::Group { at: at.arr(), rot: [0.0; 3], scale: 1.0, children: vec![blades(if k == 0 { 0.0 } else { 180.0 / nb as f64 }, if k == 0 { Slot::Primary } else { Slot::Trim })] });
                tip_ring(at, &mut shapes);
            }
            hub_x = 0.0;
            hub_y = mh - gap;
        } else {
            // A pylon out and up from the hull side, with the rotor above the deck.
            let hy = top + 0.3 + 0.05 * r;
            let arm = 0.85 * r + 0.3;
            let w = 0.05 + 0.025 * r;
            shapes.push(style::beam(p(0.0, 0.0, 0.0), p(arm, hy - 0.06, 0.0), [1.5 * w, w], p(0.0, 1.0, 0.0), "steel", Slot::Dark, 0.1 * w));
            shapes.push(style::cyl(1.2 * hub_r + 0.04, 0.2 * hub_r + 0.1, Axis::Y, 12, 1.0, p(arm, hy - 0.1, 0.0), "machinery", Slot::Secondary, None, 0.0));
            let at = p(arm, hy, 0.0);
            hub_disc(at, &mut shapes);
            shapes.push(Node::Group { at: at.arr(), rot: [0.0; 3], scale: 1.0, children: vec![blades(0.0, Slot::Primary)] });
            tip_ring(at, &mut shapes);
            hub_x = arm;
            hub_y = hy - 0.1;
        }
        // Altitude made visible: a plumb line from the hub down to the ground and the ring of downwash where it lands.
        if hub_y + alt > 1.0 {
            let ground = -alt;
            shapes.push(style::cyl(0.012 + 0.006 * r, hub_y - ground, Axis::Y, 6, 1.0, p(hub_x, ground + 0.5 * (hub_y - ground), 0.0), "scenery", Slot::Glow, None, 0.0));
            shapes.push(style::flat_ring(0.8 * r + 0.2, 0.03 + 0.02 * r, 0.02, (20.0 + 6.0 * r).min(56.0) as u32, p(hub_x, ground + 0.012, 0.0), "scenery", Slot::Glow));
        }
        let lift = rated_lift_kg(v);
        PartDef {
            id: format!("rotor_{r:.2}m_{nb}b{}", if ring { "_coax" } else { "" }),
            name: format!("Rotor {r:.1} m"),
            category: Category::Locomotion,
            size: SizeClass::Medium,
            tags: vec!["rotor".into()],
            palette: None,
            voxels: Some(96),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [0.0; 3],
                normal: if is_free(v) { [0.0, -1.0, 0.0] } else { [-1.0, 0.0, 0.0] },
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::Rotor),
                load_kg: lift,
                // A coaxial pair works one air column: the effective disc is a little smaller than a single disc.
                rotor_radius_m: if ring { 0.86 * r } else { r },
                max_kmh: Some(320.0),
                ride_height_m: Some(alt),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let r = radius(v);
        let area = std::f64::consts::PI * r * r * if is_ring(v) { 0.74 } else { 1.0 };
        let lift = rated_lift_kg(v);
        vec![
            stat("rotor mass", built.mass.mass_kg, "kg"),
            stat("rated lift", lift / 1000.0, "t"),
            stat("disc area", area, "m2"),
            // Hover at the rated lift: ideal power W^1.5 / sqrt(2 rho A) over a figure of merit 0.7.
            stat("hover power at rating", (lift * G).powf(1.5) / (2.0 * RHO * area).sqrt() / 0.7 / 1000.0, "kW"),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, _lib: &MaterialLibrary) {
        // Four blades at 200 m/s first; if the room between stations caps the radius, add blades and tip speed.
        let need = load_kg * 1.2;
        for (nb, tip) in [(4.0, 200.0), (6.0, 260.0)] {
            v.insert("blades".into(), nb);
            v.insert("tip_ms".into(), tip);
            let k = rated_lift_kg(&{
                let mut t = v.clone();
                t.insert("radius_m".into(), 1.0);
                t
            });
            let want = (need / k).sqrt();
            let room = if is_free(v) { 14.0 } else { 0.49 * ctx(v, "ctx.spacing", 100.0) };
            v.insert("radius_m".into(), want.clamp(0.25, 14.0));
            if want <= room {
                return;
            }
        }
    }
}
