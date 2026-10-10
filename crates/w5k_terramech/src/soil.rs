//! Soil laws: the pure functions a track sample, a rigid wheel or a tyre calls. No state, no allocation.
//!
//! * Bekker pressure-sinkage: `p = (kc/b + kphi) z^n`, with the exponent `n` free (not wired to 1).
//! * Mohr-Coulomb strength: `tau_max = c + p tan(phi)`.
//! * Janosi-Hanamoto shear: `tau = tau_max (1 - exp(-j / K))`.
//! * Compaction resistance: the work done pressing soil down, `R_c = b * integral p dz = b k z^(n+1) / (n+1)`.
//! * Rigid wheel (Wong): sinkage and compaction resistance of a wheel of diameter `D`, so a tyre can sink too.
//!
//! Units are SI; `kc` is Pa / m^(n-1) and `kphi` is Pa / m^n, as in `SoilParams`. Every transcendental is `libm` through `w5k_math`.

use w5k_contract::SoilParams;
use w5k_math::scalar;

/// The combined modulus `k = kc / b + kphi` of a plate of width `b_m` (the smaller plate dimension), Pa / m^n.
pub fn modulus_pa_m_n(soil: &SoilParams, b_m: f64) -> f64 {
    soil.kc_pa_m_n1 / b_m + soil.kphi_pa_m_n
}

/// Normal pressure under a plate of width `b_m` sunk to `z_m`. Zero for `z_m <= 0` (no tension in soil).
pub fn pressure_pa(soil: &SoilParams, b_m: f64, z_m: f64) -> f64 {
    if z_m <= 0.0 {
        return 0.0;
    }
    modulus_pa_m_n(soil, b_m) * scalar::pow(z_m, soil.n)
}

/// Inverse of [`pressure_pa`]: the sinkage that carries pressure `p_pa`, `z = (p / k)^(1/n)`.
pub fn sinkage_m(soil: &SoilParams, b_m: f64, p_pa: f64) -> f64 {
    if p_pa <= 0.0 {
        return 0.0;
    }
    scalar::pow(p_pa / modulus_pa_m_n(soil, b_m), 1.0 / soil.n) // const-ok: inverse exponent
}

/// Mohr-Coulomb shear strength under normal pressure `p_pa`: `c + p tan(phi)`.
pub fn shear_strength_pa(soil: &SoilParams, p_pa: f64) -> f64 {
    soil.cohesion_pa + p_pa.max(0.0) * scalar::tan(soil.friction_angle_rad)
}

/// Shear stress developed after shear displacement `j_m >= 0` (Janosi-Hanamoto): `tau_max (1 - exp(-j / K))`.
pub fn shear_stress_pa(tau_max_pa: f64, j_m: f64, k_m: f64) -> f64 {
    tau_max_pa * saturation(j_m / k_m)
}

/// `1 - exp(-s)` for `s >= 0`, accurate for small `s`.
pub fn saturation(s: f64) -> f64 {
    -scalar::exp_m1(-s.max(0.0))
}

/// `(1 - exp(-s)) / s`, the ratio shear stress / (tau_max s); 1 at s = 0. Lets a shear *vector* `j` be turned into a stress vector
/// `tau_max * ratio(|j|/K) * j / K` that is smooth through zero displacement.
pub fn saturation_over_s(s: f64) -> f64 {
    if s < 1e-8 {
        // const-ok: series switch point for (1 - e^-s) / s
        1.0 - 0.5 * s
    } else {
        saturation(s) / s
    }
}

/// Compaction resistance of a plate/track of width `b_m` pressed from sinkage `z_from_m` to `z_to_m` (`z_to >= z_from`): `b * integral p dz`.
/// From the surface (`z_from = 0`) and `n = 1` this is `b p z / 2`.
pub fn compaction_resistance_n(soil: &SoilParams, b_m: f64, z_from_m: f64, z_to_m: f64) -> f64 {
    let (a, z) = (z_from_m.max(0.0), z_to_m.max(0.0));
    if z <= a {
        return 0.0;
    }
    let e = soil.n + 1.0;
    b_m * modulus_pa_m_n(soil, b_m) * (scalar::pow(z, e) - scalar::pow(a, e)) / e
}

/// Sinkage of a rigid wheel of diameter `d_m`, width `b_m`, carrying `w_n` (Wong, small-sinkage form):
/// `z0 = [3 W / (b (3 - n) k sqrt(D))]^(2 / (2n + 1))`.
pub fn rigid_wheel_sinkage_m(soil: &SoilParams, w_n: f64, b_m: f64, d_m: f64) -> f64 {
    if w_n <= 0.0 {
        return 0.0;
    }
    let k = modulus_pa_m_n(soil, b_m);
    let base = 3.0 * w_n / (b_m * (3.0 - soil.n) * k * scalar::sqrt(d_m)); // const-ok: Wong's closed form
    scalar::pow(base, 2.0 / (2.0 * soil.n + 1.0)) // const-ok: Wong's closed form
}

/// Compaction resistance of a rigid wheel at sinkage `z0_m`: `b k z0^(n+1) / (n+1)`.
pub fn rigid_wheel_compaction_resistance_n(soil: &SoilParams, b_m: f64, z0_m: f64) -> f64 {
    compaction_resistance_n(soil, b_m, 0.0, z0_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::reference::reference_soils;

    fn soils() -> Vec<(String, SoilParams)> {
        reference_soils().into_iter().map(|m| (m.name.clone(), m.soil.unwrap())).collect()
    }

    #[test]
    fn plate_sinkage_matches_bekker_closed_form() {
        for (name, s) in soils() {
            let (b, p) = (0.5, 40_000.0);
            let z = sinkage_m(&s, b, p);
            let by_hand = scalar::pow(p / (s.kc_pa_m_n1 / b + s.kphi_pa_m_n), 1.0 / s.n);
            assert!(scalar::approx_eq(z, by_hand, 1e-15), "{name}");
            assert!(scalar::approx_eq(pressure_pa(&s, b, z), p, 1e-6), "{name}: round trip");
        }
    }

    #[test]
    fn ground_pressure_equals_weight_over_contact_area() {
        // Two tracks 0.55 m wide, 3.0 m long carry 11 t: p = W / (2 b L).
        let (w, b, l) = (11_000.0 * 9.81, 0.55, 3.0);
        let p = w / (2.0 * b * l);
        for (name, s) in soils() {
            let z = sinkage_m(&s, b, p);
            let area_load = pressure_pa(&s, b, z) * 2.0 * b * l;
            assert!(scalar::approx_eq(area_load, w, 1e-6), "{name}");
        }
    }

    #[test]
    fn shear_stress_saturates_at_cohesion_plus_p_tan_phi() {
        for (name, s) in soils() {
            let tau_max = shear_strength_pa(&s, 30_000.0);
            let by_hand = s.cohesion_pa + 30_000.0 * scalar::tan(s.friction_angle_rad);
            assert!(scalar::approx_eq(tau_max, by_hand, 1e-9), "{name}");
            let far = shear_stress_pa(tau_max, 40.0 * s.shear_k_m, s.shear_k_m);
            assert!(scalar::approx_eq(far, tau_max, 1e-9 * tau_max), "{name}");
        }
    }

    #[test]
    fn shear_curve_slope_at_zero_displacement_is_tau_max_over_k() {
        let (tau_max, k) = (12_000.0, 0.025);
        let h = 1e-7; // const-ok: finite-difference step
        let slope = shear_stress_pa(tau_max, h, k) / h;
        assert!(scalar::approx_eq(slope, tau_max / k, 1e-5 * tau_max / k));
        assert!(scalar::approx_eq(saturation_over_s(0.0), 1.0, 1e-15));
    }

    #[test]
    fn compaction_resistance_for_n_equal_one_is_half_b_p_z() {
        let s = SoilParams { n: 1.0, ..soils()[0].1 };
        let (b, z) = (0.6, 0.04);
        let p = pressure_pa(&s, b, z);
        assert!(scalar::approx_eq(compaction_resistance_n(&s, b, 0.0, z), 0.5 * b * p * z, 1e-9));
    }

    #[test]
    fn compaction_resistance_equals_the_numerical_integral_of_pressure_over_sinkage() {
        for (name, s) in soils() {
            let (b, z, steps) = (0.5, 0.06, 20_000);
            let dz = z / f64::from(steps);
            let mut sum = 0.0;
            for i in 0..steps {
                sum += pressure_pa(&s, b, (f64::from(i) + 0.5) * dz) * dz * b;
            }
            let r = compaction_resistance_n(&s, b, 0.0, z);
            assert!(scalar::approx_eq(r, sum, 1e-3 * r), "{name}: {r} vs {sum}");
        }
    }

    #[test]
    fn rigid_wheel_sinks_less_when_wider_and_sinks_more_when_heavier() {
        let s = soils()[1].1;
        let z = |w, b| rigid_wheel_sinkage_m(&s, w, b, 1.0);
        assert!(z(20_000.0, 0.4) < z(20_000.0, 0.2));
        assert!(z(30_000.0, 0.3) > z(20_000.0, 0.3));
    }
}
