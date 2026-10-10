//! Suspension: a spring (any `SpringKind`), an asymmetric digressive damper with optional dry friction, and a progressive bump stop,
//! as one [`SuspensionElement`]. Forces are totals along the strut axis, positive pushing wheel and hull apart; compression is measured
//! from the rest position. TRACKS reuses this for road wheels.

use w5k_contract::rig::{BumpStopDef, DamperDef, SpringKind, SuspensionDef};
use w5k_contract::{SuspensionElement, SuspensionOut};
use w5k_math::scalar;

/// Numerical regularisation that is not a property of any one vehicle (loaded from `content/physics/chassis` as a `Param` by the caller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuspensionTuning {
    /// Strut speed over which dry friction ramps from zero to its full value (tanh scale), m/s.
    pub friction_smoothing_m_s: f64,
    /// A gas volume is never allowed below this fraction of its rest value (stops the polytropic law diverging), dimensionless.
    pub min_gas_volume_frac: f64,
}

#[derive(Clone, Debug)]
pub struct Suspension {
    def: SuspensionDef,
    /// Travel from the rest position toward the hull, where a hard stop (if declared) acts, m.
    bump_travel_m: f64,
    tuning: SuspensionTuning,
}

impl Suspension {
    pub fn new(def: &SuspensionDef, bump_travel_m: f64, tuning: SuspensionTuning) -> Suspension {
        Suspension { def: def.clone(), bump_travel_m, tuning }
    }

    /// Spring force (N) at compression `c_m`, never negative: a spring cannot pull.
    pub fn spring_force_n(&self, c_m: f64) -> f64 {
        let f = match &self.def.spring {
            SpringKind::Rigid => self.def.preload_n,
            SpringKind::Linear { rate_n_m } => self.def.preload_n + rate_n_m * c_m,
            SpringKind::Table { points } => table_force(points, c_m),
            SpringKind::Torsion { rate_nm_rad, arm_length_m, rest_arm_angle_rad } => {
                let phi0 = *rest_arm_angle_rad;
                let t0 = self.def.preload_n * arm_length_m * scalar::cos(phi0);
                // sin(phi) = sin(phi0) - c / L, kept inside (-1, 1) so the arm cannot pass vertical
                let s = scalar::clamp(scalar::sin(phi0) - c_m / arm_length_m, -0.999, 0.999); // const-ok: keeps cos(phi) away from zero
                let phi = scalar::asin(s);
                (t0 + rate_nm_rad * (phi0 - phi)) / (arm_length_m * scalar::cos(phi))
            }
            SpringKind::Hydropneumatic { gas_pressure_pa, gas_volume_m3, piston_area_m2, gamma } => {
                let v = (gas_volume_m3 - piston_area_m2 * c_m).max(gas_volume_m3 * self.tuning.min_gas_volume_frac);
                gas_pressure_pa * scalar::pow(gas_volume_m3 / v, *gamma) * piston_area_m2
            }
        };
        f.max(0.0)
    }

    /// Damper force (N) on the wheel for a strut speed (positive = compressing): resists the motion.
    pub fn damper_force_n(&self, rate_m_s: f64) -> f64 {
        let d: &DamperDef = &self.def.damper;
        let coeff = if rate_m_s >= 0.0 { d.bump_ns_m } else { d.rebound_ns_m };
        let v = rate_m_s.abs();
        let viscous = if d.knee_speed_m_s > 0.0 && v > d.knee_speed_m_s {
            coeff * d.knee_speed_m_s + coeff * d.post_knee_ratio * (v - d.knee_speed_m_s)
        } else {
            coeff * v
        };
        let dry = d.friction_n * scalar::tanh(v / self.tuning.friction_smoothing_m_s);
        scalar::sign(rate_m_s) * (viscous + dry)
    }

    /// Bump-stop force (N): zero until `engage_m`, then a stiffness that grows with penetration, plus damping while closing.
    pub fn bump_stop_force_n(&self, c_m: f64, rate_m_s: f64) -> f64 {
        let b: &BumpStopDef = &self.def.bump_stop;
        let pen = c_m - b.engage_m;
        if pen <= 0.0 {
            return 0.0;
        }
        let k = b.rate_n_m * (1.0 + b.progression * pen / b.engage_m);
        (k * pen + b.damping_ns_m * rate_m_s.max(0.0)).max(0.0)
    }

    /// A declared rigid stop: clamp the compression at the travel limit and reflect the approach speed with the restitution.
    /// Returns `(compression, rate)`; the identity when no hard limit is declared or the limit is not reached.
    pub fn apply_hard_limit(&self, c_m: f64, rate_m_s: f64) -> (f64, f64) {
        let b = &self.def.bump_stop;
        if b.hard_limit && c_m > self.bump_travel_m {
            (self.bump_travel_m, if rate_m_s > 0.0 { -b.restitution * rate_m_s } else { rate_m_s })
        } else {
            (c_m, rate_m_s)
        }
    }
}

impl SuspensionElement for Suspension {
    fn step(&mut self, compression_m: f64, rate_m_s: f64, _dt_s: f64) -> SuspensionOut {
        let spring_n = self.spring_force_n(compression_m);
        let damper_n = self.damper_force_n(rate_m_s);
        let bump_stop_n = self.bump_stop_force_n(compression_m, rate_m_s);
        SuspensionOut {
            force_n: spring_n + damper_n + bump_stop_n,
            spring_n,
            damper_n,
            bump_stop_n,
            at_limit: compression_m >= self.bump_travel_m,
        }
    }
}

/// Piecewise-linear table of `(compression, force)`, extended linearly beyond the ends with the nearest segment.
fn table_force(points: &[(f64, f64)], c: f64) -> f64 {
    if points.len() < 2 {
        return points.first().map_or(0.0, |p| p.1);
    }
    let i = points.partition_point(|p| p.0 < c).clamp(1, points.len() - 1);
    let (a, b) = (points[i - 1], points[i]);
    a.1 + (b.1 - a.1) * (c - a.0) / (b.0 - a.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TUNING: SuspensionTuning = SuspensionTuning { friction_smoothing_m_s: 0.01, min_gas_volume_frac: 0.05 };

    fn def(spring: SpringKind, preload_n: f64) -> SuspensionDef {
        SuspensionDef {
            spring,
            preload_n,
            damper: DamperDef {
                bump_ns_m: 2000.0,
                rebound_ns_m: 4000.0,
                knee_speed_m_s: 0.0,
                post_knee_ratio: 1.0,
                friction_n: 0.0,
            },
            bump_stop: BumpStopDef {
                engage_m: 0.10,
                rate_n_m: 100e3,
                progression: 0.0,
                damping_ns_m: 0.0,
                hard_limit: false,
                restitution: 0.0,
            },
        }
    }

    #[test]
    fn hydropneumatic_spring_follows_polytropic_law() {
        let (p0, v0, a, g) = (2.0e6, 1.0e-3, 5.0e-3, 1.4);
        let s = Suspension::new(
            &def(
                SpringKind::Hydropneumatic { gas_pressure_pa: p0, gas_volume_m3: v0, piston_area_m2: a, gamma: g },
                p0 * a,
            ),
            0.2,
            TUNING,
        );
        for c in [-0.05, 0.0, 0.03, 0.08] {
            let v = v0 - a * c;
            let expect = p0 * scalar::pow(v0 / v, g) * a; // test oracle: p V^gamma = const
            assert!((s.spring_force_n(c) / expect - 1.0).abs() < 1e-12, "c={c}");
        }
        assert!((s.spring_force_n(0.0) - p0 * a).abs() < 1e-9);
    }

    #[test]
    fn torsion_bar_wheel_rate_matches_arm_geometry() {
        // arm 0.4 m, level at rest (phi0 = 0): wheel rate at rest is k_theta / L^2 (force = T/L, c = L dphi)
        let (k_theta, l) = (6000.0, 0.4);
        let s = Suspension::new(
            &def(SpringKind::Torsion { rate_nm_rad: k_theta, arm_length_m: l, rest_arm_angle_rad: 0.0 }, 8000.0),
            0.2,
            TUNING,
        );
        let h = 1e-6;
        let rate = (s.spring_force_n(h) - s.spring_force_n(-h)) / (2.0 * h);
        assert!((rate / (k_theta / (l * l)) - 1.0).abs() < 1e-3, "{rate}");
        // an arm standing below the horizontal at rest
        let phi0: f64 = 0.35;
        let s2 = Suspension::new(
            &def(SpringKind::Torsion { rate_nm_rad: k_theta, arm_length_m: l, rest_arm_angle_rad: phi0 }, 8000.0),
            0.2,
            TUNING,
        );
        assert!((s2.spring_force_n(0.0) - 8000.0).abs() < 1e-9);
        let rate2 = (s2.spring_force_n(h) - s2.spring_force_n(-h)) / (2.0 * h);
        // dF/dc = k/(L cos phi)^2 - P sin(phi)/(L cos^2 phi): the preload's lever arm shortens as the arm rises
        let c2 = scalar::cos(phi0) * scalar::cos(phi0);
        let expect = k_theta / (l * l * c2) - 8000.0 * scalar::sin(phi0) / (l * c2);
        assert!((rate2 / expect - 1.0).abs() < 1e-2, "{rate2} vs {expect}");
    }

    #[test]
    fn bump_stop_engages_at_the_stated_travel() {
        let mut s = Suspension::new(&def(SpringKind::Linear { rate_n_m: 30e3 }, 3000.0), 0.15, TUNING);
        assert!(s.step(0.099, 0.0, 0.001).bump_stop_n.abs() < 1e-12);
        let o = s.step(0.12, 0.0, 0.001);
        assert!((o.bump_stop_n - 100e3 * 0.02).abs() < 1e-9);
        assert!(!o.at_limit);
        assert!(s.step(0.15, 0.0, 0.001).at_limit);
    }

    #[test]
    fn damper_force_is_asymmetric_and_knees_at_the_stated_speed() {
        let mut d = def(SpringKind::Linear { rate_n_m: 30e3 }, 3000.0);
        d.damper = DamperDef {
            bump_ns_m: 2000.0,
            rebound_ns_m: 4000.0,
            knee_speed_m_s: 0.3,
            post_knee_ratio: 0.25,
            friction_n: 0.0,
        };
        let s = Suspension::new(&d, 0.2, TUNING);
        // below the knee: c v; rebound is twice the bump force and opposite in sign
        assert!((s.damper_force_n(0.1) - 200.0).abs() < 1e-9);
        assert!((s.damper_force_n(-0.1) + 400.0).abs() < 1e-9);
        // at the knee the force is continuous, above it the slope is the post-knee fraction
        assert!((s.damper_force_n(0.3) - 600.0).abs() < 1e-9);
        assert!((s.damper_force_n(0.5) - (600.0 + 0.25 * 2000.0 * 0.2)).abs() < 1e-9);
    }

    #[test]
    fn table_spring_is_total_force_and_never_pulls() {
        let pts = vec![(-0.1, 500.0), (0.0, 3000.0), (0.1, 9000.0)];
        let s = Suspension::new(&def(SpringKind::Table { points: pts }, 3000.0), 0.2, TUNING);
        assert!((s.spring_force_n(0.0) - 3000.0).abs() < 1e-9);
        assert!((s.spring_force_n(0.05) - 6000.0).abs() < 1e-9);
        assert!(s.spring_force_n(-0.5).abs() < 1e-12); // extended droop segment would be negative
    }

    #[test]
    fn hard_limit_clamps_and_reflects_with_restitution() {
        let mut d = def(SpringKind::Linear { rate_n_m: 30e3 }, 3000.0);
        d.bump_stop.hard_limit = true;
        d.bump_stop.restitution = 0.3;
        let s = Suspension::new(&d, 0.15, TUNING);
        assert_eq!(s.apply_hard_limit(0.16, 2.0), (0.15, -0.6));
        assert_eq!(s.apply_hard_limit(0.10, 2.0), (0.10, 2.0));
    }

    #[test]
    fn dry_friction_is_odd_in_speed_and_zero_at_rest() {
        let mut d = def(SpringKind::Linear { rate_n_m: 30e3 }, 3000.0);
        d.damper = DamperDef {
            bump_ns_m: 0.0,
            rebound_ns_m: 0.0,
            knee_speed_m_s: 0.0,
            post_knee_ratio: 1.0,
            friction_n: 500.0,
        };
        let s = Suspension::new(&d, 0.2, TUNING);
        assert!(s.damper_force_n(0.0).abs() < 1e-12);
        assert!((s.damper_force_n(0.5) - 500.0).abs() < 1e-6 && (s.damper_force_n(-0.5) + 500.0).abs() < 1e-6);
    }
}
