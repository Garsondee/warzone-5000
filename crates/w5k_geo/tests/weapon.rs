//! Weapon family oracles: the generated geometry has the stated dimensions, every part is a closed outward-facing solid, the barrel is an
//! exact n-gon prism, a gun reads the cradle it sits in, and the trunnion size gates which gun goes in which cradle.

use w5k_contract::render::NodeRole;
use w5k_geo::module::{frame, Assembly, Module, ModuleKind, Socket, SocketKind};
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::{Part, Side};
use w5k_geo::weapon::{gun_module, GunDims};
use w5k_math::{scalar, Vec3};

const PI: f64 = std::f64::consts::PI;

fn find<'a>(m: &'a Module, name: &str) -> &'a Part {
    m.parts.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no part {name}"))
}

fn closed_and_outward(m: &Module) {
    for p in &m.parts {
        assert!(p.mesh.check_closed().is_ok(), "{} is not closed", p.name);
        assert!(p.mesh.signed_volume() > 0.0, "{} faces inward", p.name);
    }
}

/// A bare roof: a Ring socket 0.9 m wide, to hang the mount on without a hull.
fn roof() -> Assembly {
    let hull = Module {
        name: "roof".into(),
        kind: ModuleKind::Hull,
        parts: Vec::new(),
        sockets: vec![Socket {
            name: "roof".into(),
            kind: SocketKind::Ring,
            side: Side::Centre,
            pose: frame(Vec3::new(0.0, 1.5, 0.3), Vec3::Y, -Vec3::Z),
            size_m: 0.9,
            station: None,
            hints: Vec::new(),
        }],
        mount: None,
        symmetric: true,
    };
    Assembly::new(hull)
}

#[test]
fn every_part_of_every_gun_preset_is_a_closed_outward_solid_at_every_detail() {
    for detail in 0..=2 {
        for name in ["machine_gun_12_7", "autocannon_25"] {
            closed_and_outward(&gun_module(&GunDims::preset(name).unwrap(), 0.30, detail));
        }
    }
    assert!(GunDims::preset("no_such_gun").is_none());
}

#[test]
fn the_barrel_is_an_exact_n_gon_prism_and_the_muzzle_is_the_stated_distance_in_front_of_the_receiver() {
    for name in ["machine_gun_12_7", "autocannon_25"] {
        let d = GunDims::preset(name).unwrap();
        let m = gun_module(&d, 0.30, 1);
        let n = f64::from(48 / 4); // detail 1: a quarter of 48 wheel segments
        let barrel = find(&m, "barrel");
        let len = d.barrel_len_m + d.overlap_m;
        let exact = n / 2.0 * d.barrel_radius_m * d.barrel_radius_m * scalar::sin(2.0 * PI / n) * len;
        assert!((barrel.mesh.signed_volume() - exact).abs() < 1e-12, "{name}: barrel volume");
        let (rw, rh, rl) = d.receiver_m;
        let (lo, hi) = find(&m, "receiver").mesh.bounds();
        assert!(
            (hi.x - lo.x - rw).abs() < 1e-9 && (hi.y - lo.y - rh).abs() < 1e-9 && (hi.z - lo.z - rl).abs() < 1e-9,
            "{name}: receiver box"
        );
        let front = lo.z;
        let (tip, _) = find(&m, "muzzle").mesh.bounds();
        assert!(
            (front - tip.z - d.barrel_len_m - d.muzzle_m.0).abs() < 1e-9,
            "{name}: the muzzle device ends barrel + device in front of the receiver"
        );
        assert!(
            (hi.z - (rl * d.pivot_back_frac)).abs() < 1e-9,
            "{name}: the share of the receiver behind the trunnion axis"
        );
    }
}

#[test]
fn the_pins_reach_the_uprights_of_whatever_cradle_the_gun_sits_in() {
    let d = GunDims::preset("machine_gun_12_7").unwrap();
    for cradle in [0.20, 0.30, 0.44] {
        let (lo, hi) = find(&gun_module(&d, cradle, 1), "pins").mesh.bounds();
        assert!(
            (hi.x - (cradle / 2.0 + d.overlap_m)).abs() < 1e-9 && (lo.x + (cradle / 2.0 + d.overlap_m)).abs() < 1e-9,
            "cradle {cradle}"
        );
    }
}

#[test]
fn a_gun_with_a_receiver_wider_than_the_cradle_is_refused_and_both_presets_fit_the_standard_cradle() {
    let mut asm = roof();
    asm.attach("roof", &ring_mount(&RingMountDims::standard(), 1), 0.0, "ring").unwrap();
    let mut gun = GunDims::preset("autocannon_25").unwrap();
    gun.receiver_m.0 = 0.42; // a 57 mm class receiver does not fit between uprights 0.30 m apart
    let err = asm.attach("trunnion.ring", &gun_module(&gun, 0.30, 1), 0.0, "gun").unwrap_err();
    assert!(err.reason.contains("needs 0.420 m, the socket offers 0.300 m"), "{err}");
    assert_eq!(asm.log.len(), 1, "the refusal changed nothing");
    for ok in ["machine_gun_12_7", "autocannon_25"] {
        let mut a = roof();
        a.attach("roof", &ring_mount(&RingMountDims::standard(), 1), 0.0, "ring").unwrap();
        assert!(
            a.attach("trunnion.ring", &gun_module(&GunDims::preset(ok).unwrap(), 0.30, 1), 0.0, "gun").is_ok(),
            "{ok} fits the standard cradle"
        );
    }
}

#[test]
fn a_gun_on_the_trunnion_has_its_pivot_on_the_cradle_and_its_barrel_forward_and_swings_and_recoils_as_tagged() {
    let mut asm = roof();
    let d = RingMountDims::standard();
    asm.attach("roof", &ring_mount(&d, 1), 0.0, "ring").unwrap();
    let gun = GunDims::preset("machine_gun_12_7").unwrap();
    asm.attach("trunnion.ring", &gun_module(&gun, d.cradle_width_m, 1), 0.0, "gun").unwrap();
    let pivot = Vec3::new(0.0, 1.5 + d.collar_height_m + d.cradle_height_m, 0.3 + d.cradle_z_m);
    for p in asm.parts.iter().filter(|p| p.name.ends_with(".gun")) {
        assert!((p.pose.pos - pivot).length() < 1e-12, "{} hangs from the trunnion", p.name);
        assert!(matches!(p.role, NodeRole::GunPitch | NodeRole::Recoil));
        let moves_back = matches!(p.name.split('.').next().unwrap(), "barrel" | "jacket" | "muzzle");
        assert_eq!(p.role == NodeRole::Recoil, moves_back, "{}", p.name);
        assert!((p.pose.apply_dir(-Vec3::Z) + Vec3::Z).length() < 1e-12, "{} points forward", p.name);
    }
    let turret = asm.parts.iter().find(|p| p.name == "turntable.ring").unwrap();
    assert!(
        (turret.pose.pos - Vec3::new(0.0, 1.5 + d.collar_height_m, 0.3)).length() < 1e-12
            && turret.role == NodeRole::Turret
    );
    assert!(asm.parts.iter().all(|p| p.pose.is_finite()));
}
