//! Tank turret with its main gun, as one parametric family.
//!
//! The gun follows scaling laws (docs/design/04-parametric-components.md):
//! * shell mass grows with the cube of the calibre (geometric similarity);
//! * muzzle velocity grows with barrel length, with diminishing returns;
//! * gun mass grows with muzzle energy (a gun must contain the energy it releases);
//! * penetration follows the de Marre formula, P ~ v^1.43 m^0.71 / d^1.07;
//! * reload time is an action cycle plus handling time per kilogram of shell, which a mechanical loader cuts.
//!
//! The turret is then **sized to fit** what is inside it: crew, the breech and its recoil travel, the
//! ammunition and the loader. Armour thickness and slope shape the shell; more slope makes a squatter,
//! steeper turret with less room, so it must grow. Mass, armour and inertia are then measured as for any part.

use super::{stat, style, Family, Param, Role, Scale, Stat, Values};
use crate::convex::Convex;
use crate::geom::{v3, Plane, V3};
use crate::schema::{MaterialLibrary, Axis, Category, Function, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot, WeaponFn};
use crate::Built;

pub struct TankTurret;

/// Gun ballistics and mass from calibre, barrel length and loader.
#[derive(Clone, Copy, Debug)]
pub struct Gun {
    /// Bore (m).
    pub d: f64,
    pub shell_kg: f64,
    pub velocity: f64,
    pub energy_j: f64,
    pub gun_kg: f64,
    /// Volume of one complete round (projectile and propellant), m^3.
    pub round_m3: f64,
    pub barrels: u32,
    /// Time between shots (s).
    pub reload_s: f64,
    /// Recoil impulse (N s), projectile plus propellant gases.
    pub recoil_ns: f64,
}

pub fn gun(cal_mm: f64, barrel_cal: f64, loader: f64) -> Gun {
    let d = cal_mm / 1000.0;
    let shell_kg = 7.0 * (cal_mm / 75.0).powi(3);
    let velocity = 1200.0 * (1.0 - (-barrel_cal / 35.0).exp());
    let energy_j = 0.5 * shell_kg * velocity * velocity;
    let rotary = cal_mm < 30.0 && loader > 0.8;
    let barrels = if rotary { 6 } else { 1 };
    let single_kg = 550.0 * (energy_j / 1e6).powf(0.9) + 1.5;
    // Handling time grows a little slower than shell mass (tackle and rammers help); a mechanical loader
    // handles about nine times faster but adds a carousel indexing cycle for anything bigger than a cannon.
    // The action cycle grows with calibre.
    let per_kg = (0.35f64.ln() + (0.04f64.ln() - 0.35f64.ln()) * loader).exp();
    let cycle = 0.06 * (cal_mm / 7.62).powf(1.1) * (3.0 - 2.0 * loader) + if cal_mm >= 30.0 { 1.2 * loader } else { 0.0 };
    Gun {
        d,
        shell_kg,
        velocity,
        energy_j,
        gun_kg: single_kg * if rotary { 3.5 } else { 1.0 },
        round_m3: 7.85 * d.powi(3),
        barrels,
        reload_s: (cycle + per_kg * shell_kg.powf(0.85)) / barrels as f64,
        recoil_ns: 1.3 * shell_kg * velocity,
    }
}

/// Volume of the loading mechanism: a belt feed for small guns, a carousel and rammer for big ones.
pub fn autoloader_m3(g: &Gun, cal_mm: f64, loader: f64) -> f64 {
    loader * (0.004 + 0.0004 * cal_mm + 25.0 * g.round_m3)
}

/// de Marre penetration (mm of steel at normal incidence), calibrated to 100 mm for a 7 kg, 75 mm shot at 895 m/s.
pub fn penetration_mm(g: &Gun, velocity: f64) -> f64 {
    100.0 * (velocity / 895.0).max(0.0).powf(1.43) * (g.shell_kg / 7.0).powf(0.71) * (0.075 / g.d).powf(1.07)
}

/// Velocity after `range` metres: drag slows light, thin shots fastest (the decay length grows with sectional
/// density m / d^2).
pub fn velocity_at(g: &Gun, range: f64) -> f64 {
    let decay = 4500.0 * (g.shell_kg / (g.d * g.d)) / (7.0 / (0.075 * 0.075));
    g.velocity * (-range / decay).exp()
}

/// Everything the geometry needs, solved from the slider values.
#[derive(Clone, Debug)]
pub struct Layout {
    pub gun: Gun,
    pub crew: u32,
    pub remote: bool,
    pub rounds: f64,
    pub t_front: f64,
    pub t_side: f64,
    pub theta_f: f64,
    pub theta_s: f64,
    pub w: f64,
    pub h: f64,
    pub lt: f64,
    pub y0: f64,
    pub ring_r: f64,
    pub v_req: f64,
    pub breech: [f64; 3],
    pub mantlet: [f64; 3],
    pub y_gun: f64,
}

const THETA_REAR: f64 = 6.0;

impl Layout {
    /// How cramped the turret is for a standing loader (0 = enough room, 1 = none). A loader stands in the
    /// turret basket, about 0.9 m below the ring, and needs about 1.7 m of headroom.
    pub fn cramped(&self) -> f64 {
        let standing = self.h - 2.0 * self.t_side + 0.9;
        ((1.7 - standing) / 1.7).clamp(0.0, 1.0)
    }

    /// Time between shots, including the cramped-turret penalty on hand loading.
    pub fn reload_s(&self, loader: f64) -> f64 {
        self.gun.reload_s * (1.0 + 1.5 * self.cramped() * (1.0 - loader))
    }

    /// Largest gun depression (degrees). The gun pivots on its trunnions at the front wall: as the barrel dips,
    /// the back of the breech (plus its recoil stroke) swings up by lever x sin(angle) until it meets the roof.
    pub fn depression_deg(&self) -> f64 {
        let headroom = (self.y0 + self.h - self.t_side) - (self.y_gun + self.breech[1] / 2.0);
        let lever = self.breech[2] + 5.0 * self.gun.d;
        (headroom.max(0.0) / lever.max(1e-3)).min(1.0).asin().to_degrees().min(20.0)
    }

    fn lf(&self) -> f64 {
        self.lt * 0.45
    }
    fn lr(&self) -> f64 {
        self.lt * 0.55
    }
    /// Outer front face position at height y.
    fn z_face(&self, y: f64) -> f64 {
        -self.lf() + (y - self.y0) * self.theta_f.tan()
    }

    fn body_points(&self) -> Vec<V3> {
        let (w, h, y0) = (self.w, self.h, self.y0);
        let top_hw = w / 2.0 - h * self.theta_s.tan();
        let zf = -self.lf() + h * self.theta_f.tan();
        let zr = self.lr() - h * THETA_REAR.to_radians().tan();
        let mut p = Vec::new();
        for sx in [-1.0, 1.0] {
            p.push(v3(sx * w / 2.0, y0, -self.lf()));
            p.push(v3(sx * w / 2.0, y0, self.lr()));
            p.push(v3(sx * top_hw, y0 + h, zf));
            p.push(v3(sx * top_hw, y0 + h, zr));
        }
        p
    }

    /// Extra front armour as a solid slab just behind the outer front wall (invisible, but rays cross it).
    fn slab_points(&self) -> Option<Vec<V3>> {
        let extra = self.t_front - self.t_side;
        if extra < 0.001 {
            return None;
        }
        let (y_lo, y_hi) = (self.y0 + self.t_side + 0.003, self.y0 + self.h - self.t_side - 0.003);
        let mut p = Vec::new();
        for y in [y_lo, y_hi] {
            let hw = self.w / 2.0 - (y - self.y0) * self.theta_s.tan() - self.t_side / self.theta_s.cos() - 0.003;
            // Starts 2 mm inside the wall so the two overlap rather than merely touch.
            for depth in [self.t_side - 0.002, self.t_front] {
                let z = self.z_face(y) + depth / self.theta_f.cos();
                p.push(v3(-hw, y, z));
                p.push(v3(hw, y, z));
            }
        }
        Some(p)
    }

    /// Free interior volume of the body (inside the side walls, minus the front slab), and whether the slab fits.
    fn interior(&self) -> (f64, bool) {
        let outer = Convex::from_points(&self.body_points());
        let inner = Convex { planes: outer.planes.iter().map(|p| Plane { n: p.n, d: p.d - self.t_side }).collect(), kinds: outer.kinds.clone() };
        let (vol, _) = inner.polyhedron().volume_centroid();
        let Some(slab) = self.slab_points() else { return (vol, true) };
        let (sv, _) = Convex::from_points(&slab).polyhedron().volume_centroid();
        // The slab must end before the rear wall at the roof, where the body is shortest.
        let y_hi = self.y0 + self.h - self.t_side;
        let slab_rear = self.z_face(y_hi) + self.t_front / self.theta_f.cos();
        let rear_inner = self.lr() - self.h * THETA_REAR.to_radians().tan() - self.t_side - 0.02;
        (vol - sv, slab_rear < rear_inner)
    }
}

fn get(v: &Values, k: &str) -> f64 {
    v[k]
}

pub fn layout(v: &Values) -> Layout {
    let cal = get(v, "calibre_mm");
    let loader = get(v, "loader");
    let slope = get(v, "slope");
    let profile = v.get("profile").copied().unwrap_or(0.5);
    let g = gun(cal, get(v, "barrel_cal"), loader);
    let rounds = get(v, "ammo").round().max(1.0);
    let remote = cal < 25.0 && loader >= 0.5;
    let loaders = if remote || loader >= 0.5 { 0 } else { (1.0 + (g.shell_kg / 40.0).floor()).min(6.0) as u32 };
    let crew = if remote { 0 } else { 2 + loaders };
    let t_front = get(v, "armour_mm") / 1000.0;
    let t_side = (0.35 * t_front).max(0.004);
    let theta_f = (10.0 + 55.0 * slope).to_radians();
    let theta_s = (4.0 + 26.0 * slope).to_radians();

    // What must fit inside.
    let barrel_v = std::f64::consts::PI / 3.0 * get(v, "barrel_cal") * g.d * {
        let (rm, rb) = (0.65 * g.d + 0.004, 0.95 * g.d + 0.006);
        rm * rm + rm * rb + rb * rb
    } * g.barrels as f64;
    let breech_v = (g.gun_kg / 7850.0 - barrel_v).max(0.2 * g.gun_kg / 7850.0);
    let a = (breech_v / 2.16).cbrt();
    let breech = [1.2 * a, 1.0 * a, 1.8 * a];
    let ammo_v = rounds * g.round_m3 * 1.4;
    let auto_v = autoloader_m3(&g, cal, loader);
    let v_req = crew as f64 * 0.75 + 3.0 * breech_v + ammo_v + auto_v + 0.01;
    let mantlet = [(4.2 * g.d).max(0.12), (3.2 * g.d).max(0.1), t_front.max(0.02)];

    let h_min = (mantlet[1] + 2.0 * t_side + 0.04).max(breech[1] + 2.0 * t_side + 0.04).max(if crew > 0 { 0.65 } else { 0.0 });
    let lt_ratio = 1.15 + 0.8 * (ammo_v + auto_v) / v_req;
    let lt_min = breech[2] + 5.0 * g.d + 0.1 + t_front;
    let w_min = (mantlet[0] + 2.0 * t_side + 0.1).max(breech[0] + 2.0 * t_side + 0.1).max(if crew > 0 { 1.3 } else { 0.25 }).max(lt_min / lt_ratio);
    let make = |w: f64| {
        // Profile sets the height; slope and heavy armour both pull it lower.
        let h = (w * (0.3 + 0.4 * profile - 0.2 * slope - 0.08 * (t_front / 0.25).min(1.0))).max(h_min);
        let ring_h = 0.05 + 0.03 * w;
        let mut l = Layout {
            gun: g,
            crew,
            remote,
            rounds,
            t_front,
            t_side,
            theta_f,
            theta_s,
            w,
            h,
            lt: w * lt_ratio,
            y0: ring_h,
            ring_r: (0.36 * w).max(0.12),
            v_req,
            breech,
            mantlet,
            y_gun: 0.0,
        };
        l.y_gun = l.y0 + (0.45 * h).clamp(mantlet[1] / 2.0 + t_side, (h - mantlet[1] / 2.0 - t_side).max(mantlet[1] / 2.0 + t_side));
        l
    };
    let fits = |w: f64| {
        let l = make(w);
        // The roof must keep some width and length after the slopes take their share.
        let roof_w = l.w / 2.0 - l.h * l.theta_s.tan();
        let roof_l = (l.lr() - l.h * THETA_REAR.to_radians().tan()) - l.z_face(l.y0 + l.h);
        if roof_w < 0.1 * l.w || roof_l < 0.15 * l.lt {
            return false;
        }
        let (vol, slab_ok) = l.interior();
        slab_ok && vol >= v_req
    };
    // Smallest width that holds everything (feasibility grows with width).
    let mut hi = w_min.max(0.3);
    while !fits(hi) && hi < 200.0 {
        hi *= 1.5;
    }
    let mut lo = w_min;
    if fits(lo) {
        hi = lo;
    }
    for _ in 0..40 {
        if hi - lo < 1e-4 * hi {
            break;
        }
        let m = 0.5 * (lo + hi);
        if fits(m) {
            hi = m;
        } else {
            lo = m;
        }
    }
    make(hi)
}

fn top_hw_of(l: &Layout) -> f64 {
    l.w / 2.0 - l.h * l.theta_s.tan()
}

/// The turret body's bounding box (centre and half-diagonal), for framing previews on the turret rather than
/// on a long barrel.
pub fn body_frame(v: &Values) -> (V3, f64, f64) {
    let l = layout(v);
    let centre = v3(0.0, (l.y0 + l.h) / 2.0, (l.lr() - l.lf()) / 2.0);
    let r = 0.5 * (l.w * l.w + (l.y0 + l.h).powi(2) + l.lt * l.lt).sqrt();
    (centre, r, 0.0)
}

// --- small node builders --------------------------------------------------------------------------------

fn hull(points: Vec<V3>, mat: &str, slot: Slot, shell: Option<f64>, chamfer: f64) -> Node {
    Node::Hull { points: points.iter().map(|p| p.arr()).collect(), at: [0.0; 3], rot: [0.0; 3], mat: mat.into(), slot, shell, chamfer }
}

fn bx(size: [f64; 3], at: V3, mat: &str, slot: Slot, chamfer: f64) -> Node {
    Node::Box { size, taper: None, shift: None, at: at.arr(), rot: [0.0; 3], mat: mat.into(), slot, shell: None, chamfer }
}

#[allow(clippy::too_many_arguments)]
fn cyl(radius: f64, length: f64, axis: Axis, segments: u32, taper: f64, at: V3, mat: &str, slot: Slot, shell: Option<f64>, chamfer: f64) -> Node {
    Node::Cylinder { radius, length, axis, segments, taper, at: at.arr(), rot: [0.0; 3], mat: mat.into(), slot, shell, chamfer }
}

fn segs(r: f64) -> u32 {
    if r > 0.15 {
        16
    } else if r > 0.04 {
        12
    } else {
        8
    }
}

impl Family for TankTurret {
    fn id(&self) -> &'static str {
        "turret_gun"
    }

    fn name(&self) -> &'static str {
        "Gun turret"
    }

    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::TurretRing]
    }

    fn params(&self) -> Vec<Param> {
        vec![
            Param {
                id: "calibre_mm",
                name: "Calibre",
                unit: "mm",
                min: 7.62,
                max: 406.0,
                default: 75.0,
                scale: Scale::Log,
                role: Role::Budgeted,
                help: "Bore of the gun. Shell mass grows with its cube: from a rifle round to a battleship shell.",
            },
            Param {
                id: "barrel_cal",
                name: "Barrel length",
                unit: "calibres",
                min: 15.0,
                max: 70.0,
                default: 45.0,
                scale: Scale::Linear,
                role: Role::Budgeted,
                help: "Longer barrels give more velocity and penetration, with diminishing returns, and weigh more.",
            },
            Param {
                id: "armour_mm",
                name: "Front armour",
                unit: "mm",
                min: 5.0,
                max: 250.0,
                default: 60.0,
                scale: Scale::Linear,
                role: Role::Budgeted,
                help: "Front plate thickness; sides get about a third. Effective protection also depends on slope.",
            },
            Param {
                id: "slope",
                name: "Slope",
                unit: "",
                min: 0.0,
                max: 1.0,
                default: 0.4,
                scale: Scale::Linear,
                role: Role::Free,
                help: "Steeper, squatter plates resist more per mm but leave less room inside, so the turret grows.",
            },
            Param {
                id: "profile",
                name: "Profile",
                unit: "",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                scale: Scale::Linear,
                role: Role::Free,
                help: "Low turrets are smaller targets but cramped (slower hand loading) and the gun cannot dip as far.",
            },
            Param {
                id: "loader",
                name: "Loader",
                unit: "",
                min: 0.0,
                max: 1.0,
                default: 0.0,
                scale: Scale::Linear,
                role: Role::Budgeted,
                help: "0 = crew loads by hand; 1 = mechanical autoloader (small guns become rotary). Costs room and mass.",
            },
            Param {
                id: "ammo",
                name: "Ammunition",
                unit: "rounds",
                min: 4.0,
                max: 4000.0,
                default: 40.0,
                scale: Scale::Log,
                role: Role::Budgeted,
                help: "Rounds carried in the turret. Each round's volume grows with the cube of the calibre.",
            },
        ]
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let l = layout(v);
        let g = l.gun;
        let mut shapes = Vec::new();
        // Ring bearing.
        shapes.push(cyl(l.ring_r, l.y0, Axis::Y, segs(l.ring_r), 1.0, v3(0.0, l.y0 / 2.0, 0.0), "machinery", Slot::Dark, None, 0.0));
        // Body (side-thickness shell) and the extra front armour behind its front wall.
        // Chamfers grow with plate thickness, so heavy armour reads as heavy.
        let chamfer = (0.5 * l.t_side + 0.012 * l.w).min(0.1 * l.w.min(l.h));
        shapes.push(hull(l.body_points(), "steel", Slot::Primary, Some(l.t_side), chamfer));
        if let Some(slab) = l.slab_points() {
            shapes.push(hull(slab, "hard_steel", Slot::Primary, None, 0.0));
        }
        // Mantlet in front of the face, at gun height.
        let zf = l.z_face(l.y_gun);
        let [mw, mh, mt] = l.mantlet;
        shapes.push(bx([mw, mh, mt], v3(0.0, l.y_gun, zf - mt / 2.0 + 0.01), "hard_steel", Slot::Secondary, (0.15 * mh).min(0.05)));
        // Breech inside the turret.
        let [bw, bh, bl] = l.breech;
        let z_breech = zf + l.t_front / l.theta_f.cos() + bl / 2.0 + 0.005;
        shapes.push(bx([bw, bh, bl], v3(0.0, l.y_gun, z_breech), "gun_steel", Slot::Dark, 0.0));
        // Autoloader mechanism along the floor at the rear: hidden inside the body, but it has volume and mass.
        let auto_v = autoloader_m3(&g, v["calibre_mm"], v["loader"]);
        if auto_v > 1e-5 {
            let inner_h = l.h - 2.0 * l.t_side - 0.01;
            let hgt = (0.45 * inner_h).max(0.005);
            let wid = (0.8 * (top_hw_of(&l) - l.t_side / l.theta_s.cos()) * 2.0).max(0.005);
            let len = (auto_v / (hgt * wid)).min(0.45 * l.lt);
            let z_back = l.lr() - l.t_side - 0.01 - (l.y0 + l.t_side + hgt) * 0.0;
            shapes.push(bx([wid, hgt, len], v3(0.0, l.y0 + l.t_side + hgt / 2.0 + 0.003, z_back - len / 2.0), "machinery", Slot::Dark, 0.0));
        }
        // Barrel(s): radius 0.65 d (+wall) at the muzzle, 0.95 d at the breech end.
        let z_m = zf - mt;
        let len = v["barrel_cal"] * g.d;
        let (rm, rb) = (0.65 * g.d + 0.004, 0.95 * g.d + 0.006);
        if g.barrels > 1 {
            let rc = 1.5 * g.d + 0.004;
            let br = 0.55 * g.d + 0.002;
            shapes.push(Node::Group {
                at: [0.0, l.y_gun, 0.0],
                rot: [0.0; 3],
                scale: 1.0,
                children: vec![
                    Node::Radial {
                        count: g.barrels,
                        axis: Axis::Z,
                        phase: 0.0,
                        children: vec![cyl(br, len, Axis::Z, 6, 1.0, v3(0.0, rc, z_m - len / 2.0), "gun_steel", Slot::Dark, None, 0.0)],
                    },
                    cyl(rc + br + 0.004, 3.0 * g.d + 0.01, Axis::Z, 10, 1.0, v3(0.0, 0.0, z_m - 0.7 * len), "gun_steel", Slot::Metal, None, 0.0),
                    cyl(rc + br + 0.004, 2.0 * g.d + 0.01, Axis::Z, 10, 1.0, v3(0.0, 0.0, z_m - len + g.d + 0.01), "gun_steel", Slot::Metal, None, 0.0),
                ],
            });
        } else {
            shapes.push(cyl(rm, len, Axis::Z, segs(rb), rb / rm, v3(0.0, l.y_gun, z_m - len / 2.0), "gun_steel", Slot::Primary, None, 0.0));
            if (0.04..=0.2).contains(&g.d) {
                let r = 1.3 * (rm + (rb - rm) * 0.45);
                shapes.push(cyl(r, 6.0 * g.d, Axis::Z, segs(r), 1.0, v3(0.0, l.y_gun, z_m - 0.55 * len), "gun_steel", Slot::Secondary, None, 0.15 * g.d));
            }
            if g.energy_j > 0.8e6 {
                shapes.push(bx([2.7 * g.d, 2.1 * g.d, 3.2 * g.d], v3(0.0, l.y_gun, z_m - len + 1.2 * g.d), "gun_steel", Slot::Dark, 0.25 * g.d));
            }
        }
        // Livery (the shared design language): a two-tone roof, darker lower cheeks with a trim line, and one
        // glowing sensor eye on the front face.
        {
            let b = l.body_points();
            let inside = v3(0.0, l.y0 + l.h / 2.0, (l.lr() - l.lf()) / 2.0);
            let t = 0.008 + 0.005 * l.w;
            let roof_q: style::Quad = [b[2], b[6], b[7], b[3]];
            shapes.push(style::plate(&roof_q, inside, (0.0, 1.0), (0.64, 1.0), t, t, "fittings", Slot::Secondary));
            shapes.push(style::plate(&roof_q, inside, (0.0, 1.0), (0.56, 0.6), t, 0.0, "fittings", Slot::Trim));
            for q in [[b[4], b[5], b[7], b[6]], [b[1], b[0], b[2], b[3]]] {
                shapes.push(style::plate(&q, inside, (0.0, 1.0), (0.0, 0.42), t, t, "fittings", Slot::Secondary));
                shapes.push(style::plate(&q, inside, (0.04, 0.55), (0.46, 0.53), t, 0.0, "fittings", Slot::Trim));
            }
            let front_q: style::Quad = [b[0], b[4], b[6], b[2]];
            style::eye(style::at(&front_q, 0.8, 0.72), style::normal(&front_q, inside), (0.03 + 0.012 * l.w).min(0.2), &mut shapes);
        }
        // Crew fittings (human-sized, so they read the turret's scale) or a sensor head for a remote station.
        let top_hw = l.w / 2.0 - l.h * l.theta_s.tan();
        let (zf_top, zr_top) = (l.z_face(l.y0 + l.h), l.lr() - l.h * THETA_REAR.to_radians().tan());
        let roof = l.y0 + l.h;
        if l.crew > 0 {
            let cx = (0.28 * l.w).min(top_hw - 0.32).max(0.0);
            let cz = (0.5 * (zf_top + zr_top) + 0.15).clamp(zf_top + 0.3, (zr_top - 0.3).max(zf_top + 0.3));
            shapes.push(cyl(0.28, 0.22, Axis::Y, 10, 0.95, v3(cx, roof + 0.09, cz), "steel", Slot::Secondary, Some(0.02), 0.02));
            shapes.push(cyl(0.24, 0.04, Axis::Y, 10, 1.0, v3(cx, roof + 0.21, cz), "fittings", Slot::Secondary, None, 0.01));
            if l.crew > 2 && top_hw > 0.6 {
                shapes.push(bx([0.5, 0.04, 0.5], v3(-cx.max(0.35), roof + 0.02, cz), "fittings", Slot::Secondary, 0.015));
            }
        } else {
            let s = 0.08 + 0.4 * g.d;
            shapes.push(bx([s * 1.3, s, s * 1.2], v3((0.55 * top_hw).max(0.0), roof + s / 2.0, zf_top + 0.6 * s), "electronics", Slot::Secondary, 0.1 * s));
            shapes.push(bx([s * 0.8, s * 0.5, 0.01], v3((0.55 * top_hw).max(0.0), roof + s / 2.0, zf_top), "glass", Slot::Glass, 0.0));
        }
        PartDef {
            id: format!("turret_gun_{:.0}mm", v["calibre_mm"]),
            name: format!("{:.0} mm gun turret", v["calibre_mm"]),
            category: Category::Turret,
            size: match l.w {
                w if w < 0.8 => SizeClass::Small,
                w if w < 3.0 => SizeClass::Medium,
                w if w < 6.0 => SizeClass::Large,
                _ => SizeClass::Huge,
            },
            tags: vec!["turret".into(), "kinetic".into()],
            palette: None,
            voxels: Some(96),
            // Crewed or not, the inside holds the breech and the loading gear: reaching it disables the turret.
            vital: Some(true),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [0.0; 3],
                normal: [0.0, -1.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                draw_kw: 0.5 + 2.0 * l.w * l.w,
                ring_m: 2.0 * l.ring_r,
                weapon: Some(WeaponFn {
                    kind: "gun".into(),
                    energy_j: g.energy_j,
                    shots_per_min: 60.0 / l.reload_s(v["loader"]),
                    penetration_mm: penetration_mm(&g, velocity_at(&g, 1000.0)),
                    // Effective range: where drag has taken a third off the muzzle velocity.
                    range_m: (0.41 * 4500.0 * (g.shell_kg / (g.d * g.d)) / (7.0 / (0.075 * 0.075))).clamp(300.0, 20000.0),
                    recoil_ns: g.recoil_ns,
                    ..Default::default()
                }),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let l = layout(v);
        let g = l.gun;
        let m = &built.mass;
        let c = m.centre_of_mass;
        // Traverse: bang-bang rotation (accelerate half way, brake half way) under a ring-drive torque that
        // grows with ring area, against the measured moment of inertia about the ring axis.
        let i_ring = m.inertia[1][1] + m.mass_kg * (c[0] * c[0] + c[2] * c[2]);
        let torque = 2500.0 * (2.0 * l.ring_r).powi(2);
        let t180 = 2.0 * (std::f64::consts::PI * i_ring / torque).sqrt();
        vec![
            stat("shell mass", g.shell_kg, "kg"),
            stat("muzzle velocity", g.velocity, "m/s"),
            stat("muzzle energy", g.energy_j / 1e6, "MJ"),
            stat("penetration at muzzle", penetration_mm(&g, g.velocity), "mm"),
            stat("penetration at 1 km", penetration_mm(&g, velocity_at(&g, 1000.0)), "mm"),
            stat("reload", l.reload_s(v["loader"]), "s"),
            stat("rate of fire", 60.0 / l.reload_s(v["loader"]), "rpm"),
            stat("gun depression", l.depression_deg(), "deg"),
            stat("frontal area", built.armour.at(0.0, 0.0).area_m2, "m2"),
            stat("gun mass", g.gun_kg, "kg"),
            stat("turret mass", m.mass_kg / 1000.0, "t"),
            stat("front armour (effective)", built.armour.at(0.0, 0.0).median_mm, "mm"),
            stat("side armour (effective)", built.armour.at(90.0, 0.0).median_mm, "mm"),
            stat("ring diameter", 2.0 * l.ring_r, "m"),
            stat("traverse 180 deg", t180, "s"),
            stat("crew", l.crew as f64, ""),
            stat("rounds", l.rounds, ""),
            stat("recoil impulse", g.recoil_ns / 1000.0, "kN s"),
            stat("hull mass needed", g.recoil_ns / 0.5 / 1000.0, "t"),
        ]
    }
}
