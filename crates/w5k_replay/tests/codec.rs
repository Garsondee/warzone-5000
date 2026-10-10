//! Replay v2 binary codec: every claim is a physics-or-budget sentence.

use w5k_contract::frame::{Event, Frame, ProjectileFrame};
use w5k_contract::testing::{tank_slew_and_pitch, truck_over_bumps};
use w5k_math::{scalar, Vec3};
use w5k_replay::{binary, ReplayFile};

fn canned() -> Vec<(&'static str, ReplayFile)> {
    let mut v = Vec::new();
    for (n, (header, frames, _)) in [("truck", truck_over_bumps()), ("tank", tank_slew_and_pitch())] {
        v.push((n, ReplayFile { header, frames }));
    }
    v
}

fn round_trip(r: &ReplayFile) -> ReplayFile {
    binary::decode(&binary::encode(r).unwrap()).unwrap()
}

fn rot_error_deg(a: w5k_math::Quat, b: w5k_math::Quat) -> f64 {
    2.0 * scalar::acos(a.dot(b).abs().min(1.0)).to_degrees()
}

#[test]
fn position_round_trips_within_1mm() {
    for (n, r) in canned() {
        let back = round_trip(&r);
        for (a, b) in r.frames.iter().zip(&back.frames) {
            let d = (a.vehicles[0].pos_m - b.vehicles[0].pos_m).length();
            assert!(d <= 1e-3, "{n}: {d} m");
        }
    }
}

#[test]
fn rotation_round_trips_within_0p01_degrees() {
    for (n, r) in canned() {
        let back = round_trip(&r);
        for (a, b) in r.frames.iter().zip(&back.frames) {
            let e = rot_error_deg(a.vehicles[0].rot, b.vehicles[0].rot);
            assert!(e <= 0.01, "{n}: {e} deg");
        }
    }
}

#[test]
fn joint_values_round_trip_within_their_quantum() {
    for (n, r) in canned() {
        let names = r.header.vehicles[0].joint_names.clone();
        let back = round_trip(&r);
        for (a, b) in r.frames.iter().zip(&back.frames) {
            for (k, (x, y)) in a.vehicles[0].joints.iter().zip(&b.vehicles[0].joints).enumerate() {
                let metres = names[k].contains("travel") || names[k].contains("recoil");
                let quantum = if metres { 0.5 / 20_000.0 } else { 0.5 * std::f64::consts::TAU / 65536.0 };
                // f32 storage adds its own rounding, small against the quantum except for large spin angles.
                let slack = f64::from(x.abs()) * 1.2e-7;
                assert!(f64::from((x - y).abs()) <= quantum + slack + 1e-9, "{n} {}: {x} vs {y}", names[k]);
            }
        }
    }
}

#[test]
fn replay_costs_under_100_bytes_per_vehicle_frame() {
    for (n, r) in canned() {
        let bytes = binary::encode(&r).unwrap();
        let header_len = serde_json::to_vec(&r.header).unwrap().len();
        let per = (bytes.len() - header_len) as f64 / r.frames.len() as f64;
        println!("{n}: {per:.1} bytes per vehicle-frame ({} bytes in all)", bytes.len());
        assert!(per < 100.0, "{n}: {per} bytes");
    }
}

#[test]
fn decoded_replay_equals_the_original_within_quantisation() {
    for (n, r) in canned() {
        let back = round_trip(&r);
        assert_eq!(back.header, r.header);
        assert_eq!(back.frames.len(), r.frames.len());
        for (a, b) in r.frames.iter().zip(&back.frames) {
            assert!((a.t_s - b.t_s).abs() < 1e-6);
            let (va, vb) = (&a.vehicles[0], &b.vehicles[0]);
            assert!((va.lin_vel_m_s - vb.lin_vel_m_s).length() < 2e-3, "{n}: velocity");
            assert!((va.ang_vel_rad_s - vb.ang_vel_rad_s).length() < 2e-3, "{n}: angular velocity");
            assert_eq!(va.gear, vb.gear);
            assert_eq!(va.limiting, vb.limiting);
            assert!((va.engine_rpm - vb.engine_rpm).abs() <= 0.5 + 1e-3);
            assert_eq!(va.contacts.len(), vb.contacts.len());
            for (ca, cb) in va.contacts.iter().zip(&vb.contacts) {
                assert_eq!((ca.flags, ca.material), (cb.flags, cb.material));
                assert!((ca.normal_force_n - cb.normal_force_n).abs() <= 2.0 + 1e-6 * ca.normal_force_n.abs());
                assert!((ca.sinkage_m - cb.sinkage_m).abs() <= 1e-4);
                assert!((ca.slip - cb.slip).abs() <= 1e-3);
            }
            assert_eq!(va.ledger_n.len(), vb.ledger_n.len());
            assert!(va.ledger_n.iter().zip(&vb.ledger_n).all(|(x, y)| (x - y).abs() <= 0.5 + 1e-6 * x.abs()));
            assert_eq!(va.weapons.len(), vb.weapons.len());
            for (wa, wb) in va.weapons.iter().zip(&vb.weapons) {
                assert_eq!((wa.ready, wa.rounds), (wb.ready, wb.rounds));
                assert!((wa.aim_error_rad - wb.aim_error_rad).abs() <= 1e-5);
            }
        }
    }
}

#[test]
fn events_and_shells_survive_the_extension_block() {
    let (_, mut r) = canned().remove(1);
    r.frames[40].events.push(Event::Fired { vehicle: 0, muzzle: 0 });
    r.frames[41].projectiles.push(ProjectileFrame {
        id: 7,
        pos_m: Vec3 { x: 1.0, y: 2.0, z: 3.0 },
        vel_m_s: Vec3::ZERO,
    });
    let back = round_trip(&r);
    assert_eq!(back.frames[40].events, r.frames[40].events);
    assert_eq!(back.frames[41].projectiles, r.frames[41].projectiles);
    assert!(back.frames[42].events.is_empty());
    let _: &Frame = &back.frames[0];
}

#[test]
fn a_truncated_or_foreign_file_is_refused_not_a_panic() {
    assert!(binary::decode(b"nope").is_err());
    let (_, r) = canned().remove(0);
    let bytes = binary::encode(&r).unwrap();
    assert!(binary::decode(&bytes[..bytes.len() / 2]).is_err());
}
