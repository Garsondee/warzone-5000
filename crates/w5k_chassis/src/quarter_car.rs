//! The quarter-car bench: one corner of a vehicle (sprung mass, unsprung mass, spring-damper, tyre spring) integrated with the same
//! semi-implicit Euler the full chassis uses. It is the oracle bench for the integrator and the ride maths: its answers have closed forms.
//!
//! Heights are positive up; the strut's zero-force length is a gap of 0, so it is compressed when the wheel is above the sprung mass's datum.
//!  The road is a prescribed height; the tyre can push but never pull (it leaves the road instead).

use w5k_math::scalar;

/// Parameters of one corner. SI: kg, N/m, N s/m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuarterCarParams {
    pub sprung_mass_kg: f64,
    pub unsprung_mass_kg: f64,
    pub spring_rate_n_m: f64,
    pub damper_ns_m: f64,
    pub tyre_rate_n_m: f64,
    /// Gravity magnitude, m/s^2 (0 for the frictionless oscillator tests).
    pub gravity_m_s2: f64,
}

/// Heights (m) and velocities (m/s) of both masses.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuarterCarState {
    pub z_sprung_m: f64,
    pub z_unsprung_m: f64,
    pub v_sprung_m_s: f64,
    pub v_unsprung_m_s: f64,
}

impl QuarterCarParams {
    /// Advance one step of `dt_s` over a road of height `road_m`: forces from the current state, velocities first, then positions.
    pub fn step(&self, s: &mut QuarterCarState, road_m: f64, dt_s: f64) {
        let strut_n = self.spring_rate_n_m * (s.z_unsprung_m - s.z_sprung_m)
            + self.damper_ns_m * (s.v_unsprung_m_s - s.v_sprung_m_s);
        let tyre_n = (self.tyre_rate_n_m * (road_m - s.z_unsprung_m)).max(0.0);
        s.v_sprung_m_s += (strut_n / self.sprung_mass_kg - self.gravity_m_s2) * dt_s;
        s.v_unsprung_m_s += ((tyre_n - strut_n) / self.unsprung_mass_kg - self.gravity_m_s2) * dt_s;
        s.z_sprung_m += s.v_sprung_m_s * dt_s;
        s.z_unsprung_m += s.v_unsprung_m_s * dt_s;
    }

    /// Static equilibrium on a road at height 0 (the tyre stays in contact there, so small oscillations about it are linear).
    pub fn equilibrium(&self) -> QuarterCarState {
        let z_unsprung_m = -(self.sprung_mass_kg + self.unsprung_mass_kg) * self.gravity_m_s2 / self.tyre_rate_n_m;
        let z_sprung_m = z_unsprung_m - self.sprung_mass_kg * self.gravity_m_s2 / self.spring_rate_n_m;
        QuarterCarState { z_sprung_m, z_unsprung_m, ..Default::default() }
    }

    /// Kinetic, elastic and gravitational energy (J), tyre compressed against a road at `road_m`.
    pub fn energy_j(&self, s: &QuarterCarState, road_m: f64) -> f64 {
        let defl = (road_m - s.z_unsprung_m).max(0.0);
        0.5 * self.sprung_mass_kg * s.v_sprung_m_s * s.v_sprung_m_s
            + 0.5 * self.unsprung_mass_kg * s.v_unsprung_m_s * s.v_unsprung_m_s
            + 0.5 * self.spring_rate_n_m * (s.z_unsprung_m - s.z_sprung_m) * (s.z_unsprung_m - s.z_sprung_m)
            + 0.5 * self.tyre_rate_n_m * defl * defl
            + self.gravity_m_s2 * (self.sprung_mass_kg * s.z_sprung_m + self.unsprung_mass_kg * s.z_unsprung_m)
    }

    /// The two undamped natural frequencies (Hz), body then wheel hop: roots of `mu ms w^4 - ((ms+mu) k_t... ) ` solved as a quadratic in w^2.
    pub fn undamped_frequencies_hz(&self) -> (f64, f64) {
        let (ms, mu, k, kt) = (self.sprung_mass_kg, self.unsprung_mass_kg, self.spring_rate_n_m, self.tyre_rate_n_m);
        // det [[k - ms w2, -k], [-k, k + kt - mu w2]] = 0  =>  ms mu w2^2 - (ms (k + kt) + mu k) w2 + k kt = 0
        let b = ms * (k + kt) + mu * k;
        let disc = scalar::sqrt(b * b - 4.0 * ms * mu * k * kt);
        let w2_lo = (b - disc) / (2.0 * ms * mu);
        let w2_hi = (b + disc) / (2.0 * ms * mu);
        let two_pi = 2.0 * core::f64::consts::PI;
        (scalar::sqrt(w2_lo) / two_pi, scalar::sqrt(w2_hi) / two_pi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn truck_corner() -> QuarterCarParams {
        QuarterCarParams {
            sprung_mass_kg: 400.0,
            unsprung_mass_kg: 50.0,
            spring_rate_n_m: 30e3,
            damper_ns_m: 2e3,
            tyre_rate_n_m: 250e3,
            gravity_m_s2: 9.81,
        }
    }

    /// Frequency of a free oscillation from the mean spacing of upward zero crossings of the sprung height.
    fn measured_hz(p: &QuarterCarParams, dt: f64, seconds: f64) -> f64 {
        let eq = p.equilibrium();
        let mut s = QuarterCarState { z_sprung_m: eq.z_sprung_m + 0.02, ..eq };
        let (mut last, mut first, mut n) = (0.02, None, 0usize);
        let mut t = 0.0;
        let mut t_last = 0.0;
        for _ in 0..(seconds / dt) as usize {
            p.step(&mut s, 0.0, dt);
            t += dt;
            let x = s.z_sprung_m - eq.z_sprung_m;
            if last < 0.0 && x >= 0.0 {
                // linear interpolation of the crossing instant
                let tc = t - dt * x / (x - last);
                if first.is_none() {
                    first = Some(tc);
                } else {
                    n += 1;
                    t_last = tc;
                }
            }
            last = x;
        }
        n as f64 / (t_last - first.unwrap())
    }

    #[test]
    fn quarter_car_natural_frequency_within_1_percent() {
        let mut p = truck_corner();
        p.damper_ns_m = 0.0;
        let (f_body, _) = p.undamped_frequencies_hz();
        for rate_hz in [60.0, 120.0, 300.0] {
            let f = measured_hz(&p, 1.0 / rate_hz, 30.0);
            assert!((f / f_body - 1.0).abs() < 0.01, "{rate_hz} Hz: measured {f} vs closed form {f_body}");
        }
    }

    #[test]
    fn quarter_car_damping_ratio_within_5_percent() {
        // rigid-wheel limit: a stiff tyre and a light wheel (so the wheel mode is far above the body mode) leave the single-mass answer zeta = c / (2 sqrt(k m))
        let p = QuarterCarParams { unsprung_mass_kg: 5.0, tyre_rate_n_m: 5e6, ..truck_corner() };
        let zeta = p.damper_ns_m / (2.0 * scalar::sqrt(p.spring_rate_n_m * p.sprung_mass_kg));
        let dt = 1.0 / 8000.0; // the stiff wheel mode rings at 160 Hz; keep it well resolved
        let eq = p.equilibrium();
        let mut s = QuarterCarState { z_sprung_m: eq.z_sprung_m + 0.02, ..eq };
        let mut peaks = Vec::new();
        let mut prev = (0.02, 0.02);
        for _ in 0..(3.0 / dt) as usize {
            p.step(&mut s, 0.0, dt);
            let x = s.z_sprung_m - eq.z_sprung_m;
            if prev.0 > prev.1 && prev.0 >= x && prev.0 > 0.0 {
                peaks.push(prev.0);
            }
            prev = (x, prev.0);
        }
        assert!(peaks.len() >= 3);
        let delta = scalar::ln(peaks[0] / peaks[2]) / 2.0; // logarithmic decrement per cycle
        let measured = delta / scalar::sqrt(4.0 * (core::f64::consts::PI * core::f64::consts::PI) + delta * delta);
        assert!((measured / zeta - 1.0).abs() < 0.05, "measured {measured} vs {zeta}");
    }

    #[test]
    fn energy_drift_below_0p1_percent_per_minute_frictionless() {
        let mut p = truck_corner();
        p.damper_ns_m = 0.0;
        let dt = 1.0 / 300.0;
        let eq = p.equilibrium();
        let mut s = QuarterCarState { z_sprung_m: eq.z_sprung_m + 0.02, ..eq };
        let n_window = (5.0 / dt) as usize;
        let total = (60.0 / dt) as usize;
        let (mut first, mut last) = (0.0, 0.0);
        for i in 0..total {
            p.step(&mut s, 0.0, dt);
            let e = p.energy_j(&s, 0.0);
            if i < n_window {
                first += e / n_window as f64;
            }
            if i >= total - n_window {
                last += e / n_window as f64;
            }
        }
        // the window means are 55 s apart; scale to one minute
        let e_ref = p.energy_j(&eq, 0.0); // energy is measured above the rest state
        let drift_per_min = ((last - first) / (first - e_ref)).abs() * 60.0 / 55.0;
        assert!(drift_per_min < 1e-3, "drift {drift_per_min} per minute");
    }

    #[test]
    fn static_deflection_equals_mg_over_k() {
        let p = truck_corner();
        let mut s = QuarterCarState::default(); // dropped from the design height, settles under the damper
        for _ in 0..(20.0 * 600.0) as usize {
            p.step(&mut s, 0.0, 1.0 / 600.0);
        }
        let strut = s.z_unsprung_m - s.z_sprung_m; // compression of the strut from its zero-force length
        let expect = p.sprung_mass_kg * p.gravity_m_s2 / p.spring_rate_n_m;
        assert!((strut / expect - 1.0).abs() < 1e-3, "{strut} vs {expect}");
        let tyre = -s.z_unsprung_m; // road is at height 0
        let expect_tyre = (p.sprung_mass_kg + p.unsprung_mass_kg) * p.gravity_m_s2 / p.tyre_rate_n_m;
        assert!((tyre / expect_tyre - 1.0).abs() < 1e-3, "{tyre} vs {expect_tyre}");
    }
}
