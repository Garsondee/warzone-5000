//! A small software rasteriser for pictures (contact sheets, flag views): z-buffer, flat shading, per-vertex flags interpolated across
//! each triangle as a GPU would, 2x2 supersampling. Returns an RGB8 buffer; encoding it is the tool's job (`png` lives in `w5k_tools`).

use crate::mesh::Mesh;
use w5k_math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Flat-shaded base colour.
    Shaded,
    /// Shaded, then cavity darkens and edge brightens: roughly what LOOK will do with the flags.
    Look,
    /// The `edge` flag in false colour (blue 0 to red 1).
    Edge,
    /// The `cavity` flag in false colour.
    Cavity,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    /// Perspective: vertical field of view in rad. Orthographic: `None`, with `ortho_half_h_m` the half height of the view.
    pub fov_rad: Option<f64>,
    pub ortho_half_h_m: f64,
}

pub struct Item<'a> {
    pub mesh: &'a Mesh,
    pub colour: [f64; 3],
    /// Per-vertex flags, empty for none.
    pub edge: &'a [f64],
    pub cavity: &'a [f64],
}

fn ramp(t: f64) -> [f64; 3] {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        [0.0, 2.0 * t, 1.0 - 2.0 * t]
    } else {
        [2.0 * t - 1.0, 2.0 - 2.0 * t, 0.0]
    }
}

pub fn render(items: &[Item], cam: &Camera, w: usize, h: usize, mode: Mode, background: [f64; 3]) -> Vec<u8> {
    const SS: usize = 2; // const-ok: supersampling factor per axis
    let (sw, sh) = (w * SS, h * SS);
    let f = (cam.target - cam.eye).normalized_or_zero();
    let r = f.cross(Vec3::Y).normalized_or_zero();
    let u = r.cross(f);
    let light = (r * -0.4 + u * 0.8 + f * -0.5).normalized_or_zero(); // const-ok: key light from upper left, behind the camera
    let aspect = sw as f64 / sh as f64;
    let project = |p: Vec3| -> (f64, f64, f64) {
        let d = p - cam.eye;
        let (x, y, z) = (d.dot(r), d.dot(u), d.dot(f));
        let (sx, sy) = match cam.fov_rad {
            Some(fov) => {
                let k = 1.0 / w5k_math::scalar::tan(fov / 2.0);
                (x / z * k / aspect, y / z * k)
            }
            None => (x / (cam.ortho_half_h_m * aspect), y / cam.ortho_half_h_m),
        };
        ((sx * 0.5 + 0.5) * sw as f64, (0.5 - sy * 0.5) * sh as f64, z)
    };
    let mut depth = vec![f64::MAX; sw * sh];
    let mut col = vec![background; sw * sh];
    for it in items {
        for (k, idx) in it.mesh.t.iter().enumerate() {
            let n = it.mesh.face_normal(k);
            let tri = it.mesh.tri(k);
            if n.dot(cam.eye - tri[0]) <= 0.0 && cam.fov_rad.is_some() || n.dot(f) >= 0.0 && cam.fov_rad.is_none() {
                continue; // back face
            }
            let p = tri.map(&project);
            let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
            if area.abs() < 1e-12 {
                // const-ok: degenerate in screen space
                continue;
            }
            let shade = 0.38 + 0.62 * n.dot(light).max(0.0); // const-ok: ambient 0.38 plus lambert
            let (x0, x1) = (
                p.iter().map(|q| q.0).fold(f64::MAX, f64::min).floor().max(0.0),
                p.iter().map(|q| q.0).fold(f64::MIN, f64::max).ceil().min(sw as f64 - 1.0),
            );
            let (y0, y1) = (
                p.iter().map(|q| q.1).fold(f64::MAX, f64::min).floor().max(0.0),
                p.iter().map(|q| q.1).fold(f64::MIN, f64::max).ceil().min(sh as f64 - 1.0),
            );
            let at = |v: &[f64], l: [f64; 3]| {
                if v.is_empty() {
                    0.0
                } else {
                    l[0] * v[idx[0] as usize] + l[1] * v[idx[1] as usize] + l[2] * v[idx[2] as usize]
                }
            };
            for py in y0 as usize..=y1 as usize {
                for px in x0 as usize..=x1 as usize {
                    let (qx, qy) = (px as f64 + 0.5, py as f64 + 0.5);
                    let l0 = ((p[1].0 - qx) * (p[2].1 - qy) - (p[2].0 - qx) * (p[1].1 - qy)) / area;
                    let l1 = ((p[2].0 - qx) * (p[0].1 - qy) - (p[0].0 - qx) * (p[2].1 - qy)) / area;
                    let l2 = 1.0 - l0 - l1;
                    if l0 < 0.0 || l1 < 0.0 || l2 < 0.0 {
                        continue;
                    }
                    // Under perspective, 1/z (not z) varies linearly across the screen, so depth and the flags are interpolated with
                    // weights l_i / z_i; with screen-space weights a large triangle's depth is off by more than a glass pane stands proud.
                    let (l, z) = if cam.fov_rad.is_some() {
                        let (a0, a1, a2) = (l0 / p[0].2, l1 / p[1].2, l2 / p[2].2);
                        let sum = a0 + a1 + a2;
                        ([a0 / sum, a1 / sum, a2 / sum], 1.0 / sum)
                    } else {
                        ([l0, l1, l2], l0 * p[0].2 + l1 * p[1].2 + l2 * p[2].2)
                    };
                    if z >= depth[py * sw + px] {
                        continue;
                    }
                    depth[py * sw + px] = z;
                    let (e, c) = (at(it.edge, l), at(it.cavity, l));
                    let base = it.colour.map(|v| v * shade);
                    col[py * sw + px] = match mode {
                        Mode::Shaded => base,
                        Mode::Look => base.map(|v| v * (1.0 - 0.6 * c) + 0.35 * e * (1.0 - v)), // const-ok: look weights
                        Mode::Edge => ramp(e).map(|v| v * (0.5 + 0.5 * shade)),
                        Mode::Cavity => ramp(c).map(|v| v * (0.5 + 0.5 * shade)),
                    };
                }
            }
        }
    }
    let mut out = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0; 3];
            for i in 0..SS * SS {
                for (a, v) in acc.iter_mut().zip(col[(y * SS + i / SS) * sw + x * SS + i % SS]) {
                    *a += v;
                }
            }
            // const-ok: 8-bit colour
            out.extend(acc.map(|s| (s / (SS * SS) as f64 * 255.0 + 0.5).clamp(0.0, 255.0) as u8));
            // const-ok: 8-bit colour
        }
    }
    out
}
