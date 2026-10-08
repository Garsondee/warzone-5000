//! Parametric families: scaling laws, generated geometry, measured response to sliders, and the coupled
//! slider solver.

use std::path::PathBuf;

use w5k_forge::family::turret::{gun, penetration_mm, TankTurret};
use w5k_forge::family::{mass_estimate, set_slider, Family, Values};
use w5k_forge::schema::MaterialLibrary;
use w5k_forge::{Built, Forge};

fn lib() -> MaterialLibrary {
    let text = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/materials.ron")).unwrap();
    ron::from_str(&text).unwrap()
}

fn build(fam: &dyn Family, v: &Values) -> Built {
    let def = fam.generate(v, &lib());
    let id = def.id.clone();
    Forge::from_parts(lib(), vec![def]).unwrap().build_part(&id).unwrap()
}

#[test]
fn gun_scaling_laws() {
    let a = gun(75.0, 45.0, 0.0);
    let b = gun(150.0, 45.0, 0.0);
    assert!((b.shell_kg / a.shell_kg - 8.0).abs() < 1e-9, "shell mass goes with the cube of calibre");
    assert!((b.velocity - a.velocity).abs() < 1e-9, "same barrel length in calibres, same velocity");
    assert!(gun(75.0, 60.0, 0.0).velocity > a.velocity);
    // Calibration point of the de Marre formula.
    let mut g = a;
    g.shell_kg = 7.0;
    assert!((penetration_mm(&g, 895.0) - 100.0).abs() < 1e-9);
    // A mechanical loader is faster for a big gun.
    assert!(gun(150.0, 45.0, 1.0).reload_s < gun(150.0, 45.0, 0.0).reload_s);
    // Small guns with a loader become rotary.
    assert_eq!(gun(7.62, 60.0, 1.0).barrels, 6);
}

#[test]
fn extremes_generate_closed_geometry() {
    let f = TankTurret;
    for cal in [7.62, 40.0, 406.0] {
        for (arm, slope, loader) in [(5.0, 0.0, 0.0), (250.0, 1.0, 1.0)] {
            let v = f.with(&[("calibre_mm", cal), ("armour_mm", arm), ("slope", slope), ("loader", loader), ("ammo", 4.0)]);
            let (pieces, _) = w5k_forge::build::build_part(&f.generate(&v, &lib()));
            assert!(!pieces.is_empty());
            for (i, p) in pieces.iter().enumerate() {
                assert!(p.poly.is_closed(), "{cal} mm, {arm} mm: piece {i} not closed");
            }
        }
    }
}

#[test]
fn armour_and_slope_raise_measured_protection() {
    let f = TankTurret;
    let front = |arm: f64, slope: f64| build(&f, &f.with(&[("armour_mm", arm), ("slope", slope)])).armour.at(0.0, 0.0).mean_mm;
    let (thin, thick) = (front(20.0, 0.4), front(120.0, 0.4));
    assert!(thick > 3.0 * thin, "{thin} -> {thick}");
    // Slope, measured on an uncrewed station so a human-sized cupola (a real weak spot, and a bigger share
    // of a squat turret's front) does not dominate the average.
    let remote = |slope: f64| {
        let v = f.with(&[("calibre_mm", 20.0), ("loader", 1.0), ("armour_mm", 80.0), ("slope", slope)]);
        build(&f, &v).armour.at(0.0, 0.0).mean_mm
    };
    let (upright, sloped) = (remote(0.0), remote(1.0));
    assert!(sloped > 1.6 * upright, "{upright} -> {sloped}");
}

#[test]
fn coupled_sliders_hold_mass_and_respect_locks() {
    let f = TankTurret;
    let lib = lib();
    let start = f.defaults();
    let m0 = mass_estimate(&f, &start, &lib);

    let free = set_slider(&f, &lib, &start, "calibre_mm", 105.0, &[]);
    assert!((free["calibre_mm"] - 105.0).abs() < 1e-6);
    assert!((mass_estimate(&f, &free, &lib) / m0 - 1.0).abs() < 0.005);
    assert!(free["armour_mm"] < start["armour_mm"], "armour gives way");
    assert_eq!(free["slope"], start["slope"], "free sliders never move");

    let locked = set_slider(&f, &lib, &start, "calibre_mm", 105.0, &["armour_mm"]);
    assert_eq!(locked["armour_mm"], start["armour_mm"]);
    assert!((mass_estimate(&f, &locked, &lib) / m0 - 1.0).abs() < 0.005);

    // Ask for far more than the budget allows with most sliders locked: the moved slider stops short.
    let capped = set_slider(&f, &lib, &start, "calibre_mm", 300.0, &["armour_mm", "ammo", "loader"]);
    assert!(capped["calibre_mm"] < 300.0 && capped["calibre_mm"] > 75.0, "{}", capped["calibre_mm"]);
    assert!(mass_estimate(&f, &capped, &lib) <= m0 * 1.005);

    // Moving a slider the cheap way lets the others grow back.
    let lighter = set_slider(&f, &lib, &start, "calibre_mm", 50.0, &[]);
    assert!(lighter["armour_mm"] > start["armour_mm"]);
    assert!((mass_estimate(&f, &lighter, &lib) / m0 - 1.0).abs() < 0.005);
}

#[test]
fn low_turrets_trade_room_for_silhouette() {
    use w5k_forge::family::turret::layout;
    let f = TankTurret;
    let low = layout(&f.with(&[("profile", 0.0)]));
    let tall = layout(&f.with(&[("profile", 1.0)]));
    assert!(low.h < tall.h);
    assert!(low.reload_s(0.0) > tall.reload_s(0.0), "cramped hand loading is slower");
    assert!(low.depression_deg() < tall.depression_deg(), "less room for the breech to swing up");
    assert!((low.reload_s(1.0) - tall.reload_s(1.0)).abs() < 1e-9, "an autoloader does not care");
    let (lb, tb) = (build(&f, &f.with(&[("profile", 0.0)])), build(&f, &f.with(&[("profile", 1.0)])));
    let (la, ta) = (lb.armour.at(0.0, 0.0).area_m2, tb.armour.at(0.0, 0.0).area_m2);
    assert!(la < ta, "lower turret, smaller frontal target: {la} vs {ta}");
}

#[test]
fn every_family_builds_closed_geometry_across_its_range() {
    for fam in w5k_forge::family::all() {
        let params = fam.params();
        for corner in [0.0, 0.5, 1.0] {
            let mut v = fam.defaults();
            for p in &params {
                v.insert(p.id.to_string(), p.value_at(corner));
            }
            let (pieces, _) = w5k_forge::build::build_part(&fam.generate(&v, &lib()));
            assert!(!pieces.is_empty(), "{} at {corner}", fam.id());
            for (i, p) in pieces.iter().enumerate() {
                assert!(p.poly.is_closed(), "{} at {corner}: piece {i} not closed", fam.id());
            }
        }
    }
}

#[test]
fn tracks_size_themselves_to_the_hull() {
    use w5k_forge::family::{hull::Lancer, track::Track, values_for};
    let hull = Lancer.generate(&Lancer.defaults(), &lib());
    let sock = hull.sockets.iter().find(|s| s.name == "gear_r").unwrap();
    let v = values_for(&Track, &Default::default(), &sock.hints);
    let (pieces, _) = w5k_forge::build::build_part(&Track.generate(&v, &lib()));
    let (lo, hi) = w5k_forge::voxel::bounds(&pieces);
    let want = sock.hints["ctx.length"];
    assert!((hi.z - lo.z) > 0.95 * want && (hi.z - lo.z) < 1.05 * want, "{} vs {want}", hi.z - lo.z);
    // The track reaches the ground (y = -bottom below the socket).
    assert!((lo.y + sock.hints["ctx.bottom"]).abs() < 0.05, "{}", lo.y);
}

#[test]
fn wider_tracks_lower_ground_pressure() {
    let mut forge = Forge::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let pressure = |forge: &mut Forge, w: f64| {
        let mut d = forge.designs["lancer_mk1"].clone();
        let src: w5k_forge::schema::DesignDef =
            ron::from_str(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/vehicles/lancer_mk1.ron")).unwrap()).unwrap();
        d.attach = src.attach.clone();
        d.hull = src.hull.clone();
        d.hull_params = src.hull_params.clone();
        for a in d.attach.iter_mut().filter(|a| a.family.as_deref() == Some("track")) {
            a.params.insert("width_m".into(), w);
        }
        let d = forge.instantiate(&d).unwrap();
        forge.build_design(&d).2.ground_pressure_kpa
    };
    let (narrow, wide) = (pressure(&mut forge, 0.4), pressure(&mut forge, 1.0));
    assert!(wide < 0.6 * narrow, "{narrow} -> {wide} kPa");
}
