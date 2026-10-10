//! The tyre as a [`ContactElement`]: vertical spring-damper, longitudinal and lateral force from a relaxing *patch stretch*, a friction
//! circle, rolling resistance and a self-aligning moment.
//!
//! The patch stretch `u` is a position state, not a slip: `du/dt = slip_velocity - (|v|/sigma) u`. While rolling it settles to
//! `sigma * slip` (the textbook slip-force curve with a relaxation length `sigma`); at standstill nothing decays, so a stopped tyre holds
//! its stretch like a spring and cannot creep (spike S1). The force is `(C Fz / sigma) u`, capped on the friction circle `mu Fz`; when
//! the cap bites the stretch is scaled back, which *is* sliding.

use w5k_contract::rig::TyreDef;
use w5k_contract::{ContactElement, ContactInput, ContactOutput};
use w5k_math::scalar;

/// Numbers that belong to the model, not to one tyre (loaded from `content/physics/chassis/tuning.ron` as `Param`s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TyreTuning {
    /// Speed over which rolling resistance fades to zero at standstill (tanh scale), m/s.
    pub rolling_fade_speed_m_s: f64,
    /// Time constant of the damping on the patch-stretch rate, s: takes the energy out of a stopped vehicle's rock on its tyres.
    pub slip_damping_time_s: f64,
    /// Load sensitivity of the peak friction and of the slip stiffnesses (`k` in `s = 1 / (1 + k (Fz / Fz0 - 1))`), dimensionless, 0..1.
    /// PROVISIONAL(CCR-chassis-4): a per-tyre `TyreDef` field once the CCR lands; one shared value until then.
    pub mu_load_sensitivity: f64,
    pub stiffness_load_sensitivity: f64,
}

#[derive(Clone, Debug)]
pub struct Tyre {
    def: TyreDef,
    free_radius_m: f64,
    tuning: TyreTuning,
    /// The tyre's static load, N: the reference of the load sensitivity (0 = no load sensitivity).
    nominal_load_n: f64,
    /// Patch stretch along the rolling direction and sideways, m.
    stretch_x_m: f64,
    stretch_y_m: f64,
    /// Rolling resistance included in the last `fx_n` (N, positive = resisting forward motion), kept for the ledger.
    last_rolling_n: f64,
}

impl Tyre {
    pub fn new(def: &TyreDef, free_radius_m: f64, tuning: TyreTuning) -> Tyre {
        Tyre {
            def: def.clone(),
            free_radius_m,
            tuning,
            nominal_load_n: 0.0,
            stretch_x_m: 0.0,
            stretch_y_m: 0.0,
            last_rolling_n: 0.0,
        }
    }

    /// Switch on load sensitivity about this static load (N).
    pub fn with_nominal_load(mut self, nominal_load_n: f64) -> Tyre {
        self.nominal_load_n = nominal_load_n.max(0.0);
        self
    }

    /// Load-sensitivity factor `1 / (1 + k (Fz / Fz0 - 1))`: 1 at the static load, below 1 above it, so the force `s * Fz` rises
    /// ever more slowly with load (toward `Fz0 / k`): a tyre carrying twice its load does not grip twice as hard.
    fn load_factor(&self, k: f64, fz: f64) -> f64 {
        if self.nominal_load_n <= 0.0 || k <= 0.0 {
            return 1.0;
        }
        1.0 / (1.0 + k * (fz / self.nominal_load_n - 1.0))
    }

    pub fn stretch_m(&self) -> (f64, f64) {
        (self.stretch_x_m, self.stretch_y_m)
    }

    /// The rolling-resistance part of the last step's `fx_n` (subtracted from it), N.
    pub fn last_rolling_n(&self) -> f64 {
        self.last_rolling_n
    }

    pub fn hash_state(&self, h: &mut w5k_math::StateHasher) {
        h.write_f64(self.stretch_x_m);
        h.write_f64(self.stretch_y_m);
    }
}

/// `(1 - e^-x) / x`, the exact-integration weight of a constant source over a decaying state; 1 as x -> 0.
fn decay_weight(x: f64) -> f64 {
    if x < 1e-8 {
        1.0 - 0.5 * x
    } else {
        -scalar::exp_m1(-x) / x
    } // const-ok: series switch point for 1 - e^-x over x
}

impl ContactElement for Tyre {
    fn step(&mut self, i: &ContactInput) -> ContactOutput {
        let d = &self.def;
        self.last_rolling_n = 0.0;
        if i.penetration_m <= 0.0 {
            self.reset();
            return ContactOutput::default();
        }
        let fz =
            (d.vertical_stiffness_n_m * i.penetration_m + d.vertical_damping_ns_m * i.penetration_rate_m_s).max(0.0);
        let mu = i.ground.mu_peak * d.mu_scale * self.load_factor(self.tuning.mu_load_sensitivity, fz);
        let fmax = mu * fz;

        // Exact update of the stretch: u' = s - a u over dt.
        let a = i.vel_long_m_s.abs() / d.relaxation_length_m;
        let x = a * i.dt_s;
        let decay = scalar::exp(-x);
        let w = decay_weight(x) * i.dt_s;
        let slip_vel_x = i.surface_speed_m_s - i.vel_long_m_s;
        let slip_vel_y = i.vel_lat_m_s;
        self.stretch_x_m = scalar::flush_tiny(self.stretch_x_m * decay + slip_vel_x * w);
        self.stretch_y_m = scalar::flush_tiny(self.stretch_y_m * decay + slip_vel_y * w);

        // Force from the stretch plus a little damping on its rate (the rate of a settled stretch is zero).
        let rate_x = slip_vel_x - a * self.stretch_x_m;
        let rate_y = slip_vel_y - a * self.stretch_y_m;
        let stiff = self.load_factor(self.tuning.stiffness_load_sensitivity, fz);
        let kx = d.slip_stiffness * stiff * fz / d.relaxation_length_m;
        let ky = d.cornering_stiffness_per_rad * stiff * fz / d.relaxation_length_m;
        let mut fx = kx * (self.stretch_x_m + self.tuning.slip_damping_time_s * rate_x);
        let mut fy = -ky * (self.stretch_y_m + self.tuning.slip_damping_time_s * rate_y);

        // Friction circle: scale the force vector (and the stretch) back onto it.
        let mag = scalar::hypot(fx, fy);
        let saturated = mag > fmax;
        if saturated {
            let s = if mag > 0.0 { fmax / mag } else { 0.0 };
            fx *= s;
            fy *= s;
            self.stretch_x_m *= s;
            self.stretch_y_m *= s;
        }

        let radius_m = self.free_radius_m - i.penetration_m;
        let rolling_n = (d.rolling_coeff + i.ground.rolling_coeff)
            * fz
            * scalar::tanh(i.vel_long_m_s / self.tuning.rolling_fade_speed_m_s);

        self.last_rolling_n = rolling_n;

        // Pneumatic trail shrinks to zero as the patch saturates.
        let trail_m = if fmax > 0.0 {
            d.aligning_trail_frac * d.patch_length_m * (1.0 - (fy.abs() / fmax).min(1.0))
        } else {
            0.0
        };

        let v_ground = i.vel_long_m_s;
        let denom = i.surface_speed_m_s.abs().max(v_ground.abs()).max(d.speed_floor_m_s).max(1e-9); // const-ok: division guard
        ContactOutput {
            fx_n: fx - rolling_n,
            fy_n: fy,
            fz_n: fz,
            mz_nm: -fy * trail_m,
            shaft_reaction_nm: fx * radius_m,
            sinkage_m: 0.0,
            slip_ratio: (i.surface_speed_m_s - v_ground) / denom,
            slip_angle_rad: scalar::atan2(i.vel_lat_m_s, v_ground.abs().max(d.speed_floor_m_s).max(1e-9)), // const-ok: division guard
            saturated,
        }
    }

    fn reset(&mut self) {
        self.stretch_x_m = 0.0;
        self.stretch_y_m = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::standard_materials;
    use w5k_contract::Material;

    const FADE: TyreTuning = TyreTuning {
        rolling_fade_speed_m_s: 0.2,
        slip_damping_time_s: 0.005,
        mu_load_sensitivity: 0.0,
        stiffness_load_sensitivity: 0.0,
    };
    const DT: f64 = 1.0 / 240.0;

    fn tyre_def() -> TyreDef {
        TyreDef {
            vertical_stiffness_n_m: 250e3,
            vertical_damping_ns_m: 2e3,
            mu_scale: 1.0,
            slip_stiffness: 10.0,
            cornering_stiffness_per_rad: 8.0,
            relaxation_length_m: 0.2,
            rolling_coeff: 0.0,
            inflation_pa: 300e3,
            patch_length_m: 0.2,
            speed_floor_m_s: 0.5,
            aligning_trail_frac: 0.3,
            kappa_peak: 0.0,
            alpha_peak_rad: 0.0,
            mu_load_sensitivity: 0.0,
            stiffness_load_sensitivity: 0.0,
            nominal_load_n: 0.0,
        }
    }

    fn ground(mu: f64, rolling: f64) -> Material {
        Material {
            mu_peak: mu,
            mu_slide: mu,
            rolling_coeff: rolling,
            ..standard_materials().get(w5k_contract::MaterialId(0)).clone()
        }
    }

    fn input(g: &Material, pen: f64, vlong: f64, vlat: f64, surface: f64) -> ContactInput<'_> {
        ContactInput {
            penetration_m: pen,
            penetration_rate_m_s: 0.0,
            vel_long_m_s: vlong,
            vel_lat_m_s: vlat,
            surface_speed_m_s: surface,
            ground: g,
            dt_s: DT,
        }
    }

    const PEN: f64 = 0.02; // 5 kN at 250 kN/m

    #[test]
    fn tyre_longitudinal_slope_at_zero_slip_equals_slip_stiffness_times_load() {
        let g = ground(1.0, 0.0);
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        let kappa = 0.002;
        let mut o = ContactOutput::default();
        for _ in 0..600 {
            o = t.step(&input(&g, PEN, 20.0, 0.0, 20.0 * (1.0 + kappa)));
        }
        let expect = 10.0 * o.fz_n * kappa;
        assert!((o.fx_n / expect - 1.0).abs() < 0.01, "{} vs {expect}", o.fx_n);
        // same for the cornering stiffness, per radian (small angle)
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        let alpha = 0.002;
        for _ in 0..600 {
            o = t.step(&input(&g, PEN, 20.0, 20.0 * alpha, 20.0));
        }
        assert!((o.fy_n / (-8.0 * o.fz_n * alpha) - 1.0).abs() < 0.01, "{}", o.fy_n);
    }

    #[test]
    fn tyre_force_peaks_at_mu_times_load() {
        let g = ground(0.8, 0.0);
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        let mut o = ContactOutput::default();
        for _ in 0..600 {
            o = t.step(&input(&g, PEN, 20.0, 0.0, 0.0)); // locked wheel
        }
        assert!(o.saturated);
        assert!((o.fx_n / (-0.8 * o.fz_n) - 1.0).abs() < 1e-9, "{}", o.fx_n);
    }

    #[test]
    fn combined_slip_never_exceeds_the_friction_circle() {
        let g = ground(0.9, 0.0);
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        for n in 0..4000 {
            let ph = f64::from(n) * 0.013;
            let (vlat, surf) = (6.0 * scalar::sin(ph), 12.0 + 12.0 * scalar::cos(1.7 * ph));
            let o = t.step(&input(&g, PEN, 12.0, vlat, surf));
            assert!(scalar::hypot(o.fx_n, o.fy_n) <= 0.9 * o.fz_n * (1.0 + 1e-9), "step {n}");
        }
    }

    #[test]
    fn stopped_tyre_on_a_grade_does_not_creep() {
        // a quarter of a 2.5 t truck on a 10% grade, wheel held by the parking brake (surface speed 0)
        let g = ground(0.8, 0.0);
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        let (m, grav) = (625.0, 9.81);
        let slope = scalar::atan(0.10);
        let pen = m * grav * scalar::cos(slope) / 250e3;
        let (mut v, mut x, mut x_at_9s) = (0.0, 0.0, 0.0);
        for n in 0..(12.0 / DT) as usize {
            let o = t.step(&input(&g, pen, v, 0.0, 0.0));
            v += (m * grav * scalar::sin(slope) + o.fx_n) / m * DT; // down-slope is +x
            x += v * DT;
            if n == (9.0 / DT) as usize {
                x_at_9s = x;
            }
        }
        assert!((x - x_at_9s).abs() < 1e-6 && v.abs() < 1e-6, "moved {} m in the last 3 s, v {v}", x - x_at_9s);
        // and it holds with the stretch that balances the pull: kx * u = m g sin(slope) (negative: it resists)
        let k_stretch = 10.0 * m * grav * scalar::cos(slope) / 0.2; // slip stiffness x load / relaxation length
        assert!((-t.stretch_m().0 * k_stretch / (m * grav * scalar::sin(slope)) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn rolling_resistance_equals_crr_times_load() {
        let g = ground(1.0, 0.004);
        let mut d = tyre_def();
        d.rolling_coeff = 0.011;
        let mut t = Tyre::new(&d, 0.4, FADE);
        let mut o = ContactOutput::default();
        for _ in 0..600 {
            o = t.step(&input(&g, PEN, 15.0, 0.0, 15.0));
        }
        assert!((o.fx_n / (-(0.011 + 0.004) * o.fz_n) - 1.0).abs() < 1e-6, "{}", o.fx_n);
        // a parked tyre feels none
        let o = Tyre::new(&d, 0.4, FADE).step(&input(&g, PEN, 0.0, 0.0, 0.0));
        assert!(o.fx_n.abs() < 1e-9);
    }

    #[test]
    fn positive_slip_angle_pushes_right_and_aligns_the_nose_left() {
        let g = ground(1.0, 0.0);
        let mut t = Tyre::new(&tyre_def(), 0.4, FADE);
        let mut o = ContactOutput::default();
        for _ in 0..300 {
            o = t.step(&input(&g, PEN, 20.0, 1.0, 20.0)); // velocity points left of the wheel
        }
        assert!(o.fy_n < 0.0 && o.mz_nm > 0.0 && o.slip_angle_rad > 0.0);
        // lifted off the ground: nothing
        assert!(t.step(&input(&g, -0.01, 20.0, 1.0, 20.0)).fz_n.abs() < 1e-12);
    }

    #[test]
    fn a_tyre_at_twice_its_static_load_grips_mu_2fz0_over_1_plus_k() {
        let g = ground(0.8, 0.0);
        let k = 0.15;
        let tuning = TyreTuning { mu_load_sensitivity: k, stiffness_load_sensitivity: 0.3, ..FADE };
        let fz0 = 250e3 * PEN; // the static load is the 5 kN of PEN
        let peak = |pen: f64| {
            let mut t = Tyre::new(&tyre_def(), 0.4, tuning).with_nominal_load(fz0);
            let mut o = ContactOutput::default();
            for _ in 0..600 {
                o = t.step(&input(&g, pen, 20.0, 0.0, 0.0)); // locked: at the friction limit
            }
            -o.fx_n
        };
        // at the static load nothing changes; at twice the load the grip is mu * 2 Fz0 / (1 + k), not mu * 2 Fz0
        assert!((peak(PEN) / (0.8 * fz0) - 1.0).abs() < 1e-9);
        assert!((peak(2.0 * PEN) / (0.8 * 2.0 * fz0 / (1.0 + k)) - 1.0).abs() < 1e-9);
    }
}
