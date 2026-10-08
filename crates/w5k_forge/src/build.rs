//! Turn a part's shape tree into convex pieces in part space.

use crate::convex::{Convex, Polyhedron};
use crate::geom::{v3, Xform, M3, V3};
use crate::schema::{Axis, Motion, Node, PartDef, Slot};

/// Material name of pieces that are drawn but are not part of the vehicle: rails and sleepers under a train,
/// for example. They have no mass or armour and do not count toward bounds or the voxel grid.
pub const SCENERY: &str = "scenery";

/// Bevel only edges sharper than this (so cylinder side segments stay crisp facets).
const BEVEL_MIN_ANGLE: f64 = 40.0;

/// A moving sub-assembly of a part, in part space (see `Node::Joint`).
#[derive(Clone, Debug)]
pub struct JointDef {
    pub name: String,
    pub pivot: V3,
    pub axis: V3,
    pub motion: Motion,
    /// The joint this one hangs from (an index into the part's joint list), if any.
    pub parent: Option<u16>,
}

impl Motion {
    /// The same motion with its points and lengths carried through a transform (positions move, lengths scale).
    pub fn transformed(&self, x: &Xform) -> Motion {
        let k = x.m.det().abs().cbrt();
        let pt = |p: &[f64; 3]| x.point(V3::from_arr(*p)).arr();
        match self {
            Motion::Roll { radius } => Motion::Roll { radius: radius * k },
            Motion::Spin { rps } => Motion::Spin { rps: *rps },
            Motion::Hip { foot, stride } => Motion::Hip { foot: pt(foot), stride: stride * k },
            Motion::Knee { foot, lift } => Motion::Knee { foot: pt(foot), lift: lift * k },
        }
    }
}

/// One convex piece of a part or vehicle.
#[derive(Clone, Debug)]
pub struct Piece {
    pub convex: Convex,
    pub poly: Polyhedron,
    pub mat: String,
    pub slot: Slot,
    /// Armour shell thickness (m). `None` means the piece is solid material all the way through.
    pub shell: Option<f64>,
    /// Which part this piece belongs to (index into a vehicle's part list; 0 for a single part).
    pub part: u16,
    /// Whether the inside of this piece's shell is vital space (see `PartDef::vital`).
    pub vital: bool,
    /// The moving sub-assembly this piece belongs to: zero for a piece that does not move, else one more than its index in the
    /// part's joint list (or, in an assembly, in the vehicle's).
    pub joint: u16,
}

impl Piece {
    pub fn is_scenery(&self) -> bool {
        self.mat == SCENERY
    }

    fn new(points: Vec<V3>, chamfer: f64, mat: &str, slot: Slot, shell: Option<f64>) -> Option<Piece> {
        let base = Convex::from_points(&points);
        if base.planes.len() < 4 {
            return None;
        }
        let convex = base.beveled(chamfer, BEVEL_MIN_ANGLE);
        let poly = convex.polyhedron();
        if poly.faces.len() < 4 {
            return None;
        }
        Some(Piece { convex, poly, mat: mat.to_string(), slot, shell, part: 0, vital: true, joint: 0 })
    }

    /// The inset region of a shell piece (every face moved inward by the shell thickness). For a convex shape
    /// this is exactly the set of points deeper than the shell. `None` for solid pieces.
    pub fn inner(&self) -> Option<Convex> {
        let s = self.shell?;
        Some(Convex {
            planes: self.convex.planes.iter().map(|p| crate::geom::Plane { n: p.n, d: p.d - s }).collect(),
            kinds: self.convex.kinds.clone(),
        })
    }

    /// Exact volume of material (the whole piece, or the shell between outside and inset) and its centroid.
    pub fn material_volume(&self) -> (f64, V3) {
        let (vo, co) = self.poly.volume_centroid();
        let Some(inner) = self.inner() else { return (vo, co) };
        let (vi, ci) = inner.polyhedron().volume_centroid();
        let v = vo - vi;
        if v <= 1e-15 {
            return (0.0, co);
        }
        (v, (co * vo - ci * vi) / v)
    }

    /// The piece moved by an affine transform (used when assembling vehicles).
    pub fn transformed(&self, x: &Xform, part: u16) -> Piece {
        let convex = self.convex.transformed(x);
        let poly = convex.polyhedron();
        Piece {
            convex,
            poly,
            mat: self.mat.clone(),
            slot: self.slot,
            shell: self.shell.map(|s| s * x.m.det().abs().cbrt()),
            part,
            vital: self.vital,
            joint: self.joint,
        }
    }
}

fn local(at: [f64; 3], rot: [f64; 3]) -> Xform {
    Xform::new(M3::euler_deg(rot), V3::from_arr(at))
}

fn axis_vec(a: Axis) -> V3 {
    match a {
        Axis::X => v3(1.0, 0.0, 0.0),
        Axis::Y => v3(0.0, 1.0, 0.0),
        Axis::Z => v3(0.0, 0.0, 1.0),
    }
}

/// Points of a (possibly tapered and shifted) box centred on the origin.
pub fn box_points(size: [f64; 3], taper: [f64; 2], shift: [f64; 2]) -> Vec<V3> {
    let (hx, hy, hz) = (size[0] / 2.0, size[1] / 2.0, size[2] / 2.0);
    let mut p = Vec::with_capacity(8);
    for &sx in &[-1.0, 1.0] {
        for &sz in &[-1.0, 1.0] {
            p.push(v3(sx * hx, -hy, sz * hz));
            p.push(v3(sx * hx * taper[0] + shift[0], hy, sz * hz * taper[1] + shift[1]));
        }
    }
    p
}

/// Points of a wedge: the top front edge (toward -Z) slopes down to `nose` x height over `slope` x length.
pub fn wedge_points(size: [f64; 3], slope: f64, nose: f64) -> Vec<V3> {
    let (hx, hy, hz) = (size[0] / 2.0, size[1] / 2.0, size[2] / 2.0);
    let slope = slope.clamp(0.0, 1.0);
    let nose = nose.clamp(0.0, 1.0);
    let z_start = -hz + slope * size[2]; // where the slope begins (front is -Z)
    let y_nose = -hy + nose * size[1];
    let mut p = Vec::new();
    for &sx in &[-1.0, 1.0] {
        p.push(v3(sx * hx, -hy, -hz));
        p.push(v3(sx * hx, -hy, hz));
        p.push(v3(sx * hx, hy, hz));
        p.push(v3(sx * hx, hy, z_start));
        p.push(v3(sx * hx, y_nose, -hz));
    }
    p
}

/// Points of a faceted cylinder along +Y (centred), far end (+Y) radius scaled by `taper`.
pub fn cylinder_points(radius: f64, length: f64, segments: u32, taper: f64) -> Vec<V3> {
    let n = segments.max(3);
    let mut p = Vec::new();
    for i in 0..n {
        // Half-step phase so a flat face, not an edge, points forward and up for most segment counts.
        let a = (i as f64 + 0.5) / n as f64 * std::f64::consts::TAU;
        let (s, c) = a.sin_cos();
        p.push(v3(c * radius, -length / 2.0, s * radius));
        p.push(v3(c * radius * taper, length / 2.0, s * radius * taper));
    }
    p
}

pub fn sphere_points(radius: f64, segments: u32, scale: [f64; 3]) -> Vec<V3> {
    let lon = segments.max(4);
    let lat = (segments / 2).max(2);
    let mut p = vec![v3(0.0, radius * scale[1], 0.0), v3(0.0, -radius * scale[1], 0.0)];
    for j in 1..lat {
        let phi = j as f64 / lat as f64 * std::f64::consts::PI;
        let (sp, cp) = phi.sin_cos();
        for i in 0..lon {
            let th = (i as f64 + 0.5 * (j % 2) as f64) / lon as f64 * std::f64::consts::TAU;
            let (st, ct) = th.sin_cos();
            p.push(v3(radius * sp * ct * scale[0], radius * cp * scale[1], radius * sp * st * scale[2]));
        }
    }
    p
}

/// Points of a beam from `a` to `b`: a box section (width, height) at each end, height along `up`.
pub fn beam_points(a: V3, b: V3, start: [f64; 2], end: [f64; 2], up: V3) -> Vec<V3> {
    let u = (b - a).norm();
    let mut h = up - u * up.dot(u);
    if h.len() < 1e-9 {
        h = u.any_perp();
    }
    let h = h.norm();
    let w = u.cross(h);
    let mut p = Vec::with_capacity(8);
    for (c, s) in [(a, start), (b, end)] {
        for sw in [-0.5, 0.5] {
            for sh in [-0.5, 0.5] {
                p.push(c + w * (sw * s[0]) + h * (sh * s[1]));
            }
        }
    }
    p
}

/// What building a shape tree collects: the pieces, the joints, and the joint the node being built sits in.
#[derive(Default)]
struct Builder {
    out: Vec<Piece>,
    joints: Vec<JointDef>,
    current: u16,
}

/// Build all pieces of a part, and the moving sub-assemblies they belong to.
pub fn build_part(def: &PartDef) -> (Vec<Piece>, Vec<JointDef>) {
    let mut b = Builder::default();
    for n in &def.shapes {
        build_node(n, &Xform::IDENTITY, &mut b);
    }
    let vital = def.vital_interior();
    for p in &mut b.out {
        p.vital = vital;
    }
    (b.out, b.joints)
}

fn emit(points: Vec<V3>, x: &Xform, chamfer: f64, mat: &str, slot: Slot, shell: Option<f64>, b: &mut Builder) {
    let scale = x.m.det().abs().cbrt();
    let pts: Vec<V3> = points.into_iter().map(|p| x.point(p)).collect();
    if let Some(mut piece) = Piece::new(pts, chamfer * scale, mat, slot, shell.map(|s| s * scale)) {
        piece.joint = b.current;
        b.out.push(piece);
    }
}

fn build_node(node: &Node, parent: &Xform, out: &mut Builder) {
    match node {
        Node::Box { size, taper, shift, at, rot, mat, slot, shell, chamfer } => {
            let x = parent.compose(&local(*at, *rot));
            emit(box_points(*size, taper.unwrap_or([1.0, 1.0]), shift.unwrap_or([0.0, 0.0])), &x, *chamfer, mat, *slot, *shell, out);
        }
        Node::Wedge { size, slope, nose, at, rot, mat, slot, shell, chamfer } => {
            let x = parent.compose(&local(*at, *rot));
            emit(wedge_points(*size, *slope, *nose), &x, *chamfer, mat, *slot, *shell, out);
        }
        Node::Cylinder { radius, length, axis, segments, taper, at, rot, mat, slot, shell, chamfer } => {
            // Built along +Y, then turned onto the requested axis.
            let to_axis = match axis {
                Axis::Y => M3::IDENTITY,
                Axis::X => M3::axis_angle(v3(0.0, 0.0, 1.0), -90.0),
                Axis::Z => M3::axis_angle(v3(1.0, 0.0, 0.0), 90.0),
            };
            let x = parent.compose(&local(*at, *rot)).compose(&Xform::new(to_axis, V3::ZERO));
            emit(cylinder_points(*radius, *length, *segments, *taper), &x, *chamfer, mat, *slot, *shell, out);
        }
        Node::Sphere { radius, segments, scale, at, rot, mat, slot, shell, chamfer } => {
            let x = parent.compose(&local(*at, *rot));
            emit(sphere_points(*radius, *segments, scale.unwrap_or([1.0, 1.0, 1.0])), &x, *chamfer, mat, *slot, *shell, out);
        }
        Node::Beam { from, to, size, end, up, mat, slot, shell, chamfer } => {
            let pts = beam_points(V3::from_arr(*from), V3::from_arr(*to), *size, end.unwrap_or(*size), V3::from_arr(*up));
            emit(pts, parent, *chamfer, mat, *slot, *shell, out);
        }
        Node::Hull { points, at, rot, mat, slot, shell, chamfer } => {
            let x = parent.compose(&local(*at, *rot));
            emit(points.iter().map(|p| V3::from_arr(*p)).collect(), &x, *chamfer, mat, *slot, *shell, out);
        }
        Node::Joint { name, pivot, axis, motion, children } => {
            let parent_joint = if out.current > 0 { Some(out.current - 1) } else { None };
            out.joints.push(JointDef {
                name: name.clone(),
                pivot: parent.point(V3::from_arr(*pivot)),
                axis: parent.dir(V3::from_arr(*axis)).norm(),
                motion: motion.transformed(parent),
                parent: parent_joint,
            });
            let saved = out.current;
            out.current = out.joints.len() as u16;
            for c in children {
                build_node(c, parent, out);
            }
            out.current = saved;
        }
        Node::Group { at, rot, scale, children } => {
            let x = parent.compose(&Xform::new(M3::euler_deg(*rot) * M3::scale(v3(*scale, *scale, *scale)), V3::from_arr(*at)));
            for c in children {
                build_node(c, &x, out);
            }
        }
        Node::Mirror { axis, keep, children } => {
            let mut s = v3(1.0, 1.0, 1.0);
            match axis {
                Axis::X => s.x = -1.0,
                Axis::Y => s.y = -1.0,
                Axis::Z => s.z = -1.0,
            }
            let mirrored = parent.compose(&Xform::new(M3::scale(s), V3::ZERO));
            for c in children {
                if *keep {
                    build_node(c, parent, out);
                }
                build_node(c, &mirrored, out);
            }
        }
        Node::Array { count, step, children } => {
            for i in 0..*count {
                let x = parent.compose(&Xform::new(M3::IDENTITY, V3::from_arr(*step) * i as f64));
                for c in children {
                    build_node(c, &x, out);
                }
            }
        }
        Node::Radial { count, axis, phase, children } => {
            let n = (*count).max(1);
            for i in 0..n {
                let ang = phase + 360.0 * i as f64 / n as f64;
                let x = parent.compose(&Xform::new(M3::axis_angle(axis_vec(*axis), ang), V3::ZERO));
                for c in children {
                    build_node(c, &x, out);
                }
            }
        }
    }
}
