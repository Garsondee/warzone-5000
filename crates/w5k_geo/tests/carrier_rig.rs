//! The tracked skin built from a compiled rig: every wheel where the rig puts it, the joint layout the rig's `joint_names()` gives, the
//! belt the rig describes, a reason when the rig cannot be drawn, and the tracked triangle budget.

use w5k_contract::def::{RunningGearDef, TrackedDef, VehicleDef};
use w5k_contract::param::Param;
use w5k_contract::render::{JointAxisKind, NodeRole, RenderRig};
use w5k_contract::rig::{PhysRig, WheelKind};
use w5k_contract::testing::{box_tank, dummy_vehicle_def};
use w5k_geo::budget::tracked_triangles;
use w5k_geo::flags::FlagParams;
use w5k_geo::skin::Skin;
use w5k_geo::track::pitch_radius_m;

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
