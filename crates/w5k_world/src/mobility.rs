//! Mobility: where each vehicle can go on a course, from its `CapabilityTable` (what it can do) and the course (what is there).
//!
//! The vehicle side is data: a [`MobilitySpec`] (name, mass, width and the contract's `CapabilityTable`). Until the scenario runner
//! measures real tables, the committed specs are **placeholders** (`content/world/mobility/*.ron`); the same format takes the measured ones.
//!
//! Two products, both pure functions of (course, spec):
//! - a **map**: every node is `Go`, `Slow` or `NoGo` (steeper than the vehicle can climb on that surface, a surface it bogs in, water
//!   deeper than it can ford, a prop in its way, a bridge it is too heavy or too wide for);
//! - a **route**: the quickest way from the start of the main road to its end, by Dijkstra over 8-neighbour *edges*. Slope limits are
//!   directional (climbing straight up is limited by the grade, traversing along a slope by the side slope), which a per-node colouring
//!   cannot express. The route lists what it used (a bridge, water, mud), which is the **answer key** an AI is graded against.

use serde::{Deserialize, Serialize};
use w5k_contract::capability::CapabilityTable;
use w5k_contract::world::{MaterialId, MaterialTable, PropKind, PropShape, WorldQuery};
use w5k_math::scalar;

use crate::bridge::BridgeKind;
use crate::course::Course;
use crate::grid::CELL_M;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilitySpec {
    pub name: String,
    /// Where the numbers come from (PLACEHOLDER until a measured table replaces them).
    pub note: String,
    pub mass_kg: f64,
    pub width_m: f64,
    pub capability: CapabilityTable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Mobility {
    Go,
    Slow,
    NoGo,
}

/// The map: a class per node, and the speed the vehicle makes there (0 where `NoGo`), m/s.
pub struct MobilityMap {
    pub n: usize,
    pub class: Vec<Mobility>,
    pub speed_m_s: Vec<f64>,
    /// Nodes where nothing can go whatever the slope: props, bogging soil, deep water, an unusable bridge.
    hard_block: Vec<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Route {
    pub length_m: f64,
    pub time_s: f64,
    pub max_grade_climbed: f64,
    pub max_cross_slope: f64,
    /// What the route used: `bridge:Road`, `bridge:Wooden`, `water`, `mud`, ... (each once, in order of first use).
    pub uses: Vec<String>,
    /// The route as plan points `(x, z)`, every few metres.
    pub points: Vec<(f64, f64)>,
}

/// A slope within this fraction of the limit is `Slow`.
const SLOW_SLOPE_FRACTION: f64 = 0.6; // const-ok: where a vehicle starts to labour on a slope
/// Speed factor in water (the vehicle wades).
const WADE_SPEED_FACTOR: f64 = 0.3; // const-ok: wading is slow whatever the ground
/// A surface slower than this fraction of the vehicle's best is `Slow`.
const SLOW_SURFACE_FRACTION: f64 = 0.5; // const-ok: where a surface counts as slow ground
/// Clearance left beside a vehicle on a bridge, m.
const BRIDGE_MARGIN_M: f64 = 0.1; // const-ok: a vehicle must clear a deck's width by this
/// Thickness of a bridge rail, m (kept in step with `bridge.rs`).
const RAIL_M: f64 = 0.2; // const-ok: rail section
/// Spacing of the route points returned, nodes.
const ROUTE_STRIDE: usize = 4; // const-ok: thins the route for output
/// Largest speed assumed when a table lists none, m/s.
const DEFAULT_TOP_SPEED_M_S: f64 = 10.0; // const-ok: fallback when the table is empty

/// Limits measured by the proving ground (VALIDATION's capability export), on its flat dry asphalt plane. `None` = not measured, or a
/// value the proving ground's own scorer rejected (red), which is not used.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    pub max_grade_ratio: Option<f64>,
    pub max_side_slope_rad: Option<f64>,
    pub max_step_m: Option<f64>,
}

impl MobilitySpec {
    pub fn from_ron(text: &str) -> Result<MobilitySpec, String> {
        ron::from_str(text).map_err(|e| format!("mobility spec: {e}"))
    }

    /// Replace the placeholder limits by measured ones. The measured grade is for asphalt; a grade is limited by traction (`tan(slope) <=
    /// mu`), so on another surface it scales with the ratio of the surface's peak friction to asphalt's, never above the asphalt value.
    /// PROVISIONAL(world-surface-grade): crude on soft ground, where soil thrust rather than friction decides; until the proving ground
    /// runs on other surfaces. Fording, bog soils and per-surface speed are not measured and stay as the placeholders say.
    pub fn with_measured(mut self, m: &Measured, table: &MaterialTable) -> MobilitySpec {
        let mu = |id: MaterialId| table.materials.get(usize::from(id.0)).map(|x| x.mu_peak);
        if let (Some(g), Some(ref_mu)) = (m.max_grade_ratio, table.id_of("asphalt").and_then(mu)) {
            for row in &mut self.capability.max_grade {
                row.1 = g * (mu(row.0).unwrap_or(ref_mu) / ref_mu).min(1.0);
            }
        }
        if let Some(v) = m.max_side_slope_rad {
            self.capability.max_side_slope_rad = v;
        }
        if let Some(v) = m.max_step_m {
            self.capability.max_step_m = v;
        }
        if *m == Measured::default() {
            return self;
        }
        self.note.push_str(" Grade, side slope and step then replaced by the proving ground's measured values where they exist and passed its scorer (grade scaled per surface by peak friction, PROVISIONAL(world-surface-grade)).");
        self
    }

    fn grade_on(&self, m: MaterialId) -> f64 {
        let t = &self.capability.max_grade;
        t.iter().find(|(id, _)| *id == m).map(|p| p.1).or_else(|| t.iter().map(|p| p.1).reduce(f64::min)).unwrap_or(0.0)
    }

    fn speed_on(&self, m: MaterialId) -> f64 {
        let t = &self.capability.top_speed_m_s;
        t.iter()
            .find(|(id, _)| *id == m)
            .map(|p| p.1)
            .or_else(|| t.iter().map(|p| p.1).reduce(f64::min))
            .unwrap_or(DEFAULT_TOP_SPEED_M_S)
    }

    fn best_speed(&self) -> f64 {
        self.capability.top_speed_m_s.iter().map(|p| p.1).fold(0.0, f64::max).max(1e-9)
        // const-ok: avoids dividing by zero
    }

    fn soil_go(&self, m: MaterialId) -> bool {
        self.capability.soil_go.iter().find(|(id, _)| *id == m).is_none_or(|p| p.1)
    }

    fn side_limit(&self) -> f64 {
        scalar::tan(self.capability.max_side_slope_rad)
    }
}

fn sq(x: f64) -> f64 {
    x * x
}

/// Distance from `(px, pz)` to the segment `a`-`b`.
fn dist_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0) } else { 0.0 };
    scalar::hypot(p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dz))
}

/// Classify every node of the course for `spec`.
pub fn classify(course: &Course, spec: &MobilitySpec) -> MobilityMap {
    let w = &course.world;
    let n = w.n();
    let half_w = spec.width_m * 0.5;
    let mut hard = vec![false; n * n];
    let mut slow = vec![false; n * n];
    // Bridge decks: usable or not for this vehicle; their rails are not obstacles on the deck itself.
    let mut deck = vec![false; n * n];
    for b in &course.bridges {
        let fits = spec.width_m + 2.0 * (RAIL_M + BRIDGE_MARGIN_M) <= b.width_m && spec.mass_kg <= b.load_limit_kg;
        for c in 0..n * n {
            let (x, z) = w.node_xz(c % n, c / n);
            if dist_to_segment((x, z), b.a, b.b) <= b.width_m * 0.5 + CELL_M {
                deck[c] = true;
                hard[c] |= !fits;
            }
        }
    }
    // Props: trees and walls block (inflated by half the vehicle's width); rocks within the step limit are crawled over.
    for p in w.props() {
        let (px, pz) = (p.transform.pos.x, p.transform.pos.z);
        let (reach, blocks) = match p.shape {
            PropShape::Cylinder { radius_m, .. } => (radius_m + half_w, true),
            PropShape::Sphere { radius_m } => {
                let top = radius_m * (1.0 + crate::course::BURIAL);
                (radius_m + half_w, top > spec.capability.max_step_m)
            }
            PropShape::Box { half_m } => (scalar::hypot(half_m.x, half_m.z) + half_w, true),
        };
        let half = (n as f64 - 1.0) * 0.5;
        let cell = |v: f64| v / CELL_M + half;
        let (i0, i1) = (cell(px - reach).floor() as i64, cell(px + reach).ceil() as i64);
        let (j0, j1) = (cell(pz - reach).floor() as i64, cell(pz + reach).ceil() as i64);
        for j in j0.max(0)..=j1.min(n as i64 - 1) {
            for i in i0.max(0)..=i1.min(n as i64 - 1) {
                let c = j as usize * n + i as usize;
                let (x, z) = w.node_xz(i as usize, j as usize);
                let inside = match p.shape {
                    PropShape::Box { half_m } => {
                        // Distance to the oriented box in plan.
                        let local = p.transform.inverse().apply_point(w5k_math::Vec3::new(x, p.transform.pos.y, z));
                        let (dx, dz) = ((local.x.abs() - half_m.x).max(0.0), (local.z.abs() - half_m.z).max(0.0));
                        scalar::hypot(dx, dz) <= half_w
                    }
                    _ => scalar::hypot(x - px, z - pz) <= reach,
                };
                if inside && !(deck[c] && p.kind == PropKind::Wall) {
                    if blocks {
                        hard[c] = true;
                    } else {
                        slow[c] = true;
                    }
                }
            }
        }
    }
    let best = spec.best_speed();
    let mut class = vec![Mobility::Go; n * n];
    let mut speed = vec![0.0; n * n];
    for c in 0..n * n {
        let (i, j) = (c % n, c / n);
        let (x, z) = w.node_xz(i, j);
        let mat = w.material_id_at(x, z);
        let nrm = w.normal(x, z);
        let g = scalar::hypot(nrm.x, nrm.z) / nrm.y;
        let depth = w.water_surface_m(x, z).map_or(0.0, |s| s - w.height_m(x, z));
        let limit = spec.grade_on(mat);
        let mut v = spec.speed_on(mat) * (1.0 - 0.5 * sq((g / limit.max(1e-9)).min(1.0)));
        let wet = depth > 0.0;
        if wet {
            v *= WADE_SPEED_FACTOR;
        }
        let nogo = hard[c] || !spec.soil_go(mat) || depth > spec.capability.fording_depth_m || g > limit;
        let labour =
            g > SLOW_SLOPE_FRACTION * limit || wet || slow[c] || spec.speed_on(mat) < SLOW_SURFACE_FRACTION * best;
        class[c] = if nogo {
            Mobility::NoGo
        } else if labour {
            Mobility::Slow
        } else {
            Mobility::Go
        };
        speed[c] = if nogo {
            0.0
        } else if slow[c] {
            v * 0.5
        } else {
            v
        };
        hard[c] |= !spec.soil_go(mat) || depth > spec.capability.fording_depth_m;
    }
    MobilityMap { n, class, speed_m_s: speed, hard_block: hard }
}

impl MobilityMap {
    /// Share of nodes in each class: `(go, slow, nogo)`.
    pub fn shares(&self) -> (f64, f64, f64) {
        let t = self.class.len() as f64;
        let c = |m: Mobility| self.class.iter().filter(|&&x| x == m).count() as f64 / t;
        (c(Mobility::Go), c(Mobility::Slow), c(Mobility::NoGo))
    }
}

/// The quickest route from `from` to `to` (plan positions) for `spec`, or `None` if there is none.
pub fn route(
    course: &Course,
    spec: &MobilitySpec,
    map: &MobilityMap,
    from: (f64, f64),
    to: (f64, f64),
) -> Option<Route> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let w = &course.world;
    let n = map.n;
    let half = (n - 1) as f64 * 0.5 * CELL_M;
    let node = |p: (f64, f64)| {
        (
            ((p.0 + half) / CELL_M).round().clamp(0.0, (n - 1) as f64) as usize,
            ((p.1 + half) / CELL_M).round().clamp(0.0, (n - 1) as f64) as usize,
        )
    };
    let (s, t) = (node(from), node(to));
    let (si, ti) = (s.1 * n + s.0, t.1 * n + t.0);
    // Gradient of the ground at every node (rise per metre in x and z), from the world's own normal.
    let grad: Vec<(f64, f64)> = (0..n * n)
        .map(|c| {
            let (x, z) = w.node_xz(c % n, c / n);
            let nr = w.normal(x, z);
            (-nr.x / nr.y, -nr.z / nr.y)
        })
        .collect();
    let (side, scale) = (spec.side_limit(), 1000.0); // const-ok: millisecond fixed point so the heap key is an integer
    let mut best = vec![u64::MAX; n * n];
    let mut prev = vec![u32::MAX; n * n];
    let mut heap = BinaryHeap::new();
    best[si] = 0;
    heap.push(Reverse((0u64, si as u32)));
    while let Some(Reverse((cost, at))) = heap.pop() {
        if at as usize == ti {
            break;
        }
        if cost > best[at as usize] {
            continue;
        }
        let (ai, aj) = ((at as usize % n) as i64, (at as usize / n) as i64);
        for dj in -1i64..=1 {
            for di in -1i64..=1 {
                if di == 0 && dj == 0 {
                    continue;
                }
                let (bi, bj) = (ai + di, aj + dj);
                if bi < 0 || bj < 0 || bi >= n as i64 || bj >= n as i64 {
                    continue;
                }
                let b = bj as usize * n + bi as usize;
                if map.hard_block[b] || map.hard_block[at as usize] {
                    continue;
                }
                let len = scalar::hypot(di as f64, dj as f64) * CELL_M;
                let d = (di as f64 / len * CELL_M, dj as f64 / len * CELL_M);
                // Slope along the edge (uphill positive) and across it, from the mean gradient of its two ends.
                let (ga, gb) = (grad[at as usize], grad[b]);
                let g = (0.5 * (ga.0 + gb.0), 0.5 * (ga.1 + gb.1));
                let (along, cross) = (g.0 * d.0 + g.1 * d.1, (g.0 * d.1 - g.1 * d.0).abs());
                let (x, z) = w.node_xz(bi as usize, bj as usize);
                let limit = spec.grade_on(w.material_id_at(x, z));
                if along.abs() > limit || cross > side {
                    continue;
                }
                let v_node = map.speed_m_s[b];
                if v_node <= 0.0 {
                    continue;
                }
                // Labouring on the slope slows the vehicle: a quadratic derate toward the limit on either axis.
                let derate = 1.0 - 0.5 * (sq(along.abs() / limit.max(1e-9)) + sq(cross / side.max(1e-9))).min(1.0);
                let v = v_node * derate.max(0.1); // const-ok: never slower than a crawl
                let c = cost + (len / v * scale) as u64;
                if c < best[b] {
                    best[b] = c;
                    prev[b] = at;
                    heap.push(Reverse((c, b as u32)));
                }
            }
        }
    }
    if best[ti] == u64::MAX {
        return None;
    }
    let mut cells = vec![ti];
    while prev[*cells.last().expect("non-empty")] != u32::MAX {
        cells.push(prev[*cells.last().expect("non-empty")] as usize);
    }
    cells.reverse();
    // Summarise what the route went through.
    let (mut length, mut max_g, mut max_c) = (0.0, 0.0f64, 0.0f64);
    let mut uses: Vec<String> = Vec::new();
    let mut note = |s: String| {
        if !uses.contains(&s) {
            uses.push(s);
        }
    };
    for pair in cells.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (ap, bp) = (w.node_xz(a % n, a / n), w.node_xz(b % n, b / n));
        let len = scalar::hypot(bp.0 - ap.0, bp.1 - ap.1);
        length += len;
        let g = (0.5 * (grad[a].0 + grad[b].0), 0.5 * (grad[a].1 + grad[b].1));
        let d = ((bp.0 - ap.0) / len, (bp.1 - ap.1) / len);
        max_g = max_g.max((g.0 * d.0 + g.1 * d.1).abs());
        max_c = max_c.max((g.0 * d.1 - g.1 * d.0).abs());
        if w.water_surface_m(bp.0, bp.1).is_some() {
            note("water".to_string());
        }
        let mat = &w.material_at(bp.0, bp.1).name;
        if mat == "mud" || mat == "sand" || mat == "sandy_loam" {
            note(mat.clone());
        }
        for br in &course.bridges {
            if dist_to_segment(bp, br.a, br.b) <= br.width_m * 0.5 {
                note(format!("bridge:{}", if br.kind == BridgeKind::Wooden { "Wooden" } else { "Road" }));
            }
        }
    }
    let points = cells.iter().step_by(ROUTE_STRIDE).map(|&c| w.node_xz(c % n, c / n)).collect();
    Some(Route {
        length_m: length,
        time_s: best[ti] as f64 / scale,
        max_grade_climbed: max_g,
        max_cross_slope: max_c,
        uses,
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::PlacedBridge;
    use crate::grid::GridWorld;
    use crate::strip::standard_material_table;
    use w5k_contract::world::{PropId, PropRef};
    use w5k_math::{Transform, Vec3};

    const MULE: &str = include_str!("../../../content/world/mobility/mule.ron");
    const SCOUT: &str = include_str!("../../../content/world/mobility/scout.ron");
    const N: usize = 41;
    const DIRT: u8 = 1;
    const MUD: u8 = 2;

    fn mule() -> MobilitySpec {
        MobilitySpec::from_ron(MULE).expect("mule")
    }

    /// A 40 m square test field: `h(x, z)` heights, `s(x, z)` material id, no props, no bridges.
    fn field(h: impl Fn(f64, f64) -> f64, s: impl Fn(f64, f64) -> u8) -> Course {
        let half = (N - 1) as f64 * 0.5;
        let (mut hs, mut ss) = (vec![0f32; N * N], vec![0u8; N * N]);
        for j in 0..N {
            for i in 0..N {
                let (x, z) = (i as f64 - half, j as f64 - half);
                hs[j * N + i] = h(x, z) as f32;
                ss[j * N + i] = s(x, z);
            }
        }
        let world = GridWorld::from_arrays(N, hs, ss, standard_material_table().expect("materials"));
        Course {
            world,
            road: vec![(-15.0, 0.0, 0.0), (15.0, 0.0, 0.0)],
            extra_roads: vec![],
            bridges: vec![],
            ground: MaterialId(1),
            road_material: MaterialId(1),
            grade_limit: vec![],
        }
    }

    fn class_at(m: &MobilityMap, x: f64, z: f64) -> Mobility {
        let half = (N - 1) as f64 * 0.5;
        m.class[(z + half) as usize * N + (x + half) as usize]
    }

    fn go(c: &Course, spec: &MobilitySpec, from: (f64, f64), to: (f64, f64)) -> Option<Route> {
        route(c, spec, &classify(c, spec), from, to)
    }

    #[test]
    fn ground_steeper_than_the_surface_grade_limit_is_no_go_and_gentler_ground_is_not() {
        let spec = mule(); // dirt limit 0.5
        for (slope, want) in [(0.2, Mobility::Go), (0.4, Mobility::Slow), (0.7, Mobility::NoGo)] {
            let c = field(|x, _| slope * x, |_, _| DIRT);
            assert_eq!(class_at(&classify(&c, &spec), 0.0, 0.0), want, "slope {slope}");
        }
    }

    #[test]
    fn a_slope_beyond_the_side_slope_limit_is_crossed_on_a_diagonal_not_along_the_contour() {
        // 0.45 rise per metre in x: under the dirt grade limit (0.5) but over the side-slope limit (tan 0.38 = 0.40).
        let c = field(|x, _| 0.45 * x, |_, _| DIRT);
        let spec = mule();
        let up = go(&c, &spec, (-15.0, 0.0), (15.0, 0.0)).expect("up the fall line");
        assert!(up.max_grade_climbed <= 0.5 + 1e-9);
        // Going along the contour would tilt the vehicle 0.45; the route zig-zags, trading distance for a gentler tilt.
        let across = go(&c, &spec, (0.0, -15.0), (0.0, 15.0)).expect("across the slope");
        assert!(across.max_cross_slope <= spec.side_limit() + 1e-9, "tilt {}", across.max_cross_slope);
        assert!(across.length_m > 30.0 * 1.02, "longer than the straight 30 m: {}", across.length_m);
    }

    #[test]
    fn a_route_detours_through_the_only_gap_in_a_wall_of_steep_ground() {
        let wall = |x: f64, z: f64| if x.abs() < 3.0 && z < 10.0 { 5.0 } else { 0.0 };
        let c = field(wall, |_, _| DIRT);
        let r = go(&c, &mule(), (-15.0, 0.0), (15.0, 0.0)).expect("route through the gap");
        assert!(r.length_m > 1.2 * 30.0, "detour is longer than the direct {} m", r.length_m);
        assert!(r.points.iter().any(|p| p.1 >= 9.0), "passes the gap");
        let sealed = field(|x, _| if x.abs() < 3.0 { 5.0 } else { 0.0 }, |_, _| DIRT);
        assert!(go(&sealed, &mule(), (-15.0, 0.0), (15.0, 0.0)).is_none(), "no gap, no route");
    }

    #[test]
    fn water_deeper_than_the_fording_depth_blocks_and_shallower_water_is_slow() {
        let river = |depth: f64| {
            let mut c = field(|x, _| -depth * ((3.0 - x.abs()) / 3.0).clamp(0.0, 1.0), |_, _| DIRT);
            let w = &c.world;
            let surface: Vec<f32> = (0..N * N)
                .map(|k| {
                    let (x, z) = w.node_xz(k % N, k / N);
                    if w.height_m(x, z) < 0.0 {
                        0.0
                    } else {
                        f32::NAN
                    }
                })
                .collect();
            c.world.set_water(surface);
            c
        };
        let spec = mule(); // fording 0.762 m
        let shallow = river(0.4);
        let m = classify(&shallow, &spec);
        assert_eq!(class_at(&m, 0.0, 0.0), Mobility::Slow);
        let r = go(&shallow, &spec, (-15.0, 0.0), (15.0, 0.0)).expect("wades across");
        assert!(r.uses.contains(&"water".to_string()));
        let deep = river(1.2);
        assert_eq!(class_at(&classify(&deep, &spec), 0.0, 0.0), Mobility::NoGo);
        assert!(go(&deep, &spec, (-15.0, 0.0), (15.0, 0.0)).is_none());
    }

    #[test]
    fn a_surface_the_vehicle_bogs_in_is_no_go_for_it_and_not_for_one_that_does_not() {
        let c = field(|_, _| 0.0, |x, _| if x.abs() < 3.0 { MUD } else { DIRT });
        let scout = MobilitySpec::from_ron(SCOUT).expect("scout"); // soil_go false on mud
        assert!(go(&c, &scout, (-15.0, 0.0), (15.0, 0.0)).is_none());
        let r = go(&c, &mule(), (-15.0, 0.0), (15.0, 0.0)).expect("the mule crosses");
        assert!(r.uses.contains(&"mud".to_string()));
    }

    #[test]
    fn a_rock_taller_than_the_step_limit_blocks_and_a_low_one_is_crawled_over() {
        let rock = |radius_m: f64| PropRef {
            id: PropId(0),
            kind: PropKind::Rock,
            shape: PropShape::Sphere { radius_m },
            transform: Transform::from_pos(Vec3::new(0.0, crate::course::BURIAL * radius_m, 0.0)),
            break_impulse_ns: f64::INFINITY,
        };
        let spec = mule(); // step 0.45 m
        for (radius, want) in [(1.0, Mobility::NoGo), (0.2, Mobility::Slow)] {
            let mut c = field(|_, _| 0.0, |_, _| DIRT);
            c.world.add_prop(rock(radius));
            assert_eq!(class_at(&classify(&c, &spec), 0.0, 0.0), want, "rock radius {radius} m");
        }
    }

    /// A 4 m deep, 6 m wide channel with water, crossed only by a deck strip 4 m wide at ground level.
    fn bridged(width_m: f64, load_limit_kg: f64) -> Course {
        let mut c = field(|x, z| if x.abs() < 3.0 && z.abs() >= 2.0 { -4.0 } else { 0.5 }, |_, _| DIRT);
        let w = &c.world;
        let surface: Vec<f32> = (0..N * N)
            .map(|k| {
                let (x, z) = w.node_xz(k % N, k / N);
                if w.height_m(x, z) < 0.0 {
                    0.0
                } else {
                    f32::NAN
                }
            })
            .collect();
        c.world.set_water(surface);
        c.bridges.push(PlacedBridge {
            kind: BridgeKind::Road,
            a: (-5.0, 0.0),
            b: (5.0, 0.0),
            deck_len_m: 6.0,
            width_m,
            deck_y_m: 0.5,
            load_limit_kg,
            rail_height_m: 0.0,
        });
        c
    }

    #[test]
    fn a_bridge_is_open_to_a_vehicle_that_fits_and_closed_when_it_is_too_heavy_or_too_wide() {
        let spec = mule(); // 2300 kg, 2.159 m: needs 2.159 + 0.6 = 2.76 m of deck
        let open = go(&bridged(4.0, 10_000.0), &spec, (-15.0, 0.0), (15.0, 0.0)).expect("crosses");
        assert!(open.uses.iter().any(|u| u.starts_with("bridge")));
        assert!(go(&bridged(4.0, 1_000.0), &spec, (-15.0, 0.0), (15.0, 0.0)).is_none(), "too heavy");
        assert!(go(&bridged(2.5, 10_000.0), &spec, (-15.0, 0.0), (15.0, 0.0)).is_none(), "too wide");
    }

    #[test]
    fn measured_limits_replace_the_placeholders_and_a_grade_scales_with_surface_friction() {
        let table = standard_material_table().expect("materials");
        let m = Measured { max_grade_ratio: Some(0.6), max_side_slope_rad: Some(0.5), max_step_m: None };
        let spec = mule().with_measured(&m, &table);
        let mu = |name: &str| table.materials[usize::from(table.id_of(name).expect(name).0)].mu_peak;
        let grade = |name: &str| spec.grade_on(table.id_of(name).expect(name));
        assert!((grade("asphalt") - 0.6).abs() < 1e-12, "asphalt takes the measured grade");
        assert!((grade("dirt") - 0.6 * mu("dirt") / mu("asphalt")).abs() < 1e-12, "dirt scales by the friction ratio");
        assert!(grade("mud") < grade("dirt"), "slipperier ground climbs less");
        assert_eq!(
            (spec.capability.max_side_slope_rad, spec.capability.max_step_m),
            (0.5, mule().capability.max_step_m)
        );
    }

    #[test]
    fn the_stand_in_specs_parse_and_name_only_materials_that_exist() {
        let table = standard_material_table().expect("materials");
        for text in [
            include_str!("../../../content/world/mobility/scout.ron"),
            MULE,
            include_str!("../../../content/world/mobility/hauler.ron"),
        ] {
            let s = MobilitySpec::from_ron(text).expect("spec");
            assert!(s.note.starts_with("PLACEHOLDER"), "{} is marked as a placeholder", s.name);
            let t = &s.capability;
            for id in t
                .max_grade
                .iter()
                .map(|p| p.0)
                .chain(t.top_speed_m_s.iter().map(|p| p.0))
                .chain(t.soil_go.iter().map(|p| p.0))
            {
                assert!(usize::from(id.0) < table.materials.len(), "{}: material {} exists", s.name, id.0);
            }
        }
    }
}
