//! Voxelisation: what a part or vehicle physically *is*.
//!
//! The pieces are sampled on a regular grid at cell centres. Each occupied cell is either **material** (part
//! of an armour shell or a solid piece) or **interior** (empty space enclosed by a shell: internal volume for
//! engines, ammunition and crew). A cell is shell material of a piece when it is inside the piece but not
//! inside the piece's inset copy, the same rule the armour rays use, so mass and armour agree.
//!
//! Sampling at cell centres miscounts thin features (a 65 mm barrel in 70 mm cells can come out double or
//! nothing). We know each piece's exact volume from its polyhedron, so each piece's cells are rescaled to
//! match it: the grid decides *where* pieces overlap and who owns the shared space, the polyhedra decide *how
//! much* material there is.

use crate::build::Piece;
use crate::geom::{v3, Xform, V3};
use crate::schema::MaterialLibrary;

pub const EMPTY: u8 = 0;
pub const MATERIAL: u8 = 1;
pub const INTERIOR: u8 = 2;

#[derive(Clone, Debug)]
pub struct Grid {
    pub origin: V3,
    /// Edge length of one cell (m).
    pub cell: f64,
    pub dims: [usize; 3],
    pub class: Vec<u8>,
    /// Piece index owning each material cell (u16::MAX otherwise). The first piece to claim a cell owns it.
    pub piece: Vec<u16>,
    /// Per piece: how many cells it claims as its own material (owned or not).
    pub claims: Vec<u32>,
}

impl Grid {
    #[inline]
    pub fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.dims[1] + y) * self.dims[0] + x
    }

    pub fn centre(&self, x: usize, y: usize, z: usize) -> V3 {
        self.origin + v3(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5) * self.cell
    }

    /// Cell containing world point `p`, if inside the grid.
    pub fn cell_of(&self, p: V3) -> Option<[usize; 3]> {
        let q = (p - self.origin) / self.cell;
        let c = [q.x.floor(), q.y.floor(), q.z.floor()];
        if c.iter().enumerate().all(|(i, &v)| v >= 0.0 && (v as usize) < self.dims[i]) {
            Some([c[0] as usize, c[1] as usize, c[2] as usize])
        } else {
            None
        }
    }

    pub fn occupied_at(&self, p: V3) -> bool {
        self.cell_of(p).map(|[x, y, z]| self.class[self.idx(x, y, z)] != EMPTY).unwrap_or(false)
    }

    pub fn max_corner(&self) -> V3 {
        self.origin + v3(self.dims[0] as f64, self.dims[1] as f64, self.dims[2] as f64) * self.cell
    }
}

/// Bounding box of all pieces.
pub fn bounds(pieces: &[Piece]) -> (V3, V3) {
    let mut lo = v3(f64::MAX, f64::MAX, f64::MAX);
    let mut hi = v3(f64::MIN, f64::MIN, f64::MIN);
    for p in pieces {
        let (a, b) = p.poly.aabb();
        lo = lo.min(a);
        hi = hi.max(b);
    }
    (lo, hi)
}

/// Resolution needed to resolve shells: at least `min_res` cells along the longest axis, and fine enough that
/// the thinnest shell spans two cells, capped at `max_res` to bound memory.
pub fn resolution_for(pieces: &[Piece], min_res: u32, max_res: u32) -> u32 {
    let (lo, hi) = bounds(pieces);
    let s = hi - lo;
    let longest = s.x.max(s.y).max(s.z);
    let thinnest = pieces.iter().filter_map(|p| p.shell).fold(f64::MAX, f64::min);
    if thinnest == f64::MAX || thinnest <= 0.0 {
        return min_res;
    }
    let need = (2.0 * longest / thinnest).ceil() as u32;
    need.clamp(min_res, max_res.max(min_res))
}

/// Voxelise pieces with `res` cells along the longest axis (plus a one-cell empty margin all round).
pub fn voxelise(pieces: &[Piece], res: u32) -> Grid {
    let (lo, hi) = bounds(pieces);
    let size = hi - lo;
    let longest = size.x.max(size.y).max(size.z).max(1e-6);
    let cell = longest / res.max(4) as f64;
    let origin = lo - v3(cell, cell, cell);
    let dims = [
        (size.x / cell).ceil() as usize + 2,
        (size.y / cell).ceil() as usize + 2,
        (size.z / cell).ceil() as usize + 2,
    ];
    let n = dims[0] * dims[1] * dims[2];
    let mut g = Grid { origin, cell, dims, class: vec![EMPTY; n], piece: vec![u16::MAX; n], claims: vec![0; pieces.len()] };

    // What covers each cell: inside a solid piece, inside a shell's inset (interior), inside a shell's wall.
    const SOLID: u8 = 1;
    const INNER: u8 = 2;
    const WALL: u8 = 4;
    let mut bits = vec![0u8; n];
    for (pi, p) in pieces.iter().enumerate() {
        let inner = p.inner();
        let (a, b) = p.poly.aabb();
        let lo_c = ((a - origin) / cell).arr().map(|v| v.floor().max(0.0) as usize);
        let hi_c = ((b - origin) / cell).arr();
        let hi_c = [0, 1, 2].map(|i| (hi_c[i].ceil() as usize).min(dims[i] - 1));
        for z in lo_c[2]..=hi_c[2] {
            for y in lo_c[1]..=hi_c[1] {
                for x in lo_c[0]..=hi_c[0] {
                    let c = g.centre(x, y, z);
                    if !p.convex.contains(c, 0.0) {
                        continue;
                    }
                    let i = g.idx(x, y, z);
                    match &inner {
                        None => {
                            g.claims[pi] += 1;
                            if bits[i] & SOLID == 0 {
                                g.piece[i] = pi as u16; // solids own their cells over shells
                            }
                            bits[i] |= SOLID;
                        }
                        Some(q) if q.contains(c, 0.0) => bits[i] |= INNER,
                        Some(_) => {
                            g.claims[pi] += 1;
                            if bits[i] & (SOLID | WALL) == 0 {
                                g.piece[i] = pi as u16;
                            }
                            bits[i] |= WALL;
                        }
                    }
                }
            }
        }
    }
    // Solid content is always material; otherwise shell interiors merge, so a wall inside another shell's
    // interior is not material (overlapping hollow pieces form one hollow volume, with no hidden bulkheads).
    for i in 0..n {
        g.class[i] = if bits[i] & SOLID != 0 {
            MATERIAL
        } else if bits[i] & INNER != 0 {
            INTERIOR
        } else if bits[i] & WALL != 0 {
            MATERIAL
        } else {
            EMPTY
        };
        if g.class[i] != MATERIAL {
            g.piece[i] = u16::MAX;
        }
    }
    g
}

/// Physical properties derived from a grid.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MassProps {
    pub mass_kg: f64,
    pub centre_of_mass: [f64; 3],
    /// Inertia tensor about the centre of mass (kg m^2), row-major.
    pub inertia: [[f64; 3]; 3],
    /// Volume of material (m^3).
    pub material_m3: f64,
    /// Enclosed internal volume still free for modules (m^3).
    pub internal_m3: f64,
    pub bounds_min: [f64; 3],
    pub bounds_max: [f64; 3],
}

/// Inertia of point mass `m` at offset `r` from the reference point: m (|r|^2 I - r r^T).
fn point_inertia(m: f64, r: V3) -> [[f64; 3]; 3] {
    let r = r.arr();
    let r2 = r[0] * r[0] + r[1] * r[1] + r[2] * r[2];
    let mut t = [[0.0; 3]; 3];
    for a in 0..3 {
        for b in 0..3 {
            t[a][b] = m * (if a == b { r2 } else { 0.0 } - r[a] * r[b]);
        }
    }
    t
}

fn add3(a: &mut [[f64; 3]; 3], b: &[[f64; 3]; 3]) {
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] += b[i][j];
        }
    }
}

pub fn mass_props(g: &Grid, pieces: &[Piece], lib: &MaterialLibrary) -> MassProps {
    let cell_v = g.cell.powi(3);
    let density: Vec<f64> = pieces.iter().map(|p| lib.materials.get(&p.mat).map(|m| m.density).unwrap_or(7850.0)).collect();
    let exact: Vec<(f64, V3)> = pieces.iter().map(|p| p.material_volume()).collect();
    // Volume per owned cell for each piece: its exact volume spread over the cells it claims.
    let cell_vol: Vec<f64> = (0..pieces.len()).map(|i| if g.claims[i] > 0 { exact[i].0 / g.claims[i] as f64 } else { 0.0 }).collect();

    // Visit every mass sample as (mass, volume, position): material cells, then pieces too small to claim any
    // cell, which still have mass and count as point masses at their centroids.
    let visit = |f: &mut dyn FnMut(f64, f64, V3)| {
        let [nx, ny, nz] = g.dims;
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let i = g.idx(x, y, z);
                    if g.class[i] == MATERIAL {
                        let p = g.piece[i] as usize;
                        f(density[p] * cell_vol[p], cell_vol[p], g.centre(x, y, z));
                    }
                }
            }
        }
        for (i, (v, c)) in exact.iter().enumerate() {
            if g.claims[i] == 0 && *v > 0.0 {
                f(density[i] * v, *v, *c);
            }
        }
    };
    let (mut m, mut first, mut mat_v) = (0.0, V3::ZERO, 0.0);
    visit(&mut |dm, dv, c| {
        m += dm;
        first += c * dm;
        mat_v += dv;
    });
    let com = if m > 0.0 { first / m } else { V3::ZERO };
    let mut inertia = [[0.0f64; 3]; 3];
    visit(&mut |dm, dv, c| {
        add3(&mut inertia, &point_inertia(dm, c - com));
        // A sample stands for a small cube of its volume: add the cube's own inertia (m a^2 / 6 per axis).
        let own = dm * dv.cbrt().powi(2) / 6.0;
        for a in 0..3 {
            inertia[a][a] += own;
        }
    });

    let mut int_v = 0.0;
    let mut lo = v3(f64::MAX, f64::MAX, f64::MAX);
    let mut hi = v3(f64::MIN, f64::MIN, f64::MIN);
    let [nx, ny, nz] = g.dims;
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let class = g.class[g.idx(x, y, z)];
                if class == EMPTY {
                    continue;
                }
                if class == INTERIOR {
                    int_v += cell_v;
                }
                let c = g.centre(x, y, z);
                lo = lo.min(c);
                hi = hi.max(c);
            }
        }
    }
    let half = g.cell / 2.0;
    MassProps {
        mass_kg: m,
        centre_of_mass: com.arr(),
        inertia,
        material_m3: mat_v,
        internal_m3: int_v,
        bounds_min: (lo - v3(half, half, half)).arr(),
        bounds_max: (hi + v3(half, half, half)).arr(),
    }
}

/// Combine part properties placed by transforms (rotations or reflections, no scale) into one body.
/// Mass and material add; the centre of mass is the mass-weighted mean; inertias are rotated into the common
/// frame and moved to the common centre with the parallel-axis theorem. Bounds and internal volume are left
/// for the caller (they depend on how parts overlap).
pub fn combine(items: &[(&MassProps, Xform)]) -> MassProps {
    let m: f64 = items.iter().map(|(p, _)| p.mass_kg).sum();
    let centres: Vec<V3> = items.iter().map(|(p, x)| x.point(V3::from_arr(p.centre_of_mass))).collect();
    let com = if m > 0.0 { items.iter().zip(&centres).fold(V3::ZERO, |a, ((p, _), c)| a + *c * p.mass_kg) / m } else { V3::ZERO };
    let mut inertia = [[0.0f64; 3]; 3];
    for ((p, x), c) in items.iter().zip(&centres) {
        // R I R^T (valid for any orthogonal R, including reflections).
        let r = x.m.0;
        let i = p.inertia;
        let mut ri = [[0.0; 3]; 3];
        for a in 0..3 {
            for b in 0..3 {
                ri[a][b] = (0..3).map(|k| (0..3).map(|l| r[a][k] * i[k][l] * r[b][l]).sum::<f64>()).sum();
            }
        }
        add3(&mut inertia, &ri);
        add3(&mut inertia, &point_inertia(p.mass_kg, *c - com));
    }
    MassProps {
        mass_kg: m,
        centre_of_mass: com.arr(),
        inertia,
        material_m3: items.iter().map(|(p, _)| p.material_m3).sum(),
        internal_m3: 0.0,
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
    }
}
