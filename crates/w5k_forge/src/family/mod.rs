//! Parametric component families: slider values in, a part out.
//!
//! A family is a generator. Its sliders (`Param`s) are turned into an ordinary `PartDef` (shapes, sockets,
//! function), so everything else (meshing, measured mass and armour, previews, export) works unchanged and
//! stats stay honest: they are measured from the geometry the sliders produce.
//!
//! Sliders come in two roles:
//! * **Free**: a matter of taste or an even trade (shape style, slope). The solver never moves them.
//! * **Budgeted**: they compete for the part's mass. With "hold mass" on, moving one makes the unlocked
//!   budgeted sliders give way so the mass stays put ([`set_slider`]); locked sliders never move.

pub mod engine;
pub mod hull;
pub mod style;
pub mod track;
pub mod turret;

use std::collections::BTreeMap;

use crate::schema::{MaterialLibrary, PartDef};
use crate::Built;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Linear,
    /// Equal slider steps multiply the value (for quantities spanning orders of magnitude, such as calibre).
    Log,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Free,
    Budgeted,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub scale: Scale,
    pub role: Role,
    pub help: &'static str,
}

impl Param {
    /// Value at slider position `t` in [0, 1].
    pub fn value_at(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self.scale {
            Scale::Linear => self.min + (self.max - self.min) * t,
            Scale::Log => (self.min.ln() + (self.max.ln() - self.min.ln()) * t).exp(),
        }
    }

    /// Slider position of value `v`.
    pub fn position_of(&self, v: f64) -> f64 {
        let t = match self.scale {
            Scale::Linear => (v - self.min) / (self.max - self.min),
            Scale::Log => (v.max(1e-12).ln() - self.min.ln()) / (self.max.ln() - self.min.ln()),
        };
        t.clamp(0.0, 1.0)
    }
}

pub type Values = BTreeMap<String, f64>;

/// One performance figure for the design screen.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Stat {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

pub fn stat(name: &str, value: f64, unit: &str) -> Stat {
    Stat { name: name.into(), value, unit: unit.into() }
}

pub trait Family: Sync + Send {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn params(&self) -> Vec<Param>;
    /// The part these slider values produce.
    fn generate(&self, v: &Values) -> PartDef;
    /// Performance figures from the values and the measured part.
    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat>;

    fn defaults(&self) -> Values {
        self.params().iter().map(|p| (p.id.to_string(), p.default)).collect()
    }

    /// Defaults with some values overridden.
    fn with(&self, overrides: &[(&str, f64)]) -> Values {
        let mut v = self.defaults();
        for (k, x) in overrides {
            v.insert(k.to_string(), *x);
        }
        v
    }
}

/// Mass of the generated part from exact piece volumes (no voxels, so it is fast enough for dragging sliders).
/// Overlaps between pieces are counted twice, which is a small error for well-made families.
pub fn mass_estimate(fam: &dyn Family, v: &Values, lib: &MaterialLibrary) -> f64 {
    let def = fam.generate(v);
    crate::build::build_part(&def)
        .iter()
        .map(|p| lib.materials.get(&p.mat).map(|m| m.density).unwrap_or(7850.0) * p.material_volume().0)
        .sum()
}

/// Move slider `id` toward `target` while holding the part's mass: every unlocked budgeted slider shifts by
/// the same amount of slider travel, each in the direction that compensates (heavier one way, lighter the
/// other). If the others run out of travel, the moved slider stops where the budget runs out.
pub fn set_slider(fam: &dyn Family, lib: &MaterialLibrary, values: &Values, id: &str, target: f64, locked: &[&str]) -> Values {
    let params = fam.params();
    let Some(moved) = params.iter().find(|p| p.id == id) else { return values.clone() };
    let budget = mass_estimate(fam, values, lib);
    let pos0: BTreeMap<&str, f64> = params.iter().map(|p| (p.id, p.position_of(values[p.id]))).collect();
    let movers: Vec<&Param> = params.iter().filter(|p| p.id != id && p.role == Role::Budgeted && !locked.contains(&p.id)).collect();
    // Which way does each mover change the mass? (sign of d mass / d position, by finite differences)
    let dir: Vec<f64> = movers
        .iter()
        .map(|m| {
            let probe = |dt: f64| {
                let mut v = values.clone();
                v.insert(m.id.into(), m.value_at(pos0[m.id] + dt));
                mass_estimate(fam, &v, lib)
            };
            let d = probe(0.02) - probe(-0.02);
            if d.abs() < 1e-9 * budget.max(1.0) { 0.0 } else { d.signum() }
        })
        .collect();
    let at = |moved_pos: f64, s: f64| -> Values {
        let mut v = values.clone();
        v.insert(id.into(), moved.value_at(moved_pos));
        for (m, d) in movers.iter().zip(&dir) {
            v.insert(m.id.into(), m.value_at(pos0[m.id] - s * d));
        }
        v
    };
    let excess = |moved_pos: f64, s: f64| mass_estimate(fam, &at(moved_pos, s), lib) - budget;
    let target_pos = moved.position_of(target);
    let e0 = excess(target_pos, 0.0);
    if movers.is_empty() || dir.iter().all(|d| *d == 0.0) || e0.abs() < 1e-6 * budget.max(1.0) {
        return at(target_pos, 0.0);
    }
    // s > 0 makes the others lighter, s < 0 heavier.
    let s_lim = if e0 > 0.0 { 1.0 } else { -1.0 };
    let solve_s = |pos: f64| {
        let (mut a, mut b) = (0.0, s_lim);
        for _ in 0..30 {
            let m = 0.5 * (a + b);
            if (excess(pos, m) > 0.0) == (e0 > 0.0) {
                a = m;
            } else {
                b = m;
            }
        }
        at(pos, 0.5 * (a + b))
    };
    if (excess(target_pos, s_lim) > 0.0) != (e0 > 0.0) {
        return solve_s(target_pos);
    }
    // The others cannot absorb it all: find how far the moved slider can go.
    let (mut ok, mut bad) = (pos0[id], target_pos);
    for _ in 0..30 {
        let m = 0.5 * (ok + bad);
        if (excess(m, s_lim) > 0.0) == (e0 > 0.0) {
            bad = m;
        } else {
            ok = m;
        }
    }
    solve_s(ok)
}

/// All families compiled into the game.
pub fn all() -> Vec<Box<dyn Family>> {
    vec![
        Box::new(turret::TankTurret),
        Box::new(hull::Lancer),
        Box::new(hull::Bastion),
        Box::new(hull::Dreadnought),
        Box::new(track::Track),
        Box::new(engine::Engine),
    ]
}

/// Slider values for a family: its defaults, then `params`, then context hints (keys starting with `ctx.`).
pub fn values_for(fam: &dyn Family, params: &BTreeMap<String, f64>, hints: &BTreeMap<String, f64>) -> Values {
    let mut v = fam.defaults();
    for (k, x) in params {
        v.insert(k.clone(), *x);
    }
    for (k, x) in hints {
        v.insert(k.clone(), *x);
    }
    v
}
