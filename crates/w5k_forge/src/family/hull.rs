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

use std::collections::BTreeMap;

use super::style::{self, p, Quad};
use super::{stat, Family, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{Category, Function, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
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

fn socket(name: &str, kind: SocketKind, at: V3, normal: V3, forward: V3, hints: &[(&str, f64)]) -> SocketDef {
    SocketDef {
        name: name.into(),
        kind,
        size: SizeClass::Medium,
        at: at.arr(),
        normal: normal.arr(),
        forward: forward.arr(),
        hints: hints.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
    }
}

/// Extra front armour as a solid slab just behind a face: from `d1` to `d2` inward, shrunk toward the face's
/// centre so it stays inside the hull (it is invisible; only the armour rays and the mass see it).
fn inner_slab(q: &Quad, inside: V3, d1: f64, d2: f64, shrink: f64) -> Option<Node> {
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
    Some(style::hull(pts, "hard_steel", Slot::Primary, None, 0.0))
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
    let (l, w) = (v["length_m"], v["width_m"]);
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
        v
    }

    fn generate(&self, v: &Values) -> PartDef {
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
            if let Some(s) = inner_slab(q, inside, ts - 0.002, tf, 0.12) {
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
        let track_hints = [("ctx.length", 0.94 * l), ("ctx.top", track_top - s_y), ("ctx.bottom", s_y)];
        let x_side = w / 2.0;
        let sockets = vec![
            socket("turret", SocketKind::TurretRing, p(0.0, yt, z_ring), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", 0.8 * 0.9 * w)]),
            socket("track_r", SocketKind::Locomotion, p(x_side, s_y, 0.02 * l), p(1.0, 0.0, 0.0), p(0.0, 0.0, -1.0), &track_hints),
            socket("track_l", SocketKind::Locomotion, p(-x_side, s_y, 0.02 * l), p(-1.0, 0.0, 0.0), p(0.0, 0.0, -1.0), &track_hints),
            socket(
                "engine",
                SocketKind::Internal,
                p(0.0, yb + ts + 0.01, l / 2.0 - 0.22 * l),
                p(0.0, 1.0, 0.0),
                p(0.0, 0.0, -1.0),
                &[("ctx.bay_length", 0.35 * l), ("ctx.bay_width", 0.8 * w), ("ctx.bay_height", 0.85 * h)],
            ),
        ];
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
        v
    }

    fn generate(&self, v: &Values) -> PartDef {
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
            if let Some(slab) = inner_slab(q, inside, ts - 0.002, tf, 0.1) {
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
        let track_hints = [("ctx.length", 0.92 * l), ("ctx.top", track_top - s_y), ("ctx.bottom", s_y)];
        let ring_max = 0.8 * (wu - 0.36 * h);
        let roof_z0 = -l / 2.0 + 0.5 * h;
        let roof_z1 = l / 2.0 - 0.06 * l;
        let turret_z: Vec<f64> = if turrets == 1 { vec![roof_z0 + 0.45 * (roof_z1 - roof_z0)] } else { vec![roof_z0 + 0.27 * (roof_z1 - roof_z0), roof_z0 + 0.73 * (roof_z1 - roof_z0)] };
        let mut sockets: Vec<SocketDef> = turret_z
            .iter()
            .enumerate()
            .map(|(i, z)| socket(&format!("turret_{}", i + 1), SocketKind::TurretRing, p(0.0, yt, *z), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.ring_max", ring_max)]))
            .collect();
        sockets.push(socket("track_r", SocketKind::Locomotion, p(w / 2.0, s_y, 0.0), p(1.0, 0.0, 0.0), p(0.0, 0.0, -1.0), &track_hints));
        sockets.push(socket("track_l", SocketKind::Locomotion, p(-w / 2.0, s_y, 0.0), p(-1.0, 0.0, 0.0), p(0.0, 0.0, -1.0), &track_hints));
        sockets.push(socket("engine", SocketKind::Internal, p(0.0, yb + ts + 0.01, 0.3 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.bay_length", 0.3 * l), ("ctx.bay_width", 0.85 * w), ("ctx.bay_height", 0.9 * h)]));
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
        v.push(param("bogies", "Track sets", "", 2.0, 6.0, 3.0, Role::Free, "Track units per side. More sets spread the load and steer a long hull better."));
        v
    }

    fn generate(&self, v: &Values) -> PartDef {
        let (l, w, h) = (v["length_m"], v["width_m"], v["height_m"]);
        let (tf, ts) = (v["front_mm"] / 1000.0, v["side_mm"] / 1000.0);
        let turrets = v["turrets"].round().clamp(1.0, 6.0) as usize;
        let bogies = v["bogies"].round().clamp(2.0, 6.0) as usize;
        let (yb, track_top) = clearance(h);
        let yt = yb + h;
        let bow = 0.16 * l;
        // Plan: a ship-like prow tapering to a third of the width, a slightly narrowed stern.
        let prof = [
            (-l / 2.0, yb + 0.25 * h, 0.3),    // prow, low
            (-l / 2.0, yb + 0.65 * h, 0.3),    // prow, high
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
        if let Some(slab) = inner_slab(&bow_q, inside, ts - 0.002, tf, 0.12) {
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
        shapes.push(style::cyl(0.025 * w, 1.2 * h, crate::schema::Axis::Y, 8, 0.6, mast_base + p(0.0, 0.6 * h, 0.0), "steel", Slot::Dark, None, 0.0));
        shapes.push(style::sphere(0.04 * w, mast_base + p(0.0, 1.22 * h, 0.0), "glass", Slot::Glow));
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
        // Track sets along each side, under the deck between the bow and the stern.
        let (k0, k1) = (-l / 2.0 + bow, l / 2.0 - 0.02 * l);
        let seg = (k1 - k0) / bogies as f64;
        let s_y = track_top / 2.0;
        let hints = [("ctx.length", 0.94 * seg), ("ctx.top", track_top - s_y), ("ctx.bottom", s_y)];
        for b in 0..bogies {
            let z = k0 + seg * (b as f64 + 0.5);
            for (name, s) in [("r", 1.0), ("l", -1.0)] {
                sockets.push(socket(&format!("track_{name}{}", b + 1), SocketKind::Locomotion, p(s * 0.9 * w / 2.0, s_y, z), p(s, 0.0, 0.0), p(0.0, 0.0, -1.0), &hints));
            }
        }
        sockets.push(socket("engine", SocketKind::Internal, p(0.0, yb + ts + 0.01, 0.3 * l), p(0.0, 1.0, 0.0), p(0.0, 0.0, -1.0), &[("ctx.bay_length", 0.25 * l), ("ctx.bay_width", 0.8 * w), ("ctx.bay_height", 0.9 * h)]));
        hull_part(format!("hull_dreadnought_{l:.0}m"), format!("Dreadnought hull {l:.0} m"), size_class(l), shapes, sockets)
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        hull_stats(v, built)
    }
}
