//! Spike S6 (lane COMBAT): articulation coupling. A free-floating hull carrying turret > cradle > barrel (a copy of `box_tank()` with the
//! recoil mass fixed) on servos and a recoil mechanism built from the contract's `ServoDef` / `RecoilDef`. No gravity, suspension or ground.
//! Scheme A: reduced-coordinate solve of the whole 9-DOF system (hull v, w + three joints), M(q) from body Jacobians, RK4 or semi-implicit Euler.
//! Scheme B: joints solved against the hull's *previous* acceleration, the reaction wrench handed to the hull one substep late (the glue pattern).
//! Run: `cargo run --release` from this directory; prints the tables recorded in `output.txt`, writes SVG/PNG plots.
use w5k_contract::rig::*;
use w5k_contract::testing::rigs::box_tank;
use w5k_math::{scalar, Mat3, Quat, Vec3};

type V9 = [f64; 9];
const N: usize = 9;
const TICK: f64 = 1.0 / 60.0; // const-ok: the 60 Hz tick of CONTRACTS.md

#[derive(Clone)]
struct Body {
    m: f64,
    com: Vec3,
    inertia: Mat3,
    parent: usize,
    anchor: Vec3,
    axis: Vec3,
    prismatic: bool,
}

#[derive(Clone, Copy, Default)]
struct Kin {
    r: Mat3,
    c: Vec3,
    w: Vec3,
    alpha: Vec3,
    vc: Vec3,
    ac: Vec3,
}

#[derive(Clone)]
struct State {
    o: Vec3,
    rot: Quat,
    u: V9, // hull origin velocity (world), hull angular velocity (world), three joint rates
    q: [f64; 3],
}

#[derive(Clone)]
struct Recoil {
    def: RecoilDef,
    k_stop: f64,
    c_stop: f64,
}

fn propagate(bodies: &[Body], rot: Quat, o: Vec3, q: &[f64; 3], u: &V9, ud: &V9) -> Vec<Kin> {
    let mut out: Vec<Kin> = Vec::with_capacity(bodies.len());
    // Per body we also track the *origin* kinematics.
    let mut org: Vec<(Vec3, Vec3, Vec3)> = Vec::new(); // (origin pos, origin vel, origin acc)
    for (i, b) in bodies.iter().enumerate() {
        let (r, o_i, w, al, vo, ao);
        if i == 0 {
            r = rot.to_mat3();
            o_i = o;
            w = Vec3::new(u[3], u[4], u[5]);
            al = Vec3::new(ud[3], ud[4], ud[5]);
            vo = Vec3::new(u[0], u[1], u[2]);
            ao = Vec3::new(ud[0], ud[1], ud[2]);
        } else {
            let k = i - 1;
            let (qk, qd, qdd) = (q[k], u[6 + k], ud[6 + k]);
            let p = &out[b.parent];
            let (op, vop, aop) = org[b.parent];
            let rr = p.r.mul_vec(b.anchor);
            let a_w = p.r.mul_vec(b.axis);
            if b.prismatic {
                let d = rr + a_w * qk;
                r = p.r;
                o_i = op + d;
                w = p.w;
                al = p.alpha;
                vo = vop + p.w.cross(d) + a_w * qd;
                ao = aop + p.alpha.cross(d) + p.w.cross(p.w.cross(d)) + p.w.cross(a_w) * (2.0 * qd) + a_w * qdd;
            } else {
                r = p.r.mul_mat(&Quat::from_axis_angle(b.axis, qk).to_mat3());
                o_i = op + rr;
                w = p.w + a_w * qd;
                al = p.alpha + a_w * qdd + p.w.cross(a_w * qd);
                vo = vop + p.w.cross(rr);
                ao = aop + p.alpha.cross(rr) + p.w.cross(p.w.cross(rr));
            }
        }
        let rho = r.mul_vec(b.com);
        out.push(Kin {
            r,
            c: o_i + rho,
            w,
            alpha: al,
            vc: vo + w.cross(rho),
            ac: ao + al.cross(rho) + w.cross(w.cross(rho)),
        });
        org.push((o_i, vo, ao));
    }
    out
}

struct Dyn {
    m: Vec<Vec<f64>>,      // total mass matrix
    m_body: Vec<Vec<Vec<f64>>>, // per body
    bias: Vec<V9>,         // per body, generalized "- bias" contribution (already negative)
    kin: Vec<Kin>,         // velocities at current u (ud = 0 accelerations are the bias accelerations)
    jv: Vec<[Vec3; N]>,
    jw: Vec<[Vec3; N]>,
}

fn build(bodies: &[Body], st: &State) -> Dyn {
    let nb = bodies.len();
    let zero = [0.0; N];
    let kin = propagate(bodies, st.rot, st.o, &st.q, &st.u, &zero);
    let mut jv = vec![[Vec3::ZERO; N]; nb];
    let mut jw = vec![[Vec3::ZERO; N]; nb];
    for k in 0..N {
        let mut e = [0.0; N];
        e[k] = 1.0;
        let kk = propagate(bodies, st.rot, st.o, &st.q, &e, &zero);
        for i in 0..nb {
            jv[i][k] = kk[i].vc;
            jw[i][k] = kk[i].w;
        }
    }
    let mut m_body = vec![vec![vec![0.0; N]; N]; nb];
    let mut bias = vec![[0.0; N]; nb];
    let mut m = vec![vec![0.0; N]; N];
    for i in 0..nb {
        let iw = bodies[i].inertia.rotate_inertia(&kin[i].r);
        let gyro = kin[i].w.cross(iw.mul_vec(kin[i].w));
        let torque_bias = iw.mul_vec(kin[i].alpha) + gyro; // alpha here is the bias angular acceleration (ud = 0)
        let force_bias = kin[i].ac * bodies[i].m;
        for a in 0..N {
            bias[i][a] = -(jv[i][a].dot(force_bias) + jw[i][a].dot(torque_bias));
            let iwj = iw.mul_vec(jw[i][a]);
            for b in 0..N {
                let v = bodies[i].m * jv[i][a].dot(jv[i][b]) + jw[i][b].dot(iwj);
                m_body[i][a][b] = v;
                m[a][b] += v;
            }
        }
    }
    Dyn { m, m_body, bias, kin, jv, jw }
}

fn solve(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = (0..n).map(|i| a[i][..n].iter().copied().chain(std::iter::once(b[i])).collect()).collect();
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if m[r][c].abs() > m[p][c].abs() {
                p = r;
            }
        }
        m.swap(c, p);
        for r in c + 1..n {
            let f = m[r][c] / m[c][c];
            for k in c..=n {
                let t = m[c][k];
                m[r][k] -= f * t;
            }
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let mut s = m[r][n];
        for k in r + 1..n {
            s -= m[r][k] * x[k];
        }
        x[r] = s / m[r][r];
    }
    x
}

fn sub(m: &[Vec<f64>], r0: usize, r1: usize, c0: usize, c1: usize) -> Vec<Vec<f64>> {
    (r0..r1).map(|r| m[r][c0..c1].to_vec()).collect()
}

// ------------------------------------------------------------------------------------------------------------------------ servo

struct Servo {
    d: ServoDef,
    goal: f64,
    qc: f64,
    qcd: f64,
    qs: f64, // the stabiliser's own contribution to the commanded position (so the position loop does not fight it)
    step_cmd: bool, // limits off: the command is a pure step (the overshoot oracle)
    eff_delay: Vec<f64>,
    stab_delay: Vec<f64>,
    stab_s: f64,
    i_eff: f64,
    clamp_hits: u32,
}

impl Servo {
    fn new(d: &ServoDef, i_eff: f64, dt: f64) -> Servo {
        let n = (d.latency_s / dt).round() as usize;
        let ns = d.stabiliser.map(|s| (s.latency_s / dt).round() as usize).unwrap_or(0);
        Servo { d: d.clone(), goal: 0.0, qc: 0.0, qcd: 0.0, qs: 0.0, step_cmd: false, eff_delay: vec![0.0; n], stab_delay: vec![0.0; ns], stab_s: 0.0, i_eff, clamp_hits: 0 }
    }
    /// One substep of the control law; returns the effort that reaches the joint (after latency).
    fn effort(&mut self, q: f64, qd: f64, w_parent_axis: f64, dt: f64) -> f64 {
        let mut qcd_stab = 0.0;
        if let Some(s) = self.d.stabiliser {
            self.stab_delay.push(w_parent_axis);
            let sensed = if self.stab_delay.len() > 1 { self.stab_delay.remove(0) } else { w_parent_axis };
            let a = 1.0 - scalar::exp(-2.0 * core::f64::consts::PI * s.bandwidth_hz * dt);
            self.stab_s += a * (sensed - self.stab_s);
            qcd_stab = -s.rejection * self.stab_s;
        }
        if self.step_cmd {
            self.qc = self.goal;
            self.qcd = 0.0;
        } else {
            let acc = self.d.max_accel_si.min(self.d.max_effort_si / self.i_eff);
            let e = self.goal - self.qc;
            let vdes = scalar::sign(e) * self.d.max_rate_si.min(scalar::sqrt(2.0 * acc * e.abs()));
            let dv = scalar::clamp(vdes - self.qcd, -acc * dt, acc * dt);
            self.qcd += dv;
            self.qc += self.qcd * dt;
        }
        self.qs += qcd_stab * dt;
        let raw = self.d.kp_si * (self.qc + self.qs - q) + self.d.kd_si * (self.qcd + qcd_stab - qd);
        let u = scalar::clamp(raw, -self.d.max_effort_si, self.d.max_effort_si);
        if u != raw {
            self.clamp_hits += 1;
        }
        self.eff_delay.push(u);
        if self.eff_delay.len() > 1 {
            self.eff_delay.remove(0)
        } else {
            u
        }
    }
}

// ------------------------------------------------------------------------------------------------------------------------ rig

#[derive(Clone, Copy, PartialEq)]
enum Scheme {
    ARk4,
    AEuler,
    BEuler,
}

struct Sim {
    bodies: Vec<Body>,
    st: State,
    servos: [Option<Servo>; 3],
    recoil: Recoil,
    limits: [Option<(f64, f64)>; 3],
    muzzle_local: Vec3,
    scheme: Scheme,
    dt: f64,
    t: f64,
    ej_p: Vec3,
    ej_l: Vec3,
    ext_hull_torque: Vec3,
    // scheme B memory
    udh_prev: [f64; 6],
    r_pending: [f64; 6],
}

fn bodies_from(rig: &PhysRig, turret_scale: f64) -> Vec<Body> {
    let mut v = vec![Body { m: rig.hull.mass_kg, com: rig.hull.com_m, inertia: rig.hull.inertia_kg_m2, parent: usize::MAX, anchor: Vec3::ZERO, axis: Vec3::Y, prismatic: false }];
    for (i, j) in rig.articulation.iter().enumerate() {
        let s = if i == 0 { turret_scale } else { 1.0 };
        v.push(Body {
            m: j.body.mass_kg * s,
            com: j.body.com_m,
            inertia: j.body.inertia_kg_m2.scaled(s),
            parent: j.parent.map(|p| p + 1).unwrap_or(0),
            anchor: j.anchor_m,
            axis: j.axis,
            prismatic: j.kind == JointKind::Prismatic,
        });
    }
    v
}

impl Sim {
    fn new(rig: &PhysRig, bodies: Vec<Body>, scheme: Scheme, nsub: u32) -> Sim {
        let dt = TICK / nsub as f64;
        let rd = match &rig.articulation[2].drive {
            JointDrive::Recoil(r) => r.clone(),
            _ => panic!("joint 2 must be the recoil"),
        };
        let st = State { o: Vec3::ZERO, rot: Quat::IDENTITY, u: [0.0; N], q: [0.0; 3] };
        let mut s = Sim {
            bodies,
            st,
            servos: [None, None, None],
            recoil: Recoil { k_stop: 2.0e7, c_stop: 2.0 * 0.7 * scalar::sqrt(2.0e7 * 2000.0), def: rd },
            limits: [rig.articulation[0].limits, rig.articulation[1].limits, rig.articulation[2].limits],
            muzzle_local: rig.muzzles[0].pose.pos,
            scheme,
            dt,
            t: 0.0,
            ej_p: Vec3::ZERO,
            ej_l: Vec3::ZERO,
            ext_hull_torque: Vec3::ZERO,
            udh_prev: [0.0; 6],
            r_pending: [0.0; 6],
        };
        s.st.q[2] = -s.recoil.def.preload_n / s.recoil.k_stop;
        let d = build(&s.bodies, &s.st);
        for k in 0..2 {
            if let JointDrive::Servo(sd) = &rig.articulation[k].drive {
                s.servos[k] = Some(Servo::new(sd, d.m[6 + k][6 + k], dt));
            }
        }
        s
    }

    /// Passive joint forces: recoil spring/preload/buffer/stops, penalty limits on the servo joints (reaction goes to the parent via M).
    fn passive(&self, q: &[f64; 3], qd: &V9) -> [f64; 3] {
        let mut f = [0.0; 3];
        for k in 0..2 {
            if let Some((lo, hi)) = self.limits[k] {
                let (kl, cl) = (1.0e7, 2.0e5);
                if q[k] < lo {
                    f[k] += kl * (lo - q[k]) - cl * qd[6 + k].min(0.0);
                }
                if q[k] > hi {
                    f[k] -= kl * (q[k] - hi) + cl * qd[6 + k].max(0.0);
                }
            }
        }
        let r = &self.recoil.def;
        let rate = match &r.spring {
            SpringKind::Linear { rate_n_m } => *rate_n_m,
            _ => 0.0,
        };
        let (x, v) = (q[2], qd[8]);
        let mut fr = -(r.preload_n + rate * x.max(0.0)) - r.damper_ns_m * v - r.damper_quad_ns2_m2 * v * v.abs();
        if x < 0.0 {
            fr += self.recoil.k_stop * (-x) - self.recoil.c_stop * v.min(0.0);
        }
        if x > r.stroke_m {
            fr -= self.recoil.k_stop * (x - r.stroke_m) + self.recoil.c_stop * v.max(0.0);
        }
        f[2] = fr;
        f
    }

    fn rhs(&self, d: &Dyn, st: &State, servo_tau: &[f64; 3]) -> V9 {
        let mut r = [0.0; N];
        for i in 0..d.bias.len() {
            for a in 0..N {
                r[a] += d.bias[i][a];
            }
        }
        let p = self.passive(&st.q, &st.u);
        for k in 0..3 {
            r[6 + k] += servo_tau[k] + p[k];
        }
        r[3] += self.ext_hull_torque.x;
        r[4] += self.ext_hull_torque.y;
        r[5] += self.ext_hull_torque.z;
        r
    }

    fn deriv(&self, s: &[f64; 19], tau: &[f64; 3]) -> [f64; 19] {
        let st = unpack(s);
        let d = build(&self.bodies, &st);
        let r = self.rhs(&d, &st, tau);
        let ud = solve(&d.m, &r);
        let w = Quat::new(0.0, st.u[3], st.u[4], st.u[5]);
        let qd = w * st.rot;
        let mut o = [0.0; 19];
        o[0] = st.u[0];
        o[1] = st.u[1];
        o[2] = st.u[2];
        o[3] = 0.5 * qd.w;
        o[4] = 0.5 * qd.x;
        o[5] = 0.5 * qd.y;
        o[6] = 0.5 * qd.z;
        for i in 0..N {
            o[7 + i] = ud[i];
        }
        for k in 0..3 {
            o[16 + k] = st.u[6 + k];
        }
        o
    }

    fn servo_taus(&mut self) -> [f64; 3] {
        let mut tau = [0.0; 3];
        let d = build(&self.bodies, &self.st);
        let (dt, q, u) = (self.dt, self.st.q, self.st.u);
        for k in 0..2 {
            if self.servos[k].is_some() {
                let axis_w = d.kin[k + 1].r.mul_vec(self.bodies[k + 1].axis); // joint axis in world (child frame has the same axis)
                let wpar = d.kin[self.bodies[k + 1].parent].w.dot(axis_w);
                tau[k] = self.servos[k].as_mut().unwrap().effort(q[k], u[6 + k], wpar, dt);
            }
        }
        tau
    }

    fn step(&mut self) {
        let tau = self.servo_taus();
        let dt = self.dt;
        match self.scheme {
            Scheme::ARk4 => {
                let s0 = pack(&self.st);
                let k1 = self.deriv(&s0, &tau);
                let k2 = self.deriv(&axpy(&s0, &k1, 0.5 * dt), &tau);
                let k3 = self.deriv(&axpy(&s0, &k2, 0.5 * dt), &tau);
                let k4 = self.deriv(&axpy(&s0, &k3, dt), &tau);
                let mut s1 = s0;
                for i in 0..19 {
                    s1[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
                }
                self.st = unpack(&s1);
                self.st.rot = self.st.rot.normalized();
            }
            Scheme::AEuler => {
                let d = build(&self.bodies, &self.st);
                let r = self.rhs(&d, &self.st, &tau);
                let ud = solve(&d.m, &r);
                self.semi_implicit(&ud);
            }
            Scheme::BEuler => {
                let d = build(&self.bodies, &self.st);
                let r = self.rhs(&d, &self.st, &tau);
                // Joints against the hull's previous acceleration.
                let mqq = sub(&d.m, 6, 9, 6, 9);
                let mut b = vec![0.0; 3];
                for k in 0..3 {
                    b[k] = r[6 + k];
                    for j in 0..6 {
                        b[k] -= d.m[6 + k][j] * self.udh_prev[j];
                    }
                }
                let qdd = solve(&mqq, &b);
                // Reaction the chain asks of the hull (computed now, applied next substep).
                let mut rh_chain = [0.0; 6];
                for a in 0..6 {
                    let mut chain_bias = 0.0;
                    for i in 1..d.bias.len() {
                        chain_bias += d.bias[i][a];
                    }
                    let mut v = chain_bias;
                    for j in 0..6 {
                        v -= (d.m[a][j] - d.m_body[0][a][j]) * self.udh_prev[j];
                    }
                    for k in 0..3 {
                        v -= d.m[a][6 + k] * qdd[k];
                    }
                    rh_chain[a] = v;
                }
                let m0 = sub(&d.m_body[0], 0, 6, 0, 6);
                let mut bh = vec![0.0; 6];
                for a in 0..6 {
                    bh[a] = d.bias[0][a] + self.r_pending[a] + if a >= 3 { [self.ext_hull_torque.x, self.ext_hull_torque.y, self.ext_hull_torque.z][a - 3] } else { 0.0 };
                }
                let udh = solve(&m0, &bh);
                let mut ud = [0.0; N];
                ud[..6].copy_from_slice(&udh);
                ud[6..].copy_from_slice(&qdd);
                self.udh_prev.copy_from_slice(&udh);
                self.r_pending = rh_chain;
                self.semi_implicit(&ud);
            }
        }
        self.t += dt;
    }

    fn semi_implicit(&mut self, ud: &[f64]) {
        let dt = self.dt;
        for i in 0..N {
            self.st.u[i] += dt * ud[i];
        }
        self.st.o += Vec3::new(self.st.u[0], self.st.u[1], self.st.u[2]) * dt;
        self.st.rot = self.st.rot.integrate_world(Vec3::new(self.st.u[3], self.st.u[4], self.st.u[5]), dt);
        for k in 0..3 {
            self.st.q[k] += dt * self.st.u[6 + k];
        }
    }

    /// Fire: net impulse `j` on the gun along the bore, equal and opposite momentum to the ejecta (shell + gas), applied at the muzzle line.
    fn fire(&mut self, j: f64) {
        let d = build(&self.bodies, &self.st);
        let kb = d.kin[3];
        let bore = -kb.r.mul_vec(Vec3::Z);
        let pm = kb.c + kb.r.mul_vec(self.muzzle_local - self.bodies[3].com);
        let gun_imp = -bore * j;
        if self.scheme == Scheme::BEuler {
            // The glue's version: the barrel alone takes the impulse, the hull learns of it through the wrench a substep later.
            self.st.u[8] += gun_imp.dot(kb.r.mul_vec(Vec3::Z)) / self.bodies[3].m;
        } else {
            let arm = pm - kb.c;
            let mut g = vec![0.0; N];
            for a in 0..N {
                g[a] = d.jv[3][a].dot(gun_imp) + d.jw[3][a].dot(arm.cross(gun_imp));
            }
            let du = solve(&d.m, &g);
            for a in 0..N {
                self.st.u[a] += du[a];
            }
        }
        self.ej_p += bore * j;
        self.ej_l += pm.cross(bore * j);
    }

    /// Total linear and angular (about the world origin) momentum of hull + chain + ejecta.
    fn momentum(&self) -> (Vec3, Vec3) {
        let k = propagate(&self.bodies, self.st.rot, self.st.o, &self.st.q, &self.st.u, &[0.0; N]);
        let (mut p, mut l) = (self.ej_p, self.ej_l);
        for (i, b) in self.bodies.iter().enumerate() {
            let iw = b.inertia.rotate_inertia(&k[i].r);
            p += k[i].vc * b.m;
            l += k[i].c.cross(k[i].vc * b.m) + iw.mul_vec(k[i].w);
        }
        (p, l)
    }
    fn kin(&self) -> Vec<Kin> {
        propagate(&self.bodies, self.st.rot, self.st.o, &self.st.q, &self.st.u, &[0.0; N])
    }
}

fn pack(s: &State) -> [f64; 19] {
    let mut o = [0.0; 19];
    o[0] = s.o.x;
    o[1] = s.o.y;
    o[2] = s.o.z;
    o[3] = s.rot.w;
    o[4] = s.rot.x;
    o[5] = s.rot.y;
    o[6] = s.rot.z;
    o[7..16].copy_from_slice(&s.u);
    o[16..19].copy_from_slice(&s.q);
    o
}
fn unpack(a: &[f64; 19]) -> State {
    let mut u = [0.0; N];
    u.copy_from_slice(&a[7..16]);
    State { o: Vec3::new(a[0], a[1], a[2]), rot: Quat::new(a[3], a[4], a[5], a[6]), u, q: [a[16], a[17], a[18]] }
}
fn axpy(a: &[f64; 19], b: &[f64; 19], h: f64) -> [f64; 19] {
    let mut o = *a;
    for i in 0..19 {
        o[i] += h * b[i];
    }
    o
}

// ------------------------------------------------------------------------------------------------------------------------ plots

fn svg(path: &str, title: &str, xl: &str, yl: &str, series: &[(&str, &str, Vec<(f64, f64)>)]) {
    let (w, h, l, r, t, b) = (760.0, 420.0, 80.0, 20.0, 40.0, 50.0);
    let (mut x0, mut x1, mut y0, mut y1) = (1e300, -1e300, 1e300, -1e300);
    for (_, _, s) in series {
        for &(x, y) in s {
            x0 = f64::min(x0, x);
            x1 = f64::max(x1, x);
            y0 = f64::min(y0, y);
            y1 = f64::max(y1, y);
        }
    }
    if y1 - y0 < 1e-12 {
        y1 = y0 + 1.0;
    }
    let pad = 0.05 * (y1 - y0);
    let (y0, y1) = (y0 - pad, y1 + pad);
    let sx = |x: f64| l + (x - x0) / (x1 - x0) * (w - l - r);
    let sy = |y: f64| h - b - (y - y0) / (y1 - y0) * (h - t - b);
    let mut o = format!("<svg xmlns='http://www.w3.org/2000/svg' style='display:block' width='{w}' height='{h}' font-family='sans-serif' font-size='12'><rect width='100%' height='100%' fill='white'/>");
    o += &format!("<text x='{l}' y='24' font-size='15' font-weight='bold'>{title}</text>");
    for i in 0..=4 {
        let y = y0 + (y1 - y0) * i as f64 / 4.0;
        o += &format!("<line x1='{l}' x2='{}' y1='{}' y2='{}' stroke='#ddd'/><text x='{}' y='{}' text-anchor='end'>{:.3e}</text>", w - r, sy(y), sy(y), l - 6.0, sy(y) + 4.0, y);
        let x = x0 + (x1 - x0) * i as f64 / 4.0;
        o += &format!("<text x='{}' y='{}' text-anchor='middle'>{:.2}</text>", sx(x), h - b + 18.0, x);
    }
    o += &format!("<text x='{}' y='{}' text-anchor='middle'>{xl}</text><text x='14' y='{}' transform='rotate(-90 14 {})' text-anchor='middle'>{yl}</text>", (l + w - r) / 2.0, h - 8.0, h / 2.0, h / 2.0);
    for (i, (name, col, s)) in series.iter().enumerate() {
        let pts: Vec<String> = s.iter().map(|&(x, y)| format!("{:.1},{:.1}", sx(x), sy(y))).collect();
        o += &format!("<polyline fill='none' stroke='{col}' stroke-width='1.8' points='{}'/>", pts.join(" "));
        o += &format!("<rect x='{}' y='{}' width='14' height='3' fill='{col}'/><text x='{}' y='{}'>{name}</text>", l + 10.0, 38.0 + 15.0 * i as f64, l + 30.0, 43.0 + 15.0 * i as f64);
    }
    o += "</svg>";
    std::fs::write(path, o).unwrap();
}

// ------------------------------------------------------------------------------------------------------------------------ cases

fn tank() -> PhysRig {
    let mut rig = box_tank().0;
    // Fix the copy (CCR input, see the finding): the recoiling mass of a 120 mm is about 2 t (tube, breech, mantlet parts), not 1.2 t, so a
    // J = 19.8 kN s shot gives 9.9 m/s, as the red-team measured for real guns (10 m/s). Cradle drops to 2 t so the chain total stays 16 t.
    rig.articulation[1].body.mass_kg = 2_000.0;
    rig.articulation[2].body.mass_kg = 2_000.0;
    rig.articulation[1].body.inertia_kg_m2 = box_inertia_pub(2_000.0, Vec3::new(0.5, 0.5, 1.5));
    rig.articulation[2].body.inertia_kg_m2 = box_inertia_pub(2_000.0, Vec3::new(0.15, 0.15, 5.0));
    // A metered buffer: constant force about 330 kN over 0.30 m stops a 2 t slide at 9.9 m/s (ideal F = m v^2 / 2 s), kept by a linear plus
    // quadratic term (F = c v + q v|v|); recuperator returns it to battery. Stroke 0.35 m keeps 0.05 m in hand. ESTIMATE, UNVALIDATED.
    if let JointDrive::Recoil(r) = &mut rig.articulation[2].drive {
        r.damper_ns_m = 36_000.0;
        r.damper_quad_ns2_m2 = 1_500.0;
        r.spring = SpringKind::Linear { rate_n_m: 150_000.0 };
    }
    rig
}

fn box_inertia_pub(m: f64, size: Vec3) -> Mat3 {
    let k = m / 12.0;
    Mat3::diagonal(k * (size.y * size.y + size.z * size.z), k * (size.x * size.x + size.z * size.z), k * (size.x * size.x + size.y * size.y))
}

fn tank_with_turret_scale(rig: &PhysRig, s: f64) -> Vec<Body> {
    bodies_from(rig, s)
}

fn main() {
    let rig = tank();
    let j = rig.muzzles[0].weapon.recoil_impulse_factor * rig.muzzles[0].weapon.projectile_mass_kg * rig.muzzles[0].weapon.muzzle_velocity_m_s;
    println!("S6: impulse J = {j:.1} N s (factor {} x {} kg x {} m/s)", rig.muzzles[0].weapon.recoil_impulse_factor, rig.muzzles[0].weapon.projectile_mass_kg, rig.muzzles[0].weapon.muzzle_velocity_m_s);
    let out = "../../../docs/lanes/combat/media";

    // ---- 0. gains: damping ratio and inertia seen by each servo ------------------------------------------------------------------
    println!("\n== servo inertia and damping ratio at the box_tank gains (nominal turret mass) ==");
    let sim0 = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), Scheme::ARk4, 6);
    let d0 = build(&sim0.bodies, &sim0.st);
    let minv_diag = |k: usize| {
        let mut e = vec![0.0; N];
        e[6 + k] = 1.0;
        solve(&d0.m, &e)[6 + k]
    };
    for k in 0..2 {
        let sd = sim0.servos[k].as_ref().unwrap();
        let (ih, ifree) = (d0.m[6 + k][6 + k], 1.0 / minv_diag(k));
        let z = |i: f64| sd.d.kd_si / (2.0 * scalar::sqrt(sd.d.kp_si * i));
        println!("joint {k}: I_held={ih:.0} I_free-hull={ifree:.0} kg m^2; zeta(held)={:.3} zeta(free)={:.3}; wn(held)={:.3} rad/s; alpha_max=min({}, {:.3})", z(ih), z(ifree), scalar::sqrt(sd.d.kp_si / ih), sd.d.max_accel_si, sd.d.max_effort_si / ih);
    }

    // ---- 1. swivel chair ---------------------------------------------------------------------------------------------------------
    println!("\n== 1. swivel chair: coaxial turret slew on a free hull ==");
    {
        let mut b = bodies_from(&rig, 1.0);
        // Coaxial: every COM on the yaw axis (x=0, z=0.2 in the hull frame), the cradle pivot and barrel pivot on the axis.
        b[1].com = Vec3::new(0.0, 0.5, 0.0);
        b[2].anchor = Vec3::new(0.0, 0.35, 0.0);
        b[2].com = Vec3::ZERO;
        b[3].anchor = Vec3::ZERO;
        b[3].com = Vec3::ZERO;
        b[0].com = Vec3::new(0.0, -0.15, 0.2);
        let (ih, it) = (b[0].inertia.m[1][1], b[1].inertia.m[1][1] + b[2].inertia.m[1][1] + b[3].inertia.m[1][1]);
        let mut s = Sim::new(&rig, b, Scheme::ARk4, 6);
        s.servos[0].as_mut().unwrap().goal = 1.2;
        s.servos[0].as_mut().unwrap().d.stabiliser = None;
        let mut ser = vec![];
        let mut worst = 0.0f64;
        for n in 0..(14.0 / s.dt) as usize {
            s.step();
            let hull_yaw = 2.0 * scalar::atan2(s.st.rot.y, s.st.rot.w);
            let pred = -s.st.q[0] * it / (ih + it);
            worst = worst.max((hull_yaw - pred).abs());
            if n % 6 == 0 {
                ser.push((s.t, hull_yaw.to_degrees()));
            }
        }
        let hull_yaw = 2.0 * scalar::atan2(s.st.rot.y, s.st.rot.w);
        println!("I_h={ih:.0} I_t={it:.0}; turret relative angle {:.4} rad, hull yaw {:.6} rad, predicted -q I_t/(I_h+I_t) = {:.6}; worst |error| over the run = {:.2e} rad", s.st.q[0], hull_yaw, -s.st.q[0] * it / (ih + it), worst);
        let (p, l) = s.momentum();
        println!("final |P|={:.2e} N s, |L|={:.2e} N m s", p.length(), l.length());
        svg(&format!("{out}/s6-swivel.svg"), "Swivel chair: hull yaw (deg) while the turret slews 1.2 rad", "t (s)", "hull yaw (deg)", &[("hull yaw", "#c0392b", ser)]);
    }

    // ---- 2. servo steps at three turret masses -----------------------------------------------------------------------------------
    // Gain rule proposed for FORGE: kp = I wn^2, kd = 2 zeta I wn with I the held-hull subtree inertia about the axis, wn from the slew accuracy.
    let derive = |sd: &mut ServoDef, i: f64, wn: f64, zeta: f64| {
        sd.kp_si = i * wn * wn;
        sd.kd_si = 2.0 * zeta * i * wn;
    };
    let mut step_series = vec![];
    for (gname, derived) in [("box_tank gains", false), ("derived gains (wn=4 rad/s, zeta=0.8 at 1x)", true)] {
        println!("\n== 2. yaw servo 0.5 rad step, limits off, held hull, 0.5x 1x 2x turret mass, FIXED gains: {gname} ==");
        let i_nom = build(&sim0.bodies, &sim0.st).m[6][6];
        for (s_mass, col) in [(0.5, "#2980b9"), (1.0, "#27ae60"), (2.0, "#c0392b")] {
            let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, s_mass), Scheme::ARk4, 6);
            let mut heavy = sim.bodies.clone();
            heavy[0].m *= 1.0e4;
            heavy[0].inertia = heavy[0].inertia.scaled(1.0e4);
            sim.bodies = heavy;
            let sv = sim.servos[0].as_mut().unwrap();
            sv.step_cmd = true;
            sv.goal = 0.5;
            sv.d.max_effort_si = 1.0e9;
            sv.d.latency_s = 0.0;
            sv.d.stabiliser = None;
            sv.eff_delay.clear();
            sv.stab_delay.clear();
            if derived {
                derive(&mut sv.d, i_nom, 4.0, 0.8);
            }
            let (kp, kd) = (sv.d.kp_si, sv.d.kd_si);
            let it = build(&sim.bodies, &sim.st).m[6][6];
            let zeta = kd / (2.0 * scalar::sqrt(kp * it));
            let mut ser = vec![];
            let mut peak = 0.0f64;
            for n in 0..(12.0 / sim.dt) as usize {
                sim.step();
                peak = peak.max(sim.st.q[0]);
                if n % 6 == 0 {
                    ser.push((sim.t, sim.st.q[0]));
                }
            }
            let os = (peak / 0.5 - 1.0).max(0.0);
            let pred = if zeta < 1.0 { scalar::exp(-core::f64::consts::PI * zeta / scalar::sqrt(1.0 - zeta * zeta)) } else { 0.0 };
            println!("mass x{s_mass}: I={it:.0}, zeta={zeta:.3}, overshoot measured {:.3}% predicted {:.3}%, |diff| = {:.3} points", os * 100.0, pred * 100.0, (os - pred).abs() * 100.0);
            if !derived {
                step_series.push((if s_mass == 0.5 { "0.5x turret mass" } else if s_mass == 1.0 { "1x" } else { "2x" }, col, ser));
            }
        }
    }
    svg(&format!("{out}/s6-servo-steps.svg"), "Yaw servo 0.5 rad step at fixed gains, three turret masses (box_tank gains)", "t (s)", "turret angle (rad)", &step_series.iter().map(|(a, b, c)| (*a, *b, c.clone())).collect::<Vec<_>>());

    println!("\n== 2b. real limits and latency: 90 deg slew time against d/w + w/alpha, held hull and free hull ==");
    for (gname, derived) in [("box_tank gains", false), ("derived gains", true)] {
        for free in [false, true] {
            let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), Scheme::ARk4, 6);
            if !free {
                let mut heavy = sim.bodies.clone();
                heavy[0].m *= 1.0e4;
                heavy[0].inertia = heavy[0].inertia.scaled(1.0e4);
                sim.bodies = heavy;
            }
            let i_held = d0.m[6][6];
            let i_eff = if free { 1.0 / minv_diag(0) } else { i_held };
            let sv = sim.servos[0].as_mut().unwrap();
            sv.d.stabiliser = None;
            if derived {
                derive(&mut sv.d, i_held, 4.0, 0.8);
            }
            sv.i_eff = i_eff;
            let d = 90.0f64.to_radians();
            sv.goal = d;
            let w = sv.d.max_rate_si;
            let a_cmd = sv.d.max_accel_si.min(sv.d.max_effort_si / i_eff);
            let (mut t_mid, mut t_settle, mut peak) = (f64::NAN, 0.0f64, 0.0f64);
            for _ in 0..(14.0 / sim.dt) as usize {
                sim.step();
                peak = peak.max(sim.st.q[0]);
                if t_mid.is_nan() && sim.st.q[0] >= 0.5 * d {
                    t_mid = sim.t;
                }
                if (sim.st.q[0] - d).abs() > 0.02 * d {
                    t_settle = sim.t;
                }
            }
            let tp = d / w + w / a_cmd;
            println!("{gname:<15} {} hull: midpoint crossed at {t_mid:.3} s -> 2x = {:.3} s against trapezoid {tp:.3} s ({:+.1}%); stays within 2% after {t_settle:.2} s; overshoot {:.2}%", if free { "free" } else { "held" }, 2.0 * t_mid, (2.0 * t_mid / tp - 1.0) * 100.0, (peak / d - 1.0) * 100.0);
        }
    }

    // ---- 3. the shot: momentum --------------------------------------------------------------------------------------------------
    println!("\n== 3. firing while the turret is slewing, 20 deg elevation demanded, bore off the hull COM ==");
    println!("{:<10} {:>3} {:>12} {:>12} {:>14} {:>10} {:>10}", "scheme", "n", "max|dP|/J", "max|dL|/(J*1m)", "max recoil m", "ends q m", "hullv m/s");
    let mut plot_series: Option<(Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>)> = None;
    for (name, sch) in [("A-RK4", Scheme::ARk4), ("A-Euler", Scheme::AEuler), ("B-Euler", Scheme::BEuler)] {
        for n in [1u32, 2, 3, 4, 6, 8, 12] {
            let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), sch, n);
            sim.servos[0].as_mut().unwrap().goal = 0.8;
            sim.servos[1].as_mut().unwrap().goal = 0.2;
            let (mut ep, mut el, mut qmax) = (0.0f64, 0.0f64, 0.0f64);
            let mut fired = false;
            let (mut sp, mut sl, mut sh) = (vec![], vec![], vec![]);
            for i in 0..(5.0 / sim.dt) as usize {
                if !fired && sim.t >= 1.0 {
                    sim.fire(j);
                    fired = true;
                }
                sim.step();
                let (p, l) = sim.momentum();
                ep = ep.max(p.length() / j);
                el = el.max(l.length() / (j * 1.0));
                qmax = qmax.max(sim.st.q[2]);
                if !ep.is_finite() || ep > 1e3 {
                    break;
                }
                if sch == Scheme::ARk4 && n == 6 && i % 3 == 0 {
                    let k = sim.kin();
                    let (mut pb, mut lb) = (Vec3::ZERO, Vec3::ZERO);
                    for (ii, b) in sim.bodies.iter().enumerate() {
                        let iw = b.inertia.rotate_inertia(&k[ii].r);
                        pb += k[ii].vc * b.m;
                        lb += k[ii].c.cross(k[ii].vc * b.m) + iw.mul_vec(k[ii].w);
                    }
                    sp.push((sim.t, (pb.z) / 1000.0));
                    sl.push((sim.t, (sim.ej_p.z) / 1000.0));
                    sh.push((sim.t, p.length() / j));
                    let _ = lb;
                }
            }
            if sch == Scheme::ARk4 && n == 6 {
                plot_series = Some((sp, sl, sh));
            }
            if ep.is_finite() && ep < 1e3 {
                println!("{:<10} {:>3} {:>12.3e} {:>12.3e} {:>14.4} {:>10.4} {:>10.4}", name, n, ep, el, qmax, sim.st.q[2], sim.st.u[2].hypot(sim.st.u[0]));
            } else {
                println!("{:<10} {:>3} {:>12}", name, n, "DIVERGED");
            }
        }
    }
    if let Some((vehicle, ejecta, total)) = plot_series {
        let tot: Vec<(f64, f64)> = total.iter().map(|&(t, y)| (t, y * 1e6)).collect();
        svg(&format!("{out}/s6-momentum.svg"), "Linear momentum along the bore axis (kN s): vehicle + ejecta = 0", "t (s)", "kN s (z)", &[("vehicle chain + hull (z)", "#2980b9", vehicle.clone()), ("ejecta (z)", "#c0392b", ejecta.clone()), ("sum", "#000000", vehicle.iter().zip(ejecta.iter()).map(|(a, b)| (a.0, a.1 + b.1)).collect())]);
        svg(&format!("{out}/s6-momentum-error.svg"), "Momentum error |P_total| / J, in parts per million", "t (s)", "ppm of J", &[("|P_total|/J (ppm)", "#27ae60", tot)]);
    }

    // ---- 3b. recoil stroke ----------------------------------------------------------------------------------------------------------
    println!("\n== 3b. recoil mechanism: barrel free-recoil speed and stroke used ==");
    {
        let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), Scheme::ARk4, 6);
        sim.fire(j);
        let v0 = sim.st.u[8];
        let (mut qmax, mut t_back, mut peak_f) = (0.0f64, f64::NAN, 0.0f64);
        for _ in 0..(3.0 / sim.dt) as usize {
            sim.step();
            qmax = qmax.max(sim.st.q[2]);
            peak_f = peak_f.max(-sim.passive(&sim.st.q, &sim.st.u)[2]);
            if t_back.is_nan() && qmax > 0.1 && sim.st.q[2] < 0.002 {
                t_back = sim.t;
            }
        }
        println!("barrel speed just after the shot {v0:.2} m/s (J/m_barrel = {:.2}); peak stroke {qmax:.3} m of {:.2} m; peak mechanism force {:.0} kN; back in battery at {t_back:.2} s", j / 2000.0, rig.articulation[2].limits.unwrap().1, peak_f / 1e3);
        let (p, _) = sim.momentum();
        println!("hull speed after the shot has settled: {:.4} m/s (momentum-only estimate J/M_total = {:.4})", Vec3::new(sim.st.u[0], sim.st.u[1], sim.st.u[2]).length(), j / sim.bodies.iter().map(|b| b.m).sum::<f64>());
        let _ = p;
    }

    // ---- 3c. scheme B against the chain-to-hull mass ratio ------------------------------------------------------------------------------
    println!("\n== 3c-0. slew only (no shot), 6 substeps: worst |P| and |L| over 4 s (turret angular momentum is of the order 4e4 N m s) ==");
    for (name, sch) in [("A-RK4", Scheme::ARk4), ("A-Euler", Scheme::AEuler), ("B-Euler", Scheme::BEuler)] {
        let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), sch, 6);
        sim.servos[0].as_mut().unwrap().goal = 0.8;
        sim.servos[1].as_mut().unwrap().goal = 0.2;
        let (mut wp, mut wl) = (0.0f64, 0.0f64);
        for _ in 0..(4.0 / sim.dt) as usize {
            sim.step();
            let (p, l) = sim.momentum();
            wp = wp.max(p.length());
            wl = wl.max(l.length());
        }
        println!("{name:<8} max|P| = {wp:.3e} N s, max|L| = {wl:.3e} N m s");
    }
    println!("\n== 3c. scheme B (reaction wrench one substep late) against hull mass, chain unchanged, 6 substeps, same shot ==");
    for hs in [1.0, 10.0, 100.0, 1000.0] {
        let mut b = bodies_from(&rig, 1.0);
        let ratio = b[1..].iter().map(|x| x.m).sum::<f64>() / (b[0].m * hs);
        b[0].m *= hs;
        b[0].inertia = b[0].inertia.scaled(hs);
        let mut sim = Sim::new(&rig, b, Scheme::BEuler, 6);
        sim.servos[0].as_mut().unwrap().goal = 0.8;
        sim.fire(j);
        let mut ep = 0.0f64;
        for _ in 0..(3.0 / sim.dt) as usize {
            sim.step();
            ep = ep.max(sim.momentum().0.length() / j);
            if !ep.is_finite() || ep > 1e3 {
                break;
            }
        }
        println!("chain/hull mass ratio {ratio:.4}: max |dP|/J = {}", if ep.is_finite() && ep < 1e3 { format!("{ep:.2e}") } else { "DIVERGED".to_string() });
    }

    // ---- 4. stabiliser ---------------------------------------------------------------------------------------------------------------
    println!("\n== 4. stabiliser law |1 - r e^(-j w tau)/(1 + j f/f_b)| on the gun pitch joint; hull driven by a sine torque about X ==");
    println!("(servo law: rate feed-forward -r*LP(hull rate); the position loop follows the stabilised command)");
    for (label, tau_s, fb, wr_mult) in [("box_tank gains, latency 0.05 s, f_b 3 Hz", -1.0, 3.0, 0.0), ("electric: latency 0.01 s, f_b 0.5 Hz, rate loop 8 f_b", 0.01, 0.5, 8.0)] {
        println!("-- {label}");
        for f in [0.1f64, 0.25, 0.5, 1.0] {
            let mut sim = Sim::new(&rig, tank_with_turret_scale(&rig, 1.0), Scheme::ARk4, 6);
            let ii = sim.servos[1].as_ref().unwrap().i_eff;
            {
                let sv = sim.servos[1].as_mut().unwrap();
                let mut st = sv.d.stabiliser.unwrap();
                st.bandwidth_hz = fb;
                if tau_s > 0.0 {
                    sv.d.latency_s = tau_s;
                    sv.eff_delay = vec![0.0; (tau_s / sim.dt).round() as usize];
                    let wr = wr_mult * 2.0 * core::f64::consts::PI * fb;
                    sv.d.kd_si = ii * wr;
                    sv.d.kp_si = ii * (wr / 6.0) * (wr / 6.0);
                    sv.d.max_effort_si = 1.0e6;
                }
                sv.d.stabiliser = Some(st);
                sv.goal = 0.0;
            }
            let omega = 2.0 * core::f64::consts::PI * f;
            let tq = 5.0e3 * omega; // hull pitch inertia ~1.5e5 kg m^2, so a hull rate amplitude of ~0.03 rad/s; a cosine torque has no DC rate
            let (mut ch, mut sh_, mut cg, mut sg, mut cnt) = (0.0, 0.0, 0.0, 0.0, 0.0);
            let t_end = 6.0 / f + 10.0 / f; // 6 periods to settle, 10 to measure
            for _ in 0..(t_end / sim.dt) as usize {
                sim.ext_hull_torque = Vec3::new(tq * scalar::cos(omega * sim.t), 0.0, 0.0);
                sim.step();
                if sim.t > 6.0 / f {
                    let k = sim.kin();
                    let (wh, wg) = (k[0].w.x, k[2].w.x);
                    let (s, c) = (scalar::sin(omega * sim.t), scalar::cos(omega * sim.t));
                    ch += wh * c;
                    sh_ += wh * s;
                    cg += wg * c;
                    sg += wg * s;
                    cnt += 1.0;
                }
            }
            let (ah, ag) = (scalar::hypot(ch, sh_), scalar::hypot(cg, sg));
            let sdef = sim.servos[1].as_ref().unwrap().d.stabiliser.unwrap();
            let tau = sdef.latency_s;
            let (a, b) = (1.0, f / sdef.bandwidth_hz);
            let den = a * a + b * b;
            let (hr, hi) = (a / den, -b / den);
            let (c0, s0) = (scalar::cos(-omega * tau), scalar::sin(-omega * tau));
            let (rr, ri) = (sdef.rejection * (hr * c0 - hi * s0), sdef.rejection * (hr * s0 + hi * c0));
            let law = scalar::hypot(1.0 - rr, -ri);
            println!("f={f:>5} Hz: hull rate amplitude {:.4} rad/s; residual |w_gun|/|w_hull| measured {:.3}, law {:.3}, diff {:+.1}% (no stabiliser: 1.000)", 2.0 * ah / cnt, ag / ah, law, (ag / ah / law - 1.0) * 100.0);
        }
    }
}
