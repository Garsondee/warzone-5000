//! The Design Impact Matrix: perturb a design lever by +10%, rerun the benchmarks, compare the sign of each change with the expected
//! table in `docs/validation/IMPACT-MATRIX.md`. Reports right signs, dead levers and orphan effects (a test that cannot fail proves nothing).

use std::collections::BTreeMap;

use serde::Serialize;
use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::Param;

/// A relative change smaller than this counts as "no change" (`~0`).
pub const EPS: f64 = 0.005; // const-ok: resolution below which a change is not a sign

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Sign {
    Minus,
    Zero,
    Plus,
}

pub fn classify(delta: f64) -> Sign {
    if delta < -EPS {
        Sign::Minus
    } else if delta > EPS {
        Sign::Plus
    } else {
        Sign::Zero
    }
}

/// Within 5% of the friction limit counts as tyre-limited.
const TYRE_LIMITED_FRACTION: f64 = 0.95; // const-ok: regime threshold, IMPACT-MATRIX.md regimes
const PERCENT: f64 = 100.0; // const-ok: unit conversion for display

/// A benchmark the proving runner can measure today: matrix id, proving test, result key.
pub struct Bench {
    pub id: &'static str,
    pub test: &'static str,
    pub key: &'static str,
}

pub const BENCHES: &[Bench] = &[
    Bench { id: "B1", test: "accel_0_48kmh", key: "t_0_48_s" }, // the table says 0-32 km/h; the runner times 0-48
    Bench { id: "B4", test: "braking_50kmh", key: "stop_distance_m" },
    Bench { id: "B6", test: "gradeability", key: "max_grade_ratio" },
    Bench { id: "B7", test: "side_slope_rollover", key: "slope_angle_rad" },
    Bench { id: "B11", test: "step_climb", key: "step_height_m" },
    Bench { id: "B12", test: "skidpad", key: "max_lat_accel_g" },
];

/// A design lever: `row` is how the table names it; `apply` scales it by `factor` in a `VehicleDef`.
pub struct Lever {
    pub id: &'static str,
    pub row: &'static str,
    pub apply: fn(&mut VehicleDef, f64),
}

fn scale(p: &mut Param, f: f64) {
    p.v *= f;
    p.lo = p.lo.map(|x| x * f);
    p.hi = p.hi.map(|x| x * f);
}

fn each_axle(d: &mut VehicleDef, mut f: impl FnMut(&mut w5k_contract::def::AxleDef)) {
    if let RunningGearDef::Wheeled(w) = &mut d.running_gear {
        w.axles.iter_mut().for_each(&mut f);
    }
}

pub fn levers() -> Vec<Lever> {
    vec![
        Lever {
            id: "engine_power",
            row: "Engine peak power",
            apply: |d, f| {
                scale(&mut d.powertrain.engine.peak_power_w, f);
                scale(&mut d.powertrain.engine.peak_torque_nm, f); // a bigger engine: power and torque together
            },
        },
        Lever {
            id: "first_gear",
            row: "First-gear ratio",
            apply: |d, f| d.powertrain.gearbox.forward_ratios.iter_mut().take(1).for_each(|p| scale(p, f)),
        },
        Lever {
            id: "final_drive",
            row: "Final-drive ratio",
            apply: |d, f| scale(&mut d.powertrain.final_drive_ratio, f),
        },
        Lever {
            id: "brake_capacity",
            row: "Brake torque capacity",
            apply: |d, f| scale(&mut d.brakes.service_decel_g, f),
        },
        Lever { id: "mass", row: "Vehicle mass", apply: |d, f| scale(&mut d.hull.mass_kg, f) },
        Lever { id: "com_height", row: "Centre-of-mass height", apply: |d, f| scale(&mut d.hull.com_height_m, f) },
        Lever { id: "track_gauge", row: "Track gauge", apply: |d, f| each_axle(d, |a| scale(&mut a.track_width_m, f)) },
        Lever {
            id: "ground_clearance",
            row: "Ground clearance",
            apply: |d, f| scale(&mut d.hull.ground_clearance_m, f),
        },
        Lever {
            id: "ride_frequency",
            row: "Ride frequency",
            apply: |d, f| {
                scale(&mut d.suspension.front_ride_frequency_hz, f);
                scale(&mut d.suspension.rear_ride_frequency_hz, f);
            },
        },
        Lever {
            id: "tyre_mu",
            row: "Tyre peak friction",
            apply: |d, f| {
                if let RunningGearDef::Wheeled(w) = &mut d.running_gear {
                    scale(&mut w.tyre.mu_peak_ref, f);
                }
                each_axle(d, |a| a.tyre.iter_mut().for_each(|t| scale(&mut t.mu_peak_ref, f)));
            },
        },
    ]
}

/// One row of the expected table: the signs the table allows for a lever on a benchmark.
#[derive(Clone, Debug, PartialEq)]
pub struct Expected {
    pub lever: String,
    pub bench: String,
    pub allowed: Vec<Sign>,
}

/// Read the expected effects from the markdown tables of `IMPACT-MATRIX.md` (rows `| lever | Bn | `sign` ... | why |`).
pub fn parse_table(md: &str) -> Vec<Expected> {
    let mut out = Vec::new();
    for line in md.lines().filter(|l| l.starts_with('|')) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 5 || !cells[2].starts_with('B') || !cells[2][1..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let allowed: Vec<Sign> = cells[3]
            .split('`')
            .skip(1)
            .step_by(2)
            .filter_map(|t| match t {
                "-" => Some(Sign::Minus),
                "+" => Some(Sign::Plus),
                "~0" => Some(Sign::Zero),
                _ => None,
            })
            .collect();
        if !allowed.is_empty() {
            out.push(Expected { lever: cells[1].to_string(), bench: cells[2].to_string(), allowed });
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Regime {
    TyreLimited,
    BrakeLimited,
}

/// The braking regime from the baseline run: a vehicle already decelerating at (nearly) `mu g` is tyre-limited.
pub fn braking_regime(peak_decel_g: f64, mu: f64) -> Regime {
    if peak_decel_g >= TYRE_LIMITED_FRACTION * mu {
        Regime::TyreLimited
    } else {
        Regime::BrakeLimited
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Verdict {
    Right,
    Wrong,
    /// Moved with no row in the table: reported, not scored.
    Unlisted,
    /// Did not move and the table expects no row.
    Quiet,
}

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub vehicle: String,
    pub lever: String,
    pub bench: String,
    pub base: f64,
    pub perturbed: f64,
    pub delta: f64,
    pub observed: Sign,
    pub allowed: Vec<Sign>,
    pub verdict: Verdict,
}

/// A measured pair per (vehicle, lever id, benchmark id): baseline and +10% values. `regimes` are per vehicle, for B4.
pub struct Observations {
    pub pairs: BTreeMap<(String, String, String), (f64, f64)>,
    pub regimes: BTreeMap<String, Regime>,
    /// The baseline run's deciding label per (vehicle, benchmark): side slope `mode` (roll or slide), skidpad `limited_by`.
    pub labels: BTreeMap<(String, String), String>,
}

/// Where the table's sign depends on the regime, what the lever must do **in that regime** (IMPACT-MATRIX.md, "Regimes"):
/// a vehicle that slides before it tips does not care about centre-of-mass height or track (B7); one limited by power on the
/// skidpad does not care about chassis levers (B12).
fn regime_override(bench: &str, lever: &str, label: &str) -> Option<Vec<Sign>> {
    let l = label.to_lowercase();
    let zero = match bench {
        "B7" => l.contains("slide") && ["com_height", "track_gauge", "ground_clearance"].contains(&lever),
        "B12" => l.contains("power") && ["com_height", "track_gauge", "ride_frequency", "tyre_mu"].contains(&lever),
        _ => false,
    };
    zero.then(|| vec![Sign::Zero])
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub entries: Vec<Entry>,
    /// Scored entries that agreed with the table, out of all scored entries.
    pub right: usize,
    pub scored: usize,
    /// Per vehicle: levers that moved no benchmark (dead levers).
    pub dead_levers: Vec<(String, String)>,
    /// Benchmarks that no lever moved on any vehicle (orphan effects).
    pub orphan_benchmarks: Vec<String>,
}

impl Report {
    pub fn right_fraction(&self) -> f64 {
        self.right as f64 / self.scored.max(1) as f64
    }
}

pub fn evaluate(table: &[Expected], levers: &[Lever], obs: &Observations) -> Report {
    let mut entries = Vec::new();
    for ((vehicle, lever_id, bench), (base, pert)) in &obs.pairs {
        let Some(lever) = levers.iter().find(|l| l.id == lever_id) else { continue };
        let delta = (pert - base) / base.abs().max(f64::MIN_POSITIVE);
        let observed = classify(delta);
        let mut allowed: Vec<Sign> = table
            .iter()
            .filter(|e| e.lever.starts_with(lever.row) && &e.bench == bench)
            .flat_map(|e| e.allowed.clone())
            .collect();
        // an "either" row on braking is decided by the regime the baseline run was in
        if bench == "B4" && allowed.contains(&Sign::Zero) && allowed.len() > 1 {
            let other = allowed.iter().copied().find(|s| *s != Sign::Zero).unwrap_or(Sign::Zero);
            allowed = match obs.regimes.get(vehicle) {
                Some(Regime::BrakeLimited) => vec![other],
                Some(Regime::TyreLimited) => vec![Sign::Zero],
                None => allowed,
            };
        }
        if let Some(o) =
            obs.labels.get(&(vehicle.clone(), bench.clone())).and_then(|l| regime_override(bench, lever_id, l))
        {
            allowed = o;
        }
        let verdict = match (allowed.is_empty(), allowed.contains(&observed)) {
            (true, _) if observed == Sign::Zero => Verdict::Quiet,
            (true, _) => Verdict::Unlisted,
            (false, true) => Verdict::Right,
            (false, false) => Verdict::Wrong,
        };
        entries.push(Entry {
            vehicle: vehicle.clone(),
            lever: lever_id.clone(),
            bench: bench.clone(),
            base: *base,
            perturbed: *pert,
            delta,
            observed,
            allowed,
            verdict,
        });
    }
    let scored = entries.iter().filter(|e| matches!(e.verdict, Verdict::Right | Verdict::Wrong)).count();
    let right = entries.iter().filter(|e| e.verdict == Verdict::Right).count();
    let mut dead_levers = Vec::new();
    for ((vehicle, lever), _) in
        obs.pairs.keys().map(|(v, l, _)| ((v.clone(), l.clone()), ())).collect::<BTreeMap<_, _>>()
    {
        if entries.iter().filter(|e| e.vehicle == vehicle && e.lever == lever).all(|e| e.observed == Sign::Zero) {
            dead_levers.push((vehicle, lever));
        }
    }
    let orphan_benchmarks = BENCHES
        .iter()
        .map(|b| b.id)
        .filter(|id| {
            entries.iter().any(|e| e.bench == *id)
                && entries.iter().filter(|e| e.bench == *id).all(|e| e.observed == Sign::Zero)
        })
        .map(String::from)
        .collect();
    Report { entries, right, scored, dead_levers, orphan_benchmarks }
}

fn sign_word(s: Sign) -> &'static str {
    match s {
        Sign::Minus => "-",
        Sign::Zero => "~0",
        Sign::Plus => "+",
    }
}

/// Markdown: one table per vehicle (lever by benchmark: change and verdict), then the summary and the failures.
pub fn render_markdown(r: &Report) -> String {
    let mut s = String::new();
    let vehicles: std::collections::BTreeSet<&str> = r.entries.iter().map(|e| e.vehicle.as_str()).collect();
    for v in vehicles {
        s += &format!(
            "### {v}\n\n| lever | {} |\n|---|{}\n",
            BENCHES.iter().map(|b| b.id).collect::<Vec<_>>().join(" | "),
            "---|".repeat(BENCHES.len())
        );
        let levers: std::collections::BTreeSet<&str> =
            r.entries.iter().filter(|e| e.vehicle == v).map(|e| e.lever.as_str()).collect();
        for l in levers {
            s += &format!("| {l} |");
            for b in BENCHES {
                match r.entries.iter().find(|e| e.vehicle == v && e.lever == l && e.bench == b.id) {
                    Some(e) => {
                        let mark = match e.verdict {
                            Verdict::Right => "ok",
                            Verdict::Wrong => "WRONG",
                            Verdict::Unlisted => "unlisted",
                            Verdict::Quiet => "",
                        };
                        let want = if e.verdict == Verdict::Wrong {
                            format!(
                                " (table: {})",
                                e.allowed.iter().map(|a| sign_word(*a)).collect::<Vec<_>>().join(" or ")
                            )
                        } else {
                            String::new()
                        };
                        s += &format!(" {:+.1}% {mark}{want} |", PERCENT * e.delta);
                    }
                    None => s += " n/a |",
                }
            }
            s += "\n";
        }
        s += "\n";
    }
    s += &format!("**Right signs: {} of {} scored ({:.0}%).**\n\n", r.right, r.scored, PERCENT * r.right_fraction());
    s += &format!(
        "Dead levers (moved no runnable benchmark): {:?}\n\nOrphan benchmarks (no lever moved them): {:?}\n",
        r.dead_levers, r.orphan_benchmarks
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<Expected> {
        parse_table(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/validation/IMPACT-MATRIX.md"),
            )
            .expect("matrix"),
        )
    }

    fn obs(v: &[(&str, &str, f64)]) -> Observations {
        Observations {
            pairs: v
                .iter()
                .map(|(l, b, d)| (("t".to_string(), l.to_string(), b.to_string()), (1.0, 1.0 + d)))
                .collect(),
            regimes: BTreeMap::new(),
            labels: BTreeMap::new(),
        }
    }

    #[test]
    fn the_expected_table_parses_with_its_signs_and_either_rows() {
        let t = table();
        assert!(t.len() > 30, "{}", t.len());
        let first = t.iter().find(|e| e.lever.starts_with("Engine peak power") && e.bench == "B1").expect("row");
        assert_eq!(first.allowed, vec![Sign::Minus]);
        let brake = t.iter().find(|e| e.lever.starts_with("Brake torque") && e.bench == "B4").expect("row");
        assert_eq!(brake.allowed, vec![Sign::Minus, Sign::Zero]);
    }

    #[test]
    fn signs_agree_with_the_expected_table() {
        let o = obs(&[
            ("engine_power", "B1", -0.08),
            ("engine_power", "B6", 0.03),
            ("mass", "B1", 0.09),
            ("mass", "B6", -0.05),
            ("com_height", "B7", -0.09),
            ("tyre_mu", "B12", 0.04),
        ]);
        let r = evaluate(&table(), &levers(), &o);
        assert_eq!(
            (r.right, r.scored),
            (6, 6),
            "{:?}",
            r.entries.iter().filter(|e| e.verdict != Verdict::Right).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_wrong_sign_is_flagged_wrong() {
        // negative control: a model where more power makes the launch slower
        let r = evaluate(&table(), &levers(), &obs(&[("engine_power", "B1", 0.08)]));
        assert_eq!((r.right, r.scored), (0, 1));
    }

    #[test]
    fn impact_matrix_flags_a_dead_lever() {
        // negative control: a lever the model ignores
        let r = evaluate(
            &table(),
            &levers(),
            &obs(&[("mass", "B1", 0.09), ("final_drive", "B1", 0.0), ("final_drive", "B6", 0.001)]),
        );
        assert_eq!(r.dead_levers, vec![("t".to_string(), "final_drive".to_string())]);
    }

    #[test]
    fn impact_matrix_flags_an_orphan_effect() {
        // negative control: nothing moves the side-slope benchmark
        let r = evaluate(
            &table(),
            &levers(),
            &obs(&[("com_height", "B7", 0.0), ("track_gauge", "B7", 0.0), ("mass", "B1", 0.09)]),
        );
        assert_eq!(r.orphan_benchmarks, vec!["B7".to_string()]);
    }

    #[test]
    fn a_vehicle_that_slides_before_it_tips_must_not_respond_to_centre_of_mass_height() {
        let mut o = obs(&[("com_height", "B7", 0.0)]);
        o.labels.insert(("t".into(), "B7".into()), "slide".into());
        assert_eq!(evaluate(&table(), &levers(), &o).right, 1);
        let mut moved = obs(&[("com_height", "B7", -0.09)]);
        moved.labels.insert(("t".into(), "B7".into()), "slide".into());
        assert_eq!(
            evaluate(&table(), &levers(), &moved).right,
            0,
            "a slide-limited vehicle that still moves with h is wrong"
        );
        let mut roll = obs(&[("com_height", "B7", -0.09)]);
        roll.labels.insert(("t".into(), "B7".into()), "roll".into());
        assert_eq!(evaluate(&table(), &levers(), &roll).right, 1);
    }

    #[test]
    fn braking_either_rows_follow_the_regime() {
        let mut o = obs(&[("brake_capacity", "B4", -0.09)]);
        o.regimes.insert("t".into(), braking_regime(0.6, 0.85));
        assert_eq!(evaluate(&table(), &levers(), &o).right, 1, "brake-limited: a bigger brake shortens the stop");
        o.regimes.insert("t".into(), braking_regime(0.84, 0.85));
        assert_eq!(evaluate(&table(), &levers(), &o).right, 0, "tyre-limited: the stop must not change");
        let mut quiet = obs(&[("brake_capacity", "B4", 0.0)]);
        quiet.regimes.insert("t".into(), Regime::TyreLimited);
        assert_eq!(evaluate(&table(), &levers(), &quiet).right, 1);
    }

    #[test]
    fn every_lever_changes_the_vehicle_def_and_keeps_its_params_valid() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/vehicles/game/mule_4x4.ron"),
        )
        .expect("mule");
        let base: VehicleDef = ron::from_str(&text).expect("parse");
        for l in levers() {
            let mut d = base.clone();
            (l.apply)(&mut d, 1.1);
            assert_ne!(d, base, "lever {} edits nothing", l.id);
            let again: VehicleDef = ron::from_str(&ron::to_string(&d).expect("ser")).expect("round trip");
            assert_eq!(again, d);
        }
    }
}
