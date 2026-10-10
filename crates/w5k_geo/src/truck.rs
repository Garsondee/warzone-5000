//! The parametric 4x4 utility truck. The hull is one lofted shell (`shapes/utility_4x4.ron`, `body`): its cross-section is a full-width
//! lower body with a narrower hood, cab or bed block on top (the upper walls lean inward, the tumblehome), the underside rises towards the
//! nose and tail (approach and departure ramps), and the wheel arches are openings whose height follows the wheel circle, cut into the
//! section along the loft. Everything else in the part list is anchored to a surface of that shell (its walls, its roof line, its front
//! and rear faces) and overlaps it, so nothing floats; glass is a panel set into the wall with a raised frame around it.
//!
//! The hull is a module (`utility_hull`): it is cut for the axles it is given and publishes one `Station` socket per wheel position. The
//! wheels are gear modules (`gear.rs`) that `utility_truck` attaches to those sockets, so the same hull takes two axles or three.
//!
//! Hull frame: origin at the hull box centre, +Y up, -Z forward, +X right; the ground is `ride_height_m` below the origin.

use crate::flags::FlagParams;
use crate::gear::{wheel_module, Knuckle};
use crate::loft::{bevel_ring, chamfer_polygon, loft_beveled, polygon_ring, sweep_arc, sweep_loop, Section};
use crate::mesh::Mesh;
use crate::module::{frame as socket_frame, Assembly, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::{cylinder_x, segments_for, WheelDims};
use serde::Deserialize;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{scalar, Transform, Vec3};

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
    sockets: Vec<SocketSpec>,
}

/// A socket the hull offers besides the wheel stations: kind and size, a point on the shell by the part list's anchors, the normal and the
/// reference direction of its frame.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SocketSpec {
    name: String,
    kind: SocketKind,
    size_m: f64,
    x: XR,
    y: YR,
    z: ZR,
    normal: (f64, f64, f64),
    reference: (f64, f64, f64),
}

/// One station of the shell's lines plan. Lengths are fractions: `z` of L from the front, half-widths of the nominal half-width, heights
/// of H from the hull box bottom. The lower body runs from the underside `yo` to the belt `yb`; the upper block (hood, cab roof, bed rail)
/// from the belt to the top `yt`, leaning inward by `tilt` metres per metre of height.
#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct StationSpec {
    z: f64,
    w: f64,
    yb: f64,
    wu: f64,
    yt: f64,
    #[serde(default)]
    tilt: f64,
    #[serde(default)]
    yo: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodySpec {
    stations: Vec<StationSpec>,
    /// How far the arch lips stand outside the lower wall: the hull box width includes them.
    lip_proud_m: f64,
    lower_chamfer_m: f64,
    /// The bevel along the bottom outer edge (the sill) and around the arch edge: a larger one tucks the lower body in.
    sill_chamfer_m: f64,
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

/// Where a side window's front edge runs.
#[derive(Deserialize)]
enum PaneFront {
    /// Vertical, at z (a fraction of L).
    Vertical(f64),
    /// Parallel to the roof line between two stations (the A-pillar and the windscreen), `pillar_m` behind it measured square to it, so the
    /// window's slanted edge and the windscreen lean at exactly the same angle.
    Pillar { from_f: f64, to_f: f64, pillar_m: f64 },
}

/// A frame around a window: a band `width_m` wide standing `proud_m` off the wall (the glass is recessed behind it), mitered at the corners.
#[derive(Deserialize)]
struct Bezel {
    width_m: f64,
    proud_m: f64,
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
    /// A side window set into the (leaning) upper wall: a front edge (vertical, or parallel to the A-pillar), a vertical rear edge at
    /// `rear_f`, a bottom edge (fraction of H) and a horizontal top edge `top_offset_m` below the roof line at the rear edge. Corners are
    /// cut by `corner_m`; the glass lies on the wall plane, embedded and proud by the `x` offsets, with an optional frame.
    Pane {
        x: XR,
        front: PaneFront,
        rear_f: f64,
        bottom: f64,
        top_offset_m: f64,
        corner_m: f64,
        bezel: Option<Bezel>,
    },
    /// The glass on the raked plane of the roof line between two stations, `margin_m` short of each end along the slope and `inset_m`
    /// in from the upper walls, with an optional frame.
    Windscreen {
        from_f: f64,
        to_f: f64,
        margin_m: f64,
        inset_m: f64,
        thickness_m: f64,
        proud_m: f64,
        corner_m: f64,
        bezel: Option<Bezel>,
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

/// One station of the shell in metres: lower half-width, belt height, upper half-width at the belt and at the top, top height, the
/// height of the underside and the lean of the upper wall.
#[derive(Clone, Copy)]
struct St {
    z: f64,
    xl: f64,
    yb: f64,
    xu: f64,
    xt: f64,
    yt: f64,
    yo: f64,
    tilt: f64,
}

/// The shell's lines: stations interpolated linearly in z (as the loft does between sections).
struct Frame {
    st: Vec<St>,
    dims: UtilityDims,
}

impl Frame {
    fn new(d: &UtilityDims, b: &BodySpec) -> Frame {
        let half = d.width_m / 2.0 - b.lip_proud_m;
        let st = b
            .stations
            .iter()
            .map(|s| {
                let (yb, yt) = ((s.yb - 0.5) * d.height_m, (s.yt - 0.5) * d.height_m);
                let xu = s.wu * half;
                St {
                    z: (s.z - 0.5) * d.length_m,
                    xl: s.w * half,
                    yb,
                    xu,
                    xt: xu - s.tilt * (yt - yb),
                    yt,
                    yo: (s.yo - 0.5) * d.height_m,
                    tilt: s.tilt,
                }
            })
            .collect();
        Frame { st, dims: *d }
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
        St {
            z,
            xl: l(p.xl, q.xl),
            yb: l(p.yb, q.yb),
            xu: l(p.xu, q.xu),
            xt: l(p.xt, q.xt),
            yt: l(p.yt, q.yt),
            yo: l(p.yo, q.yo),
            tilt: l(p.tilt, q.tilt),
        }
    }

    /// The x of the upper wall at height y (it leans inward from the belt to the top).
    fn wall_x(&self, z: f64, y: f64) -> f64 {
        let s = self.at(z);
        let t = ((y - s.yb) / (s.yt - s.yb).max(1e-9)).clamp(0.0, 1.0); // const-ok: guards a zero-height block
        s.xu + (s.xt - s.xu) * t
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

    fn y(&self, r: &YR, z: f64) -> (f64, f64) {
        let s = self.at(z);
        match *r {
            YR::Frac(a, b) => ((a - 0.5) * self.dims.height_m, (b - 0.5) * self.dims.height_m),
            YR::Belt(a, b) => (s.yb + a, s.yb + b),
            YR::Top(a, b) => (s.yt + a, s.yt + b),
        }
    }

    /// x range at station z and height y (the upper wall leans, so its x depends on the height).
    fn x(&self, r: &XR, z: f64, y: f64) -> (f64, f64) {
        match *r {
            XR::Frac(a, b) => (a * self.dims.width_m, b * self.dims.width_m),
            XR::Upper(a, b) => (self.wall_x(z, y) + a, self.wall_x(z, y) + b),
            XR::Lower(a, b) => (self.at(z).xl + a, self.at(z).xl + b),
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

/// A flat panel: the polygon `outline` (u, v) with its corners cut by `corner_m`, extruded from w = `w0` to `w1`, returned as local
/// points (u, v, w) for the caller to place.
fn panel(outline: &[[f64; 2]], corner_m: f64, w0: f64, w1: f64, band_m: f64) -> Mesh {
    let ring = polygon_ring(outline, &vec![corner_m; outline.len()], band_m);
    loft_beveled(
        &[Section { z_m: w0, ring: ring.clone() }, Section { z_m: w1, ring }],
        corner_m,
        band_m,
        &[],
        // const-ok: a panel is one strip
        10.0,
    )
}

/// The frame around a panel: the outline at `to_world(u, v, 0)`, swept with a chamfered band profile that starts 4 mm under the glass edge
/// (so there is no gap) and stands `proud_m` off the surface; it is embedded 2 cm into the wall.
fn bezel(
    outline: &[[f64; 2]],
    corner_m: f64,
    b: &Bezel,
    normal: Vec3,
    to_world: &dyn Fn(f64, f64, f64) -> Vec3,
) -> Mesh {
    let pts: Vec<Vec3> = chamfer_polygon(outline, corner_m).iter().map(|p| to_world(p[0], p[1], 0.0)).collect();
    let (s0, s1, e, p, c) = (-0.004, b.width_m - 0.004, 0.02, b.proud_m, b.proud_m / 2.0); // const-ok: 4 mm glass overlap, 2 cm embed, chamfer half the relief
    sweep_loop(&pts, normal, &[[s0, -e], [s1, -e], [s1, p - c], [s1 - c, p], [s0 + c, p], [s0, p - c]])
}

/// Where the arches are: the wheel circle's centre in the hull frame, the arch radius, the arch's half angle at the body bottom, the
/// station z of each axle and the x of the wheel well's inner wall.
struct Arches {
    y_c: f64,
    r_a: f64,
    phi_m: f64,
    axle_z: Vec<f64>,
    x_n: f64,
}

impl Arches {
    fn new(d: &UtilityDims, b: &BodySpec, axles_z: &[f64]) -> Arches {
        let r_a = d.wheel.outer_radius_m + b.arch_gap_m;
        let y_c = -d.ride_height_m() + d.wheel.outer_radius_m;
        let dy = -d.height_m / 2.0 - y_c;
        let half_chord = scalar::sqrt((r_a * r_a - dy * dy).max(0.0));
        Arches {
            y_c,
            r_a,
            phi_m: scalar::asin((half_chord / r_a).min(1.0)),
            axle_z: axles_z.to_vec(),
            x_n: d.track_m / 2.0 - d.wheel.width_m / 2.0 - b.well_gap_m,
        }
    }

    /// Height of the arch above the underside `y_o` at station z (0 outside it).
    fn height(&self, z: f64, y_o: f64) -> f64 {
        self.axle_z
            .iter()
            .map(|&zc| {
                let dz = z - zc;
                if dz.abs() < self.r_a {
                    self.y_c + scalar::sqrt(self.r_a * self.r_a - dz * dz) - y_o
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
    for &zc in &a.axle_z {
        for k in 0..=b.arch_samples {
            let phi = a.phi_m * (2.0 * f64::from(k) / f64::from(b.arch_samples) - 1.0);
            zs.push(zc + a.r_a * scalar::sin(phi));
        }
    }
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|p, q| (*p - *q).abs() < 1e-4); // const-ok: sections closer than 0.1 mm are one
    let (cl, cs, ct) = (b.lower_chamfer_m, b.sill_chamfer_m, b.upper_chamfer_m);
    let chamfers = [cl, cl, 0.0, cs, cl, 0.0, ct, ct, 0.0, cl, cs, 0.0];
    let sections: Vec<Section> = zs
        .iter()
        .map(|&z| {
            let s = f.at(z);
            let y0 = s.yo;
            let h = a.height(z, y0).clamp(b.min_notch_m, (s.yb - y0 - b.min_wall_m).max(b.min_notch_m));
            let (xn, xl, xu, xt) = (a.x_n, s.xl, s.xu, s.xt);
            let poly = [
                [-xn, y0],
                [xn, y0],
                [xn, y0 + h],
                [xl, y0 + h],
                [xl, s.yb],
                [xu, s.yb],
                [xt, s.yt],
                [-xt, s.yt],
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

fn template() -> Template {
    ron::from_str(include_str!("../shapes/utility_4x4.ron")).expect("utility_4x4.ron parses")
}

/// The 4x4: the utility hull cut for two axles, a steered front axle and a fixed rear one.
pub fn utility_4x4(d: &UtilityDims, detail: u8) -> Vec<Part> {
    utility_truck(d, &[-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0], &[true, false], detail)
}

/// The utility hull on any number of axles (`axles_z`: the z of each in the hull frame, front first; `steered[i]` fits axle `i` with a
/// steering knuckle): the hull module with a wheel module attached to every `Station` socket it published.
pub fn utility_truck(d: &UtilityDims, axles_z: &[f64], steered: &[bool], detail: u8) -> Vec<Part> {
    utility_assembly(d, axles_z, steered, detail).parts
}

/// The same, still an assembly: its open sockets (the roof ring) take a mount, the mount's trunnion a weapon.
pub fn utility_assembly(d: &UtilityDims, axles_z: &[f64], steered: &[bool], detail: u8) -> Assembly {
    let mut asm = Assembly::new(utility_hull(d, axles_z, detail));
    let stations: Vec<Socket> = asm.open_sockets(SocketKind::Station).into_iter().cloned().collect();
    for s in &stations {
        let axle = s.station.unwrap_or(0);
        let tag = if s.side == Side::Right { "r" } else { "l" };
        let knuckle = steered.get(usize::from(axle)).copied().unwrap_or(false).then(|| Knuckle {
            radius_m: template().knuckle_radius_m,
            // the stub from the hub to the wheel well's inner wall, which it overlaps by 2 cm
            reach_m: s.pose.pos.x.abs() - s.hint("well_x_m").unwrap_or(0.0) + 0.02, // const-ok: overlap with the well wall
        });
        let wheel = wheel_module(&d.wheel, segments_for(detail), knuckle);
        asm.attach(&s.name, &wheel, 0.0, &format!("{axle}.{tag}")).expect("the wheel was cut for exactly this station");
    }
    asm
}

/// The hull module: the shell with an arch cut for every axle, every anchored fitting and the arch lips, and one `Station` socket per wheel
/// position (named `station.<axle>.<r|l>`, at the hub, normal outward, sized for the wheel the arch was cut for; hints `well_x_m`, the x of
/// the wheel well's inner wall, and `max_width_m`, the widest tyre it takes).
pub fn utility_hull(d: &UtilityDims, axles_z: &[f64], detail: u8) -> Module {
    let tpl = template();
    let band = FlagParams::default_params().edge_band_m;
    let frame = Frame::new(d, &tpl.body);
    let arches = Arches::new(d, &tpl.body, axles_z);
    let (w, h, l) = (d.width_m, d.height_m, d.length_m);
    let (fx, fy, fz) = (|f: f64| f * w, |f: f64| (f - 0.5) * h, |f: f64| (f - 0.5) * l);
    let sides = |mirror: bool| if mirror { vec![Side::Right, Side::Left] } else { vec![Side::Centre] };
    let finish = Mesh::finished;
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
    let mut add = |spec: &PartSpec, name: String, slot: SlotKind, side: Side, station: Option<u8>, mesh: Mesh| {
        let mesh = if side == Side::Left { mesh.mirrored_x() } else { mesh };
        parts.push(Part {
            name,
            role: spec.role,
            station,
            side,
            slot,
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
                    add(spec, name, spec.slot, side, None, shaped(&secs, *corner_m, band));
                }
                Shape::Block { x, y, z, corner_m } => {
                    let (z0, z1) = frame.z(z);
                    let zm = (z0 + z1) / 2.0;
                    let (y0, y1) = frame.y(y, zm);
                    let (x0, x1) = frame.x(x, zm, (y0 + y1) / 2.0);
                    let (bw, bh, bl) = (x1 - x0, y1 - y0, z1 - z0);
                    let m = shaped(
                        &[(-bl / 2.0, bw, -bh / 2.0, bh / 2.0), (bl / 2.0, bw, -bh / 2.0, bh / 2.0)],
                        *corner_m,
                        band,
                    );
                    let at = Vec3::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, zm);
                    add(spec, name, spec.slot, side, None, m.transformed(&Transform::from_pos(at)));
                }
                Shape::Cyl { axis, x, y, z, radius_m } => {
                    let (z0, z1) = frame.z(z);
                    let zm = (z0 + z1) / 2.0;
                    let (y0, y1) = frame.y(y, zm);
                    let (x0, x1) = frame.x(x, zm, (y0 + y1) / 2.0);
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
                    add(spec, name, spec.slot, side, None, m);
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
                            (zz, 2.0 * (s.xt - inset_m), s.yt - embed_m, s.yt + proud_m)
                        })
                        .collect();
                    add(spec, name, spec.slot, side, None, shaped(&secs, *corner_m, band));
                }
                Shape::Pane { x, front, rear_f, bottom, top_offset_m, corner_m, bezel: frame_spec } => {
                    let z_rear = fz(*rear_f);
                    let y_sill = fy(*bottom);
                    let y_top = frame.at(z_rear).yt - top_offset_m;
                    // the front edge at the sill and at the top edge
                    let (z_fb, z_ft) = match front {
                        PaneFront::Vertical(f) => (fz(*f), fz(*f)),
                        PaneFront::Pillar { from_f, to_f, pillar_m } => {
                            let (za, zb) = (fz(*from_f), fz(*to_f));
                            let (ya, yb) = (frame.at(za).yt, frame.at(zb).yt);
                            let (dz, dy) = (zb - za, yb - ya);
                            let len = scalar::hypot(dz, dy);
                            // the roof-line ramp moved `pillar_m` into the cab, square to itself, then cut by the sill and top lines
                            let (az, ay) = (za + pillar_m * dy / len, ya - pillar_m * dz / len);
                            let at_y = |y: f64| az + (y - ay) / dy * dz;
                            (at_y(y_sill), at_y(y_top).min(z_rear - 0.05)) // const-ok: the top edge keeps 5 cm of length
                        }
                    };
                    let zm = (z_fb.min(z_ft) + z_rear) / 2.0;
                    let ym = (y_sill + y_top) / 2.0;
                    let (g0, g1) = frame.x(x, zm, ym); // glass: from embedded to proud of the wall at mid height
                    let wall = frame.wall_x(zm, ym);
                    let tilt = frame.at(zm).tilt;
                    // local (u, v) = (to the right as seen from outside, up), counter-clockwise as seen from outside
                    let outline = [
                        [zm - z_rear, y_sill - ym],
                        [zm - z_fb, y_sill - ym],
                        [zm - z_ft, y_top - ym],
                        [zm - z_rear, y_top - ym],
                    ];
                    let to_world = |u: f64, v: f64, wn: f64| Vec3::new(wall + wn - tilt * v, ym + v, zm - u);
                    let glass = panel(&outline, *corner_m, g0 - wall, g1 - wall, band);
                    let world =
                        |m: &Mesh| Mesh { v: m.v.iter().map(|p| to_world(p.x, p.y, p.z)).collect(), t: m.t.clone() };
                    add(spec, name.clone(), spec.slot, side, None, world(&glass));
                    if let Some(b) = frame_spec {
                        let n = Vec3::new(1.0, tilt, 0.0).normalized_or_zero();
                        add(
                            spec,
                            format!("{name}_frame"),
                            SlotKind::Paint,
                            side,
                            None,
                            bezel(&outline, *corner_m, b, n, &to_world),
                        );
                    }
                }
                Shape::Windscreen {
                    from_f,
                    to_f,
                    margin_m,
                    inset_m,
                    thickness_m,
                    proud_m,
                    corner_m,
                    bezel: frame_spec,
                } => {
                    let (z0, z1) = (fz(*from_f), fz(*to_f));
                    let (s0, s1) = (frame.at(z0), frame.at(z1));
                    let (dy, dz) = (s1.yt - s0.yt, z1 - z0);
                    let len = scalar::hypot(dy, dz);
                    let half_w = frame.at((z0 + z1) / 2.0).xt - inset_m;
                    let half_l = len / 2.0 - margin_m;
                    // the plane of the roof line: origin at its middle, v up the slope, w along the normal (up and forward)
                    let mid = Vec3::new(0.0, (s0.yt + s1.yt) / 2.0, (z0 + z1) / 2.0);
                    let (slope, n) = (Vec3::new(0.0, dy / len, dz / len), Vec3::new(0.0, dz / len, -dy / len));
                    let outline = [[-half_w, -half_l], [half_w, -half_l], [half_w, half_l], [-half_w, half_l]];
                    let to_world = |u: f64, v: f64, wn: f64| mid + Vec3::X * u + slope * v + n * wn;
                    let glass = panel(&outline, *corner_m, proud_m - thickness_m, *proud_m, band);
                    let world =
                        |m: &Mesh| Mesh { v: m.v.iter().map(|p| to_world(p.x, p.y, p.z)).collect(), t: m.t.clone() };
                    add(spec, name.clone(), spec.slot, side, None, world(&glass));
                    if let Some(b) = frame_spec {
                        add(
                            spec,
                            format!("{name}_frame"),
                            SlotKind::Paint,
                            side,
                            None,
                            bezel(&outline, *corner_m, b, n, &to_world),
                        );
                    }
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
                            // the lip runs from where the arch meets the underside in front of the wheel to where it meets it behind
                            // (the underside rises towards the nose and tail, so the two ends differ)
                            let reach = |sign: f64| {
                                let ok = |a: f64| {
                                    let z = zc + sign * arches.r_a * scalar::sin(a);
                                    // const-ok: the arch keeps 3 cm of height at the lip's end
                                    arches.y_c + arches.r_a * scalar::cos(a) - frame.at(z).yo > 0.03
                                };
                                let step = arches.phi_m / 90.0; // const-ok: 90 steps to search for the end
                                (0..=90).map(|k| f64::from(k) * step).take_while(|&a| ok(a)).last().unwrap_or(0.0)
                            };
                            let (a_front, a_rear) = (reach(-1.0), reach(1.0));
                            let n = 18; // const-ok: steps along the arch: a 9 degree step keeps the sag under 3 mm
                            let angles: Vec<f64> =
                                (0..=n).map(|k| -a_front + (a_front + a_rear) * f64::from(k) / f64::from(n)).collect();
                            let m = sweep_arc(&ring, (arches.y_c, *zc), &angles);
                            let n =
                                format!("{}.{}.{}", spec.name, axle, if lip_side == Side::Right { "r" } else { "l" });
                            add(spec, n, spec.slot, lip_side, Some(axle as u8), m);
                        }
                    }
                    break;
                }
            }
        }
    }
    // one Station socket per wheel position: at the hub, normal outward, sized for the wheel the arch was cut for
    let mut sockets = Vec::new();
    for (axle, &z) in arches.axle_z.iter().enumerate() {
        for side in [Side::Right, Side::Left] {
            let sx = if side == Side::Right { 1.0 } else { -1.0 };
            let hub = Vec3::new(sx * d.track_m / 2.0, arches.y_c, z);
            sockets.push(Socket {
                name: format!("station.{axle}.{}", if side == Side::Right { "r" } else { "l" }),
                kind: SocketKind::Station,
                side,
                pose: socket_frame(hub, Vec3::new(sx, 0.0, 0.0), -Vec3::Z),
                size_m: d.wheel.outer_radius_m,
                station: Some(axle as u8),
                hints: vec![("well_x_m".into(), arches.x_n), ("max_width_m".into(), d.wheel.width_m)],
            });
        }
    }
    for spec in &tpl.sockets {
        let (z0, z1) = frame.z(&spec.z);
        let zm = (z0 + z1) / 2.0;
        let (y0, y1) = frame.y(&spec.y, zm);
        let ym = (y0 + y1) / 2.0;
        let (x0, x1) = frame.x(&spec.x, zm, ym);
        let (n, r) = (spec.normal, spec.reference);
        sockets.push(Socket {
            name: spec.name.clone(),
            kind: spec.kind,
            side: Side::Centre,
            pose: socket_frame(Vec3::new((x0 + x1) / 2.0, ym, zm), Vec3::new(n.0, n.1, n.2), Vec3::new(r.0, r.1, r.2)),
            size_m: spec.size_m,
            station: None,
            hints: Vec::new(),
        });
    }
    Module { name: "utility_hull".into(), kind: ModuleKind::Hull, parts, sockets, mount: None, symmetric: true }
}
