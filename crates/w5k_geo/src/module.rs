//! Modules and sockets: how a vehicle is assembled from separate parts (a hull, running gear, a weapon mount, a weapon).
//!
//! A **module** is a set of parts in its own frame plus the **sockets** it offers to others, and, if it can be attached, the one socket
//! (`mount`) by which it attaches. A **socket** is a frame (a position and an orientation), a kind (`Station`, `Ring`, `Trunnion`: the
//! standard vocabulary) and a size. Two sockets fit when their kinds are equal and the child's size does not exceed the parent's.
//!
//! The convention for a socket frame: local +Y is the **normal**, pointing out of the module that owns the socket; local -Z is the
//! **reference** direction ("forward" for the vehicle's roof, "up" for a weapon trunnion). Attaching a child puts the child's mount frame
//! on the parent's socket frame with the two normals opposed and the two references together (the child is rotated half a turn about
//! the socket's Z axis). A module authored with its mount normal pointing down (-Y) therefore stands upright on a roof socket, and one
//! authored with its normal pointing back (+Z) points its barrel forward when it sits on a trunnion. A socket on the left of the
//! vehicle takes the mirror image of a module that is not symmetric.
//!
//! Nothing here knows what a hull or a gun is: the families (`hull`, `gear`, `mount`, `weapon`) build modules, and this file composes
//! them, so a new hull can take an old weapon and a new weapon an old mount.

use crate::part::{Part, Side};
use w5k_math::{scalar, Quat, Transform, Vec3};

/// The standard socket vocabulary (extended as families need it: keel, belly, mast, engine bay, track run).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketKind {
    /// A wheel or road-wheel position on a side of the hull; the size is the largest wheel radius it was cut for.
    Station,
    /// A turret or weapon-mount ring; the size is the ring diameter.
    Ring,
    /// The pivot of a weapon on a mount; the size is the width of the cradle that holds it.
    Trunnion,
}

/// An attachment point: a frame in the owning module's frame, a kind and a size, plus named numbers the child may read (the room along
/// the hull, the height above the ground, the wall to keep clear of).
#[derive(Clone, Debug)]
pub struct Socket {
    pub name: String,
    pub kind: SocketKind,
    pub side: Side,
    pub pose: Transform,
    pub size_m: f64,
    pub hints: Vec<(String, f64)>,
}

impl Socket {
    pub fn hint(&self, key: &str) -> Option<f64> {
        self.hints.iter().find(|h| h.0 == key).map(|h| h.1)
    }

    fn mirrored_x(&self) -> Socket {
        let side = match self.side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Centre => Side::Centre,
        };
        Socket { side, pose: mirror_pose(&self.pose), ..self.clone() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Hull,
    Gear,
    Mount,
    Weapon,
}

/// A separable piece of a vehicle.
#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub kind: ModuleKind,
    pub parts: Vec<Part>,
    pub sockets: Vec<Socket>,
    /// The socket by which this module attaches, in its own frame (`None` for a hull).
    pub mount: Option<Socket>,
    /// A module that is its own mirror image is never mirrored when it goes on a left socket.
    pub symmetric: bool,
}

/// Why a module does not fit.
#[derive(Clone, Debug, PartialEq)]
pub struct FitError {
    pub socket: String,
    pub reason: String,
}

impl std::fmt::Display for FitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.socket, self.reason)
    }
}

/// A frame from a position, a normal (local +Y) and a reference direction (local -Z, made square to the normal).
pub fn frame(pos: Vec3, normal: Vec3, reference: Vec3) -> Transform {
    let y = normal.normalized_or_zero();
    let r = reference - y * reference.dot(y);
    assert!(
        y.length_sq() > 0.5 && r.length_sq() > 1e-18,
        "a socket frame needs a normal and a reference that are not parallel"
    ); // const-ok: a zero vector or a parallel pair
    let z = -r.normalized_or_zero();
    let x = y.cross(z);
    Transform::new(pos, quat_from_basis(x, y, z))
}

/// The unit quaternion of the rotation whose columns are the orthonormal right-handed axes `x`, `y`, `z`.
fn quat_from_basis(x: Vec3, y: Vec3, z: Vec3) -> Quat {
    let trace = x.x + y.y + z.z;
    let q = if trace > 0.0 {
        let s = 2.0 * scalar::sqrt(trace + 1.0);
        Quat::new(0.25 * s, (y.z - z.y) / s, (z.x - x.z) / s, (x.y - y.x) / s)
    } else if x.x > y.y && x.x > z.z {
        let s = 2.0 * scalar::sqrt(1.0 + x.x - y.y - z.z);
        Quat::new((y.z - z.y) / s, 0.25 * s, (y.x + x.y) / s, (z.x + x.z) / s)
    } else if y.y > z.z {
        let s = 2.0 * scalar::sqrt(1.0 + y.y - x.x - z.z);
        Quat::new((z.x - x.z) / s, (y.x + x.y) / s, 0.25 * s, (z.y + y.z) / s)
    } else {
        let s = 2.0 * scalar::sqrt(1.0 + z.z - x.x - y.y);
        Quat::new((x.y - y.x) / s, (z.x + x.z) / s, (z.y + y.z) / s, 0.25 * s)
    };
    q.normalized()
}

/// A pose reflected in the plane x = 0.
fn mirror_pose(t: &Transform) -> Transform {
    Transform::new(Vec3::new(-t.pos.x, t.pos.y, t.pos.z), Quat::new(t.rot.w, t.rot.x, -t.rot.y, -t.rot.z))
}

impl Module {
    /// The mirror image: meshes, part frames, sockets and the mount reflected in the plane x = 0 (left becomes right).
    pub fn mirrored_x(&self) -> Module {
        let flip = |s: Side| match s {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Centre => Side::Centre,
        };
        let parts = self
            .parts
            .iter()
            .map(|p| Part { mesh: p.mesh.mirrored_x(), pose: mirror_pose(&p.pose), side: flip(p.side), ..p.clone() })
            .collect();
        Module {
            parts,
            sockets: self.sockets.iter().map(Socket::mirrored_x).collect(),
            mount: self.mount.as_ref().map(Socket::mirrored_x),
            ..self.clone()
        }
    }
}

/// A vehicle being put together: the hull's parts and sockets, then each module placed on a socket.
pub struct Assembly {
    /// Every part, its frame in the hull frame.
    pub parts: Vec<Part>,
    /// Every socket still open or taken, in the hull frame, named `instance.socket` for those offered by an attached module.
    pub sockets: Vec<Socket>,
    taken: Vec<String>,
    /// What was attached where: (instance name, socket name, module name).
    pub log: Vec<(String, String, String)>,
}

impl Assembly {
    pub fn new(hull: Module) -> Assembly {
        for (i, s) in hull.sockets.iter().enumerate() {
            assert!(hull.sockets[..i].iter().all(|o| o.name != s.name), "the hull publishes socket {} twice", s.name);
        }
        Assembly { parts: hull.parts, sockets: hull.sockets, taken: Vec::new(), log: Vec::new() }
    }

    pub fn socket(&self, name: &str) -> Option<&Socket> {
        self.sockets.iter().find(|s| s.name == name)
    }

    /// The open sockets of one kind, in the order the hull published them.
    pub fn open_sockets(&self, kind: SocketKind) -> Vec<&Socket> {
        self.sockets.iter().filter(|s| s.kind == kind && !self.taken.contains(&s.name)).collect()
    }

    /// Put `module` on the socket named `socket`, turned `spin_rad` about the socket's normal (positive is counter-clockwise seen from outside,
    /// so on a roof socket a positive spin turns the module's nose to the left, like positive yaw). Returns the instance name under which the
    /// module's own sockets and parts now appear (`ring_1.trunnion`, ...). The assembly is unchanged when this fails.
    pub fn attach(&mut self, socket: &str, module: &Module, spin_rad: f64) -> Result<String, FitError> {
        let fail = |reason: String| FitError { socket: socket.to_string(), reason };
        let parent = self.socket(socket).ok_or_else(|| fail("no such socket".into()))?.clone();
        if self.taken.iter().any(|t| t == socket) {
            return Err(fail("the socket is already taken".into()));
        }
        let child_mount = module.mount.as_ref().ok_or_else(|| fail(format!("{} has no mount socket", module.name)))?;
        if child_mount.kind != parent.kind {
            return Err(fail(format!(
                "{} mounts on {:?}, this is a {:?} socket",
                module.name, child_mount.kind, parent.kind
            )));
        }
        if child_mount.size_m > parent.size_m + 1e-9 {
            // const-ok: a nanometre of slack for a size that equals the socket's
            return Err(fail(format!(
                "{} needs {:.3} m, the socket offers {:.3} m",
                module.name, child_mount.size_m, parent.size_m
            )));
        }
        let child = if parent.side == Side::Left && !module.symmetric { module.mirrored_x() } else { module.clone() };
        let child_mount = child.mount.as_ref().expect("checked above");
        // The child's mount frame goes on the parent's socket frame, turned half a turn about the socket's Z axis so that the normals oppose
        // and the references agree; `spin` turns it about the normal first.
        let half_turn = Transform::new(Vec3::ZERO, Quat::new(0.0, 0.0, 0.0, 1.0)); // exactly pi about Z, so axis-aligned sockets compose without noise
        let spin = Transform::new(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, spin_rad));
        let placement = parent.pose.compose(&spin).compose(&half_turn).compose(&child_mount.pose.inverse());
        let instance = format!("{}_{}", socket, self.log.len() + 1);
        for p in &child.parts {
            let mut p = p.clone();
            if p.role == w5k_contract::render::NodeRole::Hull {
                // fixed to the hull: the mesh goes into the hull frame
                p.mesh = p.mesh.transformed(&placement.compose(&p.pose));
                p.pose = Transform::IDENTITY;
            } else {
                p.pose = placement.compose(&p.pose);
            }
            p.name = format!("{instance}/{}", p.name);
            self.parts.push(p);
        }
        for s in &child.sockets {
            let mut s = s.clone();
            s.pose = placement.compose(&s.pose);
            s.name = format!("{instance}.{}", s.name);
            self.sockets.push(s);
        }
        self.taken.push(socket.to_string());
        self.log.push((instance.clone(), socket.to_string(), module.name.clone()));
        Ok(instance)
    }
}
