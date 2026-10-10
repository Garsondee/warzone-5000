//! Belly drag: when the hull sinks past its ground clearance the underside bears on the soil, shears it and bulldozes it.
//!
//! The belly plate is one [`TrackSample`] with a stationary shoe (belt speed 0): the same Bekker pressure-sinkage, Janosi shear and compaction
//! laws as the track, over the plate's own width and area. The only new thing is the onset: the penetration the plate sees is eased in with a
//! smoothstep over `belly_ramp_m` beyond the clearance, so the force grows from zero with zero slope and never jumps (a cliff in a force makes a
//! numerical hammer; real soil has a bow wave that does the same job).

use w5k_contract::rig::{PhysRig, ProxyRole, ProxyShape};
use w5k_contract::{ContactElement, ContactInput, ContactOutput, Material};
use w5k_math::scalar;

use crate::sample::{SampleGeom, TrackSample};
use crate::tuning::Tuning;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BellyGeom {
    /// How far the belly plane is above the bottom of the belt, m.
    pub clearance_m: f64,
    /// Plate width (the smaller dimension, `b` in Bekker's law) and length, m.
    pub width_m: f64,
    pub length_m: f64,
    /// Stiffness of the hull against hard ground, N/m.
    pub stiffness_n_m: f64,
}

impl BellyGeom {
    /// From the rig's `ProxyRole::Belly` box proxy (the contract's own place for it): clearance is the lowest face above the ground plane,
    /// width the smaller and length the larger plan dimension. `None` if the rig has no such proxy.
    pub fn from_rig(rig: &PhysRig, tuning: &Tuning) -> Option<BellyGeom> {
        let proxy = rig.proxies.iter().find(|p| p.role == ProxyRole::Belly)?;
        let ProxyShape::Box { half_m } = &proxy.shape else { return None };
        let (a, b) = (2.0 * half_m.x, 2.0 * half_m.z); // const-ok: full extents from half extents
        Some(BellyGeom {
            clearance_m: proxy.pose.pos.y - half_m.y + rig.ride_height_m,
            width_m: a.min(b),
            length_m: a.max(b),
            stiffness_n_m: tuning.belly_stiffness_n_m,
        })
    }
}

pub struct Belly {
    geom: BellyGeom,
    tuning: Tuning,
    plate: TrackSample,
}

impl Belly {
    pub fn new(geom: BellyGeom, tuning: Tuning) -> Belly {
        let sample = SampleGeom {
            width_m: geom.width_m,
            length_m: geom.length_m,
            stiffness_n_m: geom.stiffness_n_m,
            damping_ns_m: 0.0,
            shoe_mu_scale: 1.0,
            shoe_mu_scale_soft: None,
        };
        Belly { geom, tuning, plate: TrackSample::new(sample, tuning) }
    }

    pub fn clearance_m(&self) -> f64 {
        self.geom.clearance_m
    }

    /// Forces on the hull for a belt-bottom penetration `belt_penetration_m` (the belly plane is `clearance_m` higher), hull velocity over the
    /// ground in the hull frame. Zero until the sinkage passes the clearance.
    pub fn step(
        &mut self,
        belt_penetration_m: f64,
        vel_long_m_s: f64,
        vel_lat_m_s: f64,
        ground: &Material,
        dt_s: f64,
    ) -> ContactOutput {
        let c = belt_penetration_m - self.geom.clearance_m;
        // The plate's contact grows as it sinks in: the load follows the eased penetration, and the shear (which has a cohesive part that does
        // not vanish at zero pressure) follows the eased contact fraction.
        let bite = scalar::smoothstep(0.0, self.tuning.belly_ramp_m, c);
        let eased = c * bite;
        let mut o = self.plate.step(&ContactInput {
            penetration_m: eased,
            penetration_rate_m_s: 0.0,
            vel_long_m_s,
            vel_lat_m_s,
            surface_speed_m_s: 0.0,
            ground,
            dt_s,
        });
        o.fx_n *= bite;
        o.fy_n *= bite;
        let dir = vel_long_m_s / (vel_long_m_s.abs() + self.tuning.direction_speed_m_s);
        o.fx_n -= self.plate.compaction_force_n(ground, 0.0) * dir;
        o
    }

    pub fn reset(&mut self) {
        self.plate.reset();
    }
}
