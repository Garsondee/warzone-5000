//! The tracked skin built from a compiled rig: every wheel where the rig puts it, the joint layout the rig's `joint_names()` gives, the
//! belt the rig describes, a reason when the rig cannot be drawn, and the tracked triangle budget; and the belt as instanced links: the
//! `track_runs` of the export carry what a viewer needs to move the links round the wheels.

use std::collections::BTreeSet;
use w5k_contract::def::{RunningGearDef, TrackedDef, VehicleDef};
use w5k_contract::param::Param;
use w5k_contract::render::{JointAxisKind, MeshPart, NodeRole, RenderRig, TrackRun};
use w5k_contract::rig::{PhysRig, Side, WheelKind};
use w5k_contract::testing::{box_tank, dummy_vehicle_def};
use w5k_geo::budget::tracked_triangles;
use w5k_geo::flags::FlagParams;
use w5k_geo::mesh::Mesh;
use w5k_geo::skin::Skin;
use w5k_geo::track::{link_frames, pitch_radius_m, place_link};
use w5k_math::{scalar, Transform, Vec3};

/// A consistent tracked rig and its definition: the contract's stand-in tank with the sprocket given its pitch radius (the stand-in's 0.4 m is
/// not that of 11 teeth of 0.15 m), the idler and sprocket raised above the road wheels (the stand-in's hubs are all at one height, so its top
/// run would cut through the road wheels), the belt length left for the loop to derive, and a definition whose hull box matches the ride height.
fn fixture() -> (PhysRig, RenderRig, VehicleDef) {
    let (mut rig, render) = box_tank();
    let radius = pitch_radius_m(rig.tracks[0].pitch_m, u32::from(rig.tracks[0].sprocket_teeth));
    for s in rig.stations.iter_mut().filter(|s| s.wheel.kind == WheelKind::Sprocket) {
        s.wheel.radius_m = radius;
    }
    rig.tracks.iter_mut().for_each(|t| t.belt_length_m = 0.0);
    // the idler and the sprocket stand above the road wheels, as on a real tank, so the top run clears the road wheels' tops
    for s in rig.stations.iter_mut().filter(|s| matches!(s.wheel.kind, WheelKind::Idler | WheelKind::Sprocket)) {
        s.rest_pos_m.y = -0.20;
    }
    // the idler is a tensioner: it moves along the hull, not up and down
    rig.stations.iter_mut().filter(|s| s.wheel.kind == WheelKind::Idler).for_each(|s| s.bump_dir = w5k_math::Vec3::Z);
    let p = |v: f64| Param::estimate(v, 0.9 * v, 1.1 * v, "test fixture");
    let mut def = dummy_vehicle_def();
    def.id = "fixture_tracked".into();
    (def.hull.length_m, def.hull.width_m, def.hull.height_m) = (p(7.0), p(3.0), p(1.0));
    def.hull.ground_clearance_m = p(rig.ride_height_m - 0.5);
    def.running_gear = RunningGearDef::Tracked(TrackedDef {
        road_wheels_per_side: 5,
        road_wheel_diameter_m: p(0.76),
        sprocket_diameter_m: p(2.0 * radius),
        sprocket_at_front: false,
        track_width_m: p(0.55),
        track_gauge_m: p(2.7),
        ground_contact_length_m: p(4.8),
        pitch_m: p(0.15),
        track_mass_per_side_kg: p(900.0),
        shoe_mu_scale: p(1.0),
    });
    (rig, render, def)
}

fn skin_rig(rig: &PhysRig, def: &VehicleDef) -> (Skin, RenderRig) {
    let skin = Skin::from_rig(rig, def).unwrap_or_else(|e| panic!("{e}"));
    let r = skin.rig(0, &FlagParams::default_params());
    (skin, r)
}

fn assert_wheels_where_the_rig_puts_them(rig: &PhysRig, def: &VehicleDef) {
    let (_, r) = skin_rig(rig, def);
    let n = rig.stations.len();
    for (i, s) in rig.stations.iter().enumerate() {
        let travel = r
            .nodes
            .iter()
            .find(|x| x.joint.is_some_and(|j| j.index == n + i))
            .unwrap_or_else(|| panic!("no travel node for {}", s.name));
        assert!(
            (travel.rest.pos - s.rest_pos_m).length() < 1e-9,
            "{} is drawn at {:?}, the rig puts it at {:?}",
            s.name,
            travel.rest.pos,
            s.rest_pos_m
        );
        let wheel = r.nodes.iter().find(|x| x.joint.is_some_and(|j| j.index == i)).unwrap();
        let role = match s.wheel.kind {
            WheelKind::RoadWheel => NodeRole::RoadWheel,
            WheelKind::Sprocket => NodeRole::Sprocket,
            WheelKind::Idler => NodeRole::Idler,
            _ => NodeRole::ReturnRoller,
        };
        assert_eq!(wheel.role, role, "{}", s.name);
        assert_eq!(
            wheel.parent.map(|p| r.nodes[p].name.clone()),
            Some(travel.name.clone()),
            "a wheel hangs from its travel node"
        );
    }
}

#[test]
fn a_skin_built_from_a_tracked_rig_puts_every_wheel_where_the_rig_puts_it() {
    let (rig, _, def) = fixture();
    assert_wheels_where_the_rig_puts_them(&rig, &def);
}

#[test]
fn a_rig_with_the_sprocket_at_the_front_is_drawn_too_and_its_wheels_are_where_the_rig_puts_them() {
    // the same tank turned round: z -> -z puts the sprocket in front and makes the belt's loop run clockwise
    let (mut rig, _, def) = fixture();
    rig.stations.iter_mut().for_each(|s| s.rest_pos_m.z = -s.rest_pos_m.z);
    assert_wheels_where_the_rig_puts_them(&rig, &def);
}

#[test]
fn its_joint_layout_is_the_rigs_and_every_skin_joint_has_a_twin_in_the_physics_render_rig() {
    let (rig, phys, def) = fixture();
    let (_, r) = skin_rig(&rig, &def);
    let n = rig.stations.len();
    assert_eq!(
        r.joint_count,
        rig.joint_names().len(),
        "the skin carries the rig's joint coordinates, the articulation ones unbound"
    );
    assert_eq!(r.joint_count, phys.joint_count);
    for j in 0..2 * n {
        let nodes: Vec<_> = r.nodes.iter().filter(|x| x.joint.is_some_and(|b| b.index == j)).collect();
        assert_eq!(nodes.len(), 1, "joint {j} ({}) is bound to {} nodes", rig.joint_names()[j], nodes.len());
        let b = nodes[0].joint.unwrap();
        let (kind, axis) = if j < n {
            (JointAxisKind::Revolute, -w5k_math::Vec3::X)
        } else {
            (JointAxisKind::Prismatic, rig.stations[j - n].bump_dir)
        };
        assert_eq!(
            (b.kind, b.axis.as_array().map(f64::to_bits)),
            (kind, axis.as_array().map(f64::to_bits)),
            "joint {j}"
        );
        // the retargeting step of the viewer finds each skin joint in the physics rig by index and kind
        assert!(
            phys.nodes.iter().any(|p| p.joint.is_some_and(|q| q.index == b.index && q.kind == b.kind)),
            "no twin for joint {j}"
        );
    }
    assert_eq!(r.nodes.iter().filter(|x| x.role == NodeRole::Track).count(), 2, "one jointless Track node a side");
    r.validate().expect("RenderRig::validate");
}

#[test]
fn the_belt_drawn_agrees_with_the_rigs_numbers_and_a_wrong_belt_length_is_reported() {
    let (mut rig, _, def) = fixture();
    let skin = Skin::from_rig(&rig, &def).unwrap();
    assert!(skin.tracks.as_ref().unwrap().notes.is_empty(), "{:?}", skin.tracks.as_ref().unwrap().notes);
    rig.tracks[1].belt_length_m = 11.4; // the contract stand-in's number: the loop is 14 m
    let notes = Skin::from_rig(&rig, &def).unwrap().tracks.unwrap().notes;
    assert!(notes.iter().any(|n| n.contains("belt_length_m")), "{notes:?}");
}

#[test]
fn a_rig_that_cannot_be_drawn_is_refused_with_the_reason() {
    let (rig, _, def) = fixture();
    let refuse = |what: &str, edit: &dyn Fn(&mut PhysRig), needle: &str| {
        let mut r = rig.clone();
        edit(&mut r);
        let e = Skin::from_rig(&r, &def).map(|_| ()).expect_err(what);
        assert!(e.contains(needle), "{what}: {e}");
    };
    refuse("an asymmetric rig", &|r| r.stations[3].rest_pos_m.z += 0.1, "mirror");
    refuse(
        "a wrong sprocket radius",
        &|r| r.stations.iter_mut().filter(|s| s.wheel.kind == WheelKind::Sprocket).for_each(|s| s.wheel.radius_m = 0.4),
        "pitch radius",
    );
    refuse(
        "a top run that cuts through the road wheels",
        &|r| {
            r.stations
                .iter_mut()
                .filter(|s| matches!(s.wheel.kind, WheelKind::Idler | WheelKind::Sprocket))
                .for_each(|s| s.rest_pos_m.y = -0.45)
        },
        "pokes",
    );
    refuse("no belt thickness", &|r| r.tracks.iter_mut().for_each(|t| t.thickness_m = 0.0), "thickness");
    refuse(
        "a road wheel off the belt",
        &|r| r.stations.iter_mut().filter(|s| s.name.ends_with("r3")).for_each(|s| s.rest_pos_m.y += 0.08),
        "does not touch",
    );
    let mut wheeled = def.clone();
    wheeled.running_gear = dummy_vehicle_def().running_gear;
    assert!(Skin::from_rig(&rig, &wheeled).map(|_| ()).unwrap_err().contains("not tracked"));
}

#[test]
fn the_stand_in_carrier_skin_stays_inside_the_tracked_triangle_budget() {
    let skin = Skin::for_id("carrier_tracked").expect("the carrier skin");
    let r = skin.rig(1, &FlagParams::default_params());
    r.validate().expect("RenderRig::validate");
    let budget = tracked_triangles();
    println!("carrier_tracked: {} triangles, budget {budget}", r.triangle_count());
    assert!(r.triangle_count() < budget, "{} triangles, budget {budget} for a tracked vehicle", r.triangle_count());
}

/// The pose of node `i` in the hull frame at joint coordinates zero.
fn at_rest(r: &RenderRig, i: usize) -> Transform {
    let n = &r.nodes[i];
    n.parent.map_or(n.rest, |p| at_rest(r, p).compose(&n.rest))
}

fn as_mesh(m: &MeshPart) -> Mesh {
    let v = |p: &[f32; 3]| Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
    Mesh { v: m.positions.iter().map(v).collect(), t: m.indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect() }
}

/// A run's wheel centres (z, y) in the hull frame, and its belt's centre line (x), at joint coordinates zero.
fn centres(r: &RenderRig, run: &TrackRun) -> (Vec<[f64; 2]>, f64) {
    let p: Vec<Vec3> = run.wheels.iter().map(|w| at_rest(r, w.node).pos).collect();
    (p.iter().map(|q| [q.z, q.y]).collect(), p[0].x)
}

/// The flag bake with a single cavity ray: these tests are about positions, and the bake is most of the time a rig takes to export.
fn quick() -> FlagParams {
    FlagParams { cavity_rays: 1, ..FlagParams::default_params() }
}

/// The skin's static export (the belt as one mesh) and its instanced one (the belt as a link and a `track_runs` entry).
fn both(rig: &PhysRig, def: &VehicleDef) -> (RenderRig, RenderRig) {
    let skin = Skin::from_rig(rig, def).unwrap_or_else(|e| panic!("{e}"));
    (skin.rig(0, &quick()), skin.rig_instanced(0, &quick()).unwrap_or_else(|e| panic!("{e}")))
}

fn run_on<'a>(r: &'a RenderRig, node: &str) -> &'a TrackRun {
    r.track_runs.iter().find(|t| r.nodes[t.node].name == node).unwrap_or_else(|| panic!("no run on {node}"))
}

#[test]
fn the_instanced_export_has_a_valid_track_run_a_side_naming_the_rigs_wheels_and_the_sprockets_spin() {
    for front in [false, true] {
        let (mut rig, _, def) = fixture();
        if front {
            rig.stations.iter_mut().for_each(|s| s.rest_pos_m.z = -s.rest_pos_m.z);
        }
        let (fixed, r) = both(&rig, &def);
        assert!(fixed.track_runs.is_empty(), "the default export draws the belt as one mesh, for a glTF");
        r.validate().expect("RenderRig::validate");
        assert_eq!(r.track_runs.len(), 2, "one run a side");
        for (node, side) in [("track_r", Side::Right), ("track_l", Side::Left)] {
            let (run, track) = (run_on(&r, node), rig.tracks.iter().find(|t| t.side == side).unwrap());
            // the wheels are the track's stations (a station's spin joint is its index), the sprocket's spin is the run's `sprocket_joint`
            let joints: Vec<usize> =
                run.wheels.iter().map(|w| r.nodes[w.node].joint.expect("a wheel spins").index).collect();
            let (mut a, mut b) = (joints.clone(), track.stations.clone());
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "{node} (sprocket at the front: {front}): the wheels are the track's stations");
            assert_eq!(joints[run.sprocket], run.sprocket_joint);
            assert_eq!(rig.stations[run.sprocket_joint].wheel.kind, WheelKind::Sprocket);
            assert_eq!(r.nodes[run.wheels[run.sprocket].node].role, NodeRole::Sprocket);
            for (w, &j) in run.wheels.iter().zip(&joints) {
                let s = &rig.stations[j];
                let tip = if s.wheel.kind == WheelKind::Sprocket { 0.0 } else { track.thickness_m / 2.0 };
                assert!((w.radius_m - (s.wheel.radius_m + tip)).abs() < 1e-9, "{}: path radius {}", s.name, w.radius_m);
            }
            assert_eq!(run.direction, 1, "the loop is written counter-clockwise, so a forward spin runs along it");
            // the link is one small Track mesh on the run's node, which sits at the hull's origin
            let link = &r.meshes[run.link_mesh];
            assert_eq!(link.node, run.node);
            assert_eq!((r.nodes[run.node].role, r.nodes[run.node].rest), (NodeRole::Track, Transform::IDENTITY));
            assert!(link.indices.len() / 3 < 1000, "{} triangles in one link", link.indices.len() / 3);
            // its flags are baked alone: in place, inside the hull, the cavity bake would read as dirt all over it (0.7 against 0.4)
            let dirt = link.cavity.iter().map(|&c| f64::from(c)).sum::<f64>() / link.cavity.len() as f64;
            assert!(dirt < 0.55, "{node}: the link's mean cavity is {dirt:.2}");
        }
    }
}

#[test]
fn the_links_a_viewer_places_from_the_exported_rig_are_the_belt_the_static_export_draws() {
    let (rig, _, def) = fixture();
    let (fixed, r) = both(&rig, &def);
    for (node, side) in [("track_r", 1.0), ("track_l", -1.0)] {
        let run = run_on(&r, node);
        let (c, x) = centres(&r, run);
        assert!(x * side > 0.0, "{node} is on its own side");
        let one = as_mesh(&r.meshes[run.link_mesh]);
        let frames = link_frames(run, &c, 0.0).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(frames.len(), usize::from(run.links));
        let placed: Vec<Vec3> = frames.iter().flat_map(|&(p, t)| place_link(&one, x, p, t).v).collect();
        let id = fixed.nodes.iter().position(|n| n.name == node).unwrap();
        let rest = at_rest(&fixed, id);
        let drawn: Vec<Vec3> = fixed
            .meshes
            .iter()
            .filter(|m| m.node == id)
            .flat_map(|m| as_mesh(m).v.into_iter().map(|v| rest.apply_point(v)))
            .collect();
        let cell = |v: &Vec3| v.as_array().map(|c| (c / 2e-4).round() as i64);
        let set = |vs: &[Vec3]| vs.iter().map(cell).collect::<BTreeSet<_>>();
        let near = |s: &BTreeSet<[i64; 3]>, v: &Vec3| {
            let k = cell(v);
            (-1..=1).any(|i| (-1..=1).any(|j| (-1..=1).any(|l| s.contains(&[k[0] + i, k[1] + j, k[2] + l]))))
        };
        let (a, b) = (set(&placed), set(&drawn));
        let missing = (drawn.iter().filter(|v| !near(&a, v)).count(), placed.iter().filter(|v| !near(&b, v)).count());
        assert_eq!(missing, (0, 0), "{node}: vertices of the static belt with no placed link nearby, and the reverse");
    }
}

#[test]
fn a_forward_sprocket_spin_runs_the_ground_run_backwards_and_the_top_run_forwards_by_the_pitch_radius_times_the_angle()
{
    let (rig, _, def) = fixture();
    let (_, r) = both(&rig, &def);
    let run = run_on(&r, "track_r");
    let (c, _) = centres(&r, run);
    let (spin, rolled) = (0.1, run.wheels[run.sprocket].radius_m * 0.1);
    let (before, after) = (link_frames(run, &c, 0.0).unwrap(), link_frames(run, &c, spin).unwrap());
    let mid = c.iter().map(|q| q[0]).sum::<f64>() / c.len() as f64;
    let in_the_middle = |k: &usize| (before[*k].0[0] - mid).abs() < 1.0;
    let by = |f: &dyn Fn(usize) -> f64| {
        (0..before.len()).filter(in_the_middle).max_by(|&a, &b| f(a).total_cmp(&f(b))).unwrap()
    };
    let (ground, top) = (by(&|k| -before[k].0[1]), by(&|k| before[k].0[1]));
    for (what, k, sign) in [("ground", ground, 1.0), ("top", top, -1.0)] {
        let (d, y) = ([after[k].0[0] - before[k].0[0], after[k].0[1] - before[k].0[1]], 0.0);
        let moved = scalar::hypot(d[0], d[1]);
        assert!((moved - rolled).abs() < 1e-9, "the {what} run link moved {moved} m, the sprocket rolled {rolled} m");
        assert!(
            d[0] * sign > 0.0,
            "the {what} run moves {} (z = {})",
            if sign > 0.0 { "backwards" } else { "forwards" },
            d[0]
        );
        let _ = y;
    }
}

#[test]
fn pulling_the_idler_forward_lengthens_the_band_and_no_link_enters_a_wheel() {
    let (rig, _, def) = fixture();
    let (_, r) = both(&rig, &def);
    let run = run_on(&r, "track_r");
    let (mut c, _) = centres(&r, run);
    let idler = run.wheels.iter().position(|w| r.nodes[w.node].role == NodeRole::Idler).unwrap();
    let gap = |f: &[([f64; 2], [f64; 2])]| {
        // two neighbouring links on the ground run, the lowest ones nearest the middle
        let mut k: Vec<usize> = (0..f.len())
            .filter(|&k| (f[k].0[1] - f.iter().map(|q| q.0[1]).fold(f64::MAX, f64::min)).abs() < 1e-9)
            .collect();
        k.sort_by(|&a, &b| f[a].0[0].total_cmp(&f[b].0[0]));
        let m = k.len() / 2;
        scalar::hypot(f[k[m]].0[0] - f[k[m - 1]].0[0], f[k[m]].0[1] - f[k[m - 1]].0[1])
    };
    let at_rest = gap(&link_frames(run, &c, 0.0).unwrap());
    c[idler][0] -= 0.05; // forward is -z
    let pulled = link_frames(run, &c, 0.0).unwrap();
    assert!(gap(&pulled) > at_rest, "the same number of links over a longer band sit further apart");
    for (p, _) in &pulled {
        for (w, o) in run.wheels.iter().zip(&c) {
            assert!(scalar::hypot(p[0] - o[0], p[1] - o[1]) >= w.radius_m - 1e-6, "a link entered a wheel");
        }
    }
}

#[test]
fn an_instanced_belt_costs_one_link_a_side_in_triangles_and_the_instanced_carrier_is_inside_the_budget() {
    let skin = Skin::for_id("carrier_tracked").expect("the carrier skin");
    let (fixed, r) = (skin.rig(1, &quick()), skin.rig_instanced(1, &quick()).expect("instanced rig"));
    r.validate().expect("RenderRig::validate");
    assert_eq!(r.track_runs.len(), 2);
    let link = r.meshes[r.track_runs[0].link_mesh].indices.len() / 3;
    let links = usize::from(r.track_runs[0].links);
    assert_eq!(
        fixed.triangle_count() - r.triangle_count(),
        2 * (links - 1) * link,
        "links {links}, link triangles {link}"
    );
    assert!(r.triangle_count() < tracked_triangles());
}
