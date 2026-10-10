//! The parametric 4x4 utility truck. The hull is one lofted shell (`shapes/utility_4x4.ron`, `body`): its cross-section is a full-width
//! lower body with a narrower hood, cab or bed block on top, and the wheel arches are openings whose height follows the wheel circle, cut
//! into the section along the loft. Everything else in the part list is anchored to a surface of that shell (its walls, its roof line, its
//! front and rear faces) and overlaps it, so nothing floats. Wheels sit at the four stations in their node frames.
//!
//! Hull frame: origin at the hull box centre, +Y up, -Z forward, +X right; the ground is `ride_height_m` below the origin.

use crate::flags::FlagParams;
use crate::loft::{bevel_ring, loft_beveled, polygon_ring, sweep_arc, Section};
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
    body: BodySpec,
    parts: Vec<PartSpec>,
    knuckle_radius_m: f64,
}

/// The shell. `stations` are (z as a fraction of L from the front, lower half-width, belt height, upper half-width, top height), the
/// half-widths as fractions of the nominal half-width and the heights as fractions of H from the hull box bottom. Lower body: from the
/// bottom to the belt; upper block: from the belt to the top (hood, cab roof, bed rail).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodySpec {
    stations: Vec<(f64, f64, f64, f64, f64)>,
    /// How far the arch lips stand outside the lower wall: the hull box width includes them.
    lip_proud_m: f64,
    lower_chamfer_m: f64,
    upper_chamfer_m: f64,
    /// Clearance between the tyre and the arch edge, and between the tyre's inner face and the wheel well's inner wall.
    arch_gap_m: f64,
    well_gap_m: f64,
    arch_samples: u32,
    /// The arch never rises closer than this to the belt; the notch is never thinner than `min_notch_m` (keeps the ring topology).
    min_wall_m: f64,
    min_notch_m: f64,
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

/// Ranges along each axis, in the right half (+X) of the vehicle. They can be fractions of the hull box or offsets from a surface of the
/// shell, so a fitting stays on its surface when the dimensions change: `Upper` and `Lower` are metres from the upper and lower walls
/// (positive = outside), `Belt` and `Top` metres from the belt line and the top line, `Front` and `Rear` metres from the front and rear
/// faces (positive = towards the rear).
#[derive(Deserialize)]
enum XR {
    Frac(f64, f64),
    Upper(f64, f64),
    Lower(f64, f64),
}

#[derive(Deserialize)]
enum YR {
    Frac(f64, f64),
    Belt(f64, f64),
    Top(f64, f64),
}

#[derive(Deserialize)]
enum ZR {
    Frac(f64, f64),
    Front(f64, f64),
    Rear(f64, f64),
}

#[derive(Deserialize)]
enum Shape {
    /// Sections (z, full width, y0, y1) as fractions of L, W, H, lofted with bevels of `corner_m`.
    Loft {
        corner_m: f64,
        sections: Vec<(f64, f64, f64, f64)>,
    },
    Block {
        x: XR,
        y: YR,
        z: ZR,
        corner_m: f64,
    },
    /// A cylinder along `axis`: its extent along the axis is that axis's range, its centre the middle of the other two.
    Cyl {
        axis: Axis,
        x: XR,
        y: YR,
        z: ZR,
        radius_m: f64,
    },
    /// A panel on the top line (hood, roof) over `z` (fractions of L), inset from the upper walls, embedded and proud of the surface.
    Overlay {
        z: (f64, f64),
        inset_m: f64,
        embed_m: f64,
        proud_m: f64,
        corner_m: f64,
    },
    /// A side window set into a wall: z (fractions of L), a bottom edge (fraction of H) and a top edge that follows the roof line at
    /// `top_offset_m` below it, so the pane keeps inside the greenhouse where the roof line rises (the A-pillar).
    Pane {
        x: XR,
        z: (f64, f64),
        bottom: f64,
        top_offset_m: f64,
        corner_m: f64,
    },
    /// The glass between two stations of the roof line (a raked plane), inset from the upper walls.
    Windscreen {
        from_f: f64,
        to_f: f64,
        inset_m: f64,
        thickness_m: f64,
        proud_m: f64,
    },
    /// A lip around each wheel arch, swept along the arch circle and standing `proud_m` outside the lower wall.
    ArchLip {
        inner_m: f64,
        width_m: f64,
        embed_m: f64,
        proud_m: f64,
        corner_m: f64,
    },
}

/// One station of the shell in metres: lower and upper half-widths, belt height and top height.
#[derive(Clone, Copy)]
struct St {
    z: f64,
    xl: f64,
    yb: f64,
    xu: f64,
    yt: f64,
}

/// The shell's lines: stations interpolated linearly in z (as the loft does between sections).
struct Frame {
    st: Vec<St>,
    y0: f64,
    dims: UtilityDims,
}

impl Frame {
    fn new(d: &UtilityDims, b: &BodySpec) -> Frame {
        let half = d.width_m / 2.0 - b.lip_proud_m;
        let st = b
            .stations
            .iter()
            .map(|&(f, w, yb, wu, yt)| St {
                z: (f - 0.5) * d.length_m,
                xl: w * half,
                yb: (yb - 0.5) * d.height_m,
                xu: wu * half,
                yt: (yt - 0.5) * d.height_m,
            })
            .collect();
        Frame { st, y0: -d.height_m / 2.0, dims: *d }
    }

    fn at(&self, z: f64) -> St {
        let (a, b) = (self.st[0], self.st[self.st.len() - 1]);
        if z <= a.z {
            return St { z, ..a };
        }
        if z >= b.z {
            return St { z, ..b };
        }
        let i = self.st.partition_point(|s| s.z <= z) - 1;
        let (p, q) = (self.st[i], self.st[i + 1]);
        let t = (z - p.z) / (q.z - p.z);
        let l = |u: f64, v: f64| u + (v - u) * t;
        St { z, xl: l(p.xl, q.xl), yb: l(p.yb, q.yb), xu: l(p.xu, q.xu), yt: l(p.yt, q.yt) }
    }

    fn front_z(&self) -> f64 {
        self.st[0].z
    }

    fn rear_z(&self) -> f64 {
        self.st[self.st.len() - 1].z
    }

    fn z(&self, r: &ZR) -> (f64, f64) {
        let d = &self.dims;
        match *r {
            ZR::Frac(a, b) => ((a - 0.5) * d.length_m, (b - 0.5) * d.length_m),
            ZR::Front(a, b) => (self.front_z() + a, self.front_z() + b),
            ZR::Rear(a, b) => (self.rear_z() + a, self.rear_z() + b),
        }
    }

    fn x(&self, r: &XR, z: f64) -> (f64, f64) {
        let s = self.at(z);
        match *r {
            XR::Frac(a, b) => (a * self.dims.width_m, b * self.dims.width_m),
            XR::Upper(a, b) => (s.xu + a, s.xu + b),
            XR::Lower(a, b) => (s.xl + a, s.xl + b),
        }
    }

    fn y(&self, r: &YR, z: f64) -> (f64, f64) {
        let s = self.at(z);
        match *r {
            YR::Frac(a, b) => ((a - 0.5) * self.dims.height_m, (b - 0.5) * self.dims.height_m),
            YR::Belt(a, b) => (s.yb + a, s.yb + b),
            YR::Top(a, b) => (s.yt + a, s.yt + b),
        }
    }
}

/// A bevelled box-like loft along z from sections (z, full width, y0, y1): chamfered ends, loops beside the hard edges.
fn shaped(secs: &[(f64, f64, f64, f64)], corner_m: f64, band_m: f64) -> Mesh {
    let sections: Vec<Section> = secs
        .iter()
        .map(|&(z, w, y0, y1)| Section { z_m: z, ring: bevel_ring(w, y1 - y0, (y0 + y1) / 2.0, corner_m, band_m) })
        .collect();
    loft_beveled(&sections, corner_m, band_m, &[], 0.5) // const-ok: a vertex at least every half metre along a hard edge (spike S-G)
}

/// Where the arches are: the wheel circle's centre in the hull frame, the arch radius, the arch's half angle at the body bottom, the
/// station z of each axle and the x of the wheel well's inner wall.
struct Arches {
    y_c: f64,
    r_a: f64,
    phi_m: f64,
    axle_z: [f64; 2],
    x_n: f64,
}

impl Arches {
    fn new(d: &UtilityDims, b: &BodySpec) -> Arches {
        let r_a = d.wheel.outer_radius_m + b.arch_gap_m;
        let y_c = -d.ride_height_m() + d.wheel.outer_radius_m;
        let dy = -d.height_m / 2.0 - y_c;
        let half_chord = scalar::sqrt((r_a * r_a - dy * dy).max(0.0));
        Arches {
            y_c,
            r_a,
            phi_m: scalar::asin((half_chord / r_a).min(1.0)),
            axle_z: [-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0],
            x_n: d.track_m / 2.0 - d.wheel.width_m / 2.0 - b.well_gap_m,
        }
    }

    /// Height of the arch above the body bottom at station z (0 outside it).
    fn height(&self, z: f64, y0: f64) -> f64 {
        self.axle_z
            .iter()
            .map(|&zc| {
                let dz = z - zc;
                if dz.abs() < self.r_a {
                    self.y_c + scalar::sqrt(self.r_a * self.r_a - dz * dz) - y0
                } else {
                    0.0
                }
            })
            .fold(0.0, f64::max)
    }
}

/// The shell: a loft of T-shaped sections, each with a notch cut from its lower outer corners where a wheel arch is.
fn shell(f: &Frame, b: &BodySpec, a: &Arches, band: f64) -> Mesh {
    let mut zs: Vec<f64> = f.st.iter().map(|s| s.z).collect();
    for zc in a.axle_z {
        for k in 0..=b.arch_samples {
            let phi = a.phi_m * (2.0 * f64::from(k) / f64::from(b.arch_samples) - 1.0);
            zs.push(zc + a.r_a * scalar::sin(phi));
        }
    }
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|p, q| (*p - *q).abs() < 1e-4); // const-ok: sections closer than 0.1 mm are one
    let (cl, ct) = (b.lower_chamfer_m, b.upper_chamfer_m);
    let chamfers = [cl, cl, 0.0, cl, cl, 0.0, ct, ct, 0.0, cl, cl, 0.0];
    let sections: Vec<Section> = zs
        .iter()
        .map(|&z| {
            let (s, y0) = (f.at(z), f.y0);
            let h = a.height(z, y0).clamp(b.min_notch_m, (s.yb - y0 - b.min_wall_m).max(b.min_notch_m));
            let (xn, xl, xu) = (a.x_n, s.xl, s.xu);
            let poly = [
                [-xn, y0],
                [xn, y0],
                [xn, y0 + h],
                [xl, y0 + h],
                [xl, s.yb],
                [xu, s.yb],
                [xu, s.yt],
                [-xu, s.yt],
                [-xu, s.yb],
                [-xl, s.yb],
                [-xl, y0 + h],
                [-xn, y0 + h],
            ];
            Section { z_m: z, ring: polygon_ring(&poly, &chamfers, band) }
        })
        .collect();
    let creases: Vec<f64> = f.st.iter().map(|s| s.z).collect();
    loft_beveled(&sections, b.lower_chamfer_m, band, &creases, 0.5) // const-ok: a vertex at least every half metre along a hard edge (spike S-G)
}

pub fn utility_4x4(d: &UtilityDims, detail: u8) -> Vec<Part> {
    let tpl: Template = ron::from_str(include_str!("../shapes/utility_4x4.ron")).expect("utility_4x4.ron parses");
    let band = FlagParams::default_params().edge_band_m;
    let frame = Frame::new(d, &tpl.body);
    let arches = Arches::new(d, &tpl.body);
    let (w, h, l) = (d.width_m, d.height_m, d.length_m);
    let (fx, fy, fz) = (|f: f64| f * w, |f: f64| (f - 0.5) * h, |f: f64| (f - 0.5) * l);
    let sides = |mirror: bool| if mirror { vec![Side::Right, Side::Left] } else { vec![Side::Centre] };
    let finish = |mut m: Mesh| {
        m.weld(1e-7, 1e-12); // const-ok: weld tolerance, 0.1 micrometre, far below any feature
        m.orient_outward();
        m
    };
    let mut parts: Vec<Part> = Vec::new();
    parts.push(Part {
        name: "shell".into(),
        role: NodeRole::Hull,
        station: None,
        side: Side::Centre,
        slot: SlotKind::Paint,
        fitting: false,
        mesh: finish(shell(&frame, &tpl.body, &arches, band)),
        pose: Transform::IDENTITY,
    });
    let mut add = |spec: &PartSpec, name: String, side: Side, station: Option<u8>, mesh: Mesh| {
        let mesh = if side == Side::Left { mesh.mirrored_x() } else { mesh };
        parts.push(Part {
            name,
            role: spec.role,
            station,
            side,
            slot: spec.slot,
            fitting: spec.fitting,
            mesh: finish(mesh),
            pose: Transform::IDENTITY,
        });
    };
    let sides_n = segments_for(detail) / 4; // const-ok: a quarter of the wheel segments for small cylinders
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
                    add(spec, name, side, None, shaped(&secs, *corner_m, band));
                }
                Shape::Block { x, y, z, corner_m } => {
                    let (z0, z1) = frame.z(z);
                    let zm = (z0 + z1) / 2.0;
                    let ((x0, x1), (y0, y1)) = (frame.x(x, zm), frame.y(y, zm));
                    let (bw, bh, bl) = (x1 - x0, y1 - y0, z1 - z0);
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
                        m.transformed(&Transform::from_pos(Vec3::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, zm))),
                    );
                }
                Shape::Cyl { axis, x, y, z, radius_m } => {
                    let (z0, z1) = frame.z(z);
                    let zm = (z0 + z1) / 2.0;
                    let ((x0, x1), (y0, y1)) = (frame.x(x, zm), frame.y(y, zm));
                    let len = match axis {
                        Axis::X => x1 - x0,
                        Axis::Y => y1 - y0,
                        Axis::Z => z1 - z0,
                    };
                    let mut m = cylinder_x(*radius_m, -len / 2.0, len / 2.0, sides_n);
                    let rot = |p: Vec3| match axis {
                        Axis::X => p,
                        Axis::Y => Vec3::new(p.z, p.x, p.y),
                        Axis::Z => Vec3::new(p.y, p.z, p.x),
                    };
                    let c = Vec3::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, zm);
                    m.v = m.v.iter().map(|&p| rot(p) + c).collect();
                    add(spec, name, side, None, m);
                }
                Shape::Overlay { z, inset_m, embed_m, proud_m, corner_m } => {
                    let (za, zb) = ((z.0 - 0.5) * l, (z.1 - 0.5) * l);
                    let mut zs = vec![za];
                    zs.extend(frame.st.iter().map(|s| s.z).filter(|&s| s > za + 1e-6 && s < zb - 1e-6)); // const-ok: stations strictly inside the panel
                    zs.push(zb);
                    let secs: Vec<_> = zs
                        .iter()
                        .map(|&zz| {
                            let s = frame.at(zz);
                            (zz, 2.0 * (s.xu - inset_m), s.yt - embed_m, s.yt + proud_m)
                        })
                        .collect();
                    add(spec, name, side, None, shaped(&secs, *corner_m, band));
                }
                Shape::Pane { x, z, bottom, top_offset_m, corner_m } => {
                    let (z0, z1) = ((z.0 - 0.5) * l, (z.1 - 0.5) * l);
                    let (x0, x1) = frame.x(x, (z0 + z1) / 2.0);
                    let top = |zz: f64| frame.at(zz).yt - top_offset_m;
                    let yb = fy(*bottom);
                    // the outline in (y, z), lofted along x: local (x', y', z') = (y, z, x), then permuted back to (x, y, z) = (z', x', y')
                    let poly = [[yb, z0], [yb, z1], [top(z1), z1], [top(z0), z0]];
                    let ring = polygon_ring(&poly, &[*corner_m; 4], band);
                    let m = loft_beveled(
                        &[Section { z_m: x0, ring: ring.clone() }, Section { z_m: x1, ring }],
                        *corner_m,
                        band,
                        &[],
                        // const-ok: a pane is one strip
                        10.0,
                    );
                    let m = Mesh { v: m.v.iter().map(|p| Vec3::new(p.z, p.x, p.y)).collect(), t: m.t };
                    add(spec, name, side, None, m);
                }
                Shape::Windscreen { from_f, to_f, inset_m, thickness_m, proud_m } => {
                    let (z0, z1) = (fz(*from_f), fz(*to_f));
                    let (s0, s1) = (frame.at(z0), frame.at(z1));
                    let (dy, dz) = (s1.yt - s0.yt, z1 - z0);
                    let len = scalar::hypot(dy, dz);
                    let width = 2.0 * (frame.at((z0 + z1) / 2.0).xu - inset_m);
                    let t = *thickness_m;
                    let m = shaped(
                        &[(-len / 2.0, width, -t / 2.0, t / 2.0), (len / 2.0, width, -t / 2.0, t / 2.0)],
                        t / 4.0, // const-ok: bevel a quarter of the thickness
                        band,
                    );
                    let (ny, nz) = (dz / len, -dy / len); // the surface normal, up and forward
                    let off = proud_m - t / 2.0;
                    let at = Vec3::new(0.0, (s0.yt + s1.yt) / 2.0 + ny * off, (z0 + z1) / 2.0 + nz * off);
                    let pose = Transform::new(at, Quat::from_axis_angle(Vec3::X, scalar::atan2(-dy, dz)));
                    add(spec, name, side, None, m.transformed(&pose));
                }
                Shape::ArchLip { inner_m, width_m, embed_m, proud_m, corner_m } => {
                    for (axle, zc) in arches.axle_z.iter().enumerate() {
                        for lip_side in [Side::Right, Side::Left] {
                            let xl = frame.at(*zc).xl;
                            let rho0 = arches.r_a + inner_m;
                            let ring: Vec<[f64; 2]> =
                                bevel_ring(embed_m + proud_m, *width_m, rho0 + width_m / 2.0, *corner_m, band)
                                    .iter()
                                    .map(|p| [p[0] + xl + (proud_m - embed_m) / 2.0, p[1]])
                                    .collect();
                            let n = 18; // const-ok: steps along the arch: a 9 degree step keeps the sag under 3 mm
                            let angles: Vec<f64> =
                                (0..=n).map(|k| arches.phi_m * (2.0 * f64::from(k) / f64::from(n) - 1.0)).collect();
                            let m = sweep_arc(&ring, (arches.y_c, *zc), &angles);
                            let n =
                                format!("{}.{}.{}", spec.name, axle, if lip_side == Side::Right { "r" } else { "l" });
                            add(spec, n, lip_side, Some(axle as u8), m);
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
    let wheel_y = arches.y_c;
    for (axle, z) in arches.axle_z.iter().enumerate() {
        let axle = axle as u8;
        for side in [Side::Right, Side::Left] {
            let sx = if side == Side::Right { 1.0 } else { -1.0 };
            let (wh, tag) = if side == Side::Right { (wheel(&d.wheel, seg), "r") } else { (base.mirrored_x(), "l") };
            let pose = Transform::from_pos(Vec3::new(sx * d.track_m / 2.0, wheel_y, *z));
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
                // the stub from the hub to the wheel well's inner wall, which it overlaps by 2 cm
                let reach = d.track_m / 2.0 - arches.x_n + 0.02; // const-ok: overlap with the well wall
                let k = cylinder_x(tpl.knuckle_radius_m, -reach, 0.0, 16); // const-ok: 16 sides
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
