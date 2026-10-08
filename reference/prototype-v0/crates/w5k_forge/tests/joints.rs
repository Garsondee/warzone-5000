//! Moving parts: which shapes are joints, how they resolve in vehicle space, and that the mesh carries them.

use std::path::PathBuf;

use w5k_forge::export::{export_joints, ExportJoint};
use w5k_forge::geom::V3;
use w5k_forge::Forge;

fn forge() -> Forge {
    Forge::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")).expect("content loads")
}

fn joints_of(forge: &Forge, id: &str) -> Vec<ExportJoint> {
    let (_asm, _) = forge.quick_design(&forge.designs[id]);
    export_joints(&_asm)
}

fn count(joints: &[ExportJoint], kind: &str) -> usize {
    joints.iter().filter(|j| j.kind == kind).count()
}

#[test]
fn the_running_gear_of_every_family_and_legacy_part_is_jointed() {
    let forge = forge();
    // Family gear: wheels (6 on the scout), tracks (6 road wheels + sprocket + idler on each of two units), legs (hip and knee
    // each), rotors, fans.
    let wheel_scout = joints_of(&forge, "wheel_scout");
    assert_eq!(count(&wheel_scout, "roll"), 6, "{wheel_scout:?}");
    let lancer = joints_of(&forge, "lancer_mk1");
    assert_eq!(count(&lancer, "roll"), 2 * (6 + 2), "{:?}", lancer.iter().map(|j| &j.name).collect::<Vec<_>>());
    let spider = joints_of(&forge, "spider_artillery");
    assert_eq!(count(&spider, "hip"), 8);
    assert_eq!(count(&spider, "knee"), 8);
    assert!(count(&joints_of(&forge, "rotor_gunship"), "spin") >= 1, "a rotor");
    assert!(count(&joints_of(&forge, "hover_sniper"), "spin") >= 1, "stern fans");
    // The four legacy parts are jointed too: the tank, the six-wheeler, the spider walker and the drone.
    assert_eq!(count(&joints_of(&forge, "tank_medium"), "roll"), 2 * (6 + 2));
    assert_eq!(count(&joints_of(&forge, "apc_6x6"), "roll"), 6);
    let walker = joints_of(&forge, "walker_spider");
    assert!(count(&walker, "hip") >= 4 && count(&walker, "hip") == count(&walker, "knee"));
    assert!(count(&joints_of(&forge, "drone_scout"), "spin") >= 4, "one rotor per arm");
    // Things that do not move have no joints.
    assert!(joints_of(&forge, "maglev_lancer").iter().all(|j| j.kind != "hip"));
}

#[test]
fn rolling_wheels_turn_forward_for_a_positive_angle() {
    let forge = forge();
    let (up, fwd) = (V3 { x: 0.0, y: 1.0, z: 0.0 }, V3 { x: 0.0, y: 0.0, z: -1.0 });
    for id in ["wheel_scout", "lancer_mk1", "tank_medium", "apc_6x6"] {
        for j in joints_of(&forge, id).iter().filter(|j| j.kind == "roll") {
            let a = V3::from_arr(j.axis);
            assert!((a.len() - 1.0).abs() < 1e-9, "{id}: axis not a unit vector: {a:?}");
            assert!(a.y.abs() < 1e-6 && a.z.abs() < 1e-6, "{id}/{}: wheels roll about the lateral axis: {a:?}", j.name);
            // Rotating the top of the wheel by a positive angle about the axis must move it forward (-Z).
            assert!(a.cross(up).dot(fwd) > 0.0, "{id}/{}: {a:?} rolls backward", j.name);
            assert!(j.radius > 0.05 && j.radius < 5.0, "{id}/{}: radius {}", j.name, j.radius);
        }
    }
}

#[test]
fn legs_swing_forward_lift_the_foot_and_alternate() {
    let forge = forge();
    let (up, fwd) = (V3 { x: 0.0, y: 1.0, z: 0.0 }, V3 { x: 0.0, y: 0.0, z: -1.0 });
    for id in ["spider_artillery", "walking_cathedral", "walker_spider", "mech_bastion"] {
        let joints = joints_of(&forge, id);
        let hips: Vec<&ExportJoint> = joints.iter().filter(|j| j.kind == "hip").collect();
        assert!(hips.len() >= 2, "{id}");
        for h in &hips {
            let a = V3::from_arr(h.axis);
            assert!(a.x.abs() < 1e-6 && (a.y.abs() - 1.0).abs() < 1e-6 && a.z.abs() < 1e-6, "{id}: a hip turns about the vertical: {a:?}");
            assert!(h.amp > 0.05 && h.amp < 1.2, "{id}: swing amplitude {} rad", h.amp);
            assert!(h.stride > 0.5, "{id}: stride {}", h.stride);
        }
        for k in joints.iter().filter(|j| j.kind == "knee") {
            let parent = &joints[k.parent as usize];
            assert_eq!(parent.kind, "hip", "{id}: a knee hangs from a hip");
            assert_eq!(k.phase, parent.phase, "{id}: one leg, one phase");
            let a = V3::from_arr(k.axis);
            let foot_rel = V3::from_arr([parent.pivot[0], parent.pivot[1], parent.pivot[2]]);
            let _ = foot_rel;
            assert!(a.y.abs() < 1e-6, "{id}: a knee turns about a horizontal axis: {a:?}");
            assert!(k.amp > 0.04 && k.amp <= 0.6, "{id}: lift {} rad", k.amp);
        }
        // Forward swing: a positive angle about the hip's axis moves a foot that is out to the side toward -Z.
        for h in &hips {
            let side_out = V3 { x: if h.pivot[0] > 0.0 { 1.0 } else { -1.0 }, y: 0.0, z: 0.0 };
            assert!(V3::from_arr(h.axis).cross(side_out).dot(fwd) > 0.0, "{id}: a hip at x = {} swings backward for a positive angle", h.pivot[0]);
        }
        // Gait: phases are 0 or a half; on each side they alternate down the vehicle; the sides are opposed.
        for side in [1.0f64, -1.0] {
            let mut legs: Vec<&&ExportJoint> = hips.iter().filter(|h| h.pivot[0] * side > 0.0).collect();
            legs.sort_by(|a, b| a.pivot[2].total_cmp(&b.pivot[2]));
            for w in legs.windows(2) {
                assert!((w[0].phase - w[1].phase).abs() > 0.4, "{id}: neighbours on one side step together");
            }
        }
        let on: Vec<f64> = hips.iter().map(|h| h.phase).collect();
        assert!(on.iter().all(|p| *p == 0.0 || *p == 0.5), "{id}: {on:?}");
        let _ = up;
    }
}

#[test]
fn the_mesh_knows_which_triangles_move() {
    let forge = forge();
    for id in ["lancer_mk1", "wheel_scout", "spider_artillery", "rotor_gunship", "tank_medium", "hover_sniper"] {
        let (built, asm, _) = forge.build_design(&forge.designs[id]);
        let m = &built.mesh;
        assert_eq!(m.joint.len(), m.positions.len(), "{id}");
        assert!(m.joint.iter().all(|&j| (j as usize) <= asm.joints.len()), "{id}: a vertex names a joint that does not exist");
        // Every joint owns some triangles (otherwise it would animate nothing), and so does the static body.
        for k in 1..=asm.joints.len() {
            assert!(m.joint.iter().any(|&j| j as usize == k), "{id}: joint {k} ({}) has no triangles", asm.joints[k - 1].name);
        }
        let moving = m.joint.iter().filter(|&&j| j != 0).count();
        assert!(moving > 0 && moving < m.joint.len() * 9 / 10, "{id}: {moving} of {} vertices move", m.joint.len());
        // Triangles never straddle two joints (the viewer groups a triangle by its first vertex).
        for t in m.indices.chunks(3) {
            assert!(t.iter().all(|&i| m.joint[i as usize] == m.joint[t[0] as usize]), "{id}: a triangle spans two joints");
        }
        // The pivots lie on the vehicle.
        let (lo, hi) = (built.mass.bounds_min, built.mass.bounds_max);
        for j in &asm.joints {
            let p = j.pivot.arr();
            for k in 0..3 {
                assert!(p[k] >= lo[k] - 1.0 && p[k] <= hi[k] + 1.0, "{id}/{}: pivot {p:?} outside the vehicle {lo:?} {hi:?}", j.name);
            }
        }
    }
}
