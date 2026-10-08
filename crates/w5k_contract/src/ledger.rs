//! The force ledger: every force term, every tick, is recorded against the body it acts on, so any outcome can be explained
//! ("the truck did not climb because the tyres were at their friction limit, not because the engine was short").
//! Recording is switched off in bulk runs (`ForceLedger::off()`); then `add` costs a branch.

use w5k_math::Vec3;

/// What produced a force. Add variants at the end only (the numbers appear in replays).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
#[non_exhaustive]
pub enum ForceTerm {
    Gravity = 0,
    SuspensionSpring = 1,
    SuspensionDamper = 2,
    BumpStop = 3,
    AntiRoll = 4,
    TyreLongitudinal = 5,
    TyreLateral = 6,
    TyreNormal = 7,
    TrackShear = 8,
    TrackNormal = 9,
    SoilCompaction = 10,
    RollingResistance = 11,
    Aero = 12,
    EngineDrive = 13,
    EngineBraking = 14,
    Brake = 15,
    Servo = 16,
    Recoil = 17,
    Collision = 18,
    Other = 19,
    /// A belly or skirt ploughing soil (the proxy with `ProxyRole::Belly`).
    BellyDrag = 20,
}

pub const FORCE_TERM_COUNT: usize = 21;

#[derive(Clone, Copy, Debug)]
struct Row {
    term: ForceTerm,
    body: u16,
    force_n: Vec3,
    torque_nm: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct ForceLedger {
    enabled: bool,
    rows: Vec<Row>,
}

impl ForceLedger {
    pub fn on() -> ForceLedger {
        ForceLedger { enabled: true, rows: Vec::new() }
    }

    pub fn off() -> ForceLedger {
        ForceLedger { enabled: false, rows: Vec::new() }
    }

    pub fn is_on(&self) -> bool {
        self.enabled
    }

    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Record a force (N) and a torque (N m, **about the body's own centre of mass**) on `body` (0 = hull, then stations, then articulation, in rig order), world frame.
    pub fn add(&mut self, term: ForceTerm, body: u16, force_n: Vec3, torque_nm: Vec3) {
        if self.enabled {
            self.rows.push(Row { term, body, force_n, torque_nm });
        }
    }

    /// Net force and torque per term, summed over bodies.
    pub fn totals(&self) -> [(Vec3, Vec3); FORCE_TERM_COUNT] {
        let mut t = [(Vec3::ZERO, Vec3::ZERO); FORCE_TERM_COUNT];
        for r in &self.rows {
            let e = &mut t[r.term as usize];
            e.0 += r.force_n;
            e.1 += r.torque_nm;
        }
        t
    }

    /// Net force on one body (all terms).
    pub fn net_force_on(&self, body: u16) -> Vec3 {
        self.rows.iter().filter(|r| r.body == body).fold(Vec3::ZERO, |a, r| a + r.force_n)
    }

    /// The magnitude of the net force of each term, in `ForceTerm` order, as a compact replay summary.
    pub fn summary_n(&self) -> Vec<f32> {
        self.totals().iter().map(|(f, _)| f.length() as f32).collect()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_sum_by_term_and_off_records_nothing() {
        let mut l = ForceLedger::on();
        l.add(ForceTerm::Gravity, 0, Vec3::new(0.0, -100.0, 0.0), Vec3::ZERO);
        l.add(ForceTerm::Gravity, 1, Vec3::new(0.0, -20.0, 0.0), Vec3::ZERO);
        l.add(ForceTerm::Aero, 0, Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        assert!((l.totals()[ForceTerm::Gravity as usize].0.y + 120.0).abs() < 1e-12);
        assert!((l.net_force_on(0).y + 100.0).abs() < 1e-12);
        let mut off = ForceLedger::off();
        off.add(ForceTerm::Gravity, 0, Vec3::Y, Vec3::ZERO);
        assert!(off.is_empty());
    }
}
