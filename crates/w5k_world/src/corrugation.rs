//! Washboard: ripples finer than the 1 m grid can hold (a wavelength of 0.5 to 1 m is below the grid's Nyquist limit of 2 m).
//!
//! So the ripples are not stored as heights. Each cell stores a *phase* (how far along the road it is, in metres), a *weight* (how much
//! ripple there is: 0 off the road and at the ends of the section) and which section it belongs to; the height is then a pure function of
//! position, `w * A * (1 - cos(2 pi phase / wavelength)) / 2`, with the phase and weight interpolated bilinearly. That is the same idea as
//! a procedural normal map: the detail is in a formula, the texture only steers it. The normal is the exact gradient of that function,
//! so height and normal agree. (`raycast` ignores the ripples: they are centimetres.)

use w5k_math::scalar;

use crate::grid::CELL_M;

/// "No section" marker in the region array.
pub const NO_REGION: u8 = u8::MAX;

/// One kind of ripple: peak-to-peak `amplitude_m`, crest to crest `wavelength_m`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ripple {
    pub wavelength_m: f64,
    pub amplitude_m: f64,
}

#[derive(Clone, Debug)]
pub struct Corrugation {
    pub(crate) region: Vec<u8>,
    pub(crate) phase: Vec<f32>,
    pub(crate) weight: Vec<f32>,
    pub(crate) ripples: Vec<Ripple>,
}

impl Corrugation {
    pub fn new(n: usize, ripples: Vec<Ripple>) -> Corrugation {
        Corrugation { region: vec![NO_REGION; n * n], phase: vec![0.0; n * n], weight: vec![0.0; n * n], ripples }
    }

    /// Set one node: it belongs to section `region` at road distance `phase_m` with ripple strength `weight` (0 to 1).
    pub fn set(&mut self, node: usize, region: u8, phase_m: f64, weight: f64) {
        self.region[node] = region;
        self.phase[node] = phase_m as f32;
        self.weight[node] = weight as f32;
    }

    /// Height and its plan gradient `(dh/dx, dh/dz)` at cell `(i, j)` with local coordinates `(u, v)`; `n` is the grid size.
    pub fn eval(&self, n: usize, i: usize, j: usize, u: f64, v: f64) -> (f64, f64, f64) {
        let nearest = if u < 0.5 { i } else { i + 1 } + n * if v < 0.5 { j } else { j + 1 };
        let region = self.region[nearest];
        if region == NO_REGION {
            return (0.0, 0.0, 0.0);
        }
        let r = self.ripples[region as usize];
        let (phi, dphi_x, dphi_z) = bilinear(&self.phase, n, i, j, u, v);
        let (w, dw_x, dw_z) = bilinear(&self.weight, n, i, j, u, v);
        let theta = scalar::TAU * phi / r.wavelength_m;
        let (shape, dshape) = (
            0.5 * r.amplitude_m * (1.0 - scalar::cos(theta)),
            0.5 * r.amplitude_m * scalar::sin(theta) * scalar::TAU / r.wavelength_m,
        );
        (w * shape, dw_x * shape + w * dshape * dphi_x, dw_z * shape + w * dshape * dphi_z)
    }

    /// Hash input for determinism checks: the section ids, phases and weights as bit patterns.
    pub fn hash_into(&self, h: &mut w5k_math::StateHasher) {
        h.write_bytes(&self.region);
        for (p, w) in self.phase.iter().zip(&self.weight) {
            h.write_u32(p.to_bits());
            h.write_u32(w.to_bits());
        }
    }
}

/// Bilinear value and plan gradient of a per-node array inside cell `(i, j)`.
fn bilinear(a: &[f32], n: usize, i: usize, j: usize, u: f64, v: f64) -> (f64, f64, f64) {
    let at = |di: usize, dj: usize| f64::from(a[(j + dj) * n + i + di]);
    let (a00, a10, a01, a11) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
    let val = a00 * (1.0 - u) * (1.0 - v) + a10 * u * (1.0 - v) + a01 * (1.0 - u) * v + a11 * u * v;
    let dx = ((a10 - a00) * (1.0 - v) + (a11 - a01) * v) / CELL_M;
    let dz = ((a01 - a00) * (1.0 - u) + (a11 - a10) * u) / CELL_M;
    (val, dx, dz)
}
