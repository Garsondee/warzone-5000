//! Laying a model's side view over a reference drawing. The model is fitted to the picture by one point (a wheel hub) and a scale (the tyre's
//! size), its silhouette is rendered at the picture's scale, and the two silhouettes are compared: their overlap (intersection over union) and a
//! difference map that shows where the model has what the drawing has not (red) and the drawing has what the model has not (blue).
//!
//! The side view is seen from the vehicle's left, so the nose is to the left as in most drawings: z runs to the right, y up.

use crate::mesh::Mesh;
use crate::raster::{render, Camera, Item, Mode};
use w5k_math::Vec3;

/// How the model's side view maps onto a picture: the picture's pixel (u to the right, v down) of the model's point (z, y), and the size of a
/// picture pixel in metres.
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    pub at_px: (f64, f64),
    pub at_m: (f64, f64),
    pub m_per_px: f64,
}

/// The model's silhouette in a picture of `w` x `h` pixels, rendered at `k` times its resolution (`w * k` x `h * k`, row by row).
pub fn silhouette(meshes: &[Mesh], fit: &Fit, w: usize, h: usize, k: usize) -> Vec<bool> {
    let (u0, v0) = fit.at_px;
    let (z0, y0) = fit.at_m;
    let s = fit.m_per_px;
    let target = Vec3::new(0.0, y0 - (h as f64 / 2.0 - v0) * s, z0 + (w as f64 / 2.0 - u0) * s);
    // the eye is on the vehicle's left: its right-hand side of the picture is +z, the tail
    let cam =
        Camera { eye: target + Vec3::new(-50.0, 0.0, 0.0), target, fov_rad: None, ortho_half_h_m: h as f64 / 2.0 * s }; // const-ok: far enough to see the whole vehicle
    let items: Vec<Item> =
        meshes.iter().map(|m| Item { mesh: m, colour: [0.2, 0.2, 0.2], edge: &[], cavity: &[] }).collect(); // const-ok: any colour but the background's
    let magenta = [255, 0, 255]; // const-ok: a background no shaded part has
    render(&items, &cam, w * k, h * k, Mode::Shaded, [1.0, 0.0, 1.0]).chunks(3).map(|p| p != magenta).collect()
}

/// What the drawing draws: the pixels above `ground_row` that are darker than `light` (a drawing on a light background; the ground and the
/// caption below the vehicle are left out by the row).
pub fn foreground(rgb: &[u8], w: usize, h: usize, ground_row: usize, light: u32) -> Vec<bool> {
    (0..w * h)
        .map(|i| {
            let p = &rgb[i * 3..i * 3 + 3];
            let lum = (u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114) / 1000; // const-ok: Rec. 601 luma weights
            i / w < ground_row.min(h) && lum < light
        })
        .collect()
}

/// A picture or a mask `k` times larger, each pixel repeated (`channels` values a pixel).
pub fn upsample<T: Copy>(a: &[T], w: usize, h: usize, k: usize, channels: usize) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() * k * k);
    for y in 0..h * k {
        for x in 0..w * k {
            let i = ((y / k) * w + x / k) * channels;
            out.extend_from_slice(&a[i..i + channels]);
        }
    }
    out
}

/// Intersection over union of two masks of the same size: 1 when they are the same shape in the same place.
pub fn overlap(a: &[bool], b: &[bool]) -> f64 {
    let both = a.iter().zip(b).filter(|(&p, &q)| p && q).count();
    let either = a.iter().zip(b).filter(|(&p, &q)| p || q).count();
    both as f64 / either.max(1) as f64
}

/// The difference map over a picture (`rgb`, already at the masks' size): where only the model is, red; where only the drawing is, blue; where
/// both are, the drawing's own pixel; elsewhere white.
pub fn difference(rgb: &[u8], model: &[bool], drawing: &[bool]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgb.len());
    for (i, (&m, &d)) in model.iter().zip(drawing).enumerate() {
        let px: [u8; 3] = match (m, d) {
            (true, false) => [230, 60, 50], // const-ok: picture colours
            (false, true) => [50, 90, 230], // const-ok: picture colours
            (true, true) => [rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]],
            (false, false) => [255, 255, 255], // const-ok: picture colours
        };
        out.extend_from_slice(&px);
    }
    out
}

/// The picture with the model's outline drawn in red, `thick` pixels wide, and its inside lightly tinted.
pub fn outline_over(rgb: &[u8], model: &[bool], w: usize, h: usize, thick: usize) -> Vec<u8> {
    let at = |x: isize, y: isize| {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && model[y as usize * w + x as usize]
    };
    let near = |x: usize, y: usize| {
        let t = thick as isize;
        (-t..=t).any(|dy| (-t..=t).any(|dx| !at(x as isize + dx, y as isize + dy)))
    };
    let mut out = rgb.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 3;
            if model[y * w + x] {
                let edge = near(x, y);
                for c in 0..3 {
                    let red = [230.0, 40.0, 40.0][c]; // const-ok: picture colours
                    let k = if edge { 1.0 } else { 0.22 }; // const-ok: the weight of the red over the drawing
                    out[i + c] = (f64::from(out[i + c]) * (1.0 - k) + red * k) as u8;
                }
            }
        }
    }
    out
}
