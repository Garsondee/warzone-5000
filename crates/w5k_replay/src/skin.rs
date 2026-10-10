//! Skinning one vehicle's meshes onto another's skeleton: the physics rig says where the wheels are (it is what the simulation
//! ran), a "skin" rig (GEOMETRY's detailed truck) says what the body looks like. The joint layouts must agree (same joint
//! indices and kinds); the rest poses need not. Suspension arms take the physics rig's rest positions, and the body (the meshes
//! on the hull node) moves rigidly by the mean difference, so wheels and body keep the relationship the skin was modelled with.

use w5k_contract::render::{NodeRole, RenderRig};
use w5k_math::Vec3;

pub fn retarget(skin: &RenderRig, phys: &RenderRig) -> Result<RenderRig, String> {
    if skin.joint_count != phys.joint_count {
        return Err(format!("joint counts differ: skin {} against physics rig {}", skin.joint_count, phys.joint_count));
    }
    let mut out = skin.clone();
    let mut delta = Vec3::ZERO;
    let mut arms = 0.0;
    for (i, n) in skin.nodes.iter().enumerate() {
        let Some(j) = n.joint else { continue };
        let twin = phys
            .nodes
            .iter()
            .find(|p| p.joint.is_some_and(|q| q.index == j.index && q.kind == j.kind))
            .ok_or_else(|| {
                format!("skin node {} (joint {}) has no counterpart in the physics rig {}", n.name, j.index, phys.id)
            })?;
        if n.role == NodeRole::SuspensionArm {
            delta += twin.rest.pos - n.rest.pos;
            arms += 1.0;
            out.nodes[i].rest.pos = twin.rest.pos;
        }
    }
    if arms > 0.0 {
        delta *= 1.0 / arms; // const-ok: mean of the suspension arms
    }
    let hull = skin.nodes.iter().position(|n| n.parent.is_none()).ok_or("the skin rig has no root node")?;
    for m in out.meshes.iter_mut().filter(|m| m.node == hull) {
        for p in &mut m.positions {
            *p = [p[0] + delta.x as f32, p[1] + delta.y as f32, p[2] + delta.z as f32];
        }
    }
    out.validate().map_err(|e| e.join("; "))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    #[test]
    fn retargeted_wheels_sit_where_the_physics_rig_says_and_the_body_moves_with_them() {
        let (_, skin) = box_truck();
        let mut phys = skin.clone();
        let shift = Vec3 { x: 0.0, y: -0.16, z: 0.3 };
        for n in phys.nodes.iter_mut().filter(|n| n.role == NodeRole::SuspensionArm) {
            n.rest.pos += shift;
        }
        let out = retarget(&skin, &phys).unwrap();
        for (a, b) in out.nodes.iter().zip(&phys.nodes).filter(|(a, _)| a.role == NodeRole::SuspensionArm) {
            assert!((a.rest.pos - b.rest.pos).length() < 1e-12);
        }
        let hull = |r: &RenderRig| r.meshes.iter().find(|m| m.node == 0).unwrap().positions[0];
        let (before, after) = (hull(&skin), hull(&out));
        assert!((f64::from(after[1] - before[1]) + 0.16).abs() < 1e-6);
        assert!((f64::from(after[2] - before[2]) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn a_skin_with_a_different_joint_layout_is_refused() {
        let (_, skin) = box_truck();
        let mut phys = skin.clone();
        phys.joint_count += 1;
        assert!(retarget(&skin, &phys).is_err());
    }
}
