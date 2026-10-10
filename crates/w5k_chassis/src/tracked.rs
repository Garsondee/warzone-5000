//! The tracked hull on the chassis integrator (design: `docs/lanes/chassis/tracked-design-delta.md`). The same 6-DoF `Hull` and ledger as the
//! wheeled chassis; road wheels on torsion arms, each with its own travel coordinate; one spinning sprocket per track carrying the inertia of everything
//! the belt links to it; the belt and the soil are TRACKS' `TrackedVehicleGear`.
//!
//! This first part holds the geometry and the inertia, with their tests; the stepping assembly follows in the next PR.

use w5k_contract::rig::{PhysRig, TrackDef};
use w5k_math::{scalar, Vec3};

/// A road wheel's swinging arm, in the hull frame: the wheel centre moves on a circle about `pivot_m` in the hull's y-z plane.
/// The travel coordinate `c` is the centre's vertical rise from the rest position (the contract's convention for every station).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TorsionArm {
    pub pivot_m: Vec3,
    /// Arm length, m.
    pub length_m: f64,
    /// Arm angle below the horizontal at rest, rad.
    pub rest_angle_rad: f64,
    /// +1 when the wheel trails the pivot toward +Z (rearward), -1 when it leads.
    pub fore_aft_sign: f64,
}

impl TorsionArm {
    /// The arm from a station's rest position and its pivot.
    pub fn new(rest_pos_m: Vec3, pivot_m: Vec3) -> TorsionArm {
        let (dy, dz) = (rest_pos_m.y - pivot_m.y, rest_pos_m.z - pivot_m.z);
        let length_m = scalar::hypot(dy, dz);
        TorsionArm { pivot_m, length_m, rest_angle_rad: scalar::atan2(-dy, dz.abs()), fore_aft_sign: scalar::sign(dz) }
    }

    /// Arm angle below the horizontal for travel `c`: `sin(phi) = sin(phi0) - c / L` (clamped short of vertical).
    pub fn angle_rad(&self, c_m: f64) -> f64 {
        let s = scalar::clamp(scalar::sin(self.rest_angle_rad) - c_m / self.length_m, -0.999, 0.999); // const-ok: keeps cos(phi) off zero
        scalar::asin(s)
    }

    /// Wheel centre in the hull frame for travel `c`.
    pub fn centre_m(&self, c_m: f64) -> Vec3 {
        let phi = self.angle_rad(c_m);
        self.pivot_m
            + Vec3::new(0.0, -self.length_m * scalar::sin(phi), self.fore_aft_sign * self.length_m * scalar::cos(phi))
    }

    /// `d centre / dc` in the hull frame: one up, and `tan(phi)` fore-aft (the wheel swings as it rises). A force `F` on the wheel does the
    /// work `F . d` per unit of travel (the generalized force of `c`).
    pub fn centre_rate(&self, c_m: f64) -> Vec3 {
        let phi = self.angle_rad(c_m);
        Vec3::new(0.0, 1.0, self.fore_aft_sign * scalar::tan(phi))
    }

    /// The generalized mass of `c` for an unsprung mass `m` at the wheel centre: `m |d centre / dc|^2 = m / cos^2(phi)`.
    pub fn generalized_mass_kg(&self, unsprung_mass_kg: f64, c_m: f64) -> f64 {
        unsprung_mass_kg * self.centre_rate(c_m).length_sq()
    }
}

/// The inertia every part the belt links rigidly to the sprocket adds to its spin, kg m^2: each other station's `J (r_s / r_i)^2` (it turns
/// `r_s / r_i` times faster) and the belt's mass at the pitch radius, `m_belt r_s^2`.
pub fn reflected_sprocket_inertia_kg_m2(rig: &PhysRig, track: &TrackDef) -> f64 {
    let sprocket = &rig.stations[track.sprocket];
    let r_s = sprocket.wheel.radius_m;
    let linked: f64 = track
        .stations
        .iter()
        .filter(|&&i| i != track.sprocket)
        .map(|&i| &rig.stations[i].wheel)
        .filter(|w| w.radius_m > 0.0)
        .map(|w| w.inertia_kg_m2 * (r_s / w.radius_m) * (r_s / w.radius_m))
        .sum();
    sprocket.wheel.inertia_kg_m2 + linked + track.mass_per_m_kg * track.belt_length_m * r_s * r_s
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_tank;

    #[test]
    fn torsion_arm_wheel_centre_follows_the_circle() {
        // a trailing arm 0.45 m long, 20 degrees below horizontal at rest
        let pivot = Vec3::new(1.2, -0.2, -1.0);
        let phi0: f64 = 0.35;
        let rest = pivot + Vec3::new(0.0, -0.45 * scalar::sin(phi0), 0.45 * scalar::cos(phi0));
        let arm = TorsionArm::new(rest, pivot);
        assert!((arm.length_m - 0.45).abs() < 1e-12 && (arm.rest_angle_rad - phi0).abs() < 1e-12);
        assert!((arm.centre_m(0.0) - rest).length() < 1e-12);
        for c in [-0.1, -0.03, 0.0, 0.05, 0.12] {
            let p = arm.centre_m(c);
            // on the circle, and risen by exactly c
            assert!(((p - pivot).length() - 0.45).abs() < 1e-12);
            assert!((p.y - rest.y - c).abs() < 1e-12, "c {c}");
            // the rate is the derivative of the position (central difference)
            let h = 1e-6;
            let fd = (arm.centre_m(c + h) - arm.centre_m(c - h)) * (0.5 / h);
            assert!((fd - arm.centre_rate(c)).length() < 1e-6, "c {c}: {fd:?} vs {:?}", arm.centre_rate(c));
        }
        // at rest the generalized mass is m / cos^2(phi0)
        let c2 = scalar::cos(phi0) * scalar::cos(phi0);
        assert!((arm.generalized_mass_kg(100.0, 0.0) - 100.0 / c2).abs() < 1e-9);
    }

    #[test]
    fn reflected_inertia_spins_up_at_t_over_j_eff() {
        // Integrate the sprocket alone under a constant torque: after t seconds the spin is T t / J_eff, and J_eff carries every linked
        // wheel at its speed ratio squared plus the belt at the pitch radius.
        let (rig, _) = box_tank();
        let track = &rig.tracks[0];
        let j = reflected_sprocket_inertia_kg_m2(&rig, track);
        let r_s = rig.stations[track.sprocket].wheel.radius_m;
        let by_hand = rig.stations[track.sprocket].wheel.inertia_kg_m2
            + track
                .stations
                .iter()
                .filter(|&&i| i != track.sprocket)
                .map(|&i| rig.stations[i].wheel.inertia_kg_m2 * scalar::powi(r_s / rig.stations[i].wheel.radius_m, 2))
                .sum::<f64>()
            + track.mass_per_m_kg * track.belt_length_m * r_s * r_s;
        assert!((j / by_hand - 1.0).abs() < 1e-12);
        assert!(j > rig.stations[track.sprocket].wheel.inertia_kg_m2, "the belt and wheels add inertia");
        let (torque, dt) = (2000.0, 1.0 / 300.0);
        let mut omega = 0.0;
        for _ in 0..300 {
            omega += torque / j * dt;
        }
        assert!((omega - torque * 1.0 / j).abs() < 1e-9);
        // the kinetic energy the torque put in is shared exactly as J_eff says: sprocket, every wheel at its own speed, the belt
        let belt_speed = omega * r_s;
        let parts = 0.5 * rig.stations[track.sprocket].wheel.inertia_kg_m2 * omega * omega
            + track
                .stations
                .iter()
                .filter(|&&i| i != track.sprocket)
                .map(|&i| {
                    let w = belt_speed / rig.stations[i].wheel.radius_m;
                    0.5 * rig.stations[i].wheel.inertia_kg_m2 * w * w
                })
                .sum::<f64>()
            + 0.5 * track.mass_per_m_kg * track.belt_length_m * belt_speed * belt_speed;
        assert!((parts / (0.5 * j * omega * omega) - 1.0).abs() < 1e-12);
    }
}
