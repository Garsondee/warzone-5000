//! Ballistics: a point mass under gravity and quadratic drag with `Cd(Mach)` from RON, advanced by a fixed-step RK4.
//!
//! `a = g - (rho Cd(M) A / 2m) |v_air| v_air`, `v_air = v - wind` (wind is a non-goal: zero), `M = |v_air| / c`. Pure functions of their arguments; no
//! allocation in the step; the same inputs give the same bits. Frame: +Y up, gravity along -Y (`w5k_math::scalar::G`).

use serde::{Deserialize, Serialize};
use w5k_contract::Param;
use w5k_math::{scalar, Vec3};

/// Still air at one density and one speed of sound (sea level; altitude and wind are out of scope).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirDef {
    pub density_kg_m3: Param,
    pub sound_speed_m_s: Param,
}

/// One projectile: mass, reference diameter and a drag curve `(Mach, Cd)`, ascending in Mach.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectileDef {
    pub id: String,
    pub mass_kg: Param,
    pub drag_diameter_m: Param,
    pub cd_table: Vec<(f64, Param)>,
}

/// The file `content/combat/ballistics.ron`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ballistics {
    pub air: AirDef,
    pub projectiles: Vec<ProjectileDef>,
}

impl Ballistics {
    pub fn from_ron(text: &str) -> Result<Ballistics, String> {
        let b: Ballistics = ron::from_str(text).map_err(|e| e.to_string())?;
        for p in &b.projectiles {
            if p.cd_table.is_empty() || p.cd_table.windows(2).any(|w| w[1].0 <= w[0].0) {
                return Err(format!("{}: cd_table must be non-empty and strictly ascending in Mach", p.id));
            }
        }
        Ok(b)
    }
}

/// The numbers the integrator needs, all SI.
#[derive(Clone, Debug, PartialEq)]
pub struct Flyer {
    pub mass_kg: f64,
    pub area_m2: f64,
    pub density_kg_m3: f64,
    pub sound_speed_m_s: f64,
    pub cd_table: Vec<(f64, f64)>,
}

impl Flyer {
    pub fn new(air: &AirDef, p: &ProjectileDef) -> Flyer {
        let d = p.drag_diameter_m.v;
        Flyer {
            mass_kg: p.mass_kg.v,
            area_m2: 0.25 * scalar::PI * d * d,
            density_kg_m3: air.density_kg_m3.v,
            sound_speed_m_s: air.sound_speed_m_s.v,
            cd_table: p.cd_table.iter().map(|(m, c)| (*m, c.v)).collect(),
        }
    }

    /// `Cd` at a Mach number: linear between table points, flat beyond the ends.
    pub fn cd(&self, mach: f64) -> f64 {
        let t = &self.cd_table;
        if mach <= t[0].0 {
            return t[0].1;
        }
        for w in t.windows(2) {
            if mach <= w[1].0 {
                return scalar::lerp(w[0].1, w[1].1, scalar::inv_lerp(w[0].0, w[1].0, mach));
            }
        }
        t[t.len() - 1].1
    }

    /// Acceleration (m/s^2) at velocity `v`: gravity plus drag against the air (at rest).
    pub fn accel(&self, v: Vec3) -> Vec3 {
        let speed = v.length();
        let k = 0.5 * self.density_kg_m3 * self.cd(speed / self.sound_speed_m_s) * self.area_m2 / self.mass_kg;
        Vec3::new(0.0, -scalar::G, 0.0) - v * (k * speed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub pos_m: Vec3,
    pub vel_m_s: Vec3,
}

/// One RK4 step of `dt_s`: fourth order in the step.
pub fn step(f: &Flyer, s: Particle, dt_s: f64) -> Particle {
    let (p0, v0) = (s.pos_m, s.vel_m_s);
    let (k1p, k1v) = (v0, f.accel(v0));
    let (k2p, k2v) = (v0 + k1v * (0.5 * dt_s), f.accel(v0 + k1v * (0.5 * dt_s)));
    let (k3p, k3v) = (v0 + k2v * (0.5 * dt_s), f.accel(v0 + k2v * (0.5 * dt_s)));
    let (k4p, k4v) = (v0 + k3v * dt_s, f.accel(v0 + k3v * dt_s));
    let sixth = dt_s / 6.0; // const-ok: RK4 weight
    Particle {
        pos_m: p0 + (k1p + (k2p + k3p) * 2.0 + k4p) * sixth,
        vel_m_s: v0 + (k1v + (k2v + k3v) * 2.0 + k4v) * sixth,
    }
}

/// Fly from `s` with a fixed step until the height returns to `plane_y_m` (descending); the last step is shortened by bisection so the
/// crossing is hit exactly. Returns the state at the plane and the flight time, or `None` if it has not landed within `max_steps`.
pub fn fly_to_plane(
    f: &Flyer,
    mut s: Particle,
    plane_y_m: f64,
    dt_s: f64,
    max_steps: usize,
) -> Option<(Particle, f64)> {
    let mut t = 0.0;
    for _ in 0..max_steps {
        let n = step(f, s, dt_s);
        if n.pos_m.y <= plane_y_m && s.vel_m_s.y < 0.0 {
            let (mut lo, mut hi) = (0.0, dt_s);
            for _ in 0..64 {
                // const-ok: bisection depth, 2^-64 of a step
                let mid = 0.5 * (lo + hi);
                if step(f, s, mid).pos_m.y > plane_y_m {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return Some((step(f, s, hi), t + hi));
        }
        s = n;
        t += dt_s;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0e-3;

    fn drag_free() -> Flyer {
        Flyer {
            mass_kg: 8.0,
            area_m2: 1.0e-3,
            density_kg_m3: 1.225,
            sound_speed_m_s: 340.29,
            cd_table: vec![(0.0, 0.0)],
        }
    }

    fn launch(v: f64, theta: f64) -> Particle {
        Particle { pos_m: Vec3::ZERO, vel_m_s: Vec3::new(v * scalar::cos(theta), v * scalar::sin(theta), 0.0) }
    }

    #[test]
    fn drag_free_range_equals_v2_sin_2theta_over_g() {
        for (v, deg) in [(100.0, 45.0), (1650.0, 3.0), (300.0, 30.0), (50.0, 75.0)] {
            let th = scalar::deg_to_rad(deg);
            let (end, t) = fly_to_plane(&drag_free(), launch(v, th), 0.0, DT, 1_000_000).expect("lands");
            let range = v * v * scalar::sin(2.0 * th) / scalar::G;
            let time = 2.0 * v * scalar::sin(th) / scalar::G;
            assert!((end.pos_m.x - range).abs() / range < 1.0e-6, "range {} vs {range}", end.pos_m.x);
            assert!((t - time).abs() / time < 1.0e-6, "time {t} vs {time}");
        }
    }

    #[test]
    fn drag_free_apex_equals_v2_sin2_over_2g() {
        let (v, th) = (200.0, scalar::deg_to_rad(40.0));
        let f = drag_free();
        let (mut s, mut apex) = (launch(v, th), 0.0f64);
        while s.vel_m_s.y > 0.0 {
            s = step(&f, s, DT);
            apex = apex.max(s.pos_m.y);
        }
        let expect = v * v * scalar::sin(th) * scalar::sin(th) / (2.0 * scalar::G);
        assert!((apex - expect).abs() / expect < 1.0e-6);
    }

    #[test]
    fn vertical_fall_under_drag_follows_vt_tanh_gt_over_vt() {
        let f = Flyer {
            mass_kg: 8.0,
            area_m2: 0.01,
            density_kg_m3: 1.225,
            sound_speed_m_s: 340.29,
            cd_table: vec![(0.0, 0.5)],
        };
        let vt = scalar::sqrt(2.0 * f.mass_kg * scalar::G / (f.density_kg_m3 * 0.5 * f.area_m2));
        let mut s = Particle { pos_m: Vec3::ZERO, vel_m_s: Vec3::ZERO };
        for i in 1..=5000 {
            s = step(&f, s, DT);
            if i % 500 == 0 {
                let t = f64::from(i) * DT;
                let expect = vt * scalar::tanh(scalar::G * t / vt);
                assert!((-s.vel_m_s.y - expect).abs() / expect < 1.0e-4, "t {t}: {} vs {expect}", -s.vel_m_s.y);
            }
        }
    }

    #[test]
    fn drag_error_falls_as_the_fourth_power_of_the_step() {
        // A smooth drag law (constant Cd): a piecewise-linear Cd(M) has kinks at its table points that cost RK4 some order, which is honest but not what this test measures.
        let f = Flyer {
            mass_kg: 8.0,
            area_m2: 1.0e-3,
            density_kg_m3: 1.225,
            sound_speed_m_s: 340.29,
            cd_table: vec![(0.0, 0.3)],
        };
        let run = |dt: f64| {
            let mut s = launch(1650.0, scalar::deg_to_rad(5.0));
            for _ in 0..(4.0 / dt).round() as usize {
                s = step(&f, s, dt);
            }
            s.pos_m
        };
        let reference = run(1.0 / 1024.0); // const-ok: reference step
        let (e1, e2) = ((run(1.0 / 8.0) - reference).length(), (run(1.0 / 16.0) - reference).length()); // const-ok: test steps
        let order = scalar::log2(e1 / e2);
        assert!((3.7..4.3).contains(&order), "observed order {order} (errors {e1} {e2})");
    }

    #[test]
    fn drag_curve_interpolates_and_is_flat_beyond_its_ends() {
        let f = Flyer { cd_table: vec![(0.5, 0.1), (1.5, 0.3)], ..drag_free() };
        assert!((f.cd(0.0) - 0.1).abs() < 1.0e-12 && (f.cd(9.0) - 0.3).abs() < 1.0e-12);
        assert!((f.cd(1.0) - 0.2).abs() < 1.0e-12);
    }

    #[test]
    fn supersonic_drag_shortens_the_range_more_than_a_constant_subsonic_cd() {
        let b = Ballistics::from_ron(include_str!("../../../content/combat/ballistics.ron")).expect("parses");
        let mach = Flyer::new(&b.air, &b.projectiles[0]);
        let flat = Flyer { cd_table: vec![(0.0, 0.12)], ..mach.clone() };
        let th = scalar::deg_to_rad(2.0);
        let r = |f: &Flyer| fly_to_plane(f, launch(1650.0, th), 0.0, 1.0e-2, 100_000).expect("lands").0.pos_m.x;
        assert!(r(&mach) < r(&flat));
    }

    #[test]
    fn the_shipped_ballistics_file_parses_and_every_number_has_provenance() {
        let b = Ballistics::from_ron(include_str!("../../../content/combat/ballistics.ron")).expect("parses");
        for p in &b.projectiles {
            assert!(p.mass_kg.v > 0.0 && p.drag_diameter_m.v > 0.0);
            assert!(
                !p.mass_kg.src.is_empty()
                    && p.cd_table.iter().all(|(_, c)| !c.src.is_empty() && c.lo.is_some() && c.hi.is_some())
            );
        }
    }

    #[test]
    fn a_cd_table_out_of_order_is_refused() {
        let bad = "Ballistics(air: AirDef(density_kg_m3: Param(v: 1.2, prov: Spec, src: \"x\"), sound_speed_m_s: Param(v: 340.0, prov: Spec, src: \"x\")), projectiles: [ProjectileDef(id: \"a\", mass_kg: Param(v: 1.0, prov: Spec, src: \"x\"), drag_diameter_m: Param(v: 0.1, prov: Spec, src: \"x\"), cd_table: [(1.0, Param(v: 0.1, prov: Spec, src: \"x\")), (0.5, Param(v: 0.1, prov: Spec, src: \"x\"))])])";
        assert!(Ballistics::from_ron(bad).is_err());
    }
}
