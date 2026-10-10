//! The hull: one 6-DoF rigid body. Forces and torques are accumulated over a substep (a *wrench*), then integrated with semi-implicit
//! Euler: momentum first, then pose with the new velocities.
//!
//! The rotational state is the world-frame **angular momentum** `L`, not the angular velocity. `dL/dt = torque` is exact in the world frame,
//! so with no torque `L` is conserved to rounding; the angular velocity is recovered as `omega = I_world^-1 L` with
//! `I_world = R I_body R^T`. This carries the gyroscopic term `omega x (I omega)` implicitly (it is what `d(I omega)/dt` becomes when
//! `I` rotates with the body), without integrating it explicitly.

use w5k_math::{Mat3, Quat, StateHasher, Vec3};

#[derive(Clone, Debug)]
pub struct Hull {
    mass_kg: f64,
    inertia_body_kg_m2: Mat3,
    inv_inertia_body: Mat3,
    /// Centre of mass, world frame, m.
    pub pos_m: Vec3,
    /// Body-to-world rotation.
    pub rot: Quat,
    pub vel_m_s: Vec3,
    /// World-frame angular momentum about the centre of mass, kg m^2/s.
    pub ang_mom: Vec3,
    force_n: Vec3,
    torque_nm: Vec3,
    /// Acceleration of the centre of mass over the last step, m/s^2 (for the articulation port and the ledger).
    pub acc_m_s2: Vec3,
    /// Torque about the centre of mass integrated in the last step, N m (for the ledger check).
    pub last_torque_nm: Vec3,
}

/// Why a body was refused.
#[derive(Clone, Debug, PartialEq)]
pub enum HullError {
    NonPositiveMass,
    SingularInertia,
}

impl Hull {
    pub fn new(mass_kg: f64, inertia_body_kg_m2: Mat3, pos_m: Vec3, rot: Quat) -> Result<Hull, HullError> {
        if !(mass_kg > 0.0 && mass_kg.is_finite()) {
            return Err(HullError::NonPositiveMass);
        }
        let inv = inertia_body_kg_m2.inverse().ok_or(HullError::SingularInertia)?;
        Ok(Hull {
            mass_kg,
            inertia_body_kg_m2,
            inv_inertia_body: inv,
            pos_m,
            rot,
            vel_m_s: Vec3::ZERO,
            ang_mom: Vec3::ZERO,
            force_n: Vec3::ZERO,
            torque_nm: Vec3::ZERO,
            acc_m_s2: Vec3::ZERO,
            last_torque_nm: Vec3::ZERO,
        })
    }

    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    /// Inertia tensor about the centre of mass in world axes: `R I_body R^T`.
    pub fn inertia_world(&self) -> Mat3 {
        self.inertia_body_kg_m2.rotate_inertia(&self.rot.to_mat3())
    }

    /// World-frame angular velocity, rad/s.
    pub fn omega_rad_s(&self) -> Vec3 {
        let r = self.rot.to_mat3();
        self.inv_inertia_body.rotate_inertia(&r).mul_vec(self.ang_mom)
    }

    pub fn set_omega(&mut self, omega_rad_s: Vec3) {
        self.ang_mom = self.inertia_world().mul_vec(omega_rad_s);
    }

    /// A body-frame point (relative to the centre of mass) in the world frame.
    pub fn point_world(&self, body_point_m: Vec3) -> Vec3 {
        self.pos_m + self.rot.rotate(body_point_m)
    }

    /// World velocity of a world-frame point rigidly attached to the hull.
    pub fn point_velocity(&self, world_point_m: Vec3) -> Vec3 {
        self.vel_m_s + self.omega_rad_s().cross(world_point_m - self.pos_m)
    }

    /// Add a world-frame force acting at a world-frame point: it also contributes `r x F` about the centre of mass.
    pub fn add_force_at(&mut self, force_n: Vec3, world_point_m: Vec3) {
        self.force_n += force_n;
        self.torque_nm += (world_point_m - self.pos_m).cross(force_n);
    }

    /// A force through the centre of mass (gravity, a lumped drag).
    pub fn add_force(&mut self, force_n: Vec3) {
        self.force_n += force_n;
    }

    pub fn add_torque(&mut self, torque_nm: Vec3) {
        self.torque_nm += torque_nm;
    }

    /// The wrench accumulated so far this substep (force, torque about the centre of mass), world frame.
    pub fn wrench(&self) -> (Vec3, Vec3) {
        (self.force_n, self.torque_nm)
    }

    /// Integrate one substep and clear the accumulated wrench.
    pub fn integrate(&mut self, dt_s: f64) {
        self.acc_m_s2 = self.force_n / self.mass_kg;
        self.vel_m_s += self.acc_m_s2 * dt_s;
        self.pos_m += self.vel_m_s * dt_s;
        self.ang_mom += self.torque_nm * dt_s;
        self.last_torque_nm = self.torque_nm;
        // the orientation advances with the angular velocity the new momentum has in the current pose
        self.rot = self.rot.integrate_world(self.omega_rad_s(), dt_s);
        self.force_n = Vec3::ZERO;
        self.torque_nm = Vec3::ZERO;
    }

    /// Translational plus rotational kinetic energy, J.
    pub fn kinetic_energy_j(&self) -> f64 {
        0.5 * self.mass_kg * self.vel_m_s.length_sq() + 0.5 * self.omega_rad_s().dot(self.ang_mom)
    }

    pub fn is_finite(&self) -> bool {
        self.pos_m.is_finite() && self.rot.is_finite() && self.vel_m_s.is_finite() && self.ang_mom.is_finite()
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        for v in [self.pos_m, self.vel_m_s, self.ang_mom] {
            h.write_f64(v.x);
            h.write_f64(v.y);
            h.write_f64(v.z);
        }
        for q in [self.rot.w, self.rot.x, self.rot.y, self.rot.z] {
            h.write_f64(q);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_math::scalar;

    const DT: f64 = 1.0 / 240.0;

    fn truck_hull() -> Hull {
        // a 2.5 t box 4.5 x 1.5 x 2 m (length along Z): I = m (a^2 + b^2) / 12
        let (m, w, h, l) = (2500.0, 2.0, 1.5, 4.5);
        let i = Mat3::diagonal(m * (h * h + l * l) / 12.0, m * (w * w + l * l) / 12.0, m * (w * w + h * h) / 12.0);
        Hull::new(m, i, Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY).unwrap()
    }

    #[test]
    fn free_fall_matches_g_t_squared_over_2() {
        let mut b = truck_hull();
        let y0 = b.pos_m.y;
        let n = 240;
        for _ in 0..n {
            b.add_force(Vec3::new(0.0, -b.mass_kg() * scalar::G, 0.0));
            b.integrate(DT);
        }
        let t = f64::from(n) * DT;
        let drop = y0 - b.pos_m.y;
        // semi-implicit Euler drops g dt^2 n(n+1)/2: the continuous g t^2/2 plus g t dt / 2
        assert!((drop - scalar::G * DT * DT * f64::from(n * (n + 1)) / 2.0).abs() < 1e-9);
        assert!((drop / (0.5 * scalar::G * t * t) - 1.0).abs() < 0.005, "{drop}");
        assert!((b.vel_m_s.y + scalar::G * t).abs() < 1e-9);
    }

    #[test]
    fn torque_free_spin_conserves_angular_momentum() {
        // spin about the intermediate axis with a small wobble: the tumbling "tennis racket" case, the hardest for an integrator
        let mut b = truck_hull();
        b.set_omega(Vec3::new(0.01, 3.0, 0.02));
        let l0 = b.ang_mom;
        let e0 = b.kinetic_energy_j();
        let mut l_err: f64 = 0.0;
        for _ in 0..(20.0 / DT) as usize {
            b.integrate(DT);
            l_err = l_err.max((b.ang_mom - l0).length() / l0.length());
        }
        assert!(l_err < 1e-12, "angular momentum drift {l_err}");
        assert!(b.rot.angle_to(Quat::IDENTITY) > 0.1); // it really did tumble
        let e = b.kinetic_energy_j();
        assert!(((e - e0) / e0).abs() < 0.02, "rotational energy changed by {}", (e - e0) / e0);
    }

    #[test]
    fn offset_force_produces_r_cross_f_torque() {
        let mut b = truck_hull();
        let f = Vec3::new(0.0, 1000.0, 0.0);
        b.add_force_at(f, b.pos_m + Vec3::new(0.0, 0.0, -2.0)); // a push up at the nose (-Z is forward)
        let (force, torque) = b.wrench();
        assert!((force - f).length() < 1e-12);
        // r x F = (0,0,-2) x (0,1000,0) = (2000, 0, 0): positive pitch about +X raises the nose
        assert!((torque - Vec3::new(2000.0, 0.0, 0.0)).length() < 1e-9);
    }

    #[test]
    fn spring_supported_hull_conserves_energy_to_0p1_percent() {
        // four vertical corner springs (undamped) under gravity, started heaved, pitched and rolled: heave, pitch and roll all ring
        let mut b = truck_hull();
        let (m, k) = (b.mass_kg(), 60e3);
        let corners = [
            Vec3::new(-0.8, -0.75, -1.8),
            Vec3::new(0.8, -0.75, -1.8),
            Vec3::new(-0.8, -0.75, 1.6),
            Vec3::new(0.8, -0.75, 1.6),
        ];
        let spring_top = 0.25; // a corner at height y is pushed up by k (spring_top - y); vertical springs are conservative
        let energy = |b: &Hull| {
            let springs: f64 = corners
                .iter()
                .map(|c| {
                    0.5 * k * {
                        let d = spring_top - b.point_world(*c).y;
                        d * d
                    }
                })
                .sum();
            b.kinetic_energy_j() + m * scalar::G * b.pos_m.y + springs
        };
        // the static rest pose (level, mean corner sag m g / 4k) is the energy floor: drift is measured against the oscillation energy
        let mut rest = truck_hull();
        rest.pos_m.y = spring_top - m * scalar::G / (4.0 * k) + 0.75;
        let e_floor = energy(&rest);
        b.pos_m = rest.pos_m + Vec3::new(0.0, 0.03, 0.0);
        b.rot = Quat::from_pitch(0.02) * Quat::from_roll(0.015);
        let (n, w) = ((60.0 / DT) as usize, (5.0 / DT) as usize);
        let (mut first, mut last) = (0.0, 0.0);
        for i in 0..n {
            b.add_force(Vec3::new(0.0, -m * scalar::G, 0.0));
            for c in corners {
                let p = b.point_world(c);
                b.add_force_at(Vec3::new(0.0, k * (spring_top - p.y), 0.0), p);
            }
            b.integrate(DT);
            let e = energy(&b) - e_floor;
            if i < w {
                first += e / w as f64;
            }
            if i >= n - w {
                last += e / w as f64;
            }
        }
        assert!(first > 100.0, "the test must start with real oscillation energy, got {first} J");
        let drift = ((last - first) / first).abs();
        assert!(drift < 1e-3, "energy drift over a minute {drift}");
    }
}
