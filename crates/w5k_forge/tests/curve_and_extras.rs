//! The engine curve fit and the PROVISIONAL extras sidecar of the first truck.

use w5k_forge::curve::{rpm_to_rad_s, torque_curve_through_peaks};
use w5k_forge::extras::Extras;

fn peaks_of(curve: &[(f64, f64)]) -> ((f64, f64), (f64, f64)) {
    let t = curve.iter().fold((0.0, 0.0), |a, &(n, t)| if t > a.1 { (n, t) } else { a });
    let p = curve.iter().map(|&(n, t)| (n, t * rpm_to_rad_s(n))).fold((0.0, 0.0), |a, x| if x.1 > a.1 { x } else { a });
    (t, p)
}

#[test]
fn torque_curve_peaks_match_the_def_peaks() {
    let curve = torque_curve_through_peaks(380.0, 1900.0, 110_000.0, 3400.0, 750.0, 3900.0, 0.55).unwrap();
    let ((n_t, t), (n_p, p)) = peaks_of(&curve);
    assert!((t - 380.0).abs() < 1e-6 && (n_t - 1900.0).abs() < 1e-6);
    assert!((p - 110_000.0).abs() < 1e-3 && (n_p - 3400.0).abs() < 1e-6);
    assert!(curve.windows(2).all(|w| w[1].0 > w[0].0), "rpm must ascend");
    assert!((curve[0].1 - 0.55 * 380.0).abs() < 1e-6, "idle torque follows the template fraction");
}

#[test]
fn peaks_that_admit_no_drivable_curve_are_rejected_with_a_reason() {
    // Torque at the power peak equal to the torque peak: power would still be rising.
    let e = torque_curve_through_peaks(300.0, 2500.0, 110_000.0, 3500.0, 800.0, 4500.0, 0.6).unwrap_err();
    assert!(e.contains("peak power"), "{e}");
    // A power peak beyond the redline.
    assert!(torque_curve_through_peaks(380.0, 1900.0, 110_000.0, 4200.0, 750.0, 3900.0, 0.55).is_err());
}

#[test]
fn the_trucks_extras_sidecar_parses_and_every_param_checks_clean() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/mule_4x4.extras.ron");
    let x: Extras = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(x.check().is_empty(), "{:?}", x.check());
    let again: Extras =
        ron::from_str(&ron::ser::to_string_pretty(&x, ron::ser::PrettyConfig::default()).unwrap()).unwrap();
    assert_eq!(again, x);
}

#[test]
fn the_trucks_def_parses_and_checks_clean() {
    use w5k_contract::def::VehicleDef;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/mule_4x4.ron");
    let d: VehicleDef = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    d.check().unwrap();
    assert_eq!(d.id, "mule_4x4");
    let again: VehicleDef =
        ron::from_str(&ron::ser::to_string_pretty(&d, ron::ser::PrettyConfig::default()).unwrap()).unwrap();
    assert_eq!(again, d);
}
