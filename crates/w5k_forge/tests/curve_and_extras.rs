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

#[test]
fn a_power_peak_that_torque_cannot_reach_is_rejected_as_inconsistent() {
    // 135 N m at 3000 rpm falling to 93 N m at 4800 rpm needs the power to peak earlier than 4800 rpm (curve exponent below 1).
    let e = torque_curve_through_peaks(135.0, 3000.0, 46_800.0, 4800.0, 800.0, 5600.0, 0.6).unwrap_err();
    assert!(e.contains("inconsistent") && e.contains("exponent"), "{e}");
}

#[test]
fn the_fitted_curve_has_one_power_maximum_and_falls_after_the_torque_peak() {
    let curve = torque_curve_through_peaks(135.0, 3000.0, 52_000.0, 4800.0, 800.0, 5600.0, 0.6).unwrap();
    let power = |&(n, t): &(f64, f64)| t * rpm_to_rad_s(n);
    let (_, p_max_at) =
        curve.iter().enumerate().fold((0.0, 0), |a, (i, p)| if power(p) > a.0 { (power(p), i) } else { a });
    assert!(curve[..=p_max_at].windows(2).all(|w| power(&w[1]) >= power(&w[0]) - 1e-9), "power rises up to its peak");
    assert!(curve[p_max_at..].windows(2).all(|w| power(&w[1]) <= power(&w[0]) + 1e-9), "and falls after it");
    let torque_peak_at = curve.iter().position(|p| (p.0 - 3000.0).abs() < 1e-6).unwrap();
    assert!(curve[torque_peak_at..].windows(2).all(|w| w[1].1 <= w[0].1 + 1e-9), "torque falls after its peak");
}
