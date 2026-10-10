//! The driveline tree: gearbox output to the driven shafts through transfer case, differentials and final drives.
//!
//! Principle: an open differential is a torque splitter, not a speed constraint. The carrier gets the input torque; each side receives its
//! share of it and spins as its own load lets it (CHASSIS integrates each wheel), while the carrier speed is the share-weighted mean of the
//! side speeds. It creates no torque: power in equals power out, less the stage losses. Locks, limited slip and steering units are layered
//! on this in later steps; this module refuses what it cannot yet model.

use w5k_contract::rig::{DiffKind, DriveModeDef, DriveNode, DrivetrainDef};

use crate::coupling::Downstream;

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

#[derive(Clone, Debug)]
pub struct Driveline {
    leaves: Vec<Leaf>,
    outputs: usize,
    modes: Vec<DriveModeDef>,
    mode: usize,
    driving: bool,
}

fn walk(
    node: &DriveNode,
    share: f64,
    ratio: f64,
    eta: f64,
    out: &mut Vec<Leaf>,
    finals: &[(f64, f64)],
) -> Result<(), String> {
    match node {
        DriveNode::Output(i) => {
            let &(fd, fe) =
                finals.get(*i).ok_or_else(|| format!("driveline names output {i} but the rig has {}", finals.len()))?;
            out.push(Leaf { output: *i, share, ratio: ratio * fd, efficiency: eta * fe });
            Ok(())
        }
        DriveNode::Diff { kind, ratio: r, split, efficiency, children, .. } => {
            if *kind != DiffKind::Open {
                return Err("locked and limited-slip differentials are not implemented yet".into());
            }
            if !(*r > 0.0 && *efficiency > 0.0 && *efficiency <= 1.0) || children.is_empty() {
                return Err("differential needs ratio > 0, efficiency in (0, 1] and children".into());
            }
            if !split.is_empty() && (split.len() != children.len() || (split.iter().sum::<f64>() - 1.0).abs() > 1e-6) {
                return Err("differential split must have one fraction per child, summing to 1".into());
            }
            for (k, child) in children.iter().enumerate() {
                let frac = if split.is_empty() { 1.0 / children.len() as f64 } else { split[k] };
                walk(child, share * frac, ratio * r, eta * efficiency, out, finals)?;
            }
            Ok(())
        }
        _ => Err("tracked steering units are not implemented yet".into()),
    }
}

impl Driveline {
    pub fn new(def: &DrivetrainDef) -> Result<Driveline, String> {
        let finals: Vec<(f64, f64)> = def.outputs.iter().map(|o| (o.final_drive_ratio, o.efficiency)).collect();
        if finals.iter().any(|&(r, e)| !(r > 0.0 && e > 0.0 && e <= 1.0)) {
            return Err("every output needs final_drive_ratio > 0 and efficiency in (0, 1]".into());
        }
        let mut leaves = Vec::new();
        walk(&def.driveline, 1.0, 1.0, 1.0, &mut leaves, &finals)?;
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
        Ok(Driveline { leaves, outputs: def.outputs.len(), modes: def.modes.clone(), mode, driving: true })
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

    /// The active leaves with their shares renormalised over the shafts still connected in this mode (a declutched axle drops out).
    fn active(&self) -> impl Iterator<Item = Leaf> + '_ {
        let declutched = self.modes.get(self.mode).map(|m| m.declutched_outputs.as_slice()).unwrap_or(&[]);
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
        let (mut w, mut inv_j, mut ext, mut slope) = (0.0, 0.0, 0.0, 0.0);
        let leaves: Vec<Leaf> = self.active().collect();
        for l in &leaves {
            let s = &shafts[l.output];
            w += l.share * l.ratio * s.omega_rad_s;
            inv_j += l.share * l.share * l.ratio * l.ratio / s.inertia_kg_m2;
            slope += l.share * l.share * l.ratio * l.ratio * s.ext_slope_nm_s_rad;
        }
        let j_eff = 1.0 / inv_j;
        for l in &leaves {
            let s = &shafts[l.output];
            let k = if self.driving { 1.0 / l.efficiency } else { l.efficiency };
            ext += l.share * l.ratio * s.ext_torque_nm / s.inertia_kg_m2 * k;
        }
        Downstream { omega_rad_s: w, inertia_kg_m2: j_eff, ext_torque_nm: j_eff * ext, ext_slope_nm_s_rad: slope }
    }

    /// Split the driveline input torque `t_in` over the shafts: `out[i] = share ratio eta t_in` (a declutched shaft gets 0).
    pub fn distribute(&mut self, t_in: f64, out: &mut [f64]) {
        self.driving = t_in >= 0.0;
        out.iter_mut().for_each(|o| *o = 0.0);
        for l in self.active().collect::<Vec<_>>() {
            let eta = if self.driving { l.efficiency } else { 1.0 / l.efficiency };
            out[l.output] = t_in * l.share * l.ratio * eta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    fn truck() -> DrivetrainDef {
        box_truck().0.drivetrain
    }

    #[test]
    fn open_differential_splits_torque_equally_and_creates_no_torque() {
        let mut def = truck();
        for o in &mut def.outputs {
            o.efficiency = 1.0;
        }
        let mut d = Driveline::new(&def).unwrap();
        let mut out = vec![0.0; d.output_count()];
        d.distribute(100.0, &mut out);
        // the shafts of one axle get the same torque, and torque x speed in = torque x speed out for any wheel speeds
        let (a, b) = (out[0], out[1]);
        assert!((a - b).abs() < 1e-9 && a > 0.0, "{out:?}");
        let ws: Vec<f64> = (0..out.len()).map(|i| 10.0 + 3.0 * i as f64).collect(); // unequal wheel speeds, as in a turn
        let shafts: Vec<Downstream> =
            ws.iter().map(|&w| Downstream { omega_rad_s: w, inertia_kg_m2: 3.0, ..Default::default() }).collect();
        let carrier = d.reflect(&shafts).omega_rad_s;
        let p_out: f64 = out.iter().zip(&ws).map(|(t, w)| t * w).sum();
        assert!((p_out - 100.0 * carrier).abs() < 1e-6 * p_out.abs(), "power in {} vs out {p_out}", 100.0 * carrier);
    }

    #[test]
    fn a_driveline_loses_the_stated_fraction_of_power() {
        let def = truck();
        let mut d = Driveline::new(&def).unwrap();
        let mut out = vec![0.0; d.output_count()];
        d.distribute(100.0, &mut out);
        let eta: f64 = d.leaves.iter().map(|l| l.share * l.ratio * l.efficiency).sum::<f64>() * 100.0;
        assert!((out.iter().sum::<f64>() - eta).abs() < 1e-9);
        let mut back = vec![0.0; out.len()];
        d.distribute(-100.0, &mut back);
        assert!(
            back.iter().zip(&out).all(|(b, o)| b.abs() > o.abs()),
            "overrunning transmits more torque magnitude for the same input"
        );
    }

    #[test]
    fn rejects_what_it_cannot_model_with_a_reason() {
        let mut def = box_truck().0.drivetrain;
        if let DriveNode::Diff { kind, .. } = &mut def.driveline {
            *kind = DiffKind::Locked;
        }
        assert!(Driveline::new(&def).unwrap_err().contains("not implemented"));
        let tank = box_tank_def();
        assert!(Driveline::new(&tank).unwrap_err().contains("steering"));
    }

    fn box_tank_def() -> DrivetrainDef {
        w5k_contract::testing::box_tank().0.drivetrain
    }
}
