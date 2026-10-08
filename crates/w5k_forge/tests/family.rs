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
    let def = fam.generate(v);
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
            let pieces = w5k_forge::build::build_part(&f.generate(&v));
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
