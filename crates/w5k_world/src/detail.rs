//! Small-scale ground detail with a *stated* roughness spectrum.
//!
//! The hills are fractal noise whose octaves halve in amplitude as they double in frequency; that gives a profile spectrum falling as
//! `n^-3`, and with five octaves there is nothing at all above about 0.25 cycles/m. Real ground, and the road profiles of ISO 8608, fall as
//! `n^-2` all the way down. So the world adds a layer of its own over the wavelengths the 1 m grid can hold (2.5 to 20 m): octaves whose
//! amplitude falls by `2^-0.5` per doubling of frequency (the Hurst exponent H = 1/2 of a `n^-2` spectrum), **normalised by measurement**
//! so that its spectral density at the ISO reference frequency is exactly 1 m^3. Multiplying by `sqrt(G)` then gives any stated `G(n0)`.
//! The normalisation measures the layer with the same Welch estimator `w5k world stats` uses, so the two cannot disagree about the scale.

use w5k_math::scalar;

use crate::grid::{value_noise, CELL_M};
use crate::stats::{psd, N0_CYC_PER_M};

/// Longest wavelength of the layer's lattice, m, and the number of half-octave steps down from it (to about 1.25 m: just under what the
/// 1 m grid can hold, so the top of the 2.5 to 20 m band is not left to the roll-off of a single lattice). Half-octave spacing, because
/// value-noise octaves a factor of two apart are each tightly band-limited and leave a ragged spectrum.
const LONGEST_LATTICE_M: f64 = 40.0; // const-ok: the band of the detail layer
const LATTICES: usize = 11; // const-ok: the band of the detail layer
/// Amplitude ratio between neighbouring half-octaves, `2^(-H/2)`. H = 1/2 would make the spectrum fall as `n^-2` in theory, but
/// interpolated value noise falls a further 0.4 or so faster than its octave sum says; H = 0.3 cancels that, and the *measured*
/// exponent (a test) is what is held to -2.
const STEP_GAIN: f64 = 0.901_250_0; // const-ok: 2^-0.15 (H = 0.3 per octave), calibrated by measurement
/// Salt separating the detail noise from the hills noise.
const DETAIL_SALT: u64 = 0xDE7A; // const-ok: noise stream label

/// Frequency band over which a material's `roughness_rms_m` is read as a spectrum: the layer's own band, cycles per metre.
pub const BAND_CYC_PER_M: (f64, f64) = (1.0 / 20.0, 1.0 / 2.5); // const-ok: the detail layer's band

/// Variance of a profile with `G(n) = G0 (n0 / n)^2` over the layer's band, per unit `G0`, m^2 per m^3.
pub fn band_variance_per_g() -> f64 {
    N0_CYC_PER_M * N0_CYC_PER_M * (1.0 / BAND_CYC_PER_M.0 - 1.0 / BAND_CYC_PER_M.1)
}

/// `G(n0)` of a surface whose resolved-band roughness has the stated RMS height, m^3.
pub fn gd_from_rms(rms_m: f64) -> f64 {
    rms_m * rms_m / band_variance_per_g()
}

/// The layer on an `n` x `n` grid (node (i, j) at `(i - half, j - half)`), scaled so that its `G(n0)` is 1 m^3.
pub fn unit_layer(seed: u64, n: usize) -> Vec<f64> {
    let half = (n - 1) as f64 * 0.5 * CELL_M;
    let mut v = vec![0.0; n * n];
    for j in 0..n {
        for i in 0..n {
            let (x, z) = (i as f64 * CELL_M - half, j as f64 * CELL_M - half);
            let mut a = 1.0;
            let mut s = 0.0;
            for k in 0..LATTICES {
                let lam = LONGEST_LATTICE_M / scalar::pow(2.0, k as f64 * 0.5);
                s += a * value_noise(seed ^ DETAIL_SALT ^ (k as u64), x / lam, z / lam);
                a *= STEP_GAIN;
            }
            v[j * n + i] = s;
        }
    }
    // Measure G(n0) on evenly spaced rows and rescale: G scales with amplitude squared.
    let seg = if n >= 512 { 512 } else { 256 }; // const-ok: Welch segment length, samples
    let rows = 16usize; // const-ok: rows averaged for the normalisation
    let (mut sum, mut cnt) = (0.0, 0usize);
    for r in 0..rows {
        let j = (r * (n - 1)) / (rows - 1).max(1);
        let row: Vec<f64> = (0..n).map(|i| v[j * n + i]).collect();
        if let Some(g) = psd(&row, CELL_M, seg.min(n.next_power_of_two() / 2).max(64)).and_then(|p| p.gd_n0()) {
            // const-ok: smallest segment
            sum += g;
            cnt += 1;
        }
    }
    let g = sum / cnt.max(1) as f64;
    let scale = if g > 0.0 { 1.0 / scalar::sqrt(g) } else { 1.0 };
    v.iter_mut().for_each(|x| *x *= scale);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mean PSD over rows of the layer.
    fn measured(n: usize) -> crate::stats::Psd {
        let v = unit_layer(5, n);
        let seg = 256;
        let mut acc: Option<crate::stats::Psd> = None;
        let mut rows = 0.0;
        for j in (0..n).step_by(8) {
            let row: Vec<f64> = (0..n).map(|i| v[j * n + i]).collect();
            let p = psd(&row, CELL_M, seg).expect("psd");
            match acc.as_mut() {
                None => acc = Some(p),
                Some(a) => a.g_m3.iter_mut().zip(&p.g_m3).for_each(|(x, y)| *x += y),
            }
            rows += 1.0;
        }
        let mut a = acc.expect("rows");
        a.g_m3.iter_mut().for_each(|x| *x /= rows);
        a
    }

    #[test]
    fn detail_layer_has_unit_spectral_density_at_the_iso_reference_frequency() {
        let g = measured(401).gd_n0().expect("reaches n0");
        assert!((g - 1.0).abs() < 0.2, "G(n0) = {g}");
    }

    #[test]
    fn detail_layer_falls_as_n_to_the_minus_two_over_its_band() {
        let w = measured(401).exponent(0.05, 0.4).expect("exponent");
        assert!((w + 2.0).abs() < 0.4, "waviness {w}");
    }

    #[test]
    fn a_stated_rms_maps_to_an_iso_level_and_back() {
        // An asphalt road with 2 mm of resolved-band roughness is class A or B; 5 cm is a rough track.
        assert!(gd_from_rms(0.002) < 32e-6, "asphalt");
        assert!(gd_from_rms(0.05) > 8192e-6, "rough ground");
        let g = 3e-3;
        assert!(
            (band_variance_per_g() * g
                - (gd_from_rms(scalar::sqrt(band_variance_per_g() * g))) * band_variance_per_g())
            .abs()
                < 1e-12
        );
    }
}
