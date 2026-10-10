//! Terrain checks: numbers a validator can score against what the real world looks like.
//!
//! - **Slope histogram**: the share of ground steeper than 5, 10, 20 and 30 degrees.
//! - **Road grade distribution** along each road.
//! - **Roughness as a power spectral density** of height along a line, compared with the **ISO 8608** road-roughness classes A to H.
//!
//! *Why a spectrum.* A road profile is a signal in distance, not time. Its power spectral density `G(n)` (m^3: variance per cycle per
//! metre) says how much of the roughness lives at each spatial frequency `n` (cycles per metre; wavelength `1/n`). Real roads follow
//! `G(n) = G(n0) (n / n0)^-2` almost everywhere, so one number, `G(n0)` at `n0 = 0.1` cycles/m (a 10 m wavelength), places a road in
//! a class: A is a new motorway, H is a ploughed field. Computing it is the signal-processing a graphics person knows from audio:
//! window a segment (Hann), take the FFT, square, average segments (Welch), scale so a white-noise test comes out right.

use serde::{Deserialize, Serialize};
use w5k_contract::world::Material;
use w5k_contract::world::WorldQuery;
use w5k_math::scalar;

/// ISO 8608 reference spatial frequency, cycles per metre.
/// Percentile reported as `p95_grade`.
const P95: f64 = 0.95; // const-ok: the 95th percentile

pub const N0_CYC_PER_M: f64 = 0.1; // const-ok: ISO 8608 reference frequency

/// ISO 8608 class upper bounds of `G(n0)`, m^3 (A to G; anything above G's bound is H). Standard: ISO 8608:2016, table of road classes.
pub const ISO_8608_UPPER_M3: [f64; 7] = [32e-6, 128e-6, 512e-6, 2048e-6, 8192e-6, 32768e-6, 131072e-6]; // const-ok: ISO 8608 class limits

/// Class letter of a road with the given `G(n0)` in m^3.
pub fn iso8608_class(gd_n0_m3: f64) -> char {
    let letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];
    for (u, l) in ISO_8608_UPPER_M3.iter().zip(letters) {
        if gd_n0_m3 < *u {
            return l;
        }
    }
    'H'
}

#[derive(Clone, Debug, Serialize)]
pub struct SlopeStats {
    pub cells: usize,
    /// Share of cells whose ground is steeper than each angle, 0 to 1.
    pub share_over_5_deg: f64,
    pub share_over_10_deg: f64,
    pub share_over_20_deg: f64,
    pub share_over_30_deg: f64,
    pub mean_deg: f64,
    pub max_deg: f64,
}

/// Slope angle at the centre of every cell, from the world's own normals (so cliffs, banks and ripples all count).
pub fn slope_stats(world: &dyn WorldQuery, half_m: f64, cell_m: f64) -> SlopeStats {
    let n = (2.0 * half_m / cell_m).round() as usize;
    let (mut over, mut sum, mut max) = ([0usize; 4], 0.0, 0.0f64);
    let limits = [5.0, 10.0, 20.0, 30.0]; // const-ok: the angles the validator asked for
    for j in 0..n {
        for i in 0..n {
            let (x, z) = (-half_m + (i as f64 + 0.5) * cell_m, -half_m + (j as f64 + 0.5) * cell_m);
            let ny = world.normal(x, z).y.clamp(-1.0, 1.0);
            let deg = scalar::rad_to_deg(scalar::acos(ny));
            sum += deg;
            max = max.max(deg);
            for (k, l) in limits.iter().enumerate() {
                over[k] += usize::from(deg > *l);
            }
        }
    }
    let cells = (n * n).max(1);
    let share = |k: usize| over[k] as f64 / cells as f64;
    SlopeStats {
        cells,
        share_over_5_deg: share(0),
        share_over_10_deg: share(1),
        share_over_20_deg: share(2),
        share_over_30_deg: share(3),
        mean_deg: sum / cells as f64,
        max_deg: max,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GradeStats {
    pub length_m: f64,
    pub max_grade: f64,
    pub mean_grade: f64,
    pub p95_grade: f64,
    /// Share of the road length in grade bins of 2 percent: `[0, 2)`, `[2, 4)`, ... the last bin is everything steeper.
    pub histogram_2pct: Vec<f64>,
}

/// Grade distribution of a centreline `(x, z, y)` (rise over run between consecutive points, weighted by length).
pub fn grade_stats(road: &[(f64, f64, f64)]) -> GradeStats {
    let bins = 8usize; // const-ok: histogram resolution
    let mut segs: Vec<(f64, f64)> = Vec::new(); // (grade, length)
    for w in road.windows(2) {
        let run = scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1);
        if run > 1e-9 {
            segs.push(((w[1].2 - w[0].2).abs() / run, run));
        }
    }
    let length: f64 = segs.iter().map(|s| s.1).sum();
    let mut hist = vec![0.0; bins];
    for &(g, l) in &segs {
        let b = ((g / 0.02) as usize).min(bins - 1); // const-ok: 2 percent bins
        hist[b] += l / length.max(1e-12);
    }
    let mut sorted = segs.clone();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut acc = 0.0;
    let mut p95 = 0.0;
    for (g, l) in &sorted {
        acc += l;
        if acc >= P95 * length {
            p95 = *g;
            break;
        }
    }
    GradeStats {
        length_m: length,
        max_grade: sorted.last().map_or(0.0, |s| s.0),
        mean_grade: segs.iter().map(|s| s.0 * s.1).sum::<f64>() / length.max(1e-12),
        p95_grade: p95,
        histogram_2pct: hist,
    }
}

/// In-place radix-2 FFT of `re + i im` (length a power of two), twiddles from the portable sin and cos.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -scalar::TAU / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = scalar::sin_cos(ang * k as f64);
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Psd {
    /// Spatial frequency of each bin, cycles per metre.
    pub n_cyc_per_m: Vec<f64>,
    /// One-sided power spectral density, m^3.
    pub g_m3: Vec<f64>,
    pub segments: usize,
    pub segment_m: f64,
}

/// Welch estimate of the one-sided PSD of equally spaced samples `z` (spacing `dx_m`): Hann window, 50 percent overlap, each segment
/// detrended by a straight line (the slope of the ground is not roughness). Returns `None` if there is not even one segment.
pub fn psd(z: &[f64], dx_m: f64, segment_len: usize) -> Option<Psd> {
    assert!(segment_len.is_power_of_two());
    if z.len() < segment_len {
        return None;
    }
    let win: Vec<f64> =
        (0..segment_len).map(|i| 0.5 * (1.0 - scalar::cos(scalar::TAU * i as f64 / segment_len as f64))).collect();
    let win_power = win.iter().map(|w| w * w).sum::<f64>() / segment_len as f64;
    let half = segment_len / 2;
    let mut acc = vec![0.0; half + 1];
    let (mut segs, mut start) = (0usize, 0usize);
    while start + segment_len <= z.len() {
        let seg = &z[start..start + segment_len];
        // Least-squares line through the segment, removed.
        let (mean_i, mean_z) = ((segment_len - 1) as f64 * 0.5, seg.iter().sum::<f64>() / segment_len as f64);
        let (mut sxy, mut sxx) = (0.0, 0.0);
        for (i, v) in seg.iter().enumerate() {
            sxy += (i as f64 - mean_i) * (v - mean_z);
            sxx += (i as f64 - mean_i) * (i as f64 - mean_i);
        }
        let slope = sxy / sxx;
        let mut re: Vec<f64> =
            seg.iter().enumerate().map(|(i, v)| (v - mean_z - slope * (i as f64 - mean_i)) * win[i]).collect();
        let mut im = vec![0.0; segment_len];
        fft(&mut re, &mut im);
        for (k, a) in acc.iter_mut().enumerate() {
            *a += re[k] * re[k] + im[k] * im[k];
        }
        segs += 1;
        start += half;
    }
    let scale = 2.0 * dx_m / (segment_len as f64 * win_power * segs as f64);
    let df = 1.0 / (segment_len as f64 * dx_m);
    Some(Psd {
        n_cyc_per_m: (1..=half).map(|k| k as f64 * df).collect(),
        g_m3: acc[1..].iter().map(|a| a * scale).collect(),
        segments: segs,
        segment_m: segment_len as f64 * dx_m,
    })
}

impl Psd {
    /// `G(n0)`: the mean PSD within a factor of 1.25 of the ISO reference frequency, m^3 (`None` if the segments are too short to reach it).
    pub fn gd_n0(&self) -> Option<f64> {
        let (lo, hi) = (N0_CYC_PER_M / 1.25, N0_CYC_PER_M * 1.25); // const-ok: averaging band around n0
        let v: Vec<f64> =
            self.n_cyc_per_m.iter().zip(&self.g_m3).filter(|(n, _)| **n >= lo && **n <= hi).map(|(_, g)| *g).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    }

    /// Slope `w` of `log G` against `log n` over `[lo, hi]` cycles/m (ISO 8608 idealises it as -2).
    pub fn exponent(&self, lo: f64, hi: f64) -> Option<f64> {
        let pts: Vec<(f64, f64)> = self
            .n_cyc_per_m
            .iter()
            .zip(&self.g_m3)
            .filter(|(n, g)| **n >= lo && **n <= hi && **g > 0.0)
            .map(|(n, g)| (scalar::ln(*n), scalar::ln(*g)))
            .collect();
        if pts.len() < 3 {
            // const-ok: a line needs a few points
            return None;
        }
        let m = pts.len() as f64;
        let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / m, pts.iter().map(|p| p.1).sum::<f64>() / m);
        let (sxy, sxx) =
            pts.iter().fold((0.0, 0.0), |(a, b), p| (a + (p.0 - mx) * (p.1 - my), b + (p.0 - mx) * (p.0 - mx)));
        Some(sxy / sxx)
    }

    /// The spectrum in `bins` logarithmic bands (mean G in each), for plotting: `(n_centre, g)`.
    pub fn log_binned(&self, bins: usize) -> Vec<(f64, f64)> {
        let (lo, hi) = (self.n_cyc_per_m[0], self.n_cyc_per_m[self.n_cyc_per_m.len() - 1]);
        let ratio = scalar::pow(hi / lo, 1.0 / bins as f64);
        let mut out = Vec::new();
        for b in 0..bins {
            let (a, c) = (lo * scalar::pow(ratio, b as f64), lo * scalar::pow(ratio, b as f64 + 1.0));
            let v: Vec<f64> =
                self.n_cyc_per_m.iter().zip(&self.g_m3).filter(|(n, _)| **n >= a && **n < c).map(|(_, g)| *g).collect();
            if !v.is_empty() {
                out.push((scalar::sqrt(a * c), v.iter().sum::<f64>() / v.len() as f64));
            }
        }
        out
    }
}

/// Heights along a polyline `(x, z)` resampled every `dx_m` of arc, read from the world's own `height_m` (so washboard and whoops count).
pub fn profile_along(world: &dyn WorldQuery, line: &[(f64, f64)], dx_m: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut carry = 0.0; // arc length already walked past the last sample
    let mut prev = line[0];
    out.push(world.height_m(prev.0, prev.1));
    for &p in &line[1..] {
        let len = scalar::hypot(p.0 - prev.0, p.1 - prev.1);
        if len <= 0.0 {
            continue;
        }
        let mut s = dx_m - carry;
        while s <= len {
            let f = s / len;
            out.push(world.height_m(prev.0 + (p.0 - prev.0) * f, prev.1 + (p.1 - prev.1) * f));
            s += dx_m;
        }
        carry = len - (s - dx_m);
        prev = p;
    }
    out
}

/// The largest power-of-two segment (at least 256 samples) that fits at least twice, so Welch has something to average.
pub fn segment_for(samples: usize) -> usize {
    let mut n = 256; // const-ok: smallest useful segment
    while n * 2 * 2 <= samples && n < 4096 {
        // const-ok: largest segment
        n *= 2;
    }
    n
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeStatus {
    /// Written from memory, to be checked against the book by VALIDATION.
    Unverified,
    Verified,
}

/// A published range for one number of one material (`content/world/published_ranges.ron`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedRange {
    pub material: String,
    /// `mu_peak`, `mu_slide`, `rolling_coeff`, or `soil.<field>` as in the material table.
    pub quantity: String,
    pub lo: f64,
    pub hi: f64,
    pub source: String,
    pub status: RangeStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct RangeCheck {
    pub material: String,
    pub quantity: String,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub in_range: bool,
    pub status: RangeStatus,
    pub source: String,
}

/// The value of a named quantity of a material (`None` if the material has no such quantity).
pub fn material_quantity(m: &Material, q: &str) -> Option<f64> {
    match q {
        "mu_peak" => Some(m.mu_peak),
        "mu_slide" => Some(m.mu_slide),
        "rolling_coeff" => Some(m.rolling_coeff),
        "roughness_rms_m" => Some(m.roughness_rms_m),
        _ => {
            let s = m.soil?;
            match q {
                "soil.n" => Some(s.n),
                "soil.kc_pa_m_n1" => Some(s.kc_pa_m_n1),
                "soil.kphi_pa_m_n" => Some(s.kphi_pa_m_n),
                "soil.cohesion_pa" => Some(s.cohesion_pa),
                "soil.friction_angle_deg" => Some(scalar::rad_to_deg(s.friction_angle_rad)),
                "soil.shear_k_m" => Some(s.shear_k_m),
                _ => None,
            }
        }
    }
}

/// Put every material number that has a published range beside it. Errors name a range that refers to nothing.
pub fn check_materials(
    table: &w5k_contract::world::MaterialTable,
    ranges: &[PublishedRange],
) -> Result<Vec<RangeCheck>, String> {
    ranges
        .iter()
        .map(|r| {
            let m = table
                .materials
                .iter()
                .find(|m| m.name == r.material)
                .ok_or_else(|| format!("published range for unknown material `{}`", r.material))?;
            let v = material_quantity(m, &r.quantity)
                .ok_or_else(|| format!("{} has no quantity `{}`", r.material, r.quantity))?;
            Ok(RangeCheck {
                material: r.material.clone(),
                quantity: r.quantity.clone(),
                value: v,
                lo: r.lo,
                hi: r.hi,
                in_range: v >= r.lo && v <= r.hi,
                status: r.status,
                source: r.source.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_math::Pcg32;

    #[test]
    fn psd_of_white_noise_has_the_expected_level() {
        let (sigma, dx) = (0.05, 0.25);
        let mut rng = Pcg32::new(1, 1);
        let z: Vec<f64> = (0..65536).map(|_| sigma * rng.normal()).collect();
        let p = psd(&z, dx, 1024).expect("psd");
        let mean = p.g_m3.iter().sum::<f64>() / p.g_m3.len() as f64;
        let expect = 2.0 * dx * sigma * sigma; // one-sided PSD of white noise
        assert!((mean / expect - 1.0).abs() < 0.08, "level {mean} vs expected {expect}");
    }

    #[test]
    fn psd_integrates_to_the_variance_of_a_sinusoid() {
        let (amp, f, dx) = (0.1, 0.2, 0.25);
        let z: Vec<f64> = (0..8192).map(|i| amp * scalar::sin(scalar::TAU * f * i as f64 * dx)).collect();
        let p = psd(&z, dx, 1024).expect("psd");
        let df = p.n_cyc_per_m[1] - p.n_cyc_per_m[0];
        let var: f64 = p.g_m3.iter().sum::<f64>() * df;
        assert!((var / (amp * amp / 2.0) - 1.0).abs() < 0.1, "variance {var} vs {}", amp * amp / 2.0);
    }

    #[test]
    fn iso_8608_class_boundaries_are_where_the_standard_puts_them() {
        for (g, c) in [
            (16e-6, 'A'),
            (31.9e-6, 'A'),
            (32.1e-6, 'B'),
            (64e-6, 'B'),
            (256e-6, 'C'),
            (1024e-6, 'D'),
            (4096e-6, 'E'),
            (16384e-6, 'F'),
            (65536e-6, 'G'),
            (131073e-6, 'H'),
            (1.0, 'H'),
        ] {
            assert_eq!(iso8608_class(g), c, "G(n0) = {g}");
        }
    }

    #[test]
    fn a_synthetic_class_b_road_is_measured_as_class_b_with_exponent_minus_two() {
        // Sum of sinusoids with amplitudes from G(n) = G0 (n/n0)^-2 and random phases (the standard way to synthesise a road).
        let (g0, dx, len) = (64e-6, 0.25, 4096usize);
        let mut rng = Pcg32::new(7, 7);
        let (n_lo, df) = (0.005, 0.005);
        let comps: Vec<(f64, f64, f64)> = (1..=399)
            .map(|k| {
                let n = n_lo + df * k as f64;
                let g = g0 * (N0_CYC_PER_M / n) * (N0_CYC_PER_M / n);
                (n, scalar::sqrt(2.0 * g * df), rng.range_f64(0.0, scalar::TAU))
            })
            .collect();
        let z: Vec<f64> = (0..len * 4)
            .map(|i| comps.iter().map(|(n, a, ph)| a * scalar::sin(scalar::TAU * n * i as f64 * dx + ph)).sum())
            .collect();
        let p = psd(&z, dx, 2048).expect("psd");
        let g = p.gd_n0().expect("reaches n0");
        assert_eq!(iso8608_class(g), 'B', "measured G(n0) = {g}");
        let w = p.exponent(0.03, 1.0).expect("exponent");
        assert!((w + 2.0).abs() < 0.3, "waviness exponent {w}");
    }

    #[test]
    fn slope_shares_of_a_planar_ramp_are_exact() {
        // Grade 0.2 is 11.3 degrees: steeper than 5 and 10, not than 20 or 30.
        let w = crate::grid::GridWorld::from_fn_sized(101, |x, _| 0.2 * x);
        let s = slope_stats(&w, 50.0, 1.0);
        assert!((s.share_over_5_deg - 1.0).abs() < 1e-12 && (s.share_over_10_deg - 1.0).abs() < 1e-12);
        assert!(s.share_over_20_deg.abs() < 1e-12 && s.share_over_30_deg.abs() < 1e-12);
        assert!((s.mean_deg - scalar::rad_to_deg(scalar::atan(0.2))).abs() < 1e-3);
    }

    #[test]
    fn road_grade_statistics_of_a_constant_grade_road_are_that_grade() {
        let road: Vec<(f64, f64, f64)> = (0..200).map(|i| (i as f64, 0.0, 0.05 * i as f64)).collect();
        let g = grade_stats(&road);
        assert!(
            (g.max_grade - 0.05).abs() < 1e-12
                && (g.mean_grade - 0.05).abs() < 1e-12
                && (g.p95_grade - 0.05).abs() < 1e-12
        );
        assert!((g.length_m - 199.0).abs() < 1e-9);
        assert!((g.histogram_2pct[2] - 1.0).abs() < 1e-12, "all of it in the 4 to 6 percent bin");
    }

    #[test]
    fn resampling_a_line_gives_equally_spaced_heights() {
        let w = crate::grid::GridWorld::from_fn_sized(101, |x, _| 0.1 * x);
        let line = [(-40.0, 0.0), (-10.0, 0.0), (-10.0, 30.0), (20.0, 30.0)];
        let z = profile_along(&w, &line, 0.5);
        assert_eq!(z.len(), 181); // 90 m of path at 0.5 m
        assert!((z[1] - z[0] - 0.05).abs() < 1e-6, "0.5 m along a 0.1 grade");
        assert!((z[61] - z[60]).abs() < 1e-9, "the leg along z crosses no slope in x");
    }

    #[test]
    fn published_ranges_parse_and_every_one_refers_to_a_real_material_number() {
        let ranges: Vec<PublishedRange> =
            ron::from_str(include_str!("../../../content/world/published_ranges.ron")).expect("parse");
        let table = crate::strip::standard_material_table().expect("table");
        let checks = check_materials(&table, &ranges).expect("every range names a real number");
        assert_eq!(checks.len(), ranges.len());
        assert!(
            ranges.iter().all(|r| r.lo < r.hi && !r.source.is_empty()),
            "every range is a real range with a source"
        );
        assert!(
            ranges.iter().all(|r| r.status == RangeStatus::Unverified),
            "nothing is marked Verified until VALIDATION opens the book"
        );
    }

    #[test]
    fn a_value_outside_its_published_range_is_flagged_and_one_inside_is_not() {
        let table = crate::strip::standard_material_table().expect("table");
        let r = |q: &str, lo: f64, hi: f64| PublishedRange {
            material: "asphalt".into(),
            quantity: q.into(),
            lo,
            hi,
            source: "test".into(),
            status: RangeStatus::Unverified,
        };
        let checks = check_materials(&table, &[r("mu_peak", 0.8, 1.0), r("mu_peak", 0.2, 0.5)]).expect("checks");
        assert!(checks[0].in_range && !checks[1].in_range);
        assert!(check_materials(&table, &[r("soil.n", 0.0, 1.0)]).is_err(), "asphalt has no soil");
    }
}
