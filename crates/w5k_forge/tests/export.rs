//! The compact mesh export round-trips what the viewers need.

use std::path::PathBuf;

use w5k_forge::export::export_mesh;
use w5k_forge::mesh::Mesh;
use w5k_forge::Forge;

fn content_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn check_round_trip(id: &str, mesh: &Mesh) {
    let e = export_mesh(mesh);
    assert_eq!(e.n_vertices as usize, mesh.positions.len(), "{id}");
    assert_eq!(e.n_indices as usize, mesh.indices.len(), "{id}: triangle count");
    let u = e.unpack();
    let extent = (0..3).map(|k| (e.hi[k] - e.lo[k]) as f64).fold(0.0, f64::max);
    // 16-bit quantisation over the box: half a step is the worst error.
    let tol = (extent / 65535.0) / 2.0 + 1e-6;
    for (i, (a, b)) in mesh.positions.iter().zip(&u.positions).enumerate() {
        for k in 0..3 {
            assert!(((a[k] - b[k]) as f64).abs() <= tol * 1.01, "{id}: vertex {i} axis {k}: {} vs {} (tolerance {tol})", a[k], b[k]);
        }
    }
    assert!(tol < 0.001 || extent > 65.0, "{id}: {extent} m long should quantise to under 1 mm");
    assert_eq!(u.indices, mesh.indices, "{id}");
    assert_eq!(u.part, mesh.part, "{id}");
    assert_eq!(u.joint, mesh.joint, "{id}");
    for i in 0..mesh.positions.len() {
        assert_eq!(u.slots[i], mesh.slots[i], "{id}");
        assert_eq!(u.edge[i], mesh.edge[i] > 0.5, "{id}");
        assert!((u.ao[i] - mesh.ao[i]).abs() <= 1.0 / 255.0 / 2.0 + 1e-6, "{id}");
    }
    let lowest = mesh.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    assert_eq!(e.ground_y, lowest, "{id}");
}

#[test]
fn vehicles_round_trip() {
    let forge = Forge::load(&content_dir()).unwrap();
    for id in ["lancer_mk1", "wheel_scout", "spider_artillery", "hover_sniper"] {
        let design = forge.designs.get(id).unwrap_or_else(|| panic!("no design {id}"));
        let (built, _, _) = forge.build_design(design);
        check_round_trip(id, &built.mesh);
    }
}

#[test]
fn an_empty_mesh_exports() {
    let e = export_mesh(&Mesh::default());
    assert_eq!((e.n_vertices, e.n_indices), (0, 0));
    assert!(e.blob.is_empty());
}

#[test]
fn big_meshes_use_wide_indices() {
    let mut m = Mesh::default();
    for i in 0..70_000u32 {
        m.positions.push([i as f32 * 0.001, 0.0, 0.0]);
        m.normals.push([0.0, 1.0, 0.0]);
        m.slots.push((i % 8) as u8);
        m.edge.push(0.0);
        m.ao.push(1.0);
        m.part.push((i % 5) as u16);
        m.joint.push((i % 3) as u16);
    }
    m.indices = vec![0, 69_999, 65_536];
    let e = export_mesh(&m);
    assert!(e.wide_indices);
    assert_eq!(e.unpack().indices, vec![0, 69_999, 65_536]);
}
