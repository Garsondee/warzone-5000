//! The render rig of a compiled wheeled vehicle: node chains `hull > travel > (steer) > wheel` per station, joint indices in the order of
//! `PhysRig::joint_names()`, and placeholder meshes (GEOMETRY's replace them). The wheel mesh takes its radius from the compiled station,
//! so the physics and the picture cannot disagree.

use w5k_contract::render::*;
use w5k_contract::rig::*;
use w5k_contract::testing::rigs::{box_mesh, cylinder_mesh};
use w5k_math::{Transform, Vec3};

const WHEEL_SEGMENTS: usize = 20; // const-ok: mesh resolution
                                  // Proportions of the placeholder art: rim radius over tyre radius, rim width over tyre width, the spin-marker lug.
const RIM_RADIUS_FRAC: f64 = 0.62; // const-ok: placeholder art proportion
const RIM_WIDTH_FRAC: f64 = 1.05; // const-ok: placeholder art proportion
const LUG_RADIUS_FRAC: f64 = 0.8; // const-ok: placeholder art proportion
const LUG_HALF_M: Vec3 = Vec3 { x: 0.02, y: 0.04, z: 0.05 }; // const-ok: placeholder art size

/// Node chains `hull > travel > (steer) > wheel` per station, with joint indices in the order of `PhysRig::joint_names()`.
pub fn render_rig(rig: &PhysRig, hull_size: Vec3) -> RenderRig {
    let n = rig.stations.len();
    let steered: Vec<usize> = (0..n).filter(|&i| rig.stations[i].steer.is_some()).collect();
    let mut rr = RenderRig {
        id: rig.id.clone(),
        nodes: vec![RenderNode {
            name: "hull".into(),
            parent: None,
            role: NodeRole::Hull,
            rest: Transform::IDENTITY,
            joint: None,
        }],
        meshes: vec![],
        material_slots: vec![
            MaterialSlot { name: "paint".into(), kind: SlotKind::Paint },
            MaterialSlot { name: "tyre".into(), kind: SlotKind::Rubber },
            MaterialSlot { name: "rim".into(), kind: SlotKind::Metal },
        ],
        joint_count: rig.joint_names().len(),
    };
    let half = 0.5 * hull_size;
    rr.meshes.push(box_mesh("hull", 0, 0, Vec3::ZERO, half));
    rr.meshes.push(box_mesh("nose_marker", 0, 2, Vec3::new(0.0, half.y, -half.z + 0.3), Vec3::new(0.4, 0.03, 0.2))); // const-ok: a visible forward marker
    for (i, s) in rig.stations.iter().enumerate() {
        let travel = rr.nodes.len();
        rr.nodes.push(RenderNode {
            name: format!("{}.travel", s.name),
            parent: Some(0),
            role: NodeRole::SuspensionArm,
            rest: Transform::from_pos(s.rest_pos_m),
            joint: Some(JointBinding {
                kind: JointAxisKind::Prismatic,
                axis: s.bump_dir,
                index: n + steered.len() + i,
            }),
        });
        let mut parent = travel;
        if let Some(k) = steered.iter().position(|&x| x == i) {
            parent = rr.nodes.len();
            rr.nodes.push(RenderNode {
                name: format!("{}.steer", s.name),
                parent: Some(travel),
                role: NodeRole::SteerKnuckle,
                rest: Transform::IDENTITY,
                joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: Vec3::Y, index: n + k }),
            });
        }
        let wheel = rr.nodes.len();
        rr.nodes.push(RenderNode {
            name: format!("{}.wheel", s.name),
            parent: Some(parent),
            role: NodeRole::Wheel,
            rest: Transform::IDENTITY,
            // Positive spin = rolling forward (-Z): rotation about -X.
            joint: Some(JointBinding { kind: JointAxisKind::Revolute, axis: -Vec3::X, index: i }),
        });
        let (r, hw) = (s.wheel.radius_m, 0.5 * s.wheel.width_m);
        rr.meshes.push(cylinder_mesh(&format!("{}.tyre", s.name), wheel, 1, Vec3::ZERO, r, hw, 0, WHEEL_SEGMENTS));
        rr.meshes.push(cylinder_mesh(
            &format!("{}.rim", s.name),
            wheel,
            2,
            Vec3::ZERO,
            RIM_RADIUS_FRAC * r,
            hw * RIM_WIDTH_FRAC,
            0,
            WHEEL_SEGMENTS,
        )); // const-ok: rim proportions of the placeholder art
        rr.meshes.push(box_mesh(
            &format!("{}.lug", s.name),
            wheel,
            2,
            Vec3::new(hw * RIM_WIDTH_FRAC, 0.0, LUG_RADIUS_FRAC * r),
            LUG_HALF_M,
        )); // const-ok: spin marker
    }
    rr
}
