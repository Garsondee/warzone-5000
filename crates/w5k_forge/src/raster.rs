//! A small software rasteriser and image toolkit for preview images.
//!
//! It exists so parts can be looked at without a GPU or a game engine: in cloud sessions, in tests and in
//! continuous integration. Images are kept in **linear** RGB (light adds up linearly, so shading, blending and
//! downsampling are done there) and converted to sRGB only when saved.

use crate::geom::{v3, V3};
use std::path::Path;

pub type Rgb = [f32; 3];

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

/// Parse `#rrggbb` (sRGB) into linear RGB.
pub fn hex_linear(s: &str) -> Option<Rgb> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    let ch = |shift: u32| srgb_to_linear(((v >> shift) & 0xff) as f32 / 255.0);
    Some([ch(16), ch(8), ch(0)])
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

pub fn scale(a: Rgb, s: f32) -> Rgb {
    [a[0] * s, a[1] * s, a[2] * s]
}

#[derive(Clone, Debug)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Image {
    pub fn new(w: usize, h: usize, c: Rgb) -> Image {
        Image { w, h, px: vec![c; w * h] }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgb {
        self.px[y * self.w + x]
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgb) {
        let w = self.w;
        self.px[y * w + x] = c;
    }

    pub fn blend(&mut self, x: i64, y: i64, c: Rgb, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 || a <= 0.0 {
            return;
        }
        let i = y as usize * self.w + x as usize;
        self.px[i] = mix(self.px[i], c, a.min(1.0));
    }

    pub fn fill_rect(&mut self, x: i64, y: i64, w: i64, h: i64, c: Rgb, a: f32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// Vertical gradient fill.
    pub fn gradient(&mut self, top: Rgb, bottom: Rgb) {
        for y in 0..self.h {
            let c = mix(top, bottom, y as f32 / (self.h.max(2) - 1) as f32);
            for x in 0..self.w {
                self.set(x, y, c);
            }
        }
    }

    /// Splat a sample bilinearly over the four nearest pixels (soft, sub-pixel placement).
    fn splat(&mut self, x: f64, y: f64, c: Rgb, a: f32) {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
        let (xi, yi) = (x0 as i64, y0 as i64);
        self.blend(xi, yi, c, a * (1.0 - tx) * (1.0 - ty));
        self.blend(xi + 1, yi, c, a * tx * (1.0 - ty));
        self.blend(xi, yi + 1, c, a * (1.0 - tx) * ty);
        self.blend(xi + 1, yi + 1, c, a * tx * ty);
    }

    /// Anti-aliased line, drawn as closely spaced soft samples.
    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb, a: f32) {
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let n = (len * 2.0).ceil().max(1.0) as usize;
        for i in 0..=n {
            let t = i as f64 / n as f64;
            self.splat(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, c, a * 0.7);
        }
    }

    /// 8x8 bitmap text; `scale` multiplies the glyph size. Returns the width drawn in pixels.
    pub fn text(&mut self, x: i64, y: i64, scale: i64, s: &str, c: Rgb) -> i64 {
        use font8x8::UnicodeFonts;
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = font8x8::BASIC_FONTS.get(ch) {
                for (row, bits) in g.iter().enumerate() {
                    for col in 0..8i64 {
                        if bits & (1 << col) != 0 {
                            self.fill_rect(cx + col * scale, y + row as i64 * scale, scale, scale, c, 1.0);
                        }
                    }
                }
            }
            cx += 8 * scale;
        }
        cx - x
    }

    pub fn blit(&mut self, src: &Image, x: usize, y: usize) {
        for sy in 0..src.h {
            for sx in 0..src.w {
                let (dx, dy) = (x + sx, y + sy);
                if dx < self.w && dy < self.h {
                    self.set(dx, dy, src.get(sx, sy));
                }
            }
        }
    }

    /// Box-filter downsample by an integer factor (the anti-aliasing step of supersampling).
    pub fn downsample(&self, f: usize) -> Image {
        let (w, h) = (self.w / f, self.h / f);
        let mut out = Image::new(w, h, [0.0; 3]);
        let inv = 1.0 / (f * f) as f32;
        for y in 0..h {
            for x in 0..w {
                let mut s = [0.0f32; 3];
                for yy in 0..f {
                    for xx in 0..f {
                        let p = self.get(x * f + xx, y * f + yy);
                        for k in 0..3 {
                            s[k] += p[k];
                        }
                    }
                }
                out.set(x, y, scale(s, inv));
            }
        }
        out
    }

    pub fn to_srgb8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.px.len() * 3);
        for p in &self.px {
            for &c in p {
                out.push((linear_to_srgb(c) * 255.0 + 0.5) as u8);
            }
        }
        out
    }

    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&self.to_srgb8()).map_err(|e| e.to_string())
    }
}

/// A perspective camera.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: V3,
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    /// Focal length in pixels.
    pub focal: f64,
    pub cx: f64,
    pub cy: f64,
}

impl Camera {
    /// Look at `target` from direction (azimuth, elevation), using the armour-table convention (azimuth 0 =
    /// from the front, -Z; 90 = from the right, +X), at a distance that fits a sphere of `radius` in view.
    pub fn orbit(target: V3, radius: f64, az_deg: f64, el_deg: f64, fov_deg: f64, w: usize, h: usize) -> Camera {
        let dir = crate::armour::direction(az_deg, el_deg);
        let half = fov_deg.to_radians() / 2.0;
        let eye = target + dir * (radius / half.sin());
        let fwd = -dir;
        let world_up = if fwd.y.abs() > 0.99 { v3(0.0, 0.0, -1.0) } else { v3(0.0, 1.0, 0.0) };
        let right = fwd.cross(world_up).norm();
        let up = right.cross(fwd);
        let focal = (w.min(h) as f64 / 2.0) / half.tan();
        Camera { eye, right, up, fwd, focal, cx: w as f64 / 2.0, cy: h as f64 / 2.0 }
    }

    /// Screen position (pixels, y down) and view depth of a point, if it is in front of the camera.
    pub fn project(&self, p: V3) -> Option<[f64; 3]> {
        let d = p - self.eye;
        let z = d.dot(self.fwd);
        if z < 1e-6 {
            return None;
        }
        Some([self.cx + self.focal * d.dot(self.right) / z, self.cy - self.focal * d.dot(self.up) / z, z])
    }
}

/// Call `f(x, y, weights)` for every pixel whose centre lies inside the screen-space triangle `p`; the
/// weights are screen-space barycentric coordinates.
pub fn raster_tri(w: usize, h: usize, p: [[f64; 2]; 3], mut f: impl FnMut(usize, usize, [f64; 3])) {
    let edge = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let area = edge(p[0], p[1], p[2]);
    if area.abs() < 1e-12 {
        return;
    }
    let xmin = p.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize;
    let ymin = p.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize;
    let xmax = (p.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil() as i64).min(w as i64 - 1);
    let ymax = (p.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil() as i64).min(h as i64 - 1);
    if xmax < 0 || ymax < 0 {
        return;
    }
    for y in ymin..=ymax as usize {
        for x in xmin..=xmax as usize {
            let c = [x as f64 + 0.5, y as f64 + 0.5];
            let w0 = edge(p[1], p[2], c) / area;
            let w1 = edge(p[2], p[0], c) / area;
            let w2 = edge(p[0], p[1], c) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                f(x, y, [w0, w1, w2]);
            }
        }
    }
}

/// Colour, depth and id buffers.
pub struct Target {
    pub w: usize,
    pub h: usize,
    pub colour: Vec<Rgb>,
    pub depth: Vec<f32>,
    /// 0 = nothing drawn.
    pub id: Vec<u32>,
}

impl Target {
    pub fn new(w: usize, h: usize) -> Target {
        Target { w, h, colour: vec![[0.0; 3]; w * h], depth: vec![f32::INFINITY; w * h], id: vec![0; w * h] }
    }

    /// Depth-tested triangle. `p` holds screen x, y and view depth per vertex; `shade` receives
    /// perspective-correct barycentric weights (so attributes interpolate correctly across the 3D face).
    pub fn triangle(&mut self, p: [[f64; 3]; 3], id: u32, mut shade: impl FnMut([f32; 3]) -> Rgb) {
        let (w, h) = (self.w, self.h);
        let inv_z = [1.0 / p[0][2], 1.0 / p[1][2], 1.0 / p[2][2]];
        raster_tri(w, h, [[p[0][0], p[0][1]], [p[1][0], p[1][1]], [p[2][0], p[2][1]]], |x, y, b| {
            let iz = b[0] * inv_z[0] + b[1] * inv_z[1] + b[2] * inv_z[2];
            let z = (1.0 / iz) as f32;
            let i = y * w + x;
            if z >= self.depth[i] {
                return;
            }
            let pc = [(b[0] * inv_z[0] / iz) as f32, (b[1] * inv_z[1] / iz) as f32, (b[2] * inv_z[2] / iz) as f32];
            self.depth[i] = z;
            self.id[i] = id;
            self.colour[i] = shade(pc);
        });
    }
}
