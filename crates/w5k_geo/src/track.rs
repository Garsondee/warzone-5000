//! Tracked running gear: the belt path round the wheel circles, links tiled along it by arc length, and the wheels a track runs on.
//!
//! Seen from the side the belt is a rubber band stretched round the wheels: straight tangent segments between neighbours and an arc on every
//! wheel it bends round (docs/theory/geometry.md). All of it is in the hull frame, for the right-hand side: the left is its mirror.

use crate::hardware::cyl_x;
use crate::loft::{loft, Section};
use crate::mesh::Mesh;
use crate::module::{frame, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::{revolve, segments_for};
use serde::Deserialize;
use std::f64::consts::{PI, TAU};
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_contract::rig::WheelKind;
use w5k_math::{scalar, Transform, Vec3};

/// A wheel the belt runs on: its hub in the hull frame (z along the vehicle, y up), the radius (to the tyre's tip; the sprocket's is its pitch
/// radius) and its width.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunWheel {
    pub kind: WheelKind,
    pub z_m: f64,
    pub y_m: f64,
    pub radius_m: f64,
    pub width_m: f64,
}

/// One side's running gear: the wheels in loop order (the sprocket, the top run through the return rollers to the idler, then the ground run
/// through the road wheels back to the sprocket) and the belt.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSpec {
    pub track_x_m: f64,
    pub belt_width_m: f64,
    pub belt_thickness_m: f64,
    pub pitch_m: f64,
    pub sprocket_teeth: u32,
    pub wheels: Vec<RunWheel>,
    /// The physics rig's index of every wheel, by loop order, on each side (`None`: the stand-in order of `station`).
    #[serde(default)]
    pub stations: Option<StationIds>,
}

/// The station index in a physics rig of each wheel of `RunSpec::wheels` (same order) on the right and on the left.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationIds {
    pub right: Vec<u8>,
    pub left: Vec<u8>,
}

impl RunSpec {
    /// `shapes/placeholder_carrier_run.ron`, `PROVISIONAL(C-002)`.
    pub fn placeholder() -> RunSpec {
        ron::from_str(include_str!("../shapes/placeholder_carrier_run.ron"))
            .expect("placeholder_carrier_run.ron parses")
    }

    /// The station index of wheel `i` of the loop on `side`: the rig's if it was given, else the stand-in order (the left side first, each side
    /// from the front to the back, like `testing::box_tank()`).
    pub fn station(&self, side: Side, i: usize) -> u8 {
        if let Some(ids) = &self.stations {
            return if side == Side::Right { ids.right[i] } else { ids.left[i] };
        }
        let n = self.wheels.len();
        let rank = (0..n).filter(|&j| (self.wheels[j].z_m, j) < (self.wheels[i].z_m, i)).count();
        (rank + if side == Side::Right { n } else { 0 }) as u8
    }

    /// The circles the belt's pin line wraps, in loop order: a wheel's tip radius plus half the belt thickness, the sprocket's pitch radius.
    pub fn circles(&self) -> Vec<Circle> {
        let half = self.belt_thickness_m / 2.0;
        let r = |w: &RunWheel| if w.kind == WheelKind::Sprocket { w.radius_m } else { w.radius_m + half };
        self.wheels.iter().map(|w| Circle { z: w.z_m, y: w.y_m, r: r(w) }).collect()
    }
}

/// The radius of the circle that `teeth` teeth engaging pins `pitch_m` apart sit on: a regular polygon of side `pitch_m`.
pub fn pitch_radius_m(pitch_m: f64, teeth: u32) -> f64 {
    pitch_m / (2.0 * scalar::sin(PI / f64::from(teeth)))
}

/// A wheel within this turn of a straight run counts as on it (a millimetre over a metre), and the total-turn check allows each such wheel to
/// have dropped that much.
const ON_THE_RUN_RAD: f64 = 2e-3; // const-ok: 2 mrad, a tolerance on the band's turn round a wheel that sits on a straight run

/// The spacing of the points the band is checked at, and how far a wheel may poke through it before the band is refused.
const BAND_STEP_M: f64 = 5e-3; // const-ok: 5 mm sampling of the path for the clearance check
const POKE_M: f64 = 1e-3; // const-ok: a millimetre of a wheel through the band is rounding, more is a wrong loop

/// A circle in the side plane.
#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub z: f64,
    pub y: f64,
    pub r: f64,
}

/// A straight run or an arc of the belt path, in the side plane (z, y).
#[derive(Clone, Copy, Debug)]
enum Piece {
    Line { p: [f64; 2], t: [f64; 2] },
    Arc { c: [f64; 2], r: f64, a0: f64 },
}

/// The belt path: the taut band round a loop of circles, walked counter-clockwise in the (z, y) plane (a rear sprocket's loop order does that).
#[derive(Clone, Debug)]
pub struct Belt {
    pieces: Vec<(f64, Piece)>,
    pub length_m: f64,
}

impl Belt {
    /// The band round `circles` in loop order. A wheel that sits inside the band's hull, where the belt would have to turn the wrong way
    /// round it, is refused and the error names it: the physics rig put it where the belt does not touch.
    pub fn round(circles: &[Circle]) -> Result<Belt, String> {
        let n = circles.len();
        if n < 2 {
            return Err("a belt needs at least two wheels".into());
        }
        // the angle of the outward normal of the tangent that leaves circle i for circle i + 1 (the same normal at both ends)
        let mut leave = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (circles[i], circles[(i + 1) % n]);
            let d = scalar::hypot(b.z - a.z, b.y - a.y);
            if d <= (a.r - b.r).abs() {
                return Err(format!("wheel {i} and wheel {} lie inside one another", (i + 1) % n));
            }
            leave.push(scalar::atan2(b.y - a.y, b.z - a.z) - scalar::acos((a.r - b.r) / d));
        }
        // the turn the band makes round each circle, counter-clockwise: from the tangent that arrives to the one that leaves
        let mut sweeps: Vec<f64> = (0..n).map(|j| (leave[j] - leave[(j + n - 1) % n]).rem_euclid(TAU)).collect();
        for sweep in &mut sweeps {
            if *sweep > TAU - ON_THE_RUN_RAD {
                *sweep = 0.0;
            }
        }
        // a closed convex band turns through exactly one full turn; a wheel off the band turns it the other way, and the sum shows it
        if (sweeps.iter().sum::<f64>() - TAU).abs() > ON_THE_RUN_RAD * n as f64 {
            let j = (0..n).max_by(|&a, &b| sweeps[a].total_cmp(&sweeps[b])).unwrap_or(0);
            return Err(format!(
                "wheel {j} does not touch the belt: the band would have to turn the wrong way round it"
            ));
        }
        let (mut pieces, mut s) = (Vec::new(), 0.0);
        for i in 0..n {
            let (a, b, ang) = (circles[i], circles[(i + 1) % n], leave[i]);
            let (sa, ca) = scalar::sin_cos(ang);
            let (p, q) = ([a.z + a.r * ca, a.y + a.r * sa], [b.z + b.r * ca, b.y + b.r * sa]);
            pieces.push((s, Piece::Line { p, t: [-sa, ca] }));
            s += scalar::hypot(q[0] - p[0], q[1] - p[1]);
            // the arc on circle i + 1, from where this tangent arrives to where the next one leaves
            let j = (i + 1) % n;
            if sweeps[j] * b.r > 1e-9 {
                // const-ok: arcs shorter than a nanometre are straight runs
                pieces.push((s, Piece::Arc { c: [b.z, b.y], r: b.r, a0: ang }));
                s += sweeps[j] * b.r;
            }
        }
        let belt = Belt { pieces, length_m: s };
        // the band passes outside every wheel: a wheel it cuts through means the loop order was wrong (the band is walked counter-clockwise)
        let steps = (belt.length_m / BAND_STEP_M).ceil() as usize;
        let points: Vec<[f64; 2]> = (0..steps).map(|k| belt.at(belt.length_m * k as f64 / steps as f64).0).collect();
        for (j, c) in circles.iter().enumerate() {
            let nearest = points.iter().map(|p| scalar::hypot(p[0] - c.z, p[1] - c.y)).fold(f64::MAX, f64::min);
            if nearest < c.r - POKE_M {
                return Err(format!(
                    "wheel {j} pokes {:.3} m through the band: the loop order must run counter-clockwise in (z, y)",
                    c.r - nearest
                ));
            }
        }
        Ok(belt)
    }

    /// The point at arc length `s` (wrapping round the loop) and the unit direction of travel there.
    pub fn at(&self, s: f64) -> ([f64; 2], [f64; 2]) {
        let s = s.rem_euclid(self.length_m);
        let k = self.pieces.partition_point(|&(s0, _)| s0 <= s).saturating_sub(1);
        let (s0, piece) = self.pieces[k];
        match piece {
            Piece::Line { p, t } => ([p[0] + t[0] * (s - s0), p[1] + t[1] * (s - s0)], t),
            Piece::Arc { c, r, a0 } => {
                let (sa, ca) = scalar::sin_cos(a0 + (s - s0) / r);
                ([c[0] + r * ca, c[1] + r * sa], [-sa, ca])
            }
        }
    }
}

/// How many links close the loop and the pitch they really have: n = round(L / p), p' = L / n.
pub fn tile(length_m: f64, pitch_m: f64) -> (usize, f64) {
    let n = (length_m / pitch_m).round().max(3.0) as usize; // const-ok: a loop of fewer than three links is not a loop
    (n, length_m / n as f64)
}

/// The proportions of one link (`shapes/track_link.ron`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    pub gap_frac: f64,
    pub connector_w_m: f64,
    pub connector_h_m: f64,
    pub connector_len_frac: f64,
    pub horn_w_m: f64,
    pub horn_h_m: f64,
    pub horn_len_frac: f64,
    pub grouser_h_m: f64,
    pub grouser_len_frac: f64,
    pub inset_m: f64,
}

impl LinkSpec {
    pub fn standard() -> LinkSpec {
        ron::from_str(include_str!("../shapes/track_link.ron")).expect("track_link.ron parses")
    }
}

/// A closed box with centre `c` and full sizes `s`: 8 vertices, 12 triangles, outward.
fn cuboid(c: [f64; 3], s: [f64; 3]) -> Mesh {
    let corner = |i: u32| {
        let h = |bit: u32, k: usize| c[k] + s[k] * if i & bit != 0 { 0.5 } else { -0.5 }; // const-ok: half a size
        Vec3::new(h(1, 0), h(2, 1), h(4, 2))
    };
    let mut m = Mesh { v: (0..8).map(corner).collect(), t: Vec::new() };
    // each face as a quad of corner indices (bit 1 = +x, 2 = +y, 4 = +z)
    for q in [[0, 2, 6, 4], [1, 3, 7, 5], [0, 1, 5, 4], [2, 3, 7, 6], [0, 1, 3, 2], [4, 5, 7, 6]] {
        m.t.push([q[0], q[1], q[2]]);
        m.t.push([q[0], q[2], q[3]]);
    }
    face_away(&mut m);
    m
}

/// Face every triangle of a convex solid away from its centre (the box above is built from loosely ordered quads).
fn face_away(m: &mut Mesh) {
    let c = m.v.iter().fold(Vec3::ZERO, |a, &p| a + p) * (1.0 / m.v.len() as f64);
    for k in 0..m.t.len() {
        let [a, b, d] = m.tri(k);
        if (b - a).cross(d - a).dot(a - c) < 0.0 {
            m.t[k].swap(1, 2);
        }
    }
}

/// One link in its own frame: x across the belt, y out of it (the ground side), z along the travel; the pin line is y = 0.
fn link_mesh(l: &LinkSpec, pitch_m: f64, width_m: f64, thick_m: f64) -> Mesh {
    let mut m = Mesh::default();
    let (t, e) = (thick_m, l.inset_m);
    m.append(&cuboid([0.0, 0.0, 0.0], [width_m, t, pitch_m * (1.0 - l.gap_frac)])); // the pad
                                                                                    // standing on the wheel side, embedded `e` into the pad: two connectors at its edges and the guide horn between the wheel discs
    let inner = |h: f64| (-t / 2.0 - (h - e) / 2.0, h + e);
    let (cy, ch) = inner(l.connector_h_m);
    let cx = width_m / 2.0 - e - l.connector_w_m / 2.0;
    for side in [-1.0, 1.0] {
        m.append(&cuboid([side * cx, cy, 0.0], [l.connector_w_m, ch, pitch_m * l.connector_len_frac]));
    }
    let (hy, hh) = inner(l.horn_h_m);
    m.append(&cuboid([0.0, hy, 0.0], [l.horn_w_m, hh, pitch_m * l.horn_len_frac]));
    // the grouser on the ground side
    let (gh, gy) = (l.grouser_h_m + e, t / 2.0 + (l.grouser_h_m - e) / 2.0);
    m.append(&cuboid([0.0, gy, 0.0], [width_m - 4.0 * e, gh, pitch_m * l.grouser_len_frac])); // const-ok: 4 = an inset at each end of the cleat and of the pad
    m
}

/// All the links placed along the belt, the first at arc length `phase_m` (the sprocket's pitch radius times its spin): one mesh in the node
/// frame of the `Track` node (x across the belt about the track centre line, y up and z along the vehicle in the hull frame).
pub fn belt_mesh(belt: &Belt, spec: &RunSpec, link: &LinkSpec, phase_m: f64) -> Mesh {
    let (n, pitch) = tile(belt.length_m, spec.pitch_m);
    let one = link_mesh(link, pitch, spec.belt_width_m, spec.belt_thickness_m);
    let mut m = Mesh::default();
    for k in 0..n {
        let (p, t) = belt.at(k as f64 * pitch + phase_m);
        // link frame (x, y, z) to the hull frame: z along the tangent t, y along the outward normal (t.y, -t.z)
        let place = |q: Vec3| Vec3::new(q.x, p[1] + q.z * t[1] - q.y * t[0], p[0] + q.z * t[0] + q.y * t[1]);
        m.append(&Mesh { v: one.v.iter().map(|&q| place(q)).collect(), t: one.t.clone() });
    }
    m
}

/// A tooth profile ring for a sprocket of pitch radius `rp` with `teeth` teeth, counter-clockwise.
fn gear_ring(rp: f64, teeth: u32) -> Vec<[f64; 2]> {
    let step = TAU / f64::from(teeth);
    let (root, tip) = (0.9 * rp, 1.07 * rp); // const-ok: tooth depth below and above the pitch circle, ESTIMATE
    let (half_root, half_tip) = (0.30 * step, 0.16 * step); // const-ok: tooth half-widths at the root and the tip as fractions of a tooth step, ESTIMATE
    let pt = |r: f64, a: f64| {
        let (s, c) = scalar::sin_cos(a);
        [r * c, r * s]
    };
    (0..teeth)
        .flat_map(|k| {
            let a = step * f64::from(k);
            [pt(root, a - half_root), pt(tip, a - half_tip), pt(tip, a + half_tip), pt(root, a + half_root)]
        })
        .collect()
}

/// The meshes of one wheel in its own frame (hub at the origin, axle along X, symmetric about the track centre plane): rubber, metal discs
/// and a hub (a sprocket: toothed discs and a hub).
fn wheel_meshes(w: &RunWheel, spec: &RunSpec, segments: u32) -> Vec<(&'static str, SlotKind, Mesh)> {
    let (r, hw) = (w.radius_m, w.width_m / 2.0);
    let gap = 0.24 * hw; // const-ok: the guide-horn gap between the two discs, ESTIMATE
    let half = (segments / 2).max(12); // const-ok: inner parts hidden by the rim take half the segments
    let ring = |r0: f64, r1: f64, x0: f64, x1: f64, n: u32| revolve(&[(r0, x0), (r1, x0), (r1, x1), (r0, x1)], n);
    let both = |f: &dyn Fn(f64, f64) -> Mesh| {
        let mut m = f(-hw, -gap);
        m.append(&f(gap, hw));
        m
    };
    let web = ring(0.30 * r, 0.62 * r, -gap - 0.01, gap + 0.01, half); // const-ok: web and hub proportions, ESTIMATE
    let hub = cyl_x(0.0, 0.0, 0.34 * r, -hw - 0.01, hw + 0.035, half); // const-ok: hub proportions, ESTIMATE
    match w.kind {
        WheelKind::Sprocket => {
            let toothed = |x0: f64, x1: f64| {
                let ring = gear_ring(w.radius_m, spec.sprocket_teeth);
                // no splitting of a short prism (const-ok: 10 m is longer than any wheel)
                let prism = loft(&[Section { z_m: x0, ring: ring.clone() }, Section { z_m: x1, ring }], 10.0); // const-ok: see above
                                                                                                               // loft axis z -> hull x: (x, y, z) -> (z, x, y) is a cyclic permutation, so it keeps the orientation
                Mesh { v: prism.v.iter().map(|p| Vec3::new(p.z, p.x, p.y)).collect(), t: prism.t }
            };
            vec![
                ("teeth", SlotKind::Metal, both(&toothed)),
                ("web", SlotKind::Metal, web),
                ("hub", SlotKind::Metal, hub),
            ]
        }
        WheelKind::ReturnRoller => {
            let roller = cyl_x(0.0, 0.0, r, -hw, hw, half);
            let hub = cyl_x(0.0, 0.0, 0.5 * r, -hw - 0.01, hw + 0.03, half); // const-ok: roller hub proportions, ESTIMATE
            vec![("roller", SlotKind::Rubber, roller), ("hub", SlotKind::Metal, hub)]
        }
        _ => {
            let rubber = both(&|x0, x1| ring(0.86 * r, r, x0, x1, segments)); // const-ok: the rubber band is the outer 14% of the radius, ESTIMATE
            let plates = both(&|x0, x1| ring(0.30 * r, 0.86 * r + 0.004, x0, x1, half)); // const-ok: plates overlap the rubber by 4 mm
            let mut disc = plates;
            disc.append(&web);
            vec![("rim", SlotKind::Rubber, rubber), ("disc", SlotKind::Metal, disc), ("hub", SlotKind::Metal, hub)]
        }
    }
}

fn role_of(k: WheelKind) -> (NodeRole, &'static str) {
    match k {
        WheelKind::RoadWheel | WheelKind::Tyre => (NodeRole::RoadWheel, "road_wheel"),
        WheelKind::Sprocket => (NodeRole::Sprocket, "sprocket"),
        WheelKind::Idler => (NodeRole::Idler, "idler"),
        WheelKind::ReturnRoller => (NodeRole::ReturnRoller, "return_roller"),
    }
}

/// The parts of one side's running gear in the hull frame: every wheel (named `<kind>.<index in the loop>.<part>`, role by kind, pose the hub)
/// and the belt (role `Track`, pose the track centre line). An error names a wheel the belt does not touch.
pub fn run_parts(spec: &RunSpec, link: &LinkSpec, detail: u8) -> Result<Vec<Part>, String> {
    let belt = Belt::round(&spec.circles())?;
    let segments = segments_for(detail);
    let part = |name: String, role: NodeRole, slot: SlotKind, mesh: Mesh, pose: Transform| Part {
        name,
        role,
        station: None,
        side: Side::Right,
        slot,
        fitting: false,
        mesh: mesh.finished(),
        pose,
        placement: None,
    };
    let mut parts = Vec::new();
    for (i, w) in spec.wheels.iter().enumerate() {
        let (role, kind) = role_of(w.kind);
        let pose = Transform::from_pos(Vec3::new(spec.track_x_m, w.y_m, w.z_m));
        for (what, slot, mesh) in wheel_meshes(w, spec, segments) {
            parts.push(part(format!("{kind}.{i}.{what}"), role, slot, mesh, pose));
        }
    }
    let centre = Transform::from_pos(Vec3::new(spec.track_x_m, 0.0, 0.0));
    parts.push(part("belt".into(), NodeRole::Track, SlotKind::Track, belt_mesh(&belt, spec, link, 0.0), centre));
    Ok(parts)
}

/// The sockets a hull publishes for this run, on both sides: one `Station` per wheel (at the hub, normal outward, size the wheel's radius,
/// `station` its index in the loop) and one `Run` on each track centre line (size the belt width). Named `station.<index>.<r|l>` and `run.<r|l>`.
pub fn run_sockets(spec: &RunSpec) -> Vec<Socket> {
    let socket = |name: String, kind, side: Side, at: Vec3, size_m: f64, station: Option<u8>| Socket {
        name,
        kind,
        side,
        pose: frame(at, Vec3::new(if side == Side::Right { 1.0 } else { -1.0 }, 0.0, 0.0), -Vec3::Z),
        size_m,
        station,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    let mut sockets = Vec::new();
    for (side, tag) in [(Side::Right, "r"), (Side::Left, "l")] {
        let sx = if side == Side::Right { 1.0 } else { -1.0 };
        for (i, w) in spec.wheels.iter().enumerate() {
            let hub = Vec3::new(sx * spec.track_x_m, w.y_m, w.z_m);
            sockets.push(socket(
                format!("station.{i}.{tag}"),
                SocketKind::Station,
                side,
                hub,
                w.radius_m,
                Some(spec.station(side, i)),
            ));
        }
        let centre = Vec3::new(sx * spec.track_x_m, 0.0, 0.0);
        sockets.push(socket(format!("run.{tag}"), SocketKind::Run, side, centre, spec.belt_width_m, None));
    }
    sockets
}

/// One wheel as a module (hub at the origin, authored for the right side, mount facing the hull and as big as the wheel), the same shape as
/// `gear::wheel_module`: it fits a `Station` socket sized for a wheel at least this large.
pub fn wheel_module(w: &RunWheel, spec: &RunSpec, detail: u8) -> Module {
    let (role, kind) = role_of(w.kind);
    let parts = wheel_meshes(w, spec, segments_for(detail))
        .into_iter()
        .map(|(what, slot, mesh)| Part {
            name: what.into(),
            role,
            station: None,
            side: Side::Right,
            slot,
            fitting: false,
            mesh: mesh.finished(),
            pose: Transform::IDENTITY,
            placement: None,
        })
        .collect();
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Station,
        side: Side::Right,
        pose: frame(Vec3::ZERO, -Vec3::X, -Vec3::Z),
        size_m: w.radius_m,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    Module {
        name: kind.into(),
        kind: ModuleKind::Gear,
        parts,
        sockets: Vec::new(),
        mount: Some(mount),
        symmetric: false,
    }
}

/// The belt of one side as a module (the links at phase 0, one `Track` part about the track centre line), for a `Run` socket.
pub fn belt_module(spec: &RunSpec, link: &LinkSpec) -> Result<Module, String> {
    let belt = Belt::round(&spec.circles())?;
    let part = Part {
        name: "belt".into(),
        role: NodeRole::Track,
        station: None,
        side: Side::Right,
        slot: SlotKind::Track,
        fitting: false,
        mesh: belt_mesh(&belt, spec, link, 0.0).finished(),
        pose: Transform::IDENTITY,
        placement: None,
    };
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Run,
        side: Side::Right,
        pose: frame(Vec3::ZERO, -Vec3::X, -Vec3::Z),
        size_m: spec.belt_width_m,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    Ok(Module {
        name: "belt".into(),
        kind: ModuleKind::Gear,
        parts: vec![part],
        sockets: Vec::new(),
        mount: Some(mount),
        symmetric: false,
    })
}
