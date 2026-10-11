//! The Design Impact Matrix: perturb a design lever by +10%, rerun the benchmarks, compare the sign of each change with the expected
//! table in `docs/validation/IMPACT-MATRIX.md`. Reports right signs, dead levers and orphan effects (a test that cannot fail proves nothing).

use std::collections::BTreeMap;

use serde::Serialize;

/// A relative change smaller than this counts as "no change" (`~0`).
pub const EPS: f64 = 0.005; // const-ok: resolution below which a change is not a sign

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Sign {
    Minus,
    Zero,
    Plus,
}

pub fn classify(delta: f64) -> Sign {
    classify_with(EPS, delta)
}

/// `classify` with another no-change threshold (the report states the count at 1% as well, never choosing one to reach a bar).
pub fn classify_with(eps: f64, delta: f64) -> Sign {
    if delta < -eps {
        Sign::Minus
    } else if delta > eps {
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
    /// What the runner measures, for charts (B1 is timed 0-48 km/h although the table says 0-32).
    pub name: &'static str,
    pub test: &'static str,
    pub key: &'static str,
}

pub const BENCHES: &[Bench] = &[
    Bench { id: "B1", name: "0-48 km/h time (s)", test: "accel_0_48kmh", key: "t_0_48_s" }, // the table says 0-32 km/h; the runner times 0-48
    Bench { id: "B4", name: "braking distance from 50 km/h (m)", test: "braking_50kmh", key: "stop_distance_m" },
    Bench { id: "B6", name: "maximum grade held (rise/run)", test: "gradeability", key: "max_grade_ratio" },
    Bench { id: "B7", name: "side-slope limit (rad)", test: "side_slope_rollover", key: "slope_angle_rad" },
    Bench { id: "B11", name: "vertical step cleared (m)", test: "step_climb", key: "step_height_m" },
    Bench { id: "B12", name: "skidpad lateral acceleration (g)", test: "skidpad", key: "max_lat_accel_g" },
];

/// A design lever: `forge` is the name in FORGE's lever API (`w5k_forge::levers::apply_both`), `row` how the table names it.
pub struct Lever {
    pub id: &'static str,
    pub forge: &'static str,
    pub row: &'static str,
}

pub fn levers() -> Vec<Lever> {
    let l = |id, forge, row| Lever { id, forge, row };
    vec![
        l("engine_power", "engine_peak_power", "Engine peak power"),
        l("first_gear", "first_gear", "First-gear ratio"),
        l("final_drive", "final_drive", "Final-drive ratio"),
        l("brake_capacity", "brake_axle_torque", "Brake torque capacity"),
        l("mass", "mass", "Vehicle mass"),
        l("com_height", "com_height", "Centre-of-mass height"),
        l("track_gauge", "track_gauge", "Track gauge"),
        l("ground_clearance", "ground_clearance", "Ground clearance"),
        l("ride_frequency", "ride_frequency", "Ride frequency"),
        l("tyre_mu", "tyre_friction", "Tyre peak friction"),
        // not a +10% scaling: the centre differential goes from open to limited slip (the bias is PROVISIONAL, see the tools command)
        l("centre_diff_limited_slip", "centre_diff_limited_slip", "Centre differential"),
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
    /// A transient response (`Exclusions::transient`): shown, not counted in the noise-aware headline.
    pub transient: bool,
    /// A change within the bench's own noise (`is_unresolved`): shown, not counted in the noise-aware headline.
    pub unresolved: bool,
}

/// A measured pair per (vehicle, lever id, benchmark id): baseline and +10% values. `regimes` are per vehicle, for B4.
pub struct Observations {
    pub pairs: BTreeMap<(String, String, String), (f64, f64)>,
    pub regimes: BTreeMap<String, Regime>,
    /// The baseline run's deciding label per (vehicle, benchmark): side slope `mode` (roll or slide), skidpad `limited_by`.
    pub labels: BTreeMap<(String, String), String>,
}

/// Where the table's sign depends on the regime, what the lever must do **in that regime** (IMPACT-MATRIX.md, "Regimes"):
/// a vehicle that slides before it tips does not care about centre-of-mass height or track (B7). A power-limited skidpad (B12) is
/// deliberately **not** an override: cornering drag couples the chassis levers to the power limit, so the table's signs apply.
fn regime_override(bench: &str, lever: &str, label: &str) -> Option<Vec<Sign>> {
    let l = label.to_lowercase();
    let zero = match bench {
        "B7" => l.contains("slide") && ["com_height", "track_gauge", "ground_clearance"].contains(&lever),
        _ => false,
    };
    zero.then(|| vec![Sign::Zero])
}

/// A named id for charts.
#[derive(Debug, Serialize)]
pub struct Named {
    pub id: String,
    pub name: String,
}

/// The perturbation, the no-change threshold and the acceptance target, in percent (for the tornado chart's captions).
const PERTURB_PCT: f64 = 10.0; // const-ok: the +10% perturbation of IMPACT-MATRIX.md
const ACCEPTANCE_PCT: f64 = 80.0; // const-ok: ARCH's target for right signs

#[derive(Debug, Serialize)]
pub struct Untested {
    pub lever: String,
    pub bench: String,
    pub reason: &'static str,
}

/// One way of counting the signs: how many agree with the table, out of how many scored, at 0.5% and at 1%.
#[derive(Debug, Serialize)]
pub struct Headline {
    pub name: String,
    pub right: usize,
    pub scored: usize,
    pub right_at_1pct: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    /// The headline three ways; `right`, `scored` and `right_at_1pct` below are the noise-aware one.
    pub headlines: Vec<Headline>,
    /// Scored entries that agree with the table when the no-change threshold is 1% instead of 0.5% (reported next to the headline).
    pub right_at_1pct: usize,
    pub table_rows: usize,
    pub table_rows_tested: usize,
    /// Table rows that no run could test today, with the reason, and the benchmarks that have no runner.
    pub untested_rows: Vec<Untested>,
    pub not_run_benchmarks: Vec<Named>,
    pub benchmarks: Vec<Named>,
    pub levers: Vec<Named>,
    pub perturb_pct: f64,
    pub deadband_pct: f64,
    pub acceptance_pct: f64,
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
    evaluate_full(table, levers, obs, &[], &Exclusions::default())
}

/// The benchmark ids and names of the table's first section (`| B1 | 0-32 km/h time (s) | ... |`).
pub fn parse_benchmarks(md: &str) -> Vec<Named> {
    md.lines()
        .filter(|l| l.starts_with("| B"))
        .filter_map(|l| {
            let c: Vec<&str> = l.split('|').map(str::trim).collect();
            (c.len() > 3 && c[3] != "Sign" && c[1][1..].chars().all(|x| x.is_ascii_digit()) && !c[1].contains(' '))
                .then(|| Named { id: c[1].into(), name: c[2].into() })
        })
        .collect()
}

/// `evaluate`, plus which benchmarks have no runner and which table rows were therefore not tested (`all_benchmarks` from
/// [`parse_benchmarks`]).
/// Cells that are shown but not counted in the noise-aware headline.
#[derive(Default)]
pub struct Exclusions {
    /// (vehicle, benchmark) cells whose response is a transient, not a smooth one (the Hauler's gradeability is a launch-transient
    /// limit, DRIVE): hand-kept until a bench echoes a spread for gradeability too.
    pub transient: std::collections::BTreeSet<(String, String)>,
    /// (vehicle, lever, benchmark) cells whose change is within the bench's own noise (see [`is_unresolved`]).
    pub unresolved: std::collections::BTreeSet<(String, String, String)>,
}

/// ARCH's rule (`docs/swarm/requests/arch-validation-step-noise.md`), adopted as written: a cell is unresolved when the change, in the
/// bench's own unit, is at most twice the larger of the two designs' run-up spreads, and never below the bisection resolution.
pub fn is_unresolved(change: f64, spread_base: f64, spread_pert: f64, resolution: f64) -> bool {
    change.abs() <= (2.0 * spread_base.max(spread_pert)).max(resolution)
}

/// `excl` marks cells that are shown, tagged, and left out of the noise-aware headline; the report states the headline three ways
/// (every cell counted, noise-aware, without B11).
pub fn evaluate_full(
    table: &[Expected],
    levers: &[Lever],
    obs: &Observations,
    all_benchmarks: &[Named],
    excl: &Exclusions,
) -> Report {
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
            transient: excl.transient.contains(&(vehicle.clone(), bench.clone())),
            unresolved: excl.unresolved.contains(&(vehicle.clone(), lever_id.clone(), bench.clone())),
        });
    }
    let count = |name: &str, skip: &dyn Fn(&Entry) -> bool| {
        let es: Vec<&Entry> =
            entries.iter().filter(|e| !skip(e) && matches!(e.verdict, Verdict::Right | Verdict::Wrong)).collect();
        Headline {
            name: name.to_string(),
            right: es.iter().filter(|e| e.verdict == Verdict::Right).count(),
            scored: es.len(),
            right_at_1pct: es.iter().filter(|e| e.allowed.contains(&classify_with(2.0 * EPS, e.delta))).count(),
        }
    };
    // The first three leave out the hand-kept transient cells (the Hauler's gradeability), as v2 did, so they are comparable with it.
    let headlines = vec![
        count("every cell counted, transient cells left out", &|e| e.transient),
        count("noise-aware (transient and unresolved cells left out)", &|e| e.transient || e.unresolved),
        count("without B11 (the step-climb cells), transient cells left out", &|e| e.transient || e.bench == "B11"),
        count("no cell left out at all", &|_| false),
    ];
    let (right, scored, right_at_1pct) = (headlines[1].right, headlines[1].scored, headlines[1].right_at_1pct);
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
    let mut untested_rows = Vec::new();
    for e in table {
        let lever = levers.iter().find(|l| e.lever.starts_with(l.row));
        let reason = match lever {
            None => Some("no lever in the runner (no lever API entry or no vehicle field)"),
            Some(_) if !BENCHES.iter().any(|b| b.id == e.bench) => Some("benchmark has no runner yet"),
            Some(l) if !obs.pairs.keys().any(|(_, lv, b)| lv == l.id && *b == e.bench) => {
                Some("no run produced a value")
            }
            Some(_) => None,
        };
        if let Some(reason) = reason {
            untested_rows.push(Untested { lever: e.lever.clone(), bench: e.bench.clone(), reason });
        }
    }
    let not_run_benchmarks = all_benchmarks
        .iter()
        .filter(|b| !BENCHES.iter().any(|r| r.id == b.id))
        .map(|b| Named { id: b.id.clone(), name: b.name.clone() })
        .collect();
    let benchmarks = BENCHES.iter().map(|b| Named { id: b.id.into(), name: b.name.into() }).collect();
    let levers = levers.iter().map(|l| Named { id: l.id.into(), name: l.row.into() }).collect();
    Report {
        headlines,
        right_at_1pct,
        table_rows: table.len(),
        table_rows_tested: table.len() - untested_rows.len(),
        untested_rows,
        not_run_benchmarks,
        benchmarks,
        levers,
        perturb_pct: PERTURB_PCT,
        deadband_pct: PERCENT * EPS,
        acceptance_pct: ACCEPTANCE_PCT,
        entries,
        right,
        scored,
        dead_levers,
        orphan_benchmarks,
    }
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
                        let mark = if e.transient {
                            "transient"
                        } else if e.unresolved {
                            "unresolved"
                        } else {
                            mark
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
    s += "**Right signs, three ways (the no-change threshold is 0.5%, the one committed in v0; the count at 1% is beside it, and neither is chosen to reach a bar):**\n\n";
    for h in &r.headlines {
        s += &format!(
            "- {}: {} of {} ({:.0}%) at 0.5%; {} of {} ({:.0}%) at 1%\n",
            h.name,
            h.right,
            h.scored,
            PERCENT * h.right as f64 / h.scored.max(1) as f64,
            h.right_at_1pct,
            h.scored,
            PERCENT * h.right_at_1pct as f64 / h.scored.max(1) as f64
        );
    }
    s += "\n";
    s += &format!(
        "**Tested today: {} of {} table rows.** Benchmarks with no runner: {}.\n\nUntested rows ({}):\n",
        r.table_rows_tested,
        r.table_rows,
        r.not_run_benchmarks.iter().map(|b| format!("{} ({})", b.id, b.name)).collect::<Vec<_>>().join(", "),
        r.untested_rows.len()
    );
    for u in &r.untested_rows {
        s += &format!("- {} x {}: {}\n", u.lever, u.bench, u.reason);
    }
    s += "\n";
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

    fn matrix() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/validation/IMPACT-MATRIX.md"),
        )
        .expect("matrix")
    }

    #[test]
    fn the_report_lists_benchmarks_with_no_runner_and_the_rows_that_cannot_be_tested() {
        let o = obs(&[("engine_power", "B1", -0.08)]);
        let r = evaluate_full(&table(), &levers(), &o, &parse_benchmarks(&matrix()), &Exclusions::default());
        assert!(
            r.not_run_benchmarks.iter().any(|b| b.id == "B2") && !r.not_run_benchmarks.iter().any(|b| b.id == "B1")
        );
        assert!(r.untested_rows.iter().any(|u| u.lever.starts_with("Engine peak power") && u.bench == "B2"));
        assert!(r.untested_rows.iter().any(|u| u.lever.starts_with("Turret mass")), "levers with no runner are listed");
        assert_eq!(r.table_rows_tested + r.untested_rows.len(), r.table_rows);
        assert!(render_markdown(&r).contains("Benchmarks with no runner"));
    }

    #[test]
    fn the_count_at_one_percent_is_reported_beside_the_headline_and_never_replaces_it() {
        // a slide-limited vehicle whose side-slope angle moves by 0.7% when the centre of mass is raised: wrong at 0.5%, right at 1%
        let mut o = obs(&[("com_height", "B7", -0.007)]);
        o.labels.insert(("t".into(), "B7".into()), "slide".into());
        let r = evaluate(&table(), &levers(), &o);
        assert_eq!((r.right, r.right_at_1pct, r.scored), (0, 1, 1));
        let md = render_markdown(&r);
        assert!(md.contains("0 of 1 (0%) at 0.5%; 1 of 1 (100%) at 1%"), "{md}");
    }

    #[test]
    fn a_power_limited_skidpad_follows_the_table_signs_because_cornering_drag_couples() {
        let mut o = obs(&[("com_height", "B12", -0.09)]);
        o.labels.insert(("t".into(), "B12".into()), "power (speed stopped rising below the grip limit)".into());
        assert_eq!(evaluate(&table(), &levers(), &o).right, 1);
    }

    #[test]
    fn step_climb_rows_for_torque_levers_allow_a_gain_or_no_change_but_not_a_loss() {
        let ok = evaluate(&table(), &levers(), &obs(&[("engine_power", "B11", 0.2), ("final_drive", "B11", 0.0)]));
        assert_eq!((ok.right, ok.scored), (2, 2));
        let bad = evaluate(&table(), &levers(), &obs(&[("engine_power", "B11", -0.3)]));
        assert_eq!((bad.right, bad.scored), (0, 1));
    }

    #[test]
    fn a_transient_cell_is_shown_with_its_tag_and_left_out_of_the_count() {
        let o = obs(&[("engine_power", "B6", -0.11), ("engine_power", "B1", -0.08)]);
        let excl = Exclusions { transient: [("t".to_string(), "B6".to_string())].into(), ..Exclusions::default() };
        let r = evaluate_full(&table(), &levers(), &o, &[], &excl);
        assert_eq!((r.right, r.scored), (1, 1), "only the smooth B1 cell counts");
        assert!(r.entries.iter().any(|e| e.bench == "B6" && e.transient && e.verdict == Verdict::Wrong));
        assert!(render_markdown(&r).contains("transient"));
    }

    #[test]
    fn a_change_within_twice_the_runup_spread_is_unresolved_and_never_below_the_resolution() {
        assert!(is_unresolved(0.004, 0.002, 0.001, 0.0005), "4 mm is within 2 x 2 mm");
        assert!(!is_unresolved(0.0041, 0.002, 0.001, 0.0005));
        assert!(
            !is_unresolved(0.0008, 0.0, 0.0, 0.0005) && is_unresolved(0.0005, 0.0, 0.0, 0.0005),
            "the resolution is the floor"
        );
    }

    #[test]
    fn the_headline_is_stated_three_ways_and_unresolved_cells_leave_only_the_noise_aware_one() {
        // B11: a torque lever with a loss (wrong, but unresolved) and a gain (right); B1: right
        let o = obs(&[("engine_power", "B11", -0.07), ("final_drive", "B11", 0.2), ("engine_power", "B1", -0.08)]);
        let excl = Exclusions {
            unresolved: [("t".to_string(), "engine_power".to_string(), "B11".to_string())].into(),
            ..Exclusions::default()
        };
        let r = evaluate_full(&table(), &levers(), &o, &[], &excl);
        let h = |name: &str| {
            r.headlines.iter().find(|h| h.name.starts_with(name)).map(|h| (h.right, h.scored)).expect("headline")
        };
        assert_eq!(h("every cell counted"), (2, 3));
        assert_eq!(h("noise-aware"), (2, 2), "the unresolved loss is shown, not counted");
        assert_eq!(h("without B11"), (1, 1));
        assert_eq!(h("no cell left out"), (2, 3));
        assert!(r.entries.iter().any(|e| e.unresolved && e.verdict == Verdict::Wrong));
        let md = render_markdown(&r);
        assert!(md.contains("three ways") && md.contains("unresolved"));
    }
}
