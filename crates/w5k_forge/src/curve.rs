//! Engine torque curve from the def's two peaks (spike S5, promoted).

use w5k_math::scalar;

/// rpm to rad/s.
pub fn rpm_to_rad_s(rpm: f64) -> f64 {
    rpm * scalar::TAU / 60.0 // const-ok: unit conversion, 60 s per minute
}

/// Full-load torque curve through the def's two peaks, exact at both. Above the torque peak `T(n) = T_pk + a d^2 + b d^3`, `d = n - n_T`
/// (vertex at the peak; `a`, `b` solve `T(n_P) = P / omega_P` and `dP/dn = 0` at `n_P`); below it a parabola with the same vertex that
/// reaches `idle_torque_frac` of the peak at idle (a single cubic blows up on the low side). Rejects peaks that admit no such curve.
pub fn torque_curve_through_peaks(
    t_pk: f64,
    n_t: f64,
    p_pk_w: f64,
    n_p: f64,
    idle: f64,
    redline: f64,
    idle_torque_frac: f64,
) -> Result<Vec<(f64, f64)>, String> {
    let t_p = p_pk_w / rpm_to_rad_s(n_p);
    let d_p = n_p - n_t;
    if d_p < 1.0 || t_p >= t_pk {
        return Err(format!(
            "peak power ({t_p:.0} N m at {n_p} rpm) must lie past the torque peak ({t_pk} N m at {n_t} rpm) and be strictly below it"
        ));
    }
    if !(idle < n_t && n_p < redline) {
        return Err("the torque peak must lie above idle and the power peak below the redline".into());
    }
    let x = t_p - t_pk;
    let b = (-t_p / n_p - 2.0 * x / d_p) / (d_p * d_p);
    let a = (x - b * d_p * d_p * d_p) / (d_p * d_p);
    let q_low = (1.0 - idle_torque_frac) * t_pk / ((n_t - idle) * (n_t - idle));
    let t = |n: f64| {
        let d = n - n_t;
        if d < 0.0 {
            t_pk - q_low * d * d
        } else {
            t_pk + a * d * d + b * d * d * d
        }
    };
    let steps = 40; // const-ok: sampling resolution of the stored curve
    let mut pts: Vec<(f64, f64)> =
        (0..=steps).map(|k| idle + (redline - idle) * f64::from(k) / f64::from(steps)).map(|n| (n, t(n))).collect();
    pts.push((n_t, t_pk));
    pts.push((n_p, t_p));
    pts.sort_by(|p, q| p.0.total_cmp(&q.0));
    pts.dedup_by(|p, q| (p.0 - q.0).abs() < 1e-6); // const-ok: rpm values closer than this are the same sample
    let tol = 1e-9; // const-ok: float noise allowance on exact-by-construction peaks
    if let Some(bad) = pts.iter().find(|p| p.1 <= 0.0) {
        return Err(format!("the peaks give a torque of {:.0} N m at {:.0} rpm: not a drivable engine", bad.1, bad.0));
    }
    if let Some(bad) = pts.iter().find(|p| p.1 > t_pk * (1.0 + tol) || p.1 * rpm_to_rad_s(p.0) > p_pk_w * (1.0 + tol)) {
        return Err(format!("the peaks give a higher torque or power at {:.0} rpm than the stated peaks", bad.0));
    }
    Ok(pts)
}
