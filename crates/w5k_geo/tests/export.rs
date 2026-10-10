//! The exported RenderRig and GLB of the utility 4x4: validity, flags in range, budget, node frames.

use w5k_contract::render::NodeRole;
use w5k_geo::export::{glb, render_rig};
use w5k_geo::flags::FlagParams;
use w5k_geo::truck::{utility_4x4, UtilityDims};

fn rig() -> w5k_contract::render::RenderRig {
    render_rig("utility_4x4", &utility_4x4(&UtilityDims::placeholder(), 1), &FlagParams::default_params())
}

#[test]
fn exported_rig_validates_carries_flags_in_range_and_stays_inside_the_triangle_budget() {
    let r = rig();
    r.validate().expect("RenderRig::validate");
    let tris = r.triangle_count();
    println!("exported triangles: {tris}");
    let budget = w5k_geo::budget::wheeled_triangles();
    assert!(tris < budget, "{tris} triangles, budget {budget} for a wheeled vehicle");
    for m in &r.meshes {
        assert!(m.edge.len() == m.positions.len() && m.cavity.len() == m.positions.len(), "{}", m.name);
        assert!(m.edge.iter().chain(&m.cavity).all(|v| v.is_finite() && (0.0..=1.0).contains(v)), "{}", m.name);
        assert!(
            m.normals.iter().all(|n| (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] - 1.0).abs() < 1e-3),
            "{} has a non-unit normal",
            m.name
        );
    }
    assert!(
        r.meshes.iter().any(|m| m.edge.iter().any(|&e| e > 0.4))
            && r.meshes.iter().any(|m| m.cavity.iter().any(|&c| c > 0.1))
    );
}

#[test]
fn wheel_meshes_are_centred_on_their_hub_in_the_wheel_node_frame() {
    let r = rig();
    let mut wheels = 0;
    for m in r.meshes.iter().filter(|m| m.name.starts_with("tyre")) {
        assert_eq!(r.nodes[m.node].role, NodeRole::Wheel);
        let (lo, hi) = m.positions.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(l, h), p| {
            (std::array::from_fn(|i| l[i].min(p[i])), std::array::from_fn(|i| h[i].max(p[i])))
        });
        assert!((lo[1] + hi[1]).abs() < 2e-3 && (lo[2] + hi[2]).abs() < 2e-3, "{} is off its hub", m.name);
        wheels += 1;
    }
    assert_eq!(wheels, 4);
    for m in r.meshes.iter().filter(|m| m.name.starts_with("knuckle")) {
        assert_eq!(r.nodes[m.node].role, NodeRole::SteerKnuckle);
    }
}

#[test]
fn glb_chunks_are_consistent() {
    let b = glb(&rig());
    assert_eq!(&b[0..4], b"glTF");
    assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), 2);
    assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize, b.len());
    let json_len = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
    assert_eq!(&b[16..20], b"JSON");
    let json = std::str::from_utf8(&b[20..20 + json_len]).unwrap();
    assert!(json.trim_end().starts_with('{') && json.trim_end().ends_with('}'));
    assert_eq!(json.matches('{').count(), json.matches('}').count());
    assert_eq!(json.matches('[').count(), json.matches(']').count());
    assert!(json.contains("COLOR_0") && json.contains("\"fl.wheel\""));
    let bin_at = 20 + json_len;
    assert_eq!(&b[bin_at + 4..bin_at + 8], b"BIN\0");
    assert_eq!(bin_at + 8 + u32::from_le_bytes(b[bin_at..bin_at + 4].try_into().unwrap()) as usize, b.len());
}
