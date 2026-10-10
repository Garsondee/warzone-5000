//! The parametric 4x4 utility truck: a data-driven part list (`shapes/utility_4x4.ron`) scaled to the hull box, plus wheels at the four
//! stations. Hull frame: origin at the hull box centre, +Y up, -Z forward, +X right; the ground is `ride_height_m` below the origin.

use crate::loft::{bevel_ring, loft, Section};
use crate::mesh::Mesh;
use crate::part::{Part, Side};
use crate::wheel::{cylinder_x, segments_for, wheel, WheelDims};
use serde::Deserialize;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{scalar, Quat, Transform, Vec3};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityDims {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub wheelbase_m: f64,
    pub track_m: f64,
    pub ground_clearance_m: f64,
    pub wheel: WheelDims,
}

impl UtilityDims {
    /// `shapes/placeholder_utility_4x4.ron`, `PROVISIONAL(C-002)`.
    pub fn placeholder() -> UtilityDims {
        ron::from_str(include_str!("../shapes/placeholder_utility_4x4.ron"))
            .expect("placeholder_utility_4x4.ron parses")
    }

    /// Distance from the hull box centre down to the ground (FORGE's ride height: clearance + height / 2).
    pub fn ride_height_m(&self) -> f64 {
        self.ground_clearance_m + self.height_m / 2.0
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    parts: Vec<PartSpec>,
    knuckle_radius_m: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartSpec {
    name: String,
    role: NodeRole,
    slot: SlotKind,
    fitting: bool,
    mirror: bool,
    shape: Shape,
}

#[derive(Deserialize)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Deserialize)]
enum Shape {
    /// Sections (z, full width, y0, y1) as fractions of L, W, H, lofted with 45 degree bevels of `corner_m`.
    Loft {
        corner_m: f64,
        sections: Vec<(f64, f64, f64, f64)>,
    },
    Block {
        x: (f64, f64),
        y: (f64, f64),
        z: (f64, f64),
        corner_m: f64,
    },
    Cyl {
        axis: Axis,
        at: (f64, f64, f64),
        radius_m: f64,
        length_m: f64,
    },
    /// A thin sloped plate in the y-z plane, from (y, z) to (y, z), `width` of W across.
    Slab {
        from_yz: (f64, f64),
        to_yz: (f64, f64),
        width: f64,
        thickness_m: f64,
        lift_m: f64,
    },
    /// One flare over each wheel.
    Flares {
        above_wheel_m: f64,
        height_m: f64,
        length_per_radius: f64,
        inboard_m: f64,
        corner_m: f64,
    },
}

/// A bevelled loft along z: the end caps are inset by `bevel_m`, so every edge of the part is a 45 degree chamfer.
fn shaped(secs: &[(f64, f64, f64, f64)], corner_m: f64, band_m: f64) -> Mesh {
    let ring = |w: f64, y0: f64, y1: f64, inset: f64| {
        bevel_ring(w - 2.0 * inset, y1 - y0 - 2.0 * inset, (y0 + y1) / 2.0, corner_m, band_m)
    };
    let c = corner_m;
    let mut s = Vec::new();
    let (a, b) = (secs[0], secs[secs.len() - 1]);
    s.push(Section { z_m: a.0, ring: ring(a.1, a.2, a.3, c) });
    for (i, q) in secs.iter().enumerate() {
        let z = if i == 0 {
            q.0 + c
        } else if i == secs.len() - 1 {
            q.0 - c
        } else {
            q.0
        };
        s.push(Section { z_m: z, ring: ring(q.1, q.2, q.3, 0.0) });
    }
    s.push(Section { z_m: b.0, ring: ring(b.1, b.2, b.3, c) });
    loft(&s, 0.5) // const-ok: a vertex at least every half metre along a hard edge (spike S-G)
}

pub fn utility_4x4(d: &UtilityDims, detail: u8) -> Vec<Part> {
    let tpl: Template = ron::from_str(include_str!("../shapes/utility_4x4.ron")).expect("utility_4x4.ron parses");
    let band = crate::flags::FlagParams::default_params().edge_band_m;
    let (w, h, l) = (d.width_m, d.height_m, d.length_m);
    let (fx, fy, fz) = (|f: f64| f * w, |f: f64| (f - 0.5) * h, |f: f64| (f - 0.5) * l);
    let sides = |mirror: bool| if mirror { vec![Side::Right, Side::Left] } else { vec![Side::Centre] };
    let finish = |mut m: Mesh| {
        m.weld(1e-7, 1e-12); // const-ok: weld tolerance, 0.1 micrometre, far below any feature
        m.orient_outward();
        m
    };
    let mut parts = Vec::new();
    let mut add = |spec: &PartSpec, name: String, side: Side, station: Option<u8>, mesh: Mesh, pose: Transform| {
        let mesh = if side == Side::Left { mesh.mirrored_x() } else { mesh };
        let pose = if side == Side::Left {
            Transform::new(Vec3::new(-pose.pos.x, pose.pos.y, pose.pos.z), pose.rot)
        } else {
            pose
        };
        parts.push(Part {
            name,
            role: spec.role,
            station,
            side,
            slot: spec.slot,
            fitting: spec.fitting,
            mesh: finish(mesh),
            pose,
        });
    };
    let wheel_y = -d.ride_height_m() + d.wheel.outer_radius_m;
    let stations = [(0u8, -d.wheelbase_m / 2.0), (1, d.wheelbase_m / 2.0)];
    for spec in &tpl.parts {
        for side in sides(spec.mirror) {
            let name = match side {
                Side::Centre => spec.name.clone(),
                Side::Right => format!("{}.r", spec.name),
                Side::Left => format!("{}.l", spec.name),
            };
            match &spec.shape {
                Shape::Loft { corner_m, sections } => {
                    let secs: Vec<_> = sections.iter().map(|s| (fz(s.0), fx(s.1), fy(s.2), fy(s.3))).collect();
                    add(spec, name, side, None, shaped(&secs, *corner_m, band), Transform::IDENTITY);
                }
                Shape::Block { x, y, z, corner_m } => {
                    let (xm, ym, zm) = ((x.0 + x.1) / 2.0, (y.0 + y.1) / 2.0, (z.0 + z.1) / 2.0);
                    let (bw, bh) = (fx(x.1) - fx(x.0), fy(y.1) - fy(y.0));
                    let bl = fz(z.1) - fz(z.0);
                    let m = shaped(
                        &[(-bl / 2.0, bw, -bh / 2.0, bh / 2.0), (bl / 2.0, bw, -bh / 2.0, bh / 2.0)],
                        *corner_m,
                        band,
                    );
                    add(
                        spec,
                        name,
                        side,
                        None,
                        m.transformed(&Transform::from_pos(Vec3::new(fx(xm), fy(ym), fz(zm)))),
                        Transform::IDENTITY,
                    );
                }
                Shape::Cyl { axis, at, radius_m, length_m } => {
                    let mut m = cylinder_x(*radius_m, -length_m / 2.0, length_m / 2.0, segments_for(detail) / 4); // const-ok: a quarter of the wheel segments
                    let rot = |p: Vec3| match axis {
                        Axis::X => p,
                        Axis::Y => Vec3::new(p.z, p.x, p.y),
                        Axis::Z => Vec3::new(p.y, p.z, p.x),
                    };
                    m.v = m.v.iter().map(|&p| rot(p) + Vec3::new(fx(at.0), fy(at.1), fz(at.2))).collect();
                    add(spec, name, side, None, m, Transform::IDENTITY);
                }
                Shape::Slab { from_yz, to_yz, width, thickness_m, lift_m } => {
                    let (p0, p1) = ((fy(from_yz.0), fz(from_yz.1)), (fy(to_yz.0), fz(to_yz.1)));
                    let (dy, dz) = (p1.0 - p0.0, p1.1 - p0.1);
                    let len = scalar::hypot(dy, dz);
                    let m = shaped(
                        &[
                            (-len / 2.0, fx(*width), -thickness_m / 2.0, thickness_m / 2.0),
                            (len / 2.0, fx(*width), -thickness_m / 2.0, thickness_m / 2.0),
                        ],
                        thickness_m / 4.0,
                        band,
                    ); // const-ok: bevel a quarter of the thickness
                    let normal = (dz / len, -dy / len);
                    let at = Vec3::new(
                        0.0,
                        (p0.0 + p1.0) / 2.0 + normal.0 * (lift_m + thickness_m / 2.0),
                        (p0.1 + p1.1) / 2.0 + normal.1 * (lift_m + thickness_m / 2.0),
                    );
                    add(
                        spec,
                        name,
                        side,
                        None,
                        m.transformed(&Transform::new(at, Quat::from_axis_angle(Vec3::X, scalar::atan2(-dy, dz)))),
                        Transform::IDENTITY,
                    );
                }
                Shape::Flares { above_wheel_m, height_m, length_per_radius, inboard_m, corner_m } => {
                    for (axle, z) in stations {
                        for fs in [Side::Right, Side::Left] {
                            let r = d.wheel.outer_radius_m;
                            let (x0, x1) = (d.track_m / 2.0 - d.wheel.width_m / 2.0 - inboard_m, w / 2.0);
                            let (len, y0) = (length_per_radius * r, wheel_y + r + above_wheel_m);
                            let m = shaped(
                                &[(-len / 2.0, x1 - x0, 0.0, *height_m), (len / 2.0, x1 - x0, 0.0, *height_m)],
                                *corner_m,
                                band,
                            );
                            let n = format!("{}.{}.{}", spec.name, axle, if fs == Side::Right { "r" } else { "l" });
                            add(
                                spec,
                                n,
                                fs,
                                Some(axle),
                                m.transformed(&Transform::from_pos(Vec3::new((x0 + x1) / 2.0, y0, z))),
                                Transform::IDENTITY,
                            );
                        }
                    }
                    break;
                }
            }
        }
    }
    // wheels and knuckles at the four stations, in their node frames
    let seg = segments_for(detail);
    let base = wheel(&d.wheel, seg);
    for (axle, z) in stations {
        for side in [Side::Right, Side::Left] {
            let sx = if side == Side::Right { 1.0 } else { -1.0 };
            let (wh, tag) = if side == Side::Right { (wheel(&d.wheel, seg), "r") } else { (base.mirrored_x(), "l") };
            let pose = Transform::from_pos(Vec3::new(sx * d.track_m / 2.0, wheel_y, z));
            let mut push = |what: &str, role: NodeRole, slot: SlotKind, mesh: Mesh| {
                parts.push(Part {
                    name: format!("{what}.{axle}.{tag}"),
                    role,
                    station: Some(axle),
                    side,
                    slot,
                    fitting: false,
                    mesh: finish(mesh),
                    pose,
                });
            };
            push("tyre", NodeRole::Wheel, SlotKind::Rubber, wh.tyre);
            push("tread", NodeRole::Wheel, SlotKind::Rubber, wh.lugs);
            push("rim", NodeRole::Wheel, SlotKind::Metal, wh.rim);
            push("nuts", NodeRole::Wheel, SlotKind::Metal, wh.nuts);
            if axle == 0 {
                let k = cylinder_x(tpl.knuckle_radius_m, -0.18, 0.0, 16); // const-ok: knuckle housing inboard of the rim: 16 sides, 18 cm deep
                push(
                    "knuckle",
                    NodeRole::SteerKnuckle,
                    SlotKind::Metal,
                    if side == Side::Right { k } else { k.mirrored_x() },
                );
            }
        }
    }
    parts
}
