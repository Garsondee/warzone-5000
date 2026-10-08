//! Forward kinematics and mass bookkeeping of a [`PhysRig`], written once so that every consumer (COMBAT for the muzzle pose, the glue for the
//! reaction wrench, FORGE for the composite centre of mass, VALIDATION for the dossier's COM height, the viewers' debug draw) agrees to the
//! millimetre. Pure functions of the rig and the articulation coordinates `qs` (one per `PhysRig::articulation` joint, rad or m).

use w5k_math::{Mat3, Quat, Transform, Vec3};

use crate::rig::{BodyDef, JointDef, JointKind, PhysRig};

/// A rigid body (a joint's whole subtree frozen at some coordinates) as mass, centre of mass and inertia about it, in the frame of the
/// subtree's root joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    pub mass_kg: f64,
    pub com_m: Vec3,
    pub inertia_kg_m2: Mat3,
}

/// Mass and centre of mass (hull frame) of the whole vehicle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composite {
    pub mass_kg: f64,
    pub com_m: Vec3,
}

/// Parallel-axis shift: the inertia about a point displaced by `d` from the centre of mass.
pub fn parallel_axis(i_com: &Mat3, mass_kg: f64, d: Vec3) -> Mat3 {
    let dd = d.dot(d);
    let shift = Mat3::diagonal(dd, dd, dd).add(&Mat3::outer(d, d).scaled(-1.0)).scaled(mass_kg);
    i_com.add(&shift)
}

/// Inertia of a rigid body about the line through the frame origin along the unit `axis` (what a servo on that axis has to move).
pub fn inertia_about_axis(r: &Rigid, axis: Vec3) -> f64 {
    let i_origin = parallel_axis(&r.inertia_kg_m2, r.mass_kg, r.com_m);
    axis.dot(i_origin.mul_vec(axis))
}

impl PhysRig {
    /// Index of articulation joint 0 in `VehicleFrame::joints` (spin, then steer of steered stations, then travel, then articulation).
    pub fn articulation_offset(&self) -> usize {
        2 * self.stations.len() + self.stations.iter().filter(|s| s.steer.is_some()).count()
    }

    /// The frame of the body carried by `j` in its parent's frame at joint coordinate `q`.
    pub fn joint_in_parent(j: &JointDef, q: f64) -> Transform {
        match j.kind {
            JointKind::Revolute => Transform::new(j.anchor_m, Quat::from_axis_angle(j.axis, q)),
            JointKind::Prismatic => Transform::from_pos(j.anchor_m + j.axis * q),
        }
    }

    /// Frame of an articulated body (`Some(joint)`) or of the hull (`None`) in the hull frame, for the coordinates `qs`.
    pub fn body_pose_in_hull(&self, body: Option<usize>, qs: &[f64]) -> Transform {
        match body {
            None => Transform::IDENTITY,
            Some(j) => {
                let jd = &self.articulation[j];
                let local = PhysRig::joint_in_parent(jd, qs[j]);
                match jd.parent {
                    None => local,
                    Some(p) => self.body_pose_in_hull(Some(p), qs).compose(&local),
                }
            }
        }
    }

    /// Muzzle pose in the hull frame (the bore is -Z of the returned rotation).
    pub fn muzzle_pose_in_hull(&self, muzzle: usize, qs: &[f64]) -> Transform {
        let m = &self.muzzles[muzzle];
        self.body_pose_in_hull(m.joint, qs).compose(&m.pose)
    }

    /// Mass and centre of mass of the whole vehicle: the hull, every articulated body at the coordinates `qs`, and each station's unsprung
    /// mass at its rest position. (Belt mass has no position in the contract; the top run is inside the hull, the ground run rests on the ground.)
    pub fn composite_mass(&self, qs: &[f64]) -> Composite {
        let mut m = self.hull.mass_kg;
        let mut mc = self.hull.com_m * self.hull.mass_kg;
        for s in &self.stations {
            m += s.unsprung_mass_kg;
            mc += s.rest_pos_m * s.unsprung_mass_kg;
        }
        for (i, j) in self.articulation.iter().enumerate() {
            let t = self.body_pose_in_hull(Some(i), qs);
            m += j.body.mass_kg;
            mc += t.apply_point(j.body.com_m) * j.body.mass_kg;
        }
        Composite { mass_kg: m, com_m: mc * (1.0 / m) }
    }

    /// The mass the springs carry: the hull and every articulated body (the stations' own unsprung mass hangs below the springs).
    pub fn sprung_mass_kg(&self) -> f64 {
        self.hull.mass_kg + self.articulation.iter().map(|j| j.body.mass_kg).sum::<f64>()
    }

    /// Joint `root` and everything below it frozen into one body, in the frame of `root`.
    pub fn subtree_rigid(&self, root: usize, qs: &[f64]) -> Rigid {
        let inv_root = self.body_pose_in_hull(Some(root), qs).inverse();
        let mut members = vec![root];
        for i in (root + 1)..self.articulation.len() {
            if let Some(p) = self.articulation[i].parent {
                if members.contains(&p) {
                    members.push(i);
                }
            }
        }
        let mut parts: Vec<(f64, Vec3, Mat3)> = Vec::new();
        for i in members {
            let rel = inv_root.compose(&self.body_pose_in_hull(Some(i), qs));
            let b: &BodyDef = &self.articulation[i].body;
            let r = rel.rot.to_mat3();
            parts.push((b.mass_kg, rel.apply_point(b.com_m), b.inertia_kg_m2.rotate_inertia(&r)));
        }
        let m: f64 = parts.iter().map(|p| p.0).sum();
        let com = parts.iter().fold(Vec3::ZERO, |a, p| a + p.1 * p.0) * (1.0 / m);
        let mut inertia = Mat3::ZERO;
        for (pm, pc, pi) in &parts {
            inertia = inertia.add(&parallel_axis(pi, *pm, *pc - com));
        }
        Rigid { mass_kg: m, com_m: com, inertia_kg_m2: inertia }
    }
}
