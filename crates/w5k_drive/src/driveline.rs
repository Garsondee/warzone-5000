//! The driveline tree: gearbox output to the driven shafts through transfer case, differentials and final drives.
//!
//! Principle: an open differential is a torque splitter, not a speed constraint. The carrier gets the input torque; each side receives its
//! share of it and spins as its own load lets it (CHASSIS integrates each wheel), while the carrier speed is the share-weighted mean of the
//! side speeds. It creates no torque: power in equals power out, less the stage losses.
//!
//! A *locked* differential adds a constraint: the two sides must turn at one speed. Like the clutch (spike S-D) it is solved as the torque
//! that would make them equal at the end of the step, so no stiff spring is needed. A *limited-slip* differential moves torque toward the
//! slower side, up to a bias ratio, in proportion to how far the speeds differ: a torque bias, never a speed constraint.

use serde::{Deserialize, Serialize};
use w5k_contract::rig::{DiffKind, DriveModeDef, DriveNode, DrivetrainDef};
use w5k_contract::Param;
use w5k_math::scalar::clamp;

use crate::coupling::Downstream;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrivelineTuning {
    /// Speed difference across a limited-slip differential at which it reaches its full bias, rad/s (below it the bias ramps in linearly).
    pub lsd_full_bias_speed_rad_s: Param,
}

impl DrivelineTuning {
    pub fn check(&self) -> Result<(), String> {
        self.lsd_full_bias_speed_rad_s.check("lsd_full_bias_speed_rad_s")
    }
}

/// One driven shaft as the tree sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    pub output: usize,
    /// Fraction of the driveline input torque that reaches this shaft's branch (sums to 1 over the active shafts).
    pub share: f64,
    /// Reduction from the driveline input to this shaft, including the final drive (input speed / shaft speed).
    pub ratio: f64,
    /// Product of the stage and final-drive efficiencies.
    pub efficiency: f64,
}

/// A set of shafts seen from one point of the tree: `(output, weight, k)` with `weight` the shaft's share of the group's torque and `k` the
/// reduction from the group's input to the shaft. Speed, inertia and free acceleration of the group at its input follow from these.
#[derive(Clone, Debug)]
struct Group(Vec<(usize, f64, f64)>);

#[derive(Clone, Copy, Debug, PartialEq)]
enum LinkKind {
    Locked,
    LimitedSlip { bias: f64 },
}

/// A constraint or torque bias between two groups (the two children of a locked or limited-slip differential, or two shafts locked by a mode).
#[derive(Clone, Debug)]
struct Link {
    kind: LinkKind,
    a: Group,
    b: Group,
}

#[derive(Clone, Debug)]
pub struct Driveline {
    leaves: Vec<Leaf>,
    tree_links: Vec<Link>,
    /// Links added by each drive mode's `locked_groups` (one list per mode).
    mode_links: Vec<Vec<Link>>,
    outputs: usize,
    modes: Vec<DriveModeDef>,
    mode: usize,
    driving: bool,
    lsd_ramp_rad_s: f64,
}

struct Walk<'a> {
    finals: &'a [(f64, f64)],
    leaves: Vec<Leaf>,
    links: Vec<Link>,
}

impl Walk<'_> {
    fn node(&mut self, node: &DriveNode, share: f64, ratio: f64, eta: f64) -> Result<(), String> {
        match node {
            DriveNode::Output(i) => {
                let &(fd, fe) = self
                    .finals
                    .get(*i)
                    .ok_or_else(|| format!("driveline names output {i} but the rig has {}", self.finals.len()))?;
                self.leaves.push(Leaf { output: *i, share, ratio: ratio * fd, efficiency: eta * fe });
                Ok(())
            }
            DriveNode::Diff { kind, ratio: r, split, efficiency, bias, children } => {
                if !(*r > 0.0 && *efficiency > 0.0 && *efficiency <= 1.0) || children.is_empty() {
                    return Err("differential needs ratio > 0, efficiency in (0, 1] and children".into());
                }
                if !split.is_empty()
                    && (split.len() != children.len() || (split.iter().sum::<f64>() - 1.0).abs() > 1e-6)
                {
                    return Err("differential split must have one fraction per child, summing to 1".into());
                }
                let link_kind = match kind {
                    DiffKind::Open => None,
                    DiffKind::Locked => Some(LinkKind::Locked),
                    DiffKind::LimitedSlip if *bias >= 1.0 => Some(LinkKind::LimitedSlip { bias: *bias }),
                    DiffKind::LimitedSlip => return Err("a limited-slip differential needs bias >= 1".into()),
                };
                if link_kind.is_some() && children.len() != 2 {
                    return Err("locked and limited-slip differentials must have exactly two children".into());
                }
                let mut spans = Vec::new();
                for (k, child) in children.iter().enumerate() {
                    let frac = if split.is_empty() { 1.0 / children.len() as f64 } else { split[k] };
                    let start = self.leaves.len();
                    self.node(child, share * frac, ratio * r, eta * efficiency)?;
                    spans.push((start, self.leaves.len(), ratio * r));
                }
                if let Some(kind) = link_kind {
                    let group = |&(a, b, r_in): &(usize, usize, f64)| {
                        let total: f64 = self.leaves[a..b].iter().map(|l| l.share).sum();
                        Group(self.leaves[a..b].iter().map(|l| (l.output, l.share / total, l.ratio / r_in)).collect())
                    };
                    let link = Link { kind, a: group(&spans[0]), b: group(&spans[1]) };
                    self.links.push(link);
                }
                Ok(())
            }
            _ => Err("tracked steering units are not implemented yet".into()),
        }
    }
}

impl Group {
    fn speed(&self, sh: &[Downstream]) -> f64 {
        self.0.iter().map(|&(i, w, k)| w * k * sh[i].omega_rad_s).sum()
    }
    fn inertia(&self, sh: &[Downstream]) -> f64 {
        1.0 / self.0.iter().map(|&(i, w, k)| w * w * k * k / sh[i].inertia_kg_m2).sum::<f64>()
    }
    /// Acceleration of the group at its input from the torques now on its shafts.
    fn accel(&self, sh: &[Downstream], out: &[f64], dt: f64) -> f64 {
        self.0
            .iter()
            .map(|&(i, w, k)| {
                w * k * (out[i] + sh[i].ext_torque_nm) / (sh[i].inertia_kg_m2 + dt * sh[i].ext_slope_nm_s_rad)
            })
            .sum()
    }
    /// Torque on the group at its input implied by an open split (the torque on any one shaft over its weight and reduction).
    fn torque(&self, out: &[f64]) -> f64 {
        let &(i, w, k) = &self.0[0];
        out[i] / (w * k)
    }
    fn add_torque(&self, out: &mut [f64], t: f64) {
        for &(i, w, k) in &self.0 {
            out[i] += t * w * k;
        }
    }
}

impl Link {
    fn apply(&self, dt: f64, sh: &[Downstream], out: &mut [f64], lsd_ramp: f64) {
        let (wa, wb) = (self.a.speed(sh), self.b.speed(sh));
        let (ja, jb) = (self.a.inertia(sh), self.b.inertia(sh));
        match self.kind {
            LinkKind::Locked => {
                // the torque on A (and the opposite on B) that leaves both groups at one speed at the end of the step
                let lambda =
                    -(ja * jb / (ja + jb)) * ((wa - wb) / dt + self.a.accel(sh, out, dt) - self.b.accel(sh, out, dt));
                self.a.add_torque(out, lambda);
                self.b.add_torque(out, -lambda);
            }
            LinkKind::LimitedSlip { bias } => {
                let (ta, tb) = (self.a.torque(out), self.b.torque(out));
                let total = ta + tb;
                if total <= 0.0 {
                    return; // overrun: no bias
                }
                let a_is_slow = wa < wb;
                let (t_slow, t_target) =
                    if a_is_slow { (ta, total * bias / (1.0 + bias)) } else { (tb, total * bias / (1.0 + bias)) };
                let ramp = clamp((wa - wb).abs() / lsd_ramp, 0.0, 1.0);
                let moved = (ramp * (t_target - t_slow)).max(0.0);
                let lambda = if a_is_slow { moved } else { -moved };
                self.a.add_torque(out, lambda);
                self.b.add_torque(out, -lambda);
            }
        }
    }
}

impl Driveline {
    pub fn new(def: &DrivetrainDef, tuning: &DrivelineTuning) -> Result<Driveline, String> {
        tuning.check()?;
        let finals: Vec<(f64, f64)> = def.outputs.iter().map(|o| (o.final_drive_ratio, o.efficiency)).collect();
        if finals.iter().any(|&(r, e)| !(r > 0.0 && e > 0.0 && e <= 1.0)) {
            return Err("every output needs final_drive_ratio > 0 and efficiency in (0, 1]".into());
        }
        let mut walk = Walk { finals: &finals, leaves: Vec::new(), links: Vec::new() };
        walk.node(&def.driveline, 1.0, 1.0, 1.0)?;
        let Walk { mut leaves, links, .. } = walk;
        if leaves.len() != def.outputs.len() {
            return Err("driveline must reach every output exactly once".into());
        }
        leaves.sort_by_key(|l| l.output);
        let mode = usize::from(def.default_mode);
        if !def.modes.is_empty() && mode >= def.modes.len() {
            return Err("default_mode is outside modes".into());
        }
        if def.modes.iter().any(|m| m.ratio_scale.is_nan() || m.ratio_scale <= 0.0) {
            return Err("every drive mode needs ratio_scale > 0".into());
        }
        let mut mode_links = Vec::new();
        for m in &def.modes {
            let mut ml = Vec::new();
            for group in &m.locked_groups {
                if group.iter().any(|&o| o >= finals.len()) {
                    return Err("a locked group names an output the rig does not have".into());
                }
                for pair in group.windows(2) {
                    ml.push(Link {
                        kind: LinkKind::Locked,
                        a: Group(vec![(pair[0], 1.0, 1.0)]),
                        b: Group(vec![(pair[1], 1.0, 1.0)]),
                    });
                }
            }
            mode_links.push(ml);
        }
        Ok(Driveline {
            leaves,
            tree_links: links,
            mode_links,
            outputs: def.outputs.len(),
            modes: def.modes.clone(),
            mode,
            driving: true,
            lsd_ramp_rad_s: tuning.lsd_full_bias_speed_rad_s.v,
        })
    }

    pub fn output_count(&self) -> usize {
        self.outputs
    }
    pub fn mode(&self) -> u8 {
        u8::try_from(self.mode).unwrap_or(u8::MAX)
    }

    pub fn select_mode(&mut self, mode: u8) {
        if usize::from(mode) < self.modes.len() {
            self.mode = usize::from(mode);
        }
    }

    fn scale(&self) -> f64 {
        self.modes.get(self.mode).map_or(1.0, |m| m.ratio_scale)
    }

    fn declutched(&self) -> &[usize] {
        self.modes.get(self.mode).map(|m| m.declutched_outputs.as_slice()).unwrap_or(&[])
    }

    /// The active leaves with their shares renormalised over the shafts still connected in this mode (a declutched axle drops out).
    fn active(&self) -> impl Iterator<Item = Leaf> + '_ {
        let declutched = self.declutched();
        let live: f64 = self.leaves.iter().filter(|l| !declutched.contains(&l.output)).map(|l| l.share).sum();
        let scale = self.scale();
        self.leaves.iter().filter(move |l| !declutched.contains(&l.output)).map(move |l| Leaf {
            share: l.share / live,
            ratio: l.ratio * scale,
            ..*l
        })
    }

    /// Everything downstream of the gearbox output, reflected to the driveline input (carrier): speed, inertia, load and slope.
    /// `shafts[i]` is output `i`'s state: speed, inertia and the net external torque on it (positive accelerates), with its slope.
    pub fn reflect(&self, shafts: &[Downstream]) -> Downstream {
        let (mut w, mut inv_j) = (0.0, 0.0);
        let leaves: Vec<Leaf> = self.active().collect();
        for l in &leaves {
            let s = &shafts[l.output];
            w += l.share * l.ratio * s.omega_rad_s;
            inv_j += l.share * l.share * l.ratio * l.ratio / s.inertia_kg_m2;
        }
        let j_eff = 1.0 / inv_j;
        let (mut ext, mut slope) = (0.0, 0.0);
        for l in &leaves {
            let s = &shafts[l.output];
            let k = if self.driving { 1.0 / l.efficiency } else { l.efficiency };
            ext += l.share * l.ratio * s.ext_torque_nm / s.inertia_kg_m2 * k;
            slope += l.share * s.ext_slope_nm_s_rad / s.inertia_kg_m2;
        }
        Downstream {
            omega_rad_s: w,
            inertia_kg_m2: j_eff,
            ext_torque_nm: j_eff * ext,
            ext_slope_nm_s_rad: j_eff * slope,
        }
    }

    /// Split the driveline input torque `t_in` over the shafts (`out[i] = share ratio eta t_in`; a declutched shaft gets 0), then apply the
    /// differential locks and torque biases. `shafts` carries each output's state, as for [`Driveline::reflect`].
    pub fn distribute(&mut self, dt: f64, t_in: f64, shafts: &[Downstream], out: &mut [f64]) {
        self.driving = t_in >= 0.0;
        out.iter_mut().for_each(|o| *o = 0.0);
        for l in self.active().collect::<Vec<_>>() {
            let eta = if self.driving { l.efficiency } else { 1.0 / l.efficiency };
            out[l.output] = t_in * l.share * l.ratio * eta;
        }
        let declutched = self.declutched();
        let live = |link: &Link| !link.a.0.iter().chain(&link.b.0).any(|&(i, _, _)| declutched.contains(&i));
        for link in self.tree_links.iter().chain(self.mode_links.get(self.mode).into_iter().flatten()) {
            if live(link) {
                link.apply(dt, shafts, out, self.lsd_ramp_rad_s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::{box_tank, box_truck};

    const DT: f64 = 1.0 / 240.0;

    fn tuning() -> DrivelineTuning {
        ron::from_str(include_str!("../../../content/physics/drive/driveline_tuning.ron"))
            .expect("driveline_tuning.ron parses")
    }

    fn truck() -> DrivetrainDef {
        box_truck().0.drivetrain
    }

    /// The truck with its front axle differential changed to `kind` (and `bias`).
    fn truck_with_front(kind: DiffKind, bias: f64) -> DrivetrainDef {
        let mut def = truck();
        if let DriveNode::Diff { children, .. } = &mut def.driveline {
            if let DriveNode::Diff { kind: k, bias: b, .. } = &mut children[0] {
                *k = kind;
                *b = bias;
            }
        }
        def
    }

    fn shafts(speeds: &[f64], loads: &[f64]) -> Vec<Downstream> {
        speeds
            .iter()
            .zip(loads)
            .map(|(&w, &n)| Downstream {
                omega_rad_s: w,
                inertia_kg_m2: 2.0,
                ext_torque_nm: n,
                ext_slope_nm_s_rad: 0.0,
            })
            .collect()
    }

    /// Integrate the four wheels (the chassis' job) under the driveline for `secs`; returns the final speeds.
    fn spin(d: &mut Driveline, t_in: f64, loads: &[f64], secs: f64) -> Vec<f64> {
        let mut w = vec![0.0; 4];
        let mut out = vec![0.0; 4];
        for _ in 0..(secs / DT) as usize {
            d.distribute(DT, t_in, &shafts(&w, loads), &mut out);
            for i in 0..4 {
                w[i] += DT * (out[i] + loads[i]) / 2.0;
            }
        }
        w
    }

    #[test]
    fn open_differential_splits_torque_equally_and_creates_no_torque() {
        let mut def = truck();
        for o in &mut def.outputs {
            o.efficiency = 1.0;
        }
        let mut d = Driveline::new(&def, &tuning()).unwrap();
        let mut out = vec![0.0; d.output_count()];
        let ws: Vec<f64> = (0..out.len()).map(|i| 10.0 + 3.0 * i as f64).collect(); // unequal wheel speeds, as in a turn
        let sh = shafts(&ws, &[0.0; 4]);
        d.distribute(DT, 100.0, &sh, &mut out);
        // the shafts of one axle get the same torque, and torque x speed in = torque x speed out for any wheel speeds
        assert!((out[0] - out[1]).abs() < 1e-9 && out[0] > 0.0, "{out:?}");
        let carrier = d.reflect(&sh).omega_rad_s;
        let p_out: f64 = out.iter().zip(&ws).map(|(t, w)| t * w).sum();
        assert!((p_out - 100.0 * carrier).abs() < 1e-6 * p_out.abs(), "power in {} vs out {p_out}", 100.0 * carrier);
    }

    #[test]
    fn an_open_differential_spins_the_wheel_with_no_grip() {
        let mut d = Driveline::new(&truck(), &tuning()).unwrap();
        let w = spin(&mut d, 400.0, &[-10.0, -300.0, -50.0, -50.0], 3.0);
        assert!(w[0] > w[1] + 1.0, "low-grip wheel should run away: {w:?}");
    }

    #[test]
    fn locked_differential_forces_equal_speeds() {
        let loads = [-10.0, -300.0, -50.0, -50.0];
        let open = Driveline::new(&truck(), &tuning()).unwrap().clone();
        let mut locked = Driveline::new(&truck_with_front(DiffKind::Locked, 0.0), &tuning()).unwrap();
        let w = spin(&mut locked, 400.0, &loads, 3.0);
        assert!((w[0] - w[1]).abs() < 1e-6, "locked wheels differ: {w:?}");
        assert!(w[0] > 1.0, "and they must actually be driven: {w:?}");
        // it creates no torque: the sum over the shafts equals the open split's
        let (mut a, mut b) = (open, locked);
        let (mut oa, mut ob) = (vec![0.0; 4], vec![0.0; 4]);
        let sh = shafts(&[5.0, 20.0, 12.0, 12.0], &loads);
        a.distribute(DT, 400.0, &sh, &mut oa);
        b.distribute(DT, 400.0, &sh, &mut ob);
        assert!((oa.iter().sum::<f64>() - ob.iter().sum::<f64>()).abs() < 1e-9);
    }

    #[test]
    fn limited_slip_bias_is_bounded() {
        let mut d = Driveline::new(&truck_with_front(DiffKind::LimitedSlip, 2.5), &tuning()).unwrap();
        let mut out = vec![0.0; 4];
        // front-left is the slow wheel (it has the grip), front-right is spinning
        d.distribute(DT, 400.0, &shafts(&[10.0, 40.0, 25.0, 25.0], &[0.0; 4]), &mut out);
        let open = {
            let mut o = vec![0.0; 4];
            Driveline::new(&truck(), &tuning()).unwrap().distribute(
                DT,
                400.0,
                &shafts(&[10.0, 40.0, 25.0, 25.0], &[0.0; 4]),
                &mut o,
            );
            o
        };
        assert!(out[0] > out[1] * 1.5, "torque must move to the slow wheel: {out:?}");
        assert!(out[0] <= out[1] * 2.5 + 1e-9, "and never beyond the bias ratio: {out:?}");
        assert!((out[0] + out[1] - open[0] - open[1]).abs() < 1e-9, "a bias moves torque, it does not create it");
        // equal speeds: open behaviour; a small speed difference: part of the bias (it ramps in)
        d.distribute(DT, 400.0, &shafts(&[25.0, 25.0, 25.0, 25.0], &[0.0; 4]), &mut out);
        assert!((out[0] - out[1]).abs() < 1e-9);
        d.distribute(DT, 400.0, &shafts(&[24.7, 25.3, 25.0, 25.0], &[0.0; 4]), &mut out);
        assert!(out[0] > out[1] && out[0] < out[1] * 2.0, "{out:?}");
    }

    #[test]
    fn a_drive_mode_can_lock_a_group_of_shafts() {
        let mut def = truck();
        def.modes = vec![
            DriveModeDef { name: "open".into(), ratio_scale: 1.0, declutched_outputs: vec![], locked_groups: vec![] },
            DriveModeDef {
                name: "locked".into(),
                ratio_scale: 1.0,
                declutched_outputs: vec![],
                locked_groups: vec![vec![0, 1]],
            },
        ];
        let loads = [-10.0, -300.0, -50.0, -50.0];
        let mut d = Driveline::new(&def, &tuning()).unwrap();
        let w_open = spin(&mut d, 400.0, &loads, 3.0);
        d.select_mode(1);
        let w_locked = spin(&mut d, 400.0, &loads, 3.0);
        assert!(
            (w_open[0] - w_open[1]).abs() > 1.0 && (w_locked[0] - w_locked[1]).abs() < 1e-6,
            "{w_open:?} {w_locked:?}"
        );
    }

    #[test]
    fn a_driveline_loses_the_stated_fraction_of_power() {
        let mut d = Driveline::new(&truck(), &tuning()).unwrap();
        let mut out = vec![0.0; d.output_count()];
        let sh = shafts(&[10.0; 4], &[0.0; 4]);
        d.distribute(DT, 100.0, &sh, &mut out);
        let eta: f64 = d.leaves.iter().map(|l| l.share * l.ratio * l.efficiency).sum::<f64>() * 100.0;
        assert!((out.iter().sum::<f64>() - eta).abs() < 1e-9);
        let mut back = vec![0.0; out.len()];
        d.distribute(DT, -100.0, &sh, &mut back);
        assert!(
            back.iter().zip(&out).all(|(b, o)| b.abs() > o.abs()),
            "overrunning transmits more torque magnitude for the same input"
        );
    }

    #[test]
    fn rejects_what_it_cannot_model_with_a_reason() {
        let tank = box_tank().0.drivetrain;
        assert!(Driveline::new(&tank, &tuning()).unwrap_err().contains("steering"));
        let mut def = truck_with_front(DiffKind::LimitedSlip, 0.5);
        assert!(Driveline::new(&def, &tuning()).unwrap_err().contains("bias"));
        def = truck_with_front(DiffKind::Locked, 0.0);
        if let DriveNode::Diff { children, .. } = &mut def.driveline {
            if let DriveNode::Diff { children: c, .. } = &mut children[0] {
                c.push(DriveNode::Output(0));
            }
        }
        assert!(Driveline::new(&def, &tuning()).is_err());
    }
}
