//! Mixing and matching: the design roller, auto-fit and sampling of the possibility space.
//!
//! Every component family is a generator that mounts on standard hull sockets, so *any* hull can be combined with
//! *any* compatible running gear, weapon and sensor, and the physics (not a rulebook) decides which combinations
//! work. This module builds the tools that explore that space:
//!
//! * [`Explorer::roll`] draws a random design from a seed (a family-level [`DesignDef`]): a hull at some relative
//!   size, running gear that fits its sockets, weapons sized to the turret rings, a mast or two, and an engine.
//! * [`Explorer::fit`] is **auto-fit**: it sizes each running-gear family to carry its share of the weight
//!   (`Family::fit_to_load`) and the engine to the vehicle's needs, iterating because every change moves the mass.
//!   It is the seed of an AI designer: a human or a search proposes a skeleton, the fitter makes it physically
//!   coherent.
//! * [`Explorer::sample`] rolls and fits thousands of designs in parallel for the charts of `w5k space`.
//!
//! Determinism: every roll draws from `Pcg32::derive(seed, ...)`, so the same seed gives the same design.

use std::collections::BTreeMap;

use w5k_math::Pcg32;

use crate::assemble::VehicleSheet;
use crate::family::{self, Family, Role, Values};
use crate::schema::{Attach, DesignDef, MaterialLibrary, SocketDef, SocketKind};
use crate::{Built, Forge};

pub const HULLS: [&str; 5] = ["hull_lancer", "hull_bastion", "hull_dreadnought", "hull_strider", "hull_skiff"];
pub const GEARS: [&str; 7] = ["track", "wheel", "legs", "rail", "hover", "antigrav", "rotor"];
pub const WEAPONS: [&str; 3] = ["turret_gun", "turret_missile", "turret_beam"];
pub const PALETTES: [&str; 4] = ["vanguard", "crimson", "verdant", "ultraviolet"];

fn unit(r: &mut Pcg32) -> f64 {
    r.next_u32() as f64 / 4_294_967_296.0
}

fn between(r: &mut Pcg32, a: f64, b: f64) -> f64 {
    a + (b - a) * unit(r)
}

fn pick<T: Copy>(r: &mut Pcg32, items: &[(T, f64)]) -> T {
    let total: f64 = items.iter().map(|(_, w)| w).sum();
    let mut x = unit(r) * total;
    for (t, w) in items {
        if x < *w {
            return *t;
        }
        x -= w;
    }
    items[items.len() - 1].0
}

/// How likely each kind of running gear is on each hull when rolling at random.
fn gear_weights(hull: &str) -> &'static [(&'static str, f64)] {
    match hull {
        "hull_lancer" | "hull_bastion" => &[("track", 4.0), ("wheel", 3.0), ("legs", 1.0), ("rail", 0.4), ("hover", 1.5), ("antigrav", 1.5), ("rotor", 1.0)],
        "hull_dreadnought" => &[("track", 4.0), ("rail", 3.0), ("wheel", 1.0), ("legs", 0.5), ("antigrav", 2.0), ("hover", 0.5), ("rotor", 0.2)],
        "hull_strider" => &[("legs", 6.0), ("antigrav", 1.0), ("hover", 1.0), ("rotor", 1.0)],
        _ => &[("hover", 3.0), ("antigrav", 3.0), ("rotor", 3.0), ("legs", 0.5), ("wheel", 0.5)],
    }
}

/// What a roll may be told. Anything left `None` is drawn at random.
#[derive(Clone, Debug, Default)]
pub struct Spec {
    pub hull: Option<String>,
    pub gear: Option<String>,
    /// Relative size within the hull family's range (0 = smallest, 1 = largest).
    pub size: Option<f64>,
    pub weapon: Option<String>,
    pub palette: Option<String>,
    /// Number of turret rings to fill (default: most of them).
    pub turrets: Option<usize>,
}

/// A design sized by the auto-fitter.
#[derive(Clone, Debug)]
pub struct Fitted {
    /// The family-level design with every slider written out (what a content file stores).
    pub spec: DesignDef,
    /// The same design with plain part ids (what the builder takes).
    pub inst: DesignDef,
    pub sheet: VehicleSheet,
    pub iterations: usize,
    /// Fraction of the rolled armour thickness the design kept (1 = untouched; less when the running gear could
    /// not carry the weight and the fitter had to thin the plating).
    pub diet: f64,
}

impl Fitted {
    pub fn valid(&self) -> bool {
        self.sheet.problems.is_empty() && self.sheet.mass_kg > 0.0
    }
}

pub struct Explorer {
    pub lib: MaterialLibrary,
    fams: Vec<Box<dyn Family>>,
}

fn matching<'a>(sockets: &'a [SocketDef], pattern: &str) -> Vec<&'a SocketDef> {
    match pattern.find('*') {
        Some(star) => {
            let (pre, post) = (&pattern[..star], &pattern[star + 1..]);
            sockets.iter().filter(|s| s.name.len() >= pre.len() + post.len() && s.name.starts_with(pre) && s.name.ends_with(post)).collect()
        }
        None => sockets.iter().filter(|s| s.name == pattern).collect(),
    }
}

fn strip_ctx(v: Values) -> BTreeMap<String, f64> {
    v.into_iter().filter(|(k, _)| !k.starts_with("ctx.")).collect()
}

fn attach(socket: &str, family: &str, params: BTreeMap<String, f64>, spin: f64) -> Attach {
    Attach { socket: socket.into(), part: String::new(), family: Some(family.into()), params, mirror: false, spin, children: vec![] }
}

impl Explorer {
    pub fn new(lib: MaterialLibrary) -> Explorer {
        Explorer { lib, fams: family::all() }
    }

    pub fn fam(&self, id: &str) -> &dyn Family {
        self.fams.iter().find(|f| f.id() == id).unwrap_or_else(|| panic!("unknown family '{id}'")).as_ref()
    }

    /// Slider values for a family: each slider placed at a random position. Sliders that set a size (budgeted, in
    /// metres) follow the relative `size`; armour thicknesses stay on the thin side; the rest are uniform.
    fn sample_params(&self, fam: &dyn Family, r: &mut Pcg32, size: f64) -> BTreeMap<String, f64> {
        let mut out = BTreeMap::new();
        for p in fam.params() {
            let t = if p.unit == "m" && p.role == Role::Budgeted {
                (size + between(r, -0.1, 0.1)).clamp(0.0, 1.0)
            } else if p.unit == "mm" {
                between(r, 0.03, 0.5)
            } else if p.id == "turrets" {
                (size + between(r, -0.2, 0.2)).clamp(0.0, 1.0)
            } else {
                unit(r)
            };
            out.insert(p.id.to_string(), p.value_at(t));
        }
        out
    }

    /// The sockets a hull of these slider values offers.
    pub fn hull_sockets(&self, hull: &str, params: &BTreeMap<String, f64>) -> Vec<SocketDef> {
        let f = self.fam(hull);
        f.generate(&family::values_for(f, params, &BTreeMap::new()), &self.lib).sockets
    }

    /// A weapon family's sliders, shrunk until the turret fits its ring.
    fn fit_weapon(&self, fid: &str, ring_max: f64, size: f64, r: &mut Pcg32) -> Option<BTreeMap<String, f64>> {
        let fam = self.fam(fid);
        let mut vals = self.sample_params(fam, r, size);
        let hints: BTreeMap<String, f64> = [("ctx.ring_max".to_string(), ring_max)].into_iter().collect();
        let lead = match fid {
            "turret_gun" => "calibre_mm",
            "turret_missile" => "tube_mm",
            _ => "power_mw",
        };
        let min = fam.params().iter().find(|p| p.id == lead).map(|p| p.min).unwrap_or(1.0);
        for k in 0..24 {
            let def = fam.generate(&family::values_for(fam, &vals, &hints), &self.lib);
            if def.function.ring_m <= ring_max * 0.999 {
                break;
            }
            let x = vals.get_mut(lead).unwrap();
            *x = (*x * 0.86).max(min);
            if fid == "turret_missile" && k % 3 == 2 {
                let t = vals.get_mut("tubes").unwrap();
                *t = (*t - 1.0).max(1.0);
            }
            if fid == "turret_beam" {
                let a = vals.get_mut("aperture_m").unwrap();
                *a = (*a * 0.92).max(0.05);
            }
        }
        let def = fam.generate(&family::values_for(fam, &vals, &hints), &self.lib);
        (def.function.ring_m <= ring_max * 0.999).then_some(vals)
    }

    /// A random design (not yet fitted: gear and engine are at their defaults).
    pub fn roll(&self, seed: u64, spec: &Spec) -> DesignDef {
        let mut r = Pcg32::derive(seed, &[0x726f6c6c]);
        let hull: String = spec.hull.clone().unwrap_or_else(|| {
            pick(&mut r, &[("hull_lancer", 3.0), ("hull_bastion", 3.0), ("hull_dreadnought", 1.5), ("hull_strider", 2.5), ("hull_skiff", 2.5)]).to_string()
        });
        let gear_pre: Option<String> = spec.gear.clone();
        let size_cap = match gear_pre.as_deref() {
            Some("rotor") => 0.55,
            Some("hover") => 0.75,
            _ => 0.95,
        };
        let size = spec.size.unwrap_or_else(|| between(&mut r, 0.05, size_cap));
        let hull_fam = self.fam(&hull);
        let hull_params = self.sample_params(hull_fam, &mut r, size);
        let sockets = self.hull_sockets(&hull, &hull_params);
        let has = |name: &str| sockets.iter().any(|s| s.name == name);
        let scale = hull_params.get("length_m").copied().unwrap_or_else(|| 2.0 * hull_params.get("radius_m").copied().unwrap_or(1.0));
        let mut attach_list: Vec<Attach> = Vec::new();

        // Running gear.
        let gear: String = spec.gear.clone().unwrap_or_else(|| pick(&mut r, gear_weights(&hull)).to_string());
        let socket = match gear.as_str() {
            "track" => {
                if has("gear_r") {
                    Some("gear_*")
                } else if has("station_r1") {
                    Some("station_*")
                } else {
                    None
                }
            }
            "wheel" | "legs" => has("station_r1").then_some("station_*"),
            "rail" => has("keel_1").then_some("keel_*"),
            "hover" => has("belly").then_some("belly"),
            "antigrav" => {
                if has("belly") && (unit(&mut r) < 0.4 || !has("station_r1")) {
                    Some("belly")
                } else {
                    has("station_r1").then_some("station_*")
                }
            }
            "rotor" => {
                if has("hub") && (unit(&mut r) < 0.5 || !has("station_r1")) {
                    Some("hub")
                } else {
                    has("station_r1").then_some("station_*")
                }
            }
            _ => None,
        };
        // The family must fit the kind of socket it would go on (a track unit does not fit a walker's hip).
        let socket = socket.filter(|pat| matching(&sockets, pat).iter().any(|s| self.fam(&gear).fits().contains(&s.kind)));
        if let Some(sock) = socket {
            let mut gp = self.sample_params(self.fam(&gear), &mut r, size);
            if gear == "legs" {
                // A stance a good fraction of the body, so the walker is neither a crawler nor a stilt.
                let stance = scale * if hull == "hull_strider" { between(&mut r, 0.2, 0.7) } else { between(&mut r, 0.1, 0.32) };
                gp.insert("stance_m".into(), stance.clamp(0.5, 16.0));
            }
            if gear != "hover" && gear != "antigrav" && gear != "rotor" {
                gp.insert("margin".into(), (between(&mut r, 0.0, 0.9)).exp());
            }
            if gear == "rotor" {
                gp.insert("altitude_m".into(), (scale * between(&mut r, 0.4, 3.0)).clamp(2.0, 60.0));
            }
            if gear == "antigrav" {
                // A ride height that is a fraction of the craft, not a skyscraper under a drone.
                gp.insert("ride_m".into(), (scale * between(&mut r, 0.08, 0.5)).clamp(0.4, 30.0));
            }
            attach_list.push(attach(sock, &gear, gp, 0.0));
        }

        // Weapons on the turret rings.
        let mut rings: Vec<&SocketDef> = sockets.iter().filter(|s| s.kind == SocketKind::TurretRing).collect();
        rings.sort_by(|a, b| a.at[2].partial_cmp(&b.at[2]).unwrap());
        let wanted = spec.turrets.unwrap_or(rings.len().max(1));
        let mut used = 0;
        for (i, ring) in rings.iter().enumerate() {
            if used >= wanted || (i > 0 && spec.turrets.is_none() && unit(&mut r) < 0.2) {
                continue;
            }
            let wf: String = spec.weapon.clone().unwrap_or_else(|| pick(&mut r, &[("turret_gun", 5.0), ("turret_missile", 3.0), ("turret_beam", 2.0)]).to_string());
            let ring_max = ring.hints.get("ctx.ring_max").copied().unwrap_or(1.0);
            let Some(params) = self.fit_weapon(&wf, ring_max, (size + between(&mut r, -0.15, 0.15)).clamp(0.0, 1.0), &mut r) else { continue };
            // Turrets aft of the middle face backwards.
            attach_list.push(attach(&ring.name, &wf, params, if ring.at[2] > 0.1 * scale { 180.0 } else { 0.0 }));
            used += 1;
        }

        // Masts: a sensor, and sometimes a repair rig.
        let masts: Vec<&SocketDef> = sockets.iter().filter(|s| s.name.starts_with("mast_")).collect();
        for (i, m) in masts.iter().enumerate() {
            let mast_max = m.hints.get("ctx.mast_max").copied().unwrap_or(0.5);
            let fid = if i == 0 && unit(&mut r) < 0.75 {
                "sensor"
            } else if i > 0 && unit(&mut r) < 0.35 {
                "repair"
            } else {
                continue;
            };
            let fam = self.fam(fid);
            let mut vals = self.sample_params(fam, &mut r, (size * 0.8).clamp(0.0, 1.0));
            let hints: BTreeMap<String, f64> = [("ctx.mast_max".to_string(), mast_max)].into_iter().collect();
            let lead = if fid == "sensor" { "aperture_m" } else { "reach_m" };
            let mut fitted = false;
            for _ in 0..20 {
                let def = fam.generate(&family::values_for(fam, &vals, &hints), &self.lib);
                if def.function.mast_m <= mast_max {
                    fitted = true;
                    break;
                }
                *vals.get_mut(lead).unwrap() *= 0.85;
            }
            if fitted {
                attach_list.push(attach(&m.name, fid, vals, 0.0));
            }
        }

        // The engine, sized later by the fitter.
        let tech = pick(&mut r, &[(0.0, 0.25), (1.0, 0.55), (2.0, 0.2)]);
        // How much spare power per tonne to give the vehicle: a slow brute or a sprinter (a factor on the default).
        let factor = (between(&mut r, -0.7, 0.9)).exp();
        let kw_t = Self::target_kw_per_t(&gear) * factor;
        attach_list.push(attach("engine", "engine", [("power_kw".to_string(), 500.0), ("tech".to_string(), tech), ("kw_per_t".to_string(), kw_t)].into_iter().collect(), 0.0));

        DesignDef {
            id: format!("roll_{seed}"),
            name: String::new(),
            hull,
            hull_params,
            palette: Some(spec.palette.clone().unwrap_or_else(|| PALETTES[r.below(4) as usize].to_string())),
            attach: attach_list,
            lift_m: 0.0,
        }
    }

    /// Kilowatts per tonne an engine must leave over after the vehicle's own draw and lift, by kind of running
    /// gear: enough for a sensible top speed.
    fn target_kw_per_t(gear: &str) -> f64 {
        match gear {
            "legs" => 35.0,
            "rail" => 12.0,
            "rotor" => 14.0,
            "hover" | "antigrav" => 15.0,
            _ => 16.0,
        }
    }

    /// Auto-fit: size the running gear to carry its share of the weight and the engine to power the whole, until
    /// the design settles (each change moves the mass, which moves the other).
    pub fn fit(&self, spec: &DesignDef) -> Result<Fitted, String> {
        // If the running gear cannot carry the vehicle, put the vehicle on a diet: thin the armour (hull, turrets,
        // skirts) step by step, because mass is the one thing every kind of lift is short of.
        let mut last = None;
        for k in [1.0, 0.7, 0.5, 0.35, 0.25, 0.17, 0.12, 0.08] {
            let d = Self::scale_armour(spec, k);
            let mut f = self.fit_inner(&d)?;
            f.diet = k;
            let weight = f.sheet.problems.iter().any(|p| p.contains("overloaded") || p.contains("cannot hover") || p.contains("cannot float") || p.contains("cannot rise") || (p.starts_with("engine ") && p.contains("does not fit")));
            if f.valid() || !weight {
                return Ok(f);
            }
            last = Some(f);
        }
        Ok(last.unwrap())
    }

    /// The design with every armour thickness multiplied by `k` (never below the slider minimum).
    fn scale_armour(d: &DesignDef, k: f64) -> DesignDef {
        if k >= 1.0 {
            return d.clone();
        }
        let mut out = d.clone();
        for key in ["front_mm", "side_mm"] {
            if let Some(x) = out.hull_params.get_mut(key) {
                *x = (*x * k).max(5.0);
            }
        }
        for a in out.attach.iter_mut() {
            for key in ["armour_mm", "skirt_mm"] {
                if let Some(x) = a.params.get_mut(key) {
                    *x = (*x * k).max(if key == "skirt_mm" { 0.0 } else { 3.0 });
                }
            }
        }
        out
    }

    fn fit_inner(&self, spec: &DesignDef) -> Result<Fitted, String> {
        let mut d = spec.clone();
        let mut forge = Forge::new(self.lib.clone());
        let mut prev_mass = 0.0;
        let mut out: Option<(DesignDef, VehicleSheet, usize)> = None;
        for it in 0..10 {
            let inst = forge.instantiate(&d)?;
            let (_asm, sheet) = forge.quick_design(&inst);
            let sockets = forge.part(&inst.hull).map(|p| p.def.sockets.clone()).unwrap_or_default();
            let mass = sheet.mass_kg;
            let mut settled = (mass - prev_mass).abs() < 0.01 * mass.max(1.0);
            let mut gear_id = "track".to_string();
            for a in d.attach.iter_mut() {
                let Some(fid) = a.family.clone() else { continue };
                if !GEARS.contains(&fid.as_str()) {
                    continue;
                }
                gear_id = fid.clone();
                let targets = matching(&sockets, &a.socket);
                if targets.is_empty() {
                    continue;
                }
                let fam = self.fam(&fid);
                let mut v = family::values_for(fam, &a.params, &targets[0].hints);
                // `margin` is a fitter hint, not a slider: how generously to size the gear (wider tracks, bigger feet).
                let margin = a.params.get("margin").copied().unwrap_or(1.0);
                fam.fit_to_load(&mut v, margin * mass / targets.len() as f64, &self.lib);
                let mut new = strip_ctx(v);
                if margin != 1.0 {
                    new.insert("margin".into(), margin);
                }
                if new.iter().any(|(k, x)| a.params.get(k).map(|o| (o - x).abs() > 1e-6 * o.abs().max(1.0)).unwrap_or(true)) {
                    settled = false;
                }
                a.params = new;
            }
            let bay_problem = sheet.problems.iter().any(|p| p.starts_with("engine ") && p.contains("does not fit"));
            for a in d.attach.iter_mut() {
                if a.family.as_deref() != Some("engine") {
                    continue;
                }
                // An engine that does not fit its bay: a denser technology is the way out.
                if bay_problem {
                    let t = a.params.get("tech").copied().unwrap_or(0.0);
                    if t < 2.0 {
                        a.params.insert("tech".into(), (t + 1.0).min(2.0));
                        settled = false;
                    }
                }
                // `kw_per_t` is a fitter hint, not an engine slider: the spare power per tonne to aim for.
                let target = a.params.get("kw_per_t").copied().unwrap_or_else(|| Self::target_kw_per_t(&gear_id));
                let want = (sheet.draw_kw + sheet.lift_kw + target * mass / 1000.0).clamp(5.0, 49_000.0);
                let old = a.params.get("power_kw").copied().unwrap_or(0.0);
                if (want - old).abs() > 0.02 * old {
                    settled = false;
                }
                a.params.insert("power_kw".into(), want);
            }
            prev_mass = mass;
            out = Some((inst, sheet, it + 1));
            if settled && it > 0 {
                break;
            }
        }
        let (inst, sheet, iterations) = out.unwrap();
        // The loop leaves the last evaluation one step behind the written parameters: evaluate once more.
        let inst2 = forge.instantiate(&d)?;
        let (_a, sheet2) = forge.quick_design(&inst2);
        let _ = (inst, sheet);
        Ok(Fitted { spec: d, inst: inst2, sheet: sheet2, iterations, diet: 1.0 })
    }

    /// Full build of a design (voxel mass, armour table, mesh) for rendering.
    pub fn build(&self, design: &DesignDef) -> Result<(Built, VehicleSheet, DesignDef), String> {
        let mut forge = Forge::new(self.lib.clone());
        let inst = forge.instantiate(design)?;
        let (built, _asm, sheet) = forge.build_design(&inst);
        Ok((built, sheet, inst))
    }

    /// Roll and fit `n` designs (seeds `first..first+n`) in parallel. Invalid designs are kept (marked), because
    /// how often combinations fail is itself part of the possibility space.
    pub fn sample(&self, first: u64, n: usize, spec: &Spec) -> Vec<Sample> {
        let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).min(16);
        let seeds: Vec<u64> = (first..first + n as u64).collect();
        let mut out: Vec<Option<Sample>> = (0..n).map(|_| None).collect();
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let seeds = &seeds;
                    s.spawn(move || {
                        let mut mine = Vec::new();
                        let mut i = t;
                        while i < seeds.len() {
                            mine.push((i, self.sample_one(seeds[i], spec)));
                            i += threads;
                        }
                        mine
                    })
                })
                .collect();
            for h in handles {
                for (i, smp) in h.join().expect("sample thread panicked") {
                    out[i] = Some(smp);
                }
            }
        });
        out.into_iter().flatten().collect()
    }

    pub fn sample_one(&self, seed: u64, spec: &Spec) -> Sample {
        let rolled = self.roll(seed, spec);
        let gear = rolled.attach.iter().filter_map(|a| a.family.as_deref()).find(|f| GEARS.contains(f)).unwrap_or("none").to_string();
        let weapon = rolled.attach.iter().filter_map(|a| a.family.as_deref()).find(|f| WEAPONS.contains(f)).unwrap_or("none").to_string();
        match self.fit(&rolled) {
            Ok(f) => Sample { seed, hull: rolled.hull.clone(), gear, weapon, valid: f.valid(), spec: f.spec, sheet: f.sheet, error: None, diet: f.diet },
            Err(e) => Sample { seed, hull: rolled.hull.clone(), gear, weapon, valid: false, spec: rolled, sheet: VehicleSheet::default(), error: Some(e), diet: 1.0 },
        }
    }
}

/// One rolled and fitted design with its sheet.
#[derive(Clone, Debug)]
pub struct Sample {
    pub seed: u64,
    pub hull: String,
    pub gear: String,
    pub weapon: String,
    pub valid: bool,
    pub spec: DesignDef,
    pub sheet: VehicleSheet,
    pub error: Option<String>,
    /// Share of the rolled armour the fitter had to keep (see [`Fitted::diet`]).
    pub diet: f64,
}

/// A short human name for a design: "Lancer / tracks / gun".
pub fn describe(d: &DesignDef) -> String {
    let hull = match d.hull.as_str() {
        "hull_lancer" => "Lancer",
        "hull_bastion" => "Bastion",
        "hull_dreadnought" => "Dreadnought",
        "hull_strider" => "Strider",
        "hull_skiff" => "Skiff",
        h => h,
    };
    let fams: Vec<&str> = d.attach.iter().filter_map(|a| a.family.as_deref()).collect();
    let gear = fams.iter().find(|f| GEARS.contains(f)).map(|g| if *g == "antigrav" { "anti-grav" } else { *g }).unwrap_or("?");
    let weapon = fams.iter().find(|f| WEAPONS.contains(f)).map(|w| w.trim_start_matches("turret_")).unwrap_or("unarmed");
    format!("{hull} / {gear} / {weapon}")
}
