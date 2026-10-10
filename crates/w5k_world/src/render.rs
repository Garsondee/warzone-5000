//! A small software renderer: a perspective picture of a course, flat-shaded, with fog and a sky (V0 of `docs/lanes/world/visual-target.md`).
//!
//! It is the graphics pipeline in miniature: vertices go to **view space** (a rotation and a shift so the camera sits at the origin
//! looking down +z), triangles are **clipped** against a near plane, projected by dividing by depth, and filled by **edge functions**
//! (the sign of a 2D cross product says which side of an edge a pixel is on; all three the same sign means inside). A **z-buffer** keeps
//! the nearest surface per pixel. Depth is stored as `1/z` because that is what is linear in screen space.
//!
//! It is a display tool: it reads the world, never writes it, and nothing the simulation does depends on it. Everything is deterministic.

use w5k_contract::world::{PropKind, PropShape, WorldQuery};
use w5k_math::{scalar, Vec3};

use crate::course::Course;
use crate::mesh::TerrainMesh;
use crate::plot::Canvas;

// Display-only look: colours, light and fog.
const SKY_TOP: [f64; 3] = [96.0, 150.0, 210.0]; // const-ok: display colour
const SKY_HORIZON: [f64; 3] = [200.0, 218.0, 230.0]; // const-ok: display colour, also the fog colour
const GROUND_RGB: [f64; 3] = [95.0, 135.0, 78.0]; // const-ok: display colour
const HEIGHT_TINT_PER_M: f64 = 1.5; // const-ok: display colour ramp
const WATER_RGB: [f64; 3] = [48.0, 100.0, 160.0]; // const-ok: display colour
const TRUNK_RGB: [f64; 3] = [95.0, 66.0, 42.0]; // const-ok: display colour
const LEAF_RGB: [f64; 3] = [40.0, 98.0, 52.0]; // const-ok: display colour
const ROCK_RGB: [f64; 3] = [128.0, 124.0, 118.0]; // const-ok: display colour
const WALL_RGB: [f64; 3] = [168.0, 70.0, 52.0]; // const-ok: display colour
const OTHER_RGB: [f64; 3] = [120.0, 120.0, 140.0]; // const-ok: display colour
const LIGHT_DIR: [f64; 3] = [0.7, 0.6, 0.35]; // const-ok: display light direction: low sun from the east and the camera side, so shadows and faces read
const SHADE_GAIN: f64 = 0.7; // const-ok: contrast of the flat shading
const SHADE_AMBIENT: f64 = 0.3; // const-ok: ambient light
const SHADE_BANDS: f64 = 12.0; // const-ok: number of flat-shading bands (keeps the picture compressible and reads as low-poly)
const CHANNEL_MAX: f64 = 255.0; // const-ok: 8-bit channel
const NEAR_M: f64 = 0.3; // const-ok: near clipping plane
/// A tree's canopy is this many trunk radii wide, and starts this fraction of the way up.
const CANOPY_OVER_TRUNK: f64 = 4.0; // const-ok: stylised tree proportions
const CANOPY_START: f64 = 0.3; // const-ok: stylised tree proportions
/// Squash of a rock's octahedron, and how many turns it is yawed per prop id (the golden angle, so neighbours differ).
const ROCK_SQUASH: f64 = 0.7; // const-ok: stylised rock proportions
const GOLDEN_ANGLE_RAD: f64 = 2.399_963_229_728_653; // const-ok: mathematical constant
/// Sides of a trunk or other prism.
const PRISM_SIDES: usize = 6; // const-ok: low-poly look

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub fov_y_rad: f64,
    /// Distances where the fog begins and where it is complete, m.
    pub fog_m: (f64, f64),
}

/// The camera's basis and projection scale for an image of a given size.
pub struct View {
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    fwd: Vec3,
    scale: f64,
    cx: f64,
    cy: f64,
}

impl View {
    pub fn new(cam: &Camera, w: usize, h: usize) -> View {
        let fwd = (cam.target - cam.eye).normalized_or_zero();
        let right = fwd.cross(Vec3::new(0.0, 1.0, 0.0)).normalized_or_zero();
        let up = right.cross(fwd);
        let scale = h as f64 * 0.5 / scalar::tan(cam.fov_y_rad * 0.5); // const-ok: half the image height over tan(half the field of view)
        View { eye: cam.eye, right, up, fwd, scale, cx: w as f64 * 0.5, cy: h as f64 * 0.5 }
        // const-ok: image centre
    }

    /// A world point in view space: x right, y up, z forward (depth).
    fn to_view(&self, p: Vec3) -> Vec3 {
        let d = p - self.eye;
        Vec3::new(d.dot(self.right), d.dot(self.up), d.dot(self.fwd))
    }

    /// Pixel position of a world point in front of the camera.
    pub fn project(&self, p: Vec3) -> Option<(f64, f64)> {
        let v = self.to_view(p);
        (v.z > NEAR_M).then(|| (self.cx + v.x / v.z * self.scale, self.cy - v.y / v.z * self.scale))
    }
}

pub struct Frame {
    pub cv: Canvas,
    inv_z: Vec<f64>,
    view: View,
    light: Vec3,
    fog_m: (f64, f64),
}

impl Frame {
    pub fn new(cam: &Camera, w: usize, h: usize) -> Frame {
        let mut cv = Canvas::new(w, h, [0, 0, 0]);
        for y in 0..h {
            let t = y as f64 / h as f64;
            let rgb = [0, 1, 2].map(|k| (SKY_TOP[k] + (SKY_HORIZON[k] - SKY_TOP[k]) * t) as u8); // sky: top to horizon down the image
            for x in 0..w {
                cv.put(x as i64, y as i64, rgb);
            }
        }
        let light = Vec3::new(LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2]).normalized_or_zero();
        Frame { cv, inv_z: vec![0.0; w * h], view: View::new(cam, w, h), light, fog_m: cam.fog_m }
    }

    /// Draw a world-space triangle in `base` colour: Lambert-lit by the fixed light, banded, fogged by distance, two-sided.
    pub fn triangle(&mut self, tri: [Vec3; 3], base: [f64; 3]) {
        self.triangle_shadowed(tri, base, false);
    }

    /// As [`Frame::triangle`]; a `shadowed` surface gets only the ambient light.
    pub fn triangle_shadowed(&mut self, tri: [Vec3; 3], base: [f64; 3], shadowed: bool) {
        let mut n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalized_or_zero();
        if n.dot(self.view.eye - tri[0]) < 0.0 {
            n = -n;
        }
        let sun = if shadowed { 0.0 } else { n.dot(self.light) * SHADE_GAIN };
        let lit = ((sun + SHADE_AMBIENT).clamp(0.0, 1.0) * SHADE_BANDS).round() / SHADE_BANDS;
        let centre = (tri[0] + tri[1] + tri[2]) * (1.0 / 3.0); // const-ok: centroid
        let fog = ((self.view.eye - centre).length() - self.fog_m.0).max(0.0) / (self.fog_m.1 - self.fog_m.0);
        let fog = fog.min(1.0);
        let channel = |k: usize| (base[k] * lit * (1.0 - fog) + SKY_HORIZON[k] * fog).clamp(0.0, CHANNEL_MAX) as u8;
        let rgb = [channel(0), channel(1), channel(2)];
        // Clip against the near plane (Sutherland-Hodgman): a triangle becomes a polygon of 0 to 4 points.
        let v = tri.map(|p| self.view.to_view(p));
        let mut poly: Vec<Vec3> = Vec::with_capacity(4); // const-ok: a clipped triangle has at most 4 vertices
        for k in 0..3 {
            let (p, q) = (v[k], v[(k + 1) % 3]);
            if p.z >= NEAR_M {
                poly.push(p);
            }
            if (p.z >= NEAR_M) != (q.z >= NEAR_M) {
                poly.push(p.lerp(q, (NEAR_M - p.z) / (q.z - p.z)));
            }
        }
        for k in 1..poly.len().saturating_sub(1) {
            self.fill([poly[0], poly[k], poly[k + 1]], rgb);
        }
    }

    fn fill(&mut self, v: [Vec3; 3], rgb: [u8; 3]) {
        let s = v.map(|p| (self.view.cx + p.x / p.z * self.view.scale, self.view.cy - p.y / p.z * self.view.scale));
        let edge = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        let area = edge(s[0], s[1], s[2]);
        if area.abs() < 1e-12 {
            // const-ok: degenerate triangle
            return;
        }
        let (w, h) = (self.cv.w as i64, self.cv.h as i64);
        let (xs, ys) = (s.map(|p| p.0), s.map(|p| p.1));
        let x0 = (xs[0].min(xs[1]).min(xs[2]).floor() as i64).max(0);
        let x1 = (xs[0].max(xs[1]).max(xs[2]).ceil() as i64).min(w - 1);
        let y0 = (ys[0].min(ys[1]).min(ys[2]).floor() as i64).max(0);
        let y1 = (ys[0].max(ys[1]).max(ys[2]).ceil() as i64).min(h - 1);
        let inv = v.map(|p| 1.0 / p.z);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = (x as f64 + 0.5, y as f64 + 0.5); // const-ok: pixel centre
                let (w0, w1, w2) = (edge(s[1], s[2], p) / area, edge(s[2], s[0], p) / area, edge(s[0], s[1], p) / area);
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let d = w0 * inv[0] + w1 * inv[1] + w2 * inv[2];
                let at = y as usize * self.cv.w + x as usize;
                if d > self.inv_z[at] {
                    self.inv_z[at] = d;
                    self.cv.put(x, y, rgb);
                }
            }
        }
    }

    fn quad(&mut self, q: [Vec3; 4], base: [f64; 3]) {
        self.quad_shadowed(q, base, false);
    }

    fn quad_shadowed(&mut self, q: [Vec3; 4], base: [f64; 3], shadowed: bool) {
        self.triangle_shadowed([q[0], q[1], q[2]], base, shadowed);
        self.triangle_shadowed([q[0], q[2], q[3]], base, shadowed);
    }

    /// A box from 8 corners ordered as bits `x + 2 y + 4 z` of the local corner.
    fn cuboid(&mut self, c: [Vec3; 8], base: [f64; 3]) {
        for f in [[0, 1, 3, 2], [4, 5, 7, 6], [0, 1, 5, 4], [2, 3, 7, 6], [0, 2, 6, 4], [1, 3, 7, 5]] {
            self.quad(f.map(|i| c[i]), base);
        }
    }
}

fn ground_colour(name: &str, y: f64, y_lo: f64) -> [f64; 3] {
    match name {
        "asphalt" => [150.0, 150.0, 156.0],             // const-ok: display colour
        "mud" => [105.0, 80.0, 52.0],                   // const-ok: display colour
        "gravel" => [140.0, 132.0, 118.0],              // const-ok: display colour
        "planks" => [120.0, 90.0, 55.0],                // const-ok: display colour
        "sand" | "sandy_loam" => [215.0, 190.0, 125.0], // const-ok: display colour
        "riverbed" => [110.0, 105.0, 95.0],             // const-ok: display colour
        _ => [GROUND_RGB[0] + (y - y_lo) * HEIGHT_TINT_PER_M, GROUND_RGB[1], GROUND_RGB[2]],
    }
}

/// Field of view of the stock cameras.
const FOV_Y_RAD: f64 = 0.8; // const-ok: about 46 degrees, display
/// How far behind and above the road start the road camera sits, and how far ahead it looks, m.
const ROAD_CAM: (f64, f64, f64) = (10.0, 5.0, 80.0); // const-ok: display framing
/// Distance along the road used to find its heading at the start, m.
/// Fog of the road view: begins 120 m out, complete at 700 m.
const ROAD_FOG_M: (f64, f64) = (120.0, 700.0); // const-ok: display framing
const HEADING_PROBE_M: f64 = 25.0; // const-ok: display framing

/// Two stock views: from behind the start of the main road looking along it, and an oblique aerial of the whole course from the south.
pub fn stock_cameras(course: &Course) -> [(&'static str, Camera); 2] {
    let world = &course.world;
    let (lo, hi) = world.bounds();
    let p0 = course.road[0];
    let p1 = course
        .road
        .iter()
        .find(|p| scalar::hypot(p.0 - p0.0, p.1 - p0.1) >= HEADING_PROBE_M)
        .unwrap_or(&course.road[course.road.len() - 1]);
    let (dx, dz) = (p1.0 - p0.0, p1.1 - p0.1);
    let len = scalar::hypot(dx, dz).max(1e-9); // const-ok: avoids dividing by zero
    let (dx, dz) = (dx / len, dz / len);
    let ground = |x: f64, z: f64| world.height_m(x, z);
    let eye = (p0.0 - dx * ROAD_CAM.0, p0.1 - dz * ROAD_CAM.0);
    let look = (p0.0 + dx * ROAD_CAM.2, p0.1 + dz * ROAD_CAM.2);
    let extent = hi.x - lo.x;
    [
        (
            "road",
            Camera {
                eye: Vec3::new(eye.0, ground(eye.0, eye.1) + ROAD_CAM.1, eye.1),
                target: Vec3::new(look.0, ground(look.0, look.1), look.1),
                fov_y_rad: FOV_Y_RAD,
                fog_m: ROAD_FOG_M,
            },
        ),
        (
            "overview",
            Camera {
                eye: Vec3::new(0.0, hi.y + 0.4 * extent, 0.8 * extent), // const-ok: framing of the whole course
                target: Vec3::new(0.0, lo.y, 0.0),                      // const-ok: framing of the whole course
                fov_y_rad: FOV_Y_RAD,
                fog_m: (1e4, 2e4), // const-ok: no fog in the overview
            },
        ),
    ]
}

/// Draw the course: terrain as a flat-shaded mesh sampled every `step` nodes, water as flat quads, props as simple solids.
/// Is `p` in the shadow of the terrain? March a ray from `p` toward the sun (`light` points at it) until it rises above `y_max`, the
/// highest ground; if the ground is ever above the ray, the sun is blocked. A shadow ray against the heightfield, the same trick as a
/// shadow map but exact for a displacement map. Props do not cast shadows yet.
pub fn in_shadow(world: &impl WorldQuery, p: Vec3, light: Vec3, y_max: f64) -> bool {
    if light.y <= 0.0 {
        return true; // the sun is below the horizon
    }
    let steps = ((y_max - p.y) / light.y / SHADOW_STEP_M).ceil().max(0.0) as usize;
    (1..=steps).any(|k| {
        let q = p + light * (k as f64 * SHADOW_STEP_M) + Vec3::new(0.0, SHADOW_BIAS_M, 0.0);
        world.height_m(q.x, q.z) > q.y
    })
}

/// Distance between samples along a shadow ray, m (the grid is 1 m, so most ridges are seen), and a small lift against self-shadowing.
const SHADOW_STEP_M: f64 = 1.0; // const-ok: one grid cell
const SHADOW_BIAS_M: f64 = 0.2; // const-ok: avoids a surface shadowing itself

pub fn render(course: &Course, cam: &Camera, w: usize, h: usize, step: usize) -> Canvas {
    let world = &course.world;
    let n = world.n();
    let mut fr = Frame::new(cam, w, h);
    let step = step.max(1);
    let (y_lo, y_hi) = (world.bounds().0.y, world.bounds().1.y);
    let node = |i: usize, j: usize| {
        let (x, z) = world.node_xz(i, j);
        Vec3::new(x, world.height_at_node(i, j), z)
    };
    let idx: Vec<usize> = (0..n - 1).step_by(step).collect();
    for &j in &idx {
        for &i in &idx {
            let (i1, j1) = ((i + step).min(n - 1), (j + step).min(n - 1));
            let q = [node(i, j), node(i1, j), node(i1, j1), node(i, j1)];
            let (cx, cz) = (0.5 * (q[0].x + q[2].x), 0.5 * (q[0].z + q[2].z)); // const-ok: quad centre
            if let Some(s) = world.water_surface_m(cx, cz) {
                fr.quad(q.map(|p| Vec3::new(p.x, s, p.z)), WATER_RGB);
            } else {
                let name = &world.material_at(cx, cz).name;
                let mean_y = 0.25 * (q[0].y + q[1].y + q[2].y + q[3].y); // const-ok: mean of four corners
                let centre = Vec3::new(cx, world.height_m(cx, cz), cz);
                let dark = in_shadow(world, centre, fr.light, y_hi);
                fr.quad_shadowed(q, ground_colour(name, mean_y, y_lo), dark);
            }
        }
    }
    draw_props(&mut fr, world);
    fr.cv
}

/// As [`render`], but the terrain is the decimated `mesh` (one flat face per triangle, coloured by the face's material).
pub fn render_mesh(course: &Course, cam: &Camera, w: usize, h: usize, mesh: &TerrainMesh) -> Canvas {
    let world = &course.world;
    let mut fr = Frame::new(cam, w, h);
    let (y_lo, y_hi) = (world.bounds().0.y, world.bounds().1.y);
    for (t, &id) in mesh.triangles.iter().zip(&mesh.material) {
        let v = t.map(|i| {
            let p = mesh.vertices[i as usize];
            Vec3::new(p[0], p[1], p[2])
        });
        let c = (v[0] + v[1] + v[2]) * (1.0 / 3.0); // const-ok: centroid
        if let Some(s) = world.water_surface_m(c.x, c.z) {
            fr.triangle(v.map(|p| Vec3::new(p.x, s, p.z)), WATER_RGB);
        } else {
            let name = &world.materials().materials[usize::from(id)].name;
            let dark = in_shadow(world, c, fr.light, y_hi);
            fr.triangle_shadowed(v, ground_colour(name, c.y, y_lo), dark);
        }
    }
    draw_props(&mut fr, world);
    fr.cv
}

fn draw_props(fr: &mut Frame, world: &crate::grid::GridWorld) {
    for p in world.props() {
        let (pos, rot) = (p.transform.pos, p.transform);
        match (p.kind, p.shape) {
            (PropKind::Tree, PropShape::Cylinder { radius_m, height_m }) => {
                prism(fr, pos, radius_m, radius_m, 0.0, CANOPY_START * height_m, TRUNK_RGB);
                prism(fr, pos, CANOPY_OVER_TRUNK * radius_m, 0.0, CANOPY_START * height_m, height_m, LEAF_RGB);
            }
            (_, PropShape::Cylinder { radius_m, height_m }) => {
                prism(fr, pos, radius_m, radius_m, 0.0, height_m, OTHER_RGB)
            }
            (_, PropShape::Sphere { radius_m }) => {
                let yaw = f64::from(p.id.0) * GOLDEN_ANGLE_RAD;
                let (s, c) = scalar::sin_cos(yaw);
                let pt =
                    |x: f64, y: f64, z: f64| pos + Vec3::new(c * x + s * z, y * ROCK_SQUASH, -s * x + c * z) * radius_m;
                let ring = [pt(1.0, 0.0, 0.0), pt(0.0, 0.0, 1.0), pt(-1.0, 0.0, 0.0), pt(0.0, 0.0, -1.0)];
                for k in 0..4 {
                    // const-ok: an octahedron's equator has four vertices
                    fr.triangle([pt(0.0, 1.0, 0.0), ring[k], ring[(k + 1) % 4]], ROCK_RGB);
                    fr.triangle([pt(0.0, -1.0, 0.0), ring[k], ring[(k + 1) % 4]], ROCK_RGB);
                }
            }
            (kind, PropShape::Box { half_m }) => {
                let corner = |k: usize| {
                    let sgn = |bit: usize| if k >> bit & 1 == 1 { 1.0 } else { -1.0 };
                    rot.apply_point(Vec3::new(sgn(0) * half_m.x, sgn(1) * half_m.y, sgn(2) * half_m.z))
                };
                fr.cuboid(
                    std::array::from_fn(corner),
                    if matches!(kind, PropKind::Wall | PropKind::Barricade) { WALL_RGB } else { OTHER_RGB },
                );
            }
        }
    }
}

/// A vertical prism or cone frustum from `y0` to `y1` above `base` with bottom and top radii.
fn prism(fr: &mut Frame, base: Vec3, r0: f64, r1: f64, y0: f64, y1: f64, rgb: [f64; 3]) {
    let ring = |r: f64, y: f64, k: usize| {
        let a = k as f64 * std::f64::consts::TAU / PRISM_SIDES as f64;
        base + Vec3::new(r * scalar::cos(a), y, r * scalar::sin(a))
    };
    for k in 0..PRISM_SIDES {
        fr.quad([ring(r0, y0, k), ring(r0, y0, k + 1), ring(r1, y1, k + 1), ring(r1, y1, k)], rgb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(eye: Vec3, target: Vec3) -> Camera {
        Camera { eye, target, fov_y_rad: 1.0, fog_m: (1e4, 2e4) }
    }

    fn pixel(cv: &Canvas, x: usize, y: usize) -> [u8; 3] {
        let at = (y * cv.w + x) * 3;
        [cv.rgb[at], cv.rgb[at + 1], cv.rgb[at + 2]]
    }

    fn tri_at(z: f64, half: f64) -> [Vec3; 3] {
        [Vec3::new(-half, -half, z), Vec3::new(half, -half, z), Vec3::new(0.0, half, z)]
    }

    #[test]
    fn a_point_straight_ahead_projects_to_the_image_centre_and_one_to_the_right_lands_right_of_it() {
        let c = cam(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 2.0, -10.0)); // looking along -z, the simulator's forward
        let v = View::new(&c, 200, 100);
        let (x, y) = v.project(Vec3::new(0.0, 2.0, -10.0)).expect("in front");
        assert!((x - 100.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9);
        let (xr, _) = v.project(Vec3::new(1.0, 2.0, -10.0)).expect("in front");
        assert!(xr > x, "+x is to the right");
        let (_, yu) = v.project(Vec3::new(0.0, 3.0, -10.0)).expect("in front");
        assert!(yu < y, "+y is up the image (smaller row)");
        assert!(v.project(Vec3::new(0.0, 2.0, 5.0)).is_none(), "behind the camera");
    }

    #[test]
    fn a_nearer_triangle_hides_a_farther_one_whatever_the_drawing_order() {
        let c = cam(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        for near_first in [true, false] {
            let mut fr = Frame::new(&c, 64, 64);
            let (near, far) = (([255.0, 0.0, 0.0], tri_at(-5.0, 3.0)), ([0.0, 0.0, 255.0], tri_at(-9.0, 6.0)));
            let order = if near_first { [near, far] } else { [far, near] };
            for (rgb, t) in order {
                fr.triangle(t, rgb);
            }
            // The buffer holds 1/z of the winner (the pictured colour also depends on the light, which this face does not catch).
            assert!((fr.inv_z[40 * 64 + 32] - 1.0 / 5.0).abs() < 1e-9, "the triangle at z = 5 wins");
        }
    }

    #[test]
    fn a_triangle_crossing_the_near_plane_is_clipped_and_one_behind_the_camera_draws_nothing() {
        let c = cam(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let mut fr = Frame::new(&c, 64, 64);
        let before = fr.cv.rgb.clone();
        fr.triangle(tri_at(5.0, 3.0), [255.0, 0.0, 0.0]); // behind
        assert_eq!(fr.cv.rgb, before);
        fr.triangle(
            [Vec3::new(-2.0, -2.0, -5.0), Vec3::new(2.0, -2.0, 5.0), Vec3::new(0.0, 2.0, -5.0)],
            [255.0, 0.0, 0.0],
        );
        assert_ne!(fr.cv.rgb, before, "the part in front is drawn");
    }

    #[test]
    fn a_pillar_casts_a_shadow_as_long_as_its_height_over_the_tangent_of_the_sun_elevation() {
        use crate::grid::GridWorld;
        let n = 81;
        let half = 40.0;
        let height = 10.0;
        let mut hs = vec![0f32; n * n];
        for j in 0..n {
            for i in 0..n {
                let (x, z) = (i as f64 - half, j as f64 - half);
                hs[j * n + i] = if x.abs() <= 2.0 && z.abs() <= 2.0 { height as f32 } else { 0.0 };
            }
        }
        let w = GridWorld::from_arrays(n, hs, vec![0; n * n], w5k_contract::world::MaterialTable::default());
        let light = Vec3::new(LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2]).normalized_or_zero();
        let flat = scalar::hypot(light.x, light.z);
        let shadow_len = height * flat / light.y; // the pillar's height over tan(elevation), m
        let away = Vec3::new(-light.x, 0.0, -light.z) * (1.0 / flat); // the horizontal direction the shadow falls in
                                                                      // The pillar's far edge along `away` is about 2.6 m from its centre (its half-width over the direction's x component).
        let edge = 2.0 / away.x.abs().max(away.z.abs());
        let at = |t: f64| Vec3::new(away.x * t, 0.0, away.z * t);
        assert!(in_shadow(&w, at(edge + 0.5 * shadow_len), light, height), "inside the shadow");
        assert!(!in_shadow(&w, at(edge + 1.5 * shadow_len), light, height), "past its tip");
        assert!(!in_shadow(&w, at(-edge - 5.0), light, height), "on the sunny side");
    }

    #[test]
    fn a_face_square_to_the_sun_is_fully_lit_and_one_edge_on_to_it_gets_only_the_ambient_light() {
        let light = Vec3::new(LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2]).normalized_or_zero();
        let u = light.cross(Vec3::new(0.0, 0.0, 1.0)).normalized_or_zero(); // in the plane square to the sun
        let v = light.cross(u);
        let centre = Vec3::new(0.0, 10.0, 0.0);
        let mut grey = [0u8; 2];
        // Face 1 lies square to the sun (normal = light), face 2 contains the sun direction (normal = v, perpendicular to it).
        for (k, (a, b, look_from)) in [(u, v, light), (u, light, v)].into_iter().enumerate() {
            let mut fr = Frame::new(&cam(centre + look_from * 5.0, centre), 64, 64);
            fr.triangle(
                [centre + a * 6.0, centre + b * 6.0 - a * 6.0, centre - b * 6.0 - a * 6.0],
                [200.0, 200.0, 200.0],
            );
            grey[k] = fr.cv.rgb[(32 * 64 + 32) * 3];
        }
        assert_eq!(grey[0], 200, "Lambert: n.L = 1 gives the albedo (gain + ambient = 1)");
        assert_eq!(
            grey[1],
            ((SHADE_AMBIENT * SHADE_BANDS).round() / SHADE_BANDS * 200.0) as u8,
            "n.L = 0: ambient, banded"
        );
    }

    #[test]
    fn fog_blends_a_distant_surface_toward_the_horizon_colour() {
        let mut c = cam(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        c.fog_m = (10.0, 20.0);
        let mut fr = Frame::new(&c, 64, 64);
        fr.triangle(tri_at(-100.0, 60.0), [255.0, 0.0, 0.0]);
        let p = pixel(&fr.cv, 32, 40);
        let want = SKY_HORIZON.map(|v| v as u8);
        assert_eq!(p, want, "fully fogged");
    }

    #[test]
    fn rendering_the_same_course_twice_gives_the_same_picture_with_sky_above_and_ground_below() {
        let def = crate::course::CourseDef::from_ron(include_str!("../../../content/world/courses/slice.ron"))
            .expect("course");
        let course = crate::course::generate(&def).expect("generate");
        let [(_, cam)] = [stock_cameras(&course).into_iter().next().expect("road camera")];
        let a = render(&course, &cam, 160, 90, 4);
        assert_eq!(a.rgb, render(&course, &cam, 160, 90, 4).rgb);
        assert_eq!(pixel(&a, 80, 0), SKY_TOP.map(|v| v as u8), "top row is sky");
        assert_ne!(pixel(&a, 80, 89), pixel(&a, 80, 0), "bottom row is ground");
    }
}
