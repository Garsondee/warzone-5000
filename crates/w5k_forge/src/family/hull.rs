//! Hull families. Hulls differ by *shape*, and each shape is good at something different:
//! * **Lancer**: a low, sharp wedge. Excellent frontal protection per tonne and a small silhouette; little room
//!   inside and one turret.
//! * **Bastion**: a tall box with side sponsons. Lots of internal volume and room for two turrets; big and upright,
//!   so easy to hit and weak per tonne.
//! * **Dreadnought**: a long armoured deck carrying a row of turrets on several track sets (or rails). Huge
//!   firepower per unit; long, so it turns badly, and heavy, so it needs wide tracks or rails.
//!
//! Hull space is vehicle space: the ground is y = 0, forward is -Z. Sockets carry `hints` that size what attaches:
//! tracks learn their length and height, turrets their largest ring, engines their bay.

use super::mounts::{self, sock as socket, Chassis};
use super::style::{self, p, Quad};
use super::{stat, Family, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{MaterialLibrary, Category, Function, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale: Scale::Linear, role, help }
}

fn common_params(len: (f64, f64, f64), wid: (f64, f64, f64), hgt: (f64, f64, f64)) -> Vec<Param> {
    vec![
        param("length_m", "Length", "m", len.0, len.1, len.2, Role::Budgeted, "Longer hulls hold more and spread their weight, but turn worse (skid steering)."),
        param("width_m", "Width", "m", wid.0, wid.1, wid.2, Role::Budgeted, "Wider hulls take bigger turret rings and turn better, but show a bigger front."),
        param("height_m", "Height", "m", hgt.0, hgt.1, hgt.2, Role::Budgeted, "Taller hulls have room inside and see further; lower ones are harder to hit."),
        param("front_mm", "Front armour", "mm", 5.0, 250.0, 80.0, Role::Budgeted, "Plate thickness on the front faces (effective protection also depends on their slope)."),
        param("side_mm", "Side armour", "mm", 5.0, 150.0, 35.0, Role::Budgeted, "Plate thickness everywhere else."),
    ]
}

/// Extra front armour as a solid slab just behind a face: from `d1` to `d2` inward, shrunk toward the face's
/// centre so it stays inside the hull (it is invisible; only the armour rays and the mass see it).
fn inner_slab(q: &Quad, inside: V3, d1: f64, d2: f64, shrink: f64, mat: &str) -> Option<Node> {
    if d2 - d1 < 0.001 {
        return None;
    }
    let n = style::normal(q, inside);
    let c = (q[0] + q[1] + q[2] + q[3]) * 0.25;
    let mut pts = Vec::new();
    for corner in q {
        let k = c + (*corner - c) * (1.0 - shrink);
        pts.push(k - n * d1);
        pts.push(k - n * d2);
    }
    Some(style::hull(pts, mat, Slot::Primary, None, 0.0))
}

fn hull_part(id: String, name: String, size: SizeClass, shapes: Vec<Node>, sockets: Vec<SocketDef>) -> PartDef {
    PartDef {
        id,
        name,
        category: Category::Hull,
        size,
        tags: vec!["hull".into()],
        palette: None,
        voxels: Some(96),
        vital: Some(true),
        shapes,
        sockets,
        function: Function::default(),
    }
}

fn size_class(l: f64) -> SizeClass {
    match l {
        l if l < 3.0 => SizeClass::Small,
        l if l < 9.0 => SizeClass::Medium,
        l if l < 18.0 => SizeClass::Large,
        l if l < 35.0 => SizeClass::Huge,
        _ => SizeClass::Titanic,
    }
}

/// Ground clearance, and the height of the tracks' top run, for a hull of height `h`.
fn clearance(h: f64) -> (f64, f64) {
    let c = 0.2 + 0.2 * h;
    (c, c + 0.45 * h)
}

fn hull_stats(v: &Values, built: &Built) -> Vec<Stat> {
    let m = &built.mass;
    let l = v.get("length_m").copied().unwrap_or(2.0 * v.get("radius_m").copied().unwrap_or(1.0));
    let w = v.get("width_m").copied().unwrap_or(l);
    vec![
        stat("hull mass", m.mass_kg / 1000.0, "t"),
        stat("internal volume", m.internal_m3, "m3"),
        stat("front armour (effective)", built.armour.at(0.0, 0.0).median_mm, "mm"),
        stat("side armour (effective)", built.armour.at(90.0, 0.0).median_mm, "mm"),
        stat("frontal area", built.armour.at(0.0, 0.0).area_m2, "m2"),
        stat("side area", built.armour.at(90.0, 0.0).area_m2, "m2"),
        // Skid steering: the torque needed to turn grows with length over track spacing.
        stat("length / width", l / (w + 0.6), ""),
    ]
}

// ------------------------------------------------------------------------------------------------ Lancer

pub struct Lancer;

impl Family for Lancer {
    fn id(&self) -> &'static str {
        "hull_lancer"
    }
    fn name(&self) -> &'static str {
        "Lancer hull"
    }
    fn params(&self) -> Vec<Param> {
        let mut v = common_params((3.0, 14.0, 6.5), (1.4, 4.5, 2.4), (0.6, 2.2, 1.05));
        v.push(param("glacis", "Glacis", "", 0.0, 1.0, 0.55, Role::Free, "Longer, flatter glacis: more effective front armour, less room inside."));
        v.push(param("nose", "Nose", "", 0.0, 1.0, 0.5, Role::Free, "Blunt to pointed. Pointed noses angle the front plates for side shots too."));
        v.push(param("stations", "Stations", "", 2.0, 8.0, 3.0, Role::Free, "Mounting stations along each side: wheels, legs, pods and rotor arms attach to them."));
        v
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (l, w, h) = (v["length_m"], v["width_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let (glacis, nose) = (v["glacis"], v["nose"]);
        let (yb, track_top) = clearance(h);
        let yt = yb + h;
        let lg = l * (0.15 + 0.3 * glacis);
        let nf = 1.0 - 0.5 * nose;
        // Side profile (z, y, width factor), front to back underneath, then back to front on top.
        let prof = [
            (-l / 2.0 + 0.6 * h, yb, 1.0),       // front bottom
            (l / 2.0, yb, 1.0),                  // rear bottom
            (l / 2.0 - 0.12 * h, yt, 0.9),       // rear top
            (-l / 2.0 + lg, yt, 0.9),            // top of the glacis
            (-l / 2.0, yb + 0.42 * h, nf),       // nose, upper edge
            (-l / 2.0 + 0.25 * h, yb + 0.1 * h, nf), // nose, lower edge
        ];
        let pt = |i: usize, s: f64| p(s * prof[i].2 * w / 2.0, prof[i].1, prof[i].0);
        let pts: Vec<V3> = [1.0, -1.0].iter().flat_map(|&s| (0..prof.len()).map(move |i| (i, s))).map(|(i, s)| pt(i, s)).collect();
        let inside = p(0.0, yb + 0.5 * h, 0.0);
        let mut shapes = vec![style::hull(pts, "steel", Slot::Primary, Some(ts), (0.4 * ts + 0.01 * w).min(0.08))];
        // Faces we decorate.
        let glacis_q: Quad = [pt(4, -1.0), pt(4, 1.0), pt(3, 1.0), pt(3, -1.0)];
        let nose_q: Quad = [pt(5, -1.0), pt(5, 1.0), pt(4, 1.0), pt(4, -1.0)];
        let top_q: Quad = [pt(3, -1.0), pt(3, 1.0), pt(2, 1.0), pt(2, -1.0)];
        let side = |s: f64| -> Quad { [pt(0, s), pt(1, s), pt(2, s), pt(3, s)] };
        let rear_q: Quad = [pt(1, -1.0), pt(1, 1.0), pt(2, 1.0), pt(2, -1.0)];
        // Front armour beyond the side thickness, behind the glacis and the nose.
        for q in [&glacis_q, &nose_q] {
            if let Some(s) = inner_slab(q, inside, ts - 0.002, tf, 0.12, "hard_steel") {
                shapes.push(s);
            }
        }
        // Design language: one bold chevron across the glacis, glowing eyes in the nose, a glow line along each
        // flank above the tracks, layered side plates, heat vents on the rear deck, tail lights.
        let t = (0.012 * w).max(0.012);
        shapes.push(style::plate(&glacis_q, inside, (0.0, 1.0), (0.42, 0.62), t, 0.4 * t, "fittings", Slot::Trim));
        for s in [-0.32, 0.32] {
            let e = style::at(&nose_q, 0.5 + s, 0.55);
            style::eye(e, style::normal(&nose_q, inside), 0.045 * w.min(3.0), &mut shapes);
        }
        let v_track = ((track_top - yb) / h).clamp(0.0, 0.9);
        for s in [-1.0, 1.0] {
            let q = side(s);
            let n = style::normal(&q, inside);
            shapes.push(style::glow_line(style::at(&q, 0.1, 0.9), style::at(&q, 0.9, 0.9), n, 0.012 + 0.004 * w));
            for (u0, u1) in [(0.08, 0.47), (0.53, 0.92)] {
                shapes.push(style::plate(&q, inside, (u0, u1), (v_track + 0.06, 0.8), 1.5 * t, 1.5 * t, "fittings", Slot::Secondary));
            }
        }
        // Seen from above (the usual camera), the deck carries the two-tone: Secondary strips along both edges.
        for u in [(0.0, 0.2), (0.8, 1.0)] {
            shapes.push(style::plate(&top_q, inside, u, (0.0, 1.0), t, t, "fittings", Slot::Secondary));
        }
        style::vents(&top_q, inside, (0.28, 0.72), (0.72, 0.95), 4, t, &mut shapes);
        shapes.push(style::plate(&top_q, inside, (0.12, 0.36), (0.08, 0.2), t, t, "fittings", Slot::Secondary));
        for u in [(0.06, 0.2), (0.8, 0.94)] {
            shapes.push(style::plate(&rear_q, inside, u, (0.55, 0.75), t, 0.0, "fittings", Slot::Trim));
        }

        let z_ring = -l / 2.0 + lg + 0.4 * (l - lg - 0.12 * h);
        let s_y = track_top / 2.0;
        let stations = v["stations"].round().clamp(2.0, 8.0) as usize;
        let ch = Chassis { z0: -l / 2.0 + 0.12 * l, z1: l / 2.0 - 0.04 * l, y_under: yb, y_side: s_y, track_top, half_w: w / 2.0, width: w, stations, gear: Some((0.02 * l, 0.94 * l)) };
        let mut sockets = vec![
            socket("turret", SocketKind::TurretRing, p(0.0, yt, z_ring), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", 0.8 * 0.9 * w)]),
            socket(
                "engine",
                SocketKind::Internal,
                p(0.0, yb + ts + 0.01, l / 2.0 - 0.22 * l),
                p(0.0, 1.0, 0.0),
                p(0.0, 0.0, -1.0),
                &[("ctx.bay_length", 0.35 * l), ("ctx.bay_width", 0.8 * w), ("ctx.bay_height", 0.85 * h)],
            ),
            socket("mast_1", SocketKind::Mast, p(-0.32 * w, yt, l / 2.0 - 0.2 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.5 * w)]),
            socket("mast_2", SocketKind::Mast, p(0.32 * w, yt, l / 2.0 - 0.2 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.5 * w)]),
        ];
        sockets.extend(mounts::running_gear(&ch));
        hull_part(format!("hull_lancer_{l:.1}m"), format!("Lancer hull {l:.1} m"), size_class(l), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        hull_stats(v, built)
    }
}

// ------------------------------------------------------------------------------------------------ Bastion

pub struct Bastion;

impl Family for Bastion {
    fn id(&self) -> &'static str {
        "hull_bastion"
    }
    fn name(&self) -> &'static str {
        "Bastion hull"
    }
    fn params(&self) -> Vec<Param> {
        let mut v = common_params((4.0, 16.0, 7.0), (2.0, 6.0, 2.8), (1.2, 3.5, 1.8));
        v.push(param("turrets", "Turrets", "", 1.0, 2.0, 1.0, Role::Free, "One central turret, or two on the long roof."));
        v.push(param("sponson", "Sponsons", "", 0.0, 1.0, 0.7, Role::Free, "How far the upper body overhangs the tracks: more room inside, a wider target."));
        v.push(param("stations", "Stations", "", 2.0, 8.0, 4.0, Role::Free, "Mounting stations along each side: wheels, legs, pods and rotor arms attach to them."));
        v
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (l, w, h) = (v["length_m"], v["width_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let over = 0.75 * v["sponson"];
        let turrets = v["turrets"].round().clamp(1.0, 2.0) as usize;
        let (yb, _) = clearance(h);
        let y_mid = yb + 0.5 * h;
        let yt = yb + h;
        let wu = w + 2.0 * over;
        // Lower hull: a box with a sloped lower front.
        let mut lower = Vec::new();
        for s in [-1.0, 1.0] {
            let x = s * w / 2.0;
            lower.extend([p(x, yb, -l / 2.0 + 0.35 * h), p(x, yb, l / 2.0), p(x, y_mid, l / 2.0), p(x, y_mid, -l / 2.0)]);
        }
        // Upper body: wider, with a steep front and chamfered upper edges.
        let mut upper = Vec::new();
        for s in [-1.0, 1.0] {
            let (xo, xi) = (s * wu / 2.0, s * (wu / 2.0 - 0.18 * h));
            upper.extend([
                p(xo, y_mid, -l / 2.0),
                p(xo, y_mid, l / 2.0 - 0.02 * l),
                p(xi, yt, l / 2.0 - 0.06 * l),
                p(xi, yt, -l / 2.0 + 0.5 * h),
            ]);
        }
        let inside = p(0.0, y_mid, 0.0);
        let ch = (0.4 * ts + 0.012 * w).min(0.1);
        let mut shapes = vec![
            style::hull(lower.clone(), "steel", Slot::Secondary, Some(ts), ch),
            style::hull(upper.clone(), "steel", Slot::Primary, Some(ts), ch),
        ];
        let front_up: Quad = [upper[4], upper[0], upper[3], upper[7]];
        let front_low: Quad = [lower[4], lower[0], lower[3], lower[7]];
        let roof: Quad = [upper[7], upper[3], upper[2], upper[6]];
        let side_up = |s: usize| -> Quad {
            let o = if s == 0 { 4 } else { 0 };
            [upper[o], upper[o + 1], upper[o + 2], upper[o + 3]]
        };
        for q in [&front_up, &front_low] {
            if let Some(slab) = inner_slab(q, inside, ts - 0.002, tf, 0.1, "hard_steel") {
                shapes.push(slab);
            }
        }
        // Design language: trim bands on the upper front corners, a row of glowing vision slits along each
        // sponson, headlight eyes low on the front, a two-tone roof and heat vents at the back.
        let t = (0.012 * w).max(0.012);
        for u in [(0.0, 0.14), (0.86, 1.0)] {
            shapes.push(style::plate(&front_up, inside, u, (0.1, 0.9), t, 0.0, "fittings", Slot::Trim));
        }
        for s in 0..2 {
            let q = side_up(s);
            let n = style::normal(&q, inside);
            for k in 0..5 {
                let u = 0.18 + 0.12 * k as f64;
                shapes.push(style::plate(&q, inside, (u, u + 0.07), (0.55, 0.68), 0.6 * t, 0.0, "glass", Slot::Glow));
            }
            shapes.push(style::glow_line(style::at(&q, 0.05, 0.08), style::at(&q, 0.95, 0.08), n, 0.012 + 0.004 * w));
        }
        for u in [0.25, 0.75] {
            style::eye(style::at(&front_low, u, 0.6), style::normal(&front_low, inside), 0.04 * w.min(4.0), &mut shapes);
        }
        shapes.push(style::plate(&roof, inside, (0.0, 1.0), (0.0, 0.16), t, t, "fittings", Slot::Secondary));
        style::vents(&roof, inside, (0.25, 0.75), (0.82, 0.97), 5, t, &mut shapes);

        let track_top = y_mid - 0.04;
        let s_y = track_top / 2.0;
        let stations = v["stations"].round().clamp(2.0, 8.0) as usize;
        let ring_max = 0.8 * (wu - 0.36 * h);
        let roof_z0 = -l / 2.0 + 0.5 * h;
        let roof_z1 = l / 2.0 - 0.06 * l;
        let turret_z: Vec<f64> = if turrets == 1 { vec![roof_z0 + 0.45 * (roof_z1 - roof_z0)] } else { vec![roof_z0 + 0.27 * (roof_z1 - roof_z0), roof_z0 + 0.73 * (roof_z1 - roof_z0)] };
        let mut sockets: Vec<SocketDef> = turret_z
            .iter()
            .enumerate()
            .map(|(i, z)| socket(&format!("turret_{}", i + 1), SocketKind::TurretRing, p(0.0, yt, *z), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", ring_max)]))
            .collect();
        sockets.push(socket("engine", SocketKind::Internal, p(0.0, yb + ts + 0.01, 0.3 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.bay_length", 0.3 * l), ("ctx.bay_width", 0.85 * w), ("ctx.bay_height", 0.9 * h)]));
        let mast_z = roof_z1 - 0.1 * l;
        sockets.push(socket("mast_1", SocketKind::Mast, p(-0.3 * (wu - 0.36 * h), yt, mast_z), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.5 * w)]));
        sockets.push(socket("mast_2", SocketKind::Mast, p(0.3 * (wu - 0.36 * h), yt, mast_z), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.5 * w)]));
        let ch = Chassis { z0: -l / 2.0 + 0.1 * l, z1: l / 2.0 - 0.03 * l, y_under: yb, y_side: s_y, track_top, half_w: w / 2.0, width: w, stations, gear: Some((0.0, 0.92 * l)) };
        sockets.extend(mounts::running_gear(&ch));
        hull_part(format!("hull_bastion_{l:.1}m"), format!("Bastion hull {l:.1} m"), size_class(l), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        hull_stats(v, built)
    }
}

// ------------------------------------------------------------------------------------------------ Dreadnought

pub struct Dreadnought;

impl Family for Dreadnought {
    fn id(&self) -> &'static str {
        "hull_dreadnought"
    }
    fn name(&self) -> &'static str {
        "Dreadnought hull"
    }
    fn params(&self) -> Vec<Param> {
        let mut v = common_params((10.0, 60.0, 22.0), (3.0, 10.0, 6.5), (1.0, 4.0, 2.6));
        v.push(param("turrets", "Turrets", "", 1.0, 6.0, 3.0, Role::Free, "Turret rings along the deck, either side of the command tower."));
        v.push(param("stations", "Stations", "", 2.0, 10.0, 4.0, Role::Free, "Mounting stations along each side (and rail bogie slots under the keel): more of them spread the load."));
        v.push(param("prow", "Prow", "", 0.0, 1.0, 1.0, Role::Free, "From a flat, train-like front (0) to a ship's prow (1)."));
        v
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (l, w, h) = (v["length_m"], v["width_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let turrets = v["turrets"].round().clamp(1.0, 6.0) as usize;
        let stations = v["stations"].round().clamp(2.0, 10.0) as usize;
        let prow = v["prow"].clamp(0.0, 1.0);
        let (yb, track_top) = clearance(h);
        let yt = yb + h;
        let bow = (0.04 + 0.12 * prow) * l;
        let nf = 1.0 - 0.7 * prow;
        // Plan: a ship-like prow tapering to a third of the width, a slightly narrowed stern.
        let prof = [
            (-l / 2.0, yb + 0.25 * h, nf),     // prow, low
            (-l / 2.0, yb + 0.65 * h, nf),     // prow, high
            (-l / 2.0 + bow, yt, 1.0),         // deck starts
            (l / 2.0 - 0.04 * l, yt, 0.9),     // deck ends
            (l / 2.0, yb + 0.3 * h, 0.85),     // stern
            (-l / 2.0 + bow, yb, 1.0),         // keel forward
            (l / 2.0 - 0.02 * l, yb, 0.9),     // keel aft
        ];
        let pt = |i: usize, s: f64| p(s * prof[i].2 * w / 2.0, prof[i].1, prof[i].0);
        let pts: Vec<V3> = [1.0, -1.0].iter().flat_map(|&s| (0..prof.len()).map(move |i| (i, s))).map(|(i, s)| pt(i, s)).collect();
        let inside = p(0.0, yb + 0.5 * h, 0.0);
        let mut shapes = vec![style::hull(pts, "steel", Slot::Primary, Some(ts), (0.4 * ts + 0.01 * w).min(0.15))];
        let bow_q: Quad = [pt(1, -1.0), pt(1, 1.0), pt(2, 1.0), pt(2, -1.0)];
        let deck_q: Quad = [pt(2, -1.0), pt(2, 1.0), pt(3, 1.0), pt(3, -1.0)];
        let side = |s: f64| -> Quad { [pt(5, s), pt(6, s), pt(3, s), pt(2, s)] };
        if let Some(slab) = inner_slab(&bow_q, inside, ts - 0.002, tf, 0.12, "hard_steel") {
            shapes.push(slab);
        }
        let t = (0.01 * w).max(0.015);
        // Command tower amidships, slightly aft, with a glowing bridge band and a sensor mast.
        let z_tower = 0.08 * l;
        let (tw, th, tl) = (0.42 * w, 0.75 * h, (0.1 * l).max(2.5 * h * 0.6));
        let tower = vec![
            p(-tw / 2.0, yt, z_tower - tl / 2.0),
            p(tw / 2.0, yt, z_tower - tl / 2.0),
            p(-tw / 2.0, yt, z_tower + tl / 2.0),
            p(tw / 2.0, yt, z_tower + tl / 2.0),
            p(-0.38 * tw, yt + th, z_tower - 0.3 * tl),
            p(0.38 * tw, yt + th, z_tower - 0.3 * tl),
            p(-0.38 * tw, yt + th, z_tower + 0.45 * tl),
            p(0.38 * tw, yt + th, z_tower + 0.45 * tl),
        ];
        let tower_front: Quad = [tower[0], tower[1], tower[5], tower[4]];
        shapes.push(style::hull(tower.clone(), "steel", Slot::Secondary, Some(ts * 0.6), (0.3 * ts + 0.01 * w).min(0.1)));
        let tin = p(0.0, yt + th / 2.0, z_tower);
        shapes.push(style::plate(&tower_front, tin, (0.08, 0.92), (0.62, 0.8), t, 0.0, "glass", Slot::Glow));
        shapes.push(style::plate(&tower_front, tin, (0.0, 1.0), (0.2, 0.32), t, 0.0, "fittings", Slot::Trim));
        let mast_base = p(0.0, yt + th, z_tower + 0.1 * tl);
        // Bow chevron and eyes, deck-edge glow lines, darker lower flanks, stern vents.
        shapes.push(style::plate(&bow_q, inside, (0.0, 1.0), (0.3, 0.55), t, t, "fittings", Slot::Trim));
        for u in [0.3, 0.7] {
            style::eye(style::at(&bow_q, u, 0.8), style::normal(&bow_q, inside), 0.035 * w, &mut shapes);
        }
        for s in [-1.0, 1.0] {
            let q = side(s);
            let n = style::normal(&q, inside);
            shapes.push(style::glow_line(style::at(&q, 0.03, 0.94), style::at(&q, 0.97, 0.94), n, 0.012 + 0.004 * w));
            shapes.push(style::plate(&q, inside, (0.0, 1.0), (((track_top - yb) / h) + 0.05, 0.7), 1.5 * t, 1.5 * t, "fittings", Slot::Secondary));
        }
        style::vents(&deck_q, inside, (0.3, 0.7), (0.9, 0.985), 4, t, &mut shapes);

        // Turret rings along the deck, keeping clear of the tower.
        let ring_max = 0.85 * w;
        let (z0, z1) = (-l / 2.0 + bow + 0.06 * l, l / 2.0 - 0.1 * l);
        let gap = (z_tower - tl / 2.0 - 0.5 * ring_max, z_tower + tl / 2.0 + 0.5 * ring_max);
        let usable = (gap.0 - z0).max(0.0) + (z1 - gap.1).max(0.0);
        let mut turret_z = Vec::new();
        for k in 0..turrets {
            let mut d = usable * (k as f64 + 0.5) / turrets as f64;
            let fore = (gap.0 - z0).max(0.0);
            turret_z.push(if d < fore { z0 + d } else {
                d -= fore;
                gap.1 + d
            });
        }
        let mut sockets: Vec<SocketDef> = turret_z
            .iter()
            .enumerate()
            .map(|(i, z)| socket(&format!("turret_{}", i + 1), SocketKind::TurretRing, p(0.0, yt, *z), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", ring_max)]))
            .collect();
        // Running gear along the sides and keel, between the bow and the stern.
        let (k0, k1) = (-l / 2.0 + bow, l / 2.0 - 0.02 * l);
        let s_y = track_top / 2.0;
        let ch = Chassis { z0: k0, z1: k1, y_under: yb, y_side: s_y, track_top, half_w: 0.9 * w / 2.0, width: w, stations, gear: Some((0.5 * (k0 + k1), 0.94 * (k1 - k0))) };
        sockets.extend(mounts::running_gear(&ch));
        sockets.push(socket("engine", SocketKind::Internal, p(0.0, yb + ts + 0.01, 0.3 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.bay_length", 0.25 * l), ("ctx.bay_width", 0.8 * w), ("ctx.bay_height", 0.9 * h)]));
        sockets.push(socket("mast_1", SocketKind::Mast, mast_base, p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * tw)]));
        sockets.push(socket("mast_2", SocketKind::Mast, p(0.0, yt, l / 2.0 - 0.1 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * w)]));
        hull_part(format!("hull_dreadnought_{l:.0}m"), format!("Dreadnought hull {l:.0} m"), size_class(l), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        hull_stats(v, built)
    }
}

// ------------------------------------------------------------------------------------------------ Strider

/// A round, armoured carapace on a ring of hips: the body of walkers (and, on its belly, hover or anti-gravity craft).
/// It has no tracks or rails; its hips take legs, pods and rotor arms.
pub struct Strider;

fn ring(radius: f64, y: f64, n: usize) -> Vec<V3> {
    (0..n)
        .map(|k| {
            let phi = (k as f64 + 0.5) / n as f64 * std::f64::consts::TAU;
            p(radius * phi.sin(), y, -radius * phi.cos())
        })
        .collect()
}

impl Family for Strider {
    fn id(&self) -> &'static str {
        "hull_strider"
    }
    fn name(&self) -> &'static str {
        "Strider hull"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            Param { id: "radius_m", name: "Radius", unit: "m", min: 0.4, max: 14.0, default: 1.6, scale: Scale::Log, role: Role::Budgeted, help: "Body radius. A wide body takes big turret rings and engines." },
            Param { id: "height_m", name: "Height", unit: "m", min: 0.3, max: 8.0, default: 1.1, scale: Scale::Log, role: Role::Budgeted, help: "Body height: room inside against a bigger target." },
            param("front_mm", "Front armour", "mm", 5.0, 250.0, 60.0, Role::Budgeted, "Plate thickness on the front facets."),
            param("side_mm", "Side armour", "mm", 5.0, 150.0, 30.0, Role::Budgeted, "Plate thickness everywhere else."),
            param("stations", "Leg pairs", "", 2.0, 5.0, 4.0, Role::Free, "Hips on each side: 4 to 10 legs."),
            param("head", "Sensor head", "", 0.0, 1.0, 0.6, Role::Free, "How far the sensor head juts out at the front."),
            param("dome", "Dome", "", 0.0, 1.0, 0.5, Role::Free, "A flat shield or a tall dome: the dome has more room and a bigger shadow."),
        ]
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (r, h) = (v["radius_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let pairs = v["stations"].round().clamp(2.0, 5.0) as usize;
        let (head, dome) = (v["head"], v["dome"]);
        let (yb, _) = clearance(h);
        let n = 12usize;
        let (y_wst, y_sh) = (yb + 0.38 * h, yb + 0.7 * h);
        let y_top = yb + h * (0.88 + 0.12 * dome);
        let r_top = (0.62 - 0.3 * dome) * r;
        let bot = ring(0.72 * r, yb, n);
        let wst = ring(r, y_wst, n);
        let sh = ring(0.84 * r, y_sh, n);
        let top = ring(r_top, y_top, n);
        let pts: Vec<V3> = bot.iter().chain(&wst).chain(&sh).chain(&top).cloned().collect();
        let inside = p(0.0, yb + 0.5 * h, 0.0);
        let ch = (0.4 * ts + 0.01 * r).min(0.08);
        let mut shapes = vec![style::hull(pts, "steel", Slot::Primary, Some(ts), ch)];
        // Front armour: slabs behind the front facets of the upper body.
        let front_up: Quad = [wst[n - 1], wst[0], sh[0], sh[n - 1]];
        let front_cap: Quad = [sh[n - 1], sh[0], top[0], top[n - 1]];
        for q in [&front_up, &front_cap] {
            if let Some(slab) = inner_slab(q, inside, ts - 0.002, tf, 0.12, "hard_steel") {
                shapes.push(slab);
            }
        }
        // Design language: a dark-blue lower belt, a glowing waist, a chevron, a sensor head with eyes, vents.
        let t = (0.012 * r).max(0.01);
        for k in 0..n {
            let q: Quad = [bot[k], bot[(k + 1) % n], wst[(k + 1) % n], wst[k]];
            shapes.push(style::plate(&q, inside, (0.0, 1.0), (0.0, 1.0), t, t, "fittings", Slot::Secondary));
            let mid = (wst[k] + wst[(k + 1) % n]) * 0.5;
            let radial = p(mid.x, 0.0, mid.z).norm();
            shapes.push(style::glow_line(wst[k] + radial * (0.4 * t), wst[(k + 1) % n] + radial * (0.4 * t), radial, 0.012 + 0.004 * r));
        }
        shapes.push(style::plate(&front_up, inside, (0.05, 0.95), (0.4, 0.62), t, 0.5 * t, "fittings", Slot::Trim));
        let rear_cap: Quad = [sh[n / 2 - 1], sh[n / 2], top[n / 2], top[n / 2 - 1]];
        style::vents(&rear_cap, inside, (0.25, 0.75), (0.15, 0.85), 4, t, &mut shapes);
        // Sensor head: a tapered block at the front with three pairs of eyes.
        let hz = -(r + 0.04 * r + 0.22 * r * head);
        let (hw, hh) = (0.34 * r, 0.3 * h);
        let hy = yb + 0.5 * h;
        let hpts = vec![
            p(-hw / 2.0, hy - hh / 2.0, hz),
            p(hw / 2.0, hy - hh / 2.0, hz),
            p(-hw / 2.0, hy + hh / 2.0, hz),
            p(hw / 2.0, hy + hh / 2.0, hz),
            p(-hw, hy - hh * 0.6, -0.8 * r),
            p(hw, hy - hh * 0.6, -0.8 * r),
            p(-hw, hy + hh * 0.75, -0.8 * r),
            p(hw, hy + hh * 0.75, -0.8 * r),
        ];
        shapes.push(style::hull(hpts, "steel", Slot::Secondary, Some(ts.max(0.01)), (0.5 * ts).min(0.05)));
        let face = p(0.0, 0.0, -1.0);
        for (dx, dy, er) in [(0.26, 0.22, 0.13), (0.12, -0.05, 0.09), (0.32, -0.2, 0.06)] {
            for s in [-1.0, 1.0] {
                style::eye(p(s * dx * hw, hy + dy * hh, hz), face, er * hh.min(hw), &mut shapes);
            }
        }
        // Hip ring under the waist.
        shapes.push(style::cyl(0.62 * r, 0.12 * h, crate::schema::Axis::Y, 12, 1.0, p(0.0, yb + 0.02 * h, 0.0), "machinery", Slot::Dark, None, 0.0));

        // Sockets: hips on the waist, a turret on top, masts behind it, the engine and the belly.
        let y_hip = y_wst - 0.05 * h;
        let r_hip = 0.97 * r;
        let spacing = 2.0 * r * ((154.0f64 - 26.0).to_radians() / (pairs as f64 - 1.0) / 2.0).sin();
        let mut sockets: Vec<SocketDef> = Vec::new();
        for i in 0..pairs {
            let th = (26.0 + 128.0 * i as f64 / (pairs as f64 - 1.0)).to_radians();
            for (name, side) in [("r", 1.0), ("l", -1.0)] {
                let nrm = p(side * th.sin(), 0.0, -th.cos());
                // Tangent to the ring, pointing forward: the leg's plane is the vertical plane through `nrm`.
                let fwd = p(-side * th.cos(), 0.0, -th.sin());
                sockets.push(socket(
                    &format!("station_{name}{}", i + 1),
                    SocketKind::Hip,
                    p(r_hip * nrm.x, y_hip, r_hip * nrm.z),
                    nrm,
                    fwd,
                    &[("ctx.hip_height", y_hip), ("ctx.spacing", spacing), ("ctx.length", 0.9 * spacing), ("ctx.hull_width", 2.0 * r), ("ctx.stations", pairs as f64)],
                ));
            }
        }
        let roof_y = |rho: f64| y_top - (rho - r_top).max(0.0) / (0.84 * r - r_top).max(1e-6) * (y_top - y_sh);
        sockets.push(socket("turret_1", SocketKind::TurretRing, p(0.0, y_top, 0.04 * r), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", 1.8 * r_top)]));
        sockets.push(socket("mast_1", SocketKind::Mast, p(-0.3 * r, roof_y(0.5 * r), 0.42 * r), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * r)]));
        sockets.push(socket("mast_2", SocketKind::Mast, p(0.3 * r, roof_y(0.5 * r), 0.42 * r), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * r)]));
        sockets.push(socket(
            "engine",
            SocketKind::Internal,
            p(0.0, yb + ts + 0.01, 0.05 * r),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, -1.0),
            &[("ctx.bay_length", 1.0 * r), ("ctx.bay_width", 1.0 * r), ("ctx.bay_height", 0.7 * h)],
        ));
        sockets.push(socket("belly", SocketKind::Belly, p(0.0, yb, 0.0), p(0.0, -1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.length", 1.4 * r), ("ctx.width", 1.4 * r), ("ctx.hip_height", yb)]));
        hull_part(format!("hull_strider_{r:.1}m"), format!("Strider hull {r:.1} m"), size_class(2.0 * r), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let mut s = hull_stats(v, built);
        s.pop();
        s
    }
}

// ------------------------------------------------------------------------------------------------ Skiff

/// A flat-bottomed craft with an arrowhead bow and a raised cabin: the body of air-cushion boats, rotor gunships and
/// anti-gravity cruisers. It rides low and light, with a wide underside for cushions and plates, side stations for
/// pods, rotor arms, legs or wheels, and a foredeck ring for one turret.
pub struct Skiff;

impl Family for Skiff {
    fn id(&self) -> &'static str {
        "hull_skiff"
    }
    fn name(&self) -> &'static str {
        "Skiff hull"
    }
    fn params(&self) -> Vec<Param> {
        let mut v = common_params((3.0, 60.0, 9.0), (1.4, 24.0, 3.4), (0.4, 7.0, 1.1));
        v.push(param("sweep", "Sweep", "", 0.0, 1.0, 0.6, Role::Free, "A blunt barge bow to a sharp arrowhead: a sharper bow cuts the frontal area and pushes the shoulders aft."));
        v.push(param("cabin", "Cabin", "", 0.0, 1.0, 0.5, Role::Free, "A flat deck or a tall central cabin: room and eye height against a taller target."));
        v.push(param("stations", "Stations", "", 2.0, 6.0, 3.0, Role::Free, "Mounting stations along each side: wheels, legs, pods and rotor arms attach to them."));
        v
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (l, w, h) = (v["length_m"], v["width_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let (sweep, cabin) = (v["sweep"], v["cabin"]);
        let yb = 0.1 + 0.1 * h;
        let yt = yb + h;
        let yd = yb + h * (0.62 - 0.3 * cabin);
        let hw = 0.5 * w;
        let z_nose = -l / 2.0;
        let z_sh = z_nose + l * (0.5 - 0.3 * sweep);
        let z_tail = l / 2.0;
        let nw = hw * (0.03 + 0.34 * (1.0 - sweep));
        let y_bow = yb + 0.72 * (yd - yb);
        // Hull body: a flat underside, a raked foredeck and parallel sides aft of the shoulders.
        let mut pts = Vec::new();
        for s in [-1.0, 1.0] {
            pts.push(p(s * 0.9 * nw, yb, z_nose));
            pts.push(p(s * 0.9 * hw, yb, z_sh));
            pts.push(p(s * 0.9 * hw, yb, z_tail));
            pts.push(p(s * nw, y_bow, z_nose));
            pts.push(p(s * hw, yd, z_sh));
            pts.push(p(s * hw, yd, z_tail));
        }
        let inside = p(0.0, yb + 0.4 * (yd - yb), 0.0);
        let ch = (0.4 * ts + 0.01 * w).min(0.08);
        let mut shapes = vec![style::hull(pts, "composite", Slot::Primary, Some(ts), ch)];
        let fore_q: Quad = [p(-nw, y_bow, z_nose), p(nw, y_bow, z_nose), p(hw, yd, z_sh), p(-hw, yd, z_sh)];
        let aft_q: Quad = [p(-hw, yd, z_sh), p(hw, yd, z_sh), p(hw, yd, z_tail), p(-hw, yd, z_tail)];
        if let Some(s) = inner_slab(&fore_q, inside, ts - 0.002, tf, 0.1, "composite") {
            shapes.push(s);
        }
        // The cabin: a raked, tapering block amidships.
        let z_c0 = z_nose + l * 0.52;
        let z_c1 = z_tail - 0.14 * l;
        let (cb0, cb1) = (0.6 * hw, 0.4 * hw);
        let rake = 0.7 * (yt - yd);
        let cpts = vec![
            p(-cb0, yd - 0.02, z_c0),
            p(cb0, yd - 0.02, z_c0),
            p(-cb0, yd - 0.02, z_c1),
            p(cb0, yd - 0.02, z_c1),
            p(-cb1, yt, z_c0 + rake),
            p(cb1, yt, z_c0 + rake),
            p(-0.92 * cb1, yt, z_c1 - 0.05 * l),
            p(0.92 * cb1, yt, z_c1 - 0.05 * l),
        ];
        shapes.push(style::hull(cpts, "composite", Slot::Primary, Some(ts.max(0.01)), (0.4 * ts).min(0.05)));
        let c_inside = p(0.0, 0.5 * (yd + yt), 0.5 * (z_c0 + z_c1));
        let wind_q: Quad = [p(-cb0, yd, z_c0), p(cb0, yd, z_c0), p(cb1, yt, z_c0 + rake), p(-cb1, yt, z_c0 + rake)];
        let t = (0.012 * w).max(0.01);
        // Design language: a glowing visor on the cabin, a chevron on the foredeck, Secondary strips along the
        // aft deck, glow lines along the deck edges, vents behind the cabin, and two swept tail fins.
        shapes.push(style::plate(&wind_q, c_inside, (0.08, 0.92), (0.42, 0.85), 0.6 * t, 0.0, "glass", Slot::Glow));
        shapes.push(style::plate(&wind_q, c_inside, (0.0, 1.0), (0.0, 0.3), t, 0.5 * t, "fittings", Slot::Secondary));
        shapes.push(style::plate(&fore_q, inside, (0.2, 0.8), (0.3, 0.5), t, 0.4 * t, "fittings", Slot::Trim));
        shapes.push(style::plate(&fore_q, inside, (0.28, 0.72), (0.56, 0.74), t, 0.4 * t, "fittings", Slot::Secondary));
        for u in [(0.0, 0.14), (0.86, 1.0)] {
            shapes.push(style::plate(&aft_q, inside, u, (0.0, 1.0), t, t, "fittings", Slot::Secondary));
        }
        for u in [0.03, 0.97] {
            shapes.push(style::glow_line(style::at(&fore_q, u, 0.04), style::at(&fore_q, u, 0.96), p(0.0, 1.0, 0.0), 0.012 + 0.004 * w));
            shapes.push(style::glow_line(style::at(&aft_q, u, 0.02), style::at(&aft_q, u, 0.98), p(0.0, 1.0, 0.0), 0.012 + 0.004 * w));
        }
        let vz = |z: f64| (z - z_sh) / (z_tail - z_sh);
        style::vents(&aft_q, inside, (0.3, 0.7), (vz(z_c1) + 0.05, 0.97), 4, t, &mut shapes);
        let (fa, fb) = (z_tail - 0.3 * l, z_tail - 0.02 * l);
        let fy = yt + 0.45 * (yt - yd);
        for s in [-1.0, 1.0] {
            let (x0, x1) = (s * 0.52 * hw, s * (0.52 * hw + 0.05 * w + 0.02));
            let (x2, x3) = (s * (0.52 * hw + 0.18 * w), s * (0.52 * hw + 0.18 * w + 0.04 * w));
            let fin = vec![p(x0, yd, fa), p(x1, yd, fa), p(x0, yd, fb), p(x1, yd, fb), p(x2, fy, fb - 0.02 * l), p(x3, fy, fb - 0.02 * l)];
            shapes.push(style::hull(fin, "composite", Slot::Secondary, None, 0.0));
            shapes.push(style::beam(p(x2, fy, fb - 0.02 * l), p(x2, fy, fb - 0.02 * l + 0.012), [0.05, 0.05], p(0.0, 1.0, 0.0), "fittings", Slot::Glow, 0.0));
        }

        // Sockets.
        let z_ring = z_nose + 0.26 * l;
        let f_ring = ((z_ring - z_nose) / (z_sh - z_nose)).clamp(0.0, 1.0);
        let w_ring = 2.0 * (nw + (hw - nw) * f_ring);
        let y_ring = y_bow + (yd - y_bow) * f_ring;
        let ring_max = (0.72 * w_ring).min(1.7 * (z_c0 - z_ring));
        let n_st = v["stations"].round().clamp(2.0, 6.0) as usize;
        let y_side = yb + 0.35 * (yd - yb);
        let half_w = hw * (0.9 + 0.1 * 0.35);
        let ch = Chassis { z0: z_sh + 0.04 * l, z1: z_tail - 0.03 * l, y_under: yb, y_side, track_top: yd, half_w, width: 0.9 * w, stations: n_st, gear: None };
        let mut sockets = vec![
            socket("turret", SocketKind::TurretRing, p(0.0, y_ring, z_ring), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", ring_max)]),
            socket(
                "engine",
                SocketKind::Internal,
                p(0.0, yb + ts + 0.01, 0.5 * (z_c0 + z_c1) + 0.08 * l),
                p(0.0, 1.0, 0.0),
                p(0.0, 0.0, -1.0),
                &[("ctx.bay_length", 0.3 * l), ("ctx.bay_width", 0.58 * w), ("ctx.bay_height", 0.8 * (yt - yb))],
            ),
            socket("hub", SocketKind::Mast, p(0.0, yt, 0.5 * (z_c0 + rake + z_c1 - 0.05 * l)), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.6 * w), ("ctx.top", 0.0)]),
            socket("mast_1", SocketKind::Mast, p(-0.55 * cb1, yt, z_c1 - 0.18 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * w)]),
            socket("mast_2", SocketKind::Mast, p(0.55 * cb1, yt, z_c1 - 0.18 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.mast_max", 0.4 * w)]),
        ];
        let mut run = mounts::running_gear(&ch);
        // The cushion or plate covers the underside from the bow's quarter point aft, an equal-area rectangle.
        let area = 0.5 * (2.0 * nw + 2.0 * hw) * 0.9 * (z_sh - z_nose) + 2.0 * hw * 0.9 * (z_tail - z_sh);
        let (b_w, b_l) = (1.8 * hw * 0.97, area / (1.8 * hw));
        for s in run.iter_mut() {
            if s.name == "belly" {
                s.at = p(0.0, yb, z_tail - 0.5 * b_l).arr();
                s.hints.insert("ctx.length".into(), b_l);
                s.hints.insert("ctx.width".into(), b_w);
            }
        }
        sockets.extend(run);
        hull_part(format!("hull_skiff_{l:.1}m"), format!("Skiff hull {l:.1} m"), size_class(l), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let mut s = hull_stats(v, built);
        s.pop();
        s
    }
}
