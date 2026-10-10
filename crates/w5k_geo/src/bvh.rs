//! A bounding-volume hierarchy for any-hit ray queries (median split on the longest axis, ties broken by index: deterministic).

use crate::mesh::Mesh;
use w5k_math::Vec3;

const LEAF: usize = 4; // const-ok: triangles per leaf, a speed knob that does not change any result

struct Node {
    lo: Vec3,
    hi: Vec3,
    /// Leaf: first triangle and count. Inner: `first` is the left child, the right child is `first + 1`.
    first: u32,
    count: u32,
}

pub struct Bvh {
    nodes: Vec<Node>,
    tris: Vec<[Vec3; 3]>,
}

fn axis(v: Vec3, a: usize) -> f64 {
    [v.x, v.y, v.z][a]
}

impl Bvh {
    pub fn build(m: &Mesh) -> Bvh {
        let mut order: Vec<usize> = (0..m.t.len()).collect();
        let tris: Vec<[Vec3; 3]> = (0..m.t.len()).map(|k| m.tri(k)).collect();
        let mut b = Bvh { nodes: Vec::new(), tris: Vec::new() };
        b.nodes.push(Node { lo: Vec3::ZERO, hi: Vec3::ZERO, first: 0, count: 0 });
        b.split(0, &mut order, 0, m.t.len(), &tris);
        b.tris = order.iter().map(|&k| tris[k]).collect();
        b
    }

    fn split(&mut self, node: usize, order: &mut [usize], s: usize, e: usize, tris: &[[Vec3; 3]]) {
        let (mut lo, mut hi) = (Vec3::splat(f64::MAX), Vec3::splat(f64::MIN));
        for &k in &order[s..e] {
            for p in tris[k] {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        self.nodes[node].lo = lo;
        self.nodes[node].hi = hi;
        if e - s <= LEAF {
            self.nodes[node].first = s as u32;
            self.nodes[node].count = (e - s) as u32;
            return;
        }
        let ext = hi - lo;
        let a = if ext.x >= ext.y && ext.x >= ext.z {
            0
        } else if ext.y >= ext.z {
            1
        } else {
            2
        };
        let centre = |k: usize| axis(tris[k][0] + tris[k][1] + tris[k][2], a);
        order[s..e].sort_by(|&p, &q| centre(p).total_cmp(&centre(q)).then(p.cmp(&q)));
        let mid = (s + e) / 2;
        let left = self.nodes.len();
        for _ in 0..2 {
            self.nodes.push(Node { lo: Vec3::ZERO, hi: Vec3::ZERO, first: 0, count: 0 });
        }
        self.nodes[node].first = left as u32;
        self.split(left, order, s, mid, tris);
        self.split(left + 1, order, mid, e, tris);
    }

    /// Does the ray `o + t d`, 0 < t <= `t_max`, hit any triangle (either side)?
    pub fn any_hit(&self, o: Vec3, d: Vec3, t_max: f64) -> bool {
        let inv = Vec3::new(1.0 / d.x, 1.0 / d.y, 1.0 / d.z);
        let mut stack = [0u32; 64]; // const-ok: depth bound for 2^64 triangles
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let n = &self.nodes[stack[sp] as usize];
            let (t1, t2) = ((n.lo - o).mul_elem(inv), (n.hi - o).mul_elem(inv));
            let near = t1.min(t2);
            let far = t1.max(t2);
            if f64::max(f64::max(near.x, near.y), f64::max(near.z, 0.0))
                > f64::min(f64::min(far.x, far.y), f64::min(far.z, t_max))
            {
                continue;
            }
            if n.count == 0 {
                stack[sp] = n.first;
                stack[sp + 1] = n.first + 1;
                sp += 2;
            } else if self.tris[n.first as usize..(n.first + n.count) as usize].iter().any(|t| hits(t, o, d, t_max)) {
                return true;
            }
        }
        false
    }

    /// The same query without the hierarchy: the oracle for the tests.
    pub fn any_hit_brute(&self, o: Vec3, d: Vec3, t_max: f64) -> bool {
        self.tris.iter().any(|t| hits(t, o, d, t_max))
    }
}

/// Moller-Trumbore, two-sided.
fn hits(t: &[Vec3; 3], o: Vec3, d: Vec3, t_max: f64) -> bool {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        // const-ok: parallel-ray tolerance
        return false;
    }
    let s = o - t[0];
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = d.dot(q) / det;
    let at = e2.dot(q) / det;
    u >= 0.0 && v >= 0.0 && u + v <= 1.0 && at > 0.0 && at <= t_max
}
