//! A tyre on soft ground (Bekker's rigid wheel; Wong, *Theory of Ground Vehicles*, ch. 2). `UNVALIDATED` until TRACKS' spike S4 cites
//! Wong's worked examples.
//!
//! The soil under a plate of width `b` pushes back with `p(z) = (kc / b + kphi) z^n` at sinkage `z`. Integrated over the arc of a rigid wheel
//! of radius `R` that has sunk by `z`, that gives the load the soil carries, `W(z) = b (kc / b + kphi) sqrt(2R) z^(n + 1/2) (3 - n) / 3`, and the
//! work spent making the rut, per metre travelled, is the compaction resistance `R_c = b (kc / b + kphi) z^(n+1) / (n + 1)`. The tyre's own
//! spring sits in series with the soil: a penetration `pen` splits into tyre deflection and sinkage with `k_t (pen - z) = W(z)`.

use w5k_contract::SoilParams;
use w5k_math::scalar;

/// Plate pressure at sinkage `z_m` under a plate of width `b_m` (Bekker), Pa.
/// PROVISIONAL: replace with `w5k_terramech`'s soil function when TRACKS lands it (one law, one place; ARCH approved the dependency).
pub fn bekker_pressure_pa(soil: &SoilParams, b_m: f64, z_m: f64) -> f64 {
    (soil.kc_pa_m_n1 / b_m + soil.kphi_pa_m_n) * scalar::pow(z_m.max(0.0), soil.n)
}

/// Load a rigid wheel of radius `r_m` and width `b_m` carries at sinkage `z_m`, N.
pub fn rigid_wheel_load_n(soil: &SoilParams, b_m: f64, r_m: f64, z_m: f64) -> f64 {
    let k = soil.kc_pa_m_n1 + b_m * soil.kphi_pa_m_n; // = b (kc / b + kphi)
    k * scalar::sqrt(2.0 * r_m) * scalar::pow(z_m.max(0.0), soil.n + 0.5) * (3.0 - soil.n) / 3.0
}

/// Compaction resistance of a wheel of width `b_m` at sinkage `z_m`, N (the rut's work per metre).
pub fn compaction_resistance_n(soil: &SoilParams, b_m: f64, z_m: f64) -> f64 {
    (soil.kc_pa_m_n1 + b_m * soil.kphi_pa_m_n) * scalar::pow(z_m.max(0.0), soil.n + 1.0) / (soil.n + 1.0)
}

/// Mohr-Coulomb shear strength of a contact patch of area `area_m2` carrying `load_n`: `A c + W tan(phi)`, N.
pub fn shear_strength_n(soil: &SoilParams, area_m2: f64, load_n: f64) -> f64 {
    area_m2 * soil.cohesion_pa + load_n.max(0.0) * scalar::tan(soil.friction_angle_rad)
}

/// Split a penetration between the tyre spring (`k_t`, N/m) and the soil: the sinkage `z` in `[0, pen]` with `k_t (pen - z) = W(z)`.
/// `W` rises and the tyre force falls with `z`, so the root is unique; bisection with a fixed number of halvings keeps it deterministic.
pub fn series_sinkage_m(soil: &SoilParams, b_m: f64, r_m: f64, k_t_n_m: f64, pen_m: f64) -> f64 {
    if pen_m <= 0.0 {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, pen_m);
    for _ in 0..52 {
        // const-ok: halvings to resolve the sinkage to the last bit of an f64 mantissa
        let mid = 0.5 * (lo + hi);
        if rigid_wheel_load_n(soil, b_m, r_m, mid) > k_t_n_m * (pen_m - mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mud() -> SoilParams {
        SoilParams {
            n: 0.8,
            kc_pa_m_n1: 13_190.0,
            kphi_pa_m_n: 692_200.0,
            cohesion_pa: 4_140.0,
            friction_angle_rad: 0.2269,
            shear_k_m: 0.025,
        }
    }

    #[test]
    fn rigid_wheel_sinkage_matches_bekker_closed_form() {
        // a very stiff tyre is a rigid wheel: z0 = [3 W / ((3 - n)(kc + b kphi) sqrt(D))]^(2 / (2n + 1)), D = 2R
        let (s, b, r, w) = (mud(), 0.3, 0.45, 8000.0);
        let z0 = scalar::pow(
            3.0 * w / ((3.0 - s.n) * (s.kc_pa_m_n1 + b * s.kphi_pa_m_n) * scalar::sqrt(2.0 * r)),
            2.0 / (2.0 * s.n + 1.0),
        );
        assert!((rigid_wheel_load_n(&s, b, r, z0) / w - 1.0).abs() < 1e-12);
        // in series with a 1e12 N/m tyre, a penetration of z0 + W / k_t sinks by z0
        let k_t = 1e12;
        let z = series_sinkage_m(&s, b, r, k_t, z0 + w / k_t);
        assert!((z / z0 - 1.0).abs() < 1e-6, "{z} vs {z0}");
    }

    #[test]
    fn compaction_resistance_is_the_work_of_pressing_the_rut() {
        // R_c = integral of b p(z') dz' from 0 to z: the work per metre of a plate of width b pressed to depth z
        let (s, b, z) = (mud(), 0.3, 0.06);
        let steps = 20_000;
        let dz = z / steps as f64;
        let work: f64 = (0..steps).map(|i| b * bekker_pressure_pa(&s, b, (i as f64 + 0.5) * dz) * dz).sum();
        assert!((compaction_resistance_n(&s, b, z) / work - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_tyre_and_the_soil_share_the_penetration_and_carry_the_same_force() {
        let (s, b, r, k_t, pen) = (mud(), 0.3, 0.45, 250e3, 0.08);
        let z = series_sinkage_m(&s, b, r, k_t, pen);
        assert!(z > 0.0 && z < pen);
        assert!((k_t * (pen - z) / rigid_wheel_load_n(&s, b, r, z) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_wider_tyre_sinks_less_and_rolls_easier() {
        let (s, r, k_t, w) = (mud(), 0.45, 250e3, 8000.0);
        // carry the same load on two widths: find the penetration that gives W, then compare sinkage and compaction resistance
        let at_load = |b: f64| {
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                let z = series_sinkage_m(&s, b, r, k_t, mid);
                if k_t * (mid - z) > w {
                    hi = mid
                } else {
                    lo = mid
                }
            }
            let z = series_sinkage_m(&s, b, r, k_t, 0.5 * (lo + hi));
            (z, compaction_resistance_n(&s, b, z))
        };
        let ((z1, rc1), (z2, rc2)) = (at_load(0.25), at_load(0.5));
        assert!(z2 < z1 && rc2 < rc1, "narrow {z1} m {rc1} N, wide {z2} m {rc2} N");
    }

    #[test]
    fn soft_ground_strength_is_a_c_plus_w_tan_phi() {
        let s = mud();
        assert!((shear_strength_n(&s, 0.06, 8000.0) - (0.06 * 4140.0 + 8000.0 * scalar::tan(0.2269))).abs() < 1e-9);
    }
}
