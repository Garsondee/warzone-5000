//! The comparison report of `course-compare`: `compare.json`, `compare.md`, `compare.png` and one `trace_<name>.csv` per vehicle.
//!
//! Every number is computed from the vehicle's trace samples (one per replay frame), so the report can be checked against the CSV.
//! Vertical acceleration is the rate of change of the hull's vertical velocity between samples (the hull stands in for the sprung
//! mass: per-body accelerations are not in the telemetry), lateral acceleration is speed times yaw rate.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use serde::Serialize;
use w5k_math::scalar;

use super::VehicleRun;
use crate::cmd::arch_course::{Sample, Setup};

const SPLIT_M: f64 = 100.0; // const-ok: split-time table spacing (report layout)
const PLOT_STEP_M: f64 = 5.0; // const-ok: chart resolution along the road (report layout)
const TABLE_COLUMNS: usize = 14; // const-ok: columns of the summary table in `markdown`

#[derive(Serialize)]
pub(crate) struct Row {
    pub name: String,
    pub finished: bool,
    pub stop_reason: String,
    pub distance_m: f64,
    pub time_s: f64,
    pub avg_speed_m_s: f64,
    pub top_speed_m_s: f64,
    pub worst_cross_track_m: f64,
    pub gear_changes: u32,
    pub fuel_kg: f64,
    pub peak_vert_accel_m_s2: f64,
    pub rms_vert_accel_m_s2: f64,
    pub peak_steer_angle_rad: f64,
    pub peak_lat_accel_m_s2: f64,
    pub tyre_limit_frames: u32,
    pub frames: u32,
}

/// When each vehicle first passed `mark_m` along the road, and the time since the previous mark (`None` = never got there).
#[derive(Serialize)]
pub(crate) struct Split {
    pub mark_m: f64,
    pub t_s: Vec<Option<f64>>,
    pub segment_s: Vec<Option<f64>>,
}

#[derive(Serialize)]
pub(crate) struct Report {
    pub course: String,
    pub seed: u64,
    pub route_m: f64,
    pub driver: String,
    pub rows: Vec<Row>,
    pub splits: Vec<Split>,
}

fn max_of(s: &[Sample], f: impl Fn(&Sample) -> f64) -> f64 {
    s.iter().map(f).fold(0.0, f64::max)
}

pub(crate) fn row(r: &VehicleRun) -> Row {
    let s = &r.run.samples;
    let n = s.len().max(1) as f64;
    Row {
        name: r.name.clone(),
        finished: r.run.stop_reason.starts_with("finished"),
        stop_reason: r.run.stop_reason.clone(),
        distance_m: r.run.stopped_at_s_m,
        time_s: r.run.time_s,
        avg_speed_m_s: if r.run.time_s > 0.0 { r.run.stopped_at_s_m / r.run.time_s } else { 0.0 },
        top_speed_m_s: max_of(s, |p| p.speed_m_s.abs()),
        worst_cross_track_m: max_of(s, |p| p.cross_track_m),
        gear_changes: s.windows(2).filter(|w| w[0].gear != w[1].gear).count() as u32,
        fuel_kg: s.last().map_or(0.0, |p| p.fuel_kg),
        peak_vert_accel_m_s2: max_of(s, |p| p.vert_accel_m_s2.abs()),
        rms_vert_accel_m_s2: scalar::sqrt(s.iter().map(|p| p.vert_accel_m_s2 * p.vert_accel_m_s2).sum::<f64>() / n),
        peak_steer_angle_rad: max_of(s, |p| p.steer_angle_rad),
        peak_lat_accel_m_s2: max_of(s, |p| p.lat_accel_m_s2.abs()),
        tyre_limit_frames: s.iter().filter(|p| p.tyre_limit).count() as u32,
        frames: s.len() as u32,
    }
}

/// Time of the first sample at or beyond `mark_m`, interpolated between samples (mark 0 is the start).
fn crossing_s(s: &[Sample], mark_m: f64) -> Option<f64> {
    if mark_m <= 0.0 {
        return Some(0.0);
    }
    let i = s.iter().position(|p| p.s_m >= mark_m)?;
    Some(if i == 0 {
        s[0].t_s
    } else {
        scalar::lerp(s[i - 1].t_s, s[i].t_s, scalar::inv_lerp(s[i - 1].s_m, s[i].s_m, mark_m))
    })
}

pub(crate) fn build(setup: &Setup, runs: &[VehicleRun]) -> Report {
    let route_m = runs.first().map_or(0.0, |r| r.run.route_m);
    let splits = (1..=((route_m / SPLIT_M) as u32))
        .map(|k| {
            let mark_m = f64::from(k) * SPLIT_M;
            let t_s: Vec<Option<f64>> = runs.iter().map(|r| crossing_s(&r.run.samples, mark_m)).collect();
            let segment_s = runs
                .iter()
                .zip(&t_s)
                .map(|(r, t)| Some((*t)? - crossing_s(&r.run.samples, mark_m - SPLIT_M)?))
                .collect();
            Split { mark_m, t_s, segment_s }
        })
        .collect();
    Report {
        rows: runs.iter().map(row).collect(),
        course: setup.course_name.clone(),
        seed: setup.course_seed,
        route_m,
        driver: "Speed targets come from the course alone (cruise speed, a cornering cap from the road curvature, the final approach); \
                 they are not capped by the vehicle, so a weaker vehicle falls behind the target."
            .into(),
        splits,
    }
}

/// `trace_<name>.csv`: one line per replay frame, the data every report number comes from (SI units).
pub(crate) fn trace_csv(samples: &[Sample]) -> String {
    let mut t = String::from(
        "t_s,s_m,speed_m_s,target_m_s,cross_track_m,steer_angle_rad,lat_accel_m_s2,vert_accel_m_s2,gear,fuel_kg,tyre_limit\n",
    );
    for p in samples {
        let _ = writeln!(
            t,
            "{:.4},{:.4},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{:.6},{}",
            p.t_s,
            p.s_m,
            p.speed_m_s,
            p.target_m_s,
            p.cross_track_m,
            p.steer_angle_rad,
            p.lat_accel_m_s2,
            p.vert_accel_m_s2,
            p.gear,
            p.fuel_kg,
            u8::from(p.tyre_limit)
        );
    }
    t
}

fn markdown(r: &Report) -> String {
    let mut m = format!(
        "# Course comparison: {} ({} vehicles)\n\nRoute {:.0} m, seed {}. {}\n\n",
        r.course,
        r.rows.len(),
        r.route_m,
        r.seed,
        r.driver
    );
    m.push_str("| vehicle | result | distance m | time s | avg km/h | top km/h | worst XTE m | gear changes | fuel kg | peak az m/s2 | rms az m/s2 | peak steer deg | peak ay m/s2 | tyre-limit frames |\n");
    let _ = writeln!(m, "|{}", "---|".repeat(TABLE_COLUMNS));
    for x in &r.rows {
        let _ = writeln!(
            m,
            "| {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.2} | {} | {:.2} | {:.2} | {:.2} | {:.1} | {:.2} | {}/{} |",
            x.name,
            if x.finished { "finished" } else { "DNF" },
            x.distance_m,
            x.time_s,
            scalar::ms_to_kmh(x.avg_speed_m_s),
            scalar::ms_to_kmh(x.top_speed_m_s),
            x.worst_cross_track_m,
            x.gear_changes,
            x.fuel_kg,
            x.peak_vert_accel_m_s2,
            x.rms_vert_accel_m_s2,
            scalar::rad_to_deg(x.peak_steer_angle_rad),
            x.peak_lat_accel_m_s2,
            x.tyre_limit_frames,
            x.frames
        );
    }
    m.push('\n');
    for x in &r.rows {
        let _ = writeln!(m, "- {}: {}", x.name, x.stop_reason);
    }
    let names: Vec<&str> = r.rows.iter().map(|x| x.name.as_str()).collect();
    let _ = write!(
        m,
        "\n## Split times\n\nCumulative time in s (time for the {SPLIT_M:.0} m since the previous mark); the last column is the fastest segment.\n\n| mark m | {} | fastest |\n|{}\n",
        names.join(" | "),
        "---|".repeat(names.len() + 2)
    );
    for sp in &r.splits {
        let cells: Vec<String> = sp
            .t_s
            .iter()
            .zip(&sp.segment_s)
            .map(|(t, g)| match (t, g) {
                (Some(t), Some(g)) => format!("{t:.1} ({g:.1})"),
                _ => "-".into(),
            })
            .collect();
        let best =
            sp.segment_s.iter().enumerate().filter_map(|(i, g)| g.map(|g| (i, g))).min_by(|a, b| a.1.total_cmp(&b.1));
        let _ = writeln!(m, "| {:.0} | {} | {} |", sp.mark_m, cells.join(" | "), best.map_or("-", |(i, _)| names[i]));
    }
    m
}

/// Every vehicle resampled on a common distance grid for a chart; a vehicle that never got that far has stopped, so its value is 0.
fn grid_csv(runs: &[VehicleRun], value: impl Fn(&[Sample], f64) -> f64) -> String {
    let far = runs.iter().map(|r| r.run.stopped_at_s_m).fold(0.0, f64::max);
    let mut t = String::from("s_m");
    for r in runs {
        let _ = write!(t, ",{}", r.name.replace(',', ";"));
    }
    for k in 0..=((far / PLOT_STEP_M) as u32 + 1) {
        let g = f64::from(k) * PLOT_STEP_M;
        let _ = write!(t, "\n{g}");
        for r in runs {
            let _ = write!(t, ",{:.4}", value(&r.run.samples, g));
        }
    }
    t.push('\n');
    t
}

fn speed_at(s: &[Sample], g: f64) -> f64 {
    let Some(i) = s.iter().position(|p| p.s_m >= g) else { return 0.0 };
    let v = |p: &Sample| scalar::ms_to_kmh(p.speed_m_s);
    if i == 0 {
        v(&s[0])
    } else {
        scalar::lerp(v(&s[i - 1]), v(&s[i]), scalar::inv_lerp(s[i - 1].s_m, s[i].s_m, g))
    }
}

/// Largest |vertical acceleration| among the samples in the bin that ends at `g`.
fn vert_in_bin(s: &[Sample], g: f64) -> f64 {
    s.iter().filter(|p| p.s_m > g - PLOT_STEP_M && p.s_m <= g).map(|p| p.vert_accel_m_s2.abs()).fold(0.0, f64::max)
}

/// Draw one chart with the viewer's Node plotter (headless Chromium).
fn plot(csv: &Path, png: &Path, title: &str, ylabel: &str) -> Result<(), String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/viewer/plot.mjs");
    let status = Command::new("node")
        .arg(script)
        .arg(csv)
        .arg("--out")
        .arg(png)
        .args(["--title", title, "--xlabel", "distance along the road (m)", "--ylabel", ylabel])
        .args(["--width", "1000", "--height", "420"]) // const-ok: chart size in pixels
        .status()
        .map_err(|e| format!("cannot run node: {e}"))?;
    status.success().then_some(()).ok_or_else(|| format!("plot.mjs failed ({status})"))
}

/// Stack two same-width 8-bit PNGs into one.
fn stack(top: &Path, bottom: &Path, out: &Path) -> Result<(), String> {
    let read = |p: &Path| -> Result<(png::OutputInfo, Vec<u8>), String> {
        let file = std::fs::File::open(p).map_err(|e| e.to_string())?;
        let mut r = png::Decoder::new(std::io::BufReader::new(file)).read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
        let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
        buf.truncate(info.buffer_size());
        Ok((info, buf))
    };
    let ((a, mut pa), (b, pb)) = (read(top)?, read(bottom)?);
    if a.width != b.width || a.color_type != b.color_type || a.bit_depth != b.bit_depth {
        return Err("the two charts differ in format".into());
    }
    pa.extend(pb);
    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), a.width, a.height + b.height);
    enc.set_color(a.color_type);
    enc.set_depth(a.bit_depth);
    enc.write_header().and_then(|mut w| w.write_image_data(&pa)).map_err(|e| e.to_string())
}

/// `compare.png`: speed against distance on top, peak vertical acceleration against distance below. Needs Node and Chromium.
fn charts(runs: &[VehicleRun], out: &Path) -> Result<(), String> {
    let speed = out.join("chart_speed.csv");
    let vert = out.join("chart_vert.csv");
    std::fs::write(&speed, grid_csv(runs, speed_at)).map_err(|e| e.to_string())?;
    std::fs::write(&vert, grid_csv(runs, vert_in_bin)).map_err(|e| e.to_string())?;
    let (p1, p2) = (out.join("chart_speed.png"), out.join("chart_vert.png"));
    plot(&speed, &p1, "Speed against distance", "speed (km/h)")?;
    plot(&vert, &p2, "Peak |vertical acceleration| per 5 m of road", "m/s^2")?;
    stack(&p1, &p2, &out.join("compare.png"))?;
    for p in [p1, p2] {
        let _ = std::fs::remove_file(p);
    }
    Ok(())
}

/// Write the traces, `compare.json`, `compare.md` and (when Node and Chromium are available) `compare.png`.
pub(crate) fn write_all(setup: &Setup, runs: &[VehicleRun], out: &Path) -> Result<(), String> {
    let write = |name: String, text: String| {
        std::fs::write(out.join(&name), text).map_err(|e| format!("cannot write {name}: {e}"))
    };
    for r in runs {
        write(format!("trace_{}.csv", r.stem), trace_csv(&r.run.samples))?;
    }
    let report = build(setup, runs);
    write("compare.json".into(), serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?)?;
    write("compare.md".into(), markdown(&report))?;
    if let Err(e) = charts(runs, out) {
        eprintln!("warning: compare.png not written ({e}); chart_speed.csv and chart_vert.csv hold the data");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::tests::three_mules;
    use super::*;

    fn column(csv: &str, name: &str) -> Vec<f64> {
        let mut lines = csv.lines();
        let k = lines.next().expect("header").split(',').position(|h| h == name).expect("column");
        lines.map(|l| l.split(',').nth(k).expect("field").parse().expect("number")).collect()
    }

    #[test]
    fn report_numbers_match_the_csv_trace_they_are_computed_from() {
        let (_, runs) = three_mules();
        let x = row(&runs[0]);
        let csv = trace_csv(&runs[0].run.samples);
        let max_abs = |c: &str| column(&csv, c).iter().fold(0.0, |m: f64, v| m.max(v.abs()));
        let az = column(&csv, "vert_accel_m_s2");
        let rms = scalar::sqrt(az.iter().map(|v| v * v).sum::<f64>() / az.len() as f64);
        let near = |a: f64, b: f64, what: &str| {
            assert!((a - b).abs() <= 1e-5 * (1.0 + b.abs()), "{what}: report {a}, trace {b}")
        }; // const-ok: CSV rounding
        near(x.top_speed_m_s, max_abs("speed_m_s"), "top speed");
        near(x.worst_cross_track_m, max_abs("cross_track_m"), "cross-track");
        near(x.peak_vert_accel_m_s2, max_abs("vert_accel_m_s2"), "peak vertical acceleration");
        near(x.rms_vert_accel_m_s2, rms, "rms vertical acceleration");
        near(x.peak_steer_angle_rad, max_abs("steer_angle_rad"), "steer angle");
        near(x.peak_lat_accel_m_s2, max_abs("lat_accel_m_s2"), "lateral acceleration");
        near(x.fuel_kg, *column(&csv, "fuel_kg").last().expect("rows"), "fuel");
        assert_eq!(x.tyre_limit_frames as usize, column(&csv, "tyre_limit").iter().filter(|v| **v > 0.5).count()); // const-ok: a 0/1 flag
        let gears = column(&csv, "gear");
        assert_eq!(x.gear_changes as usize, gears.windows(2).filter(|w| (w[0] - w[1]).abs() > 0.5).count());
        assert!(x.finished && x.top_speed_m_s > 1.0 && x.gear_changes > 0);
    }

    #[test]
    fn split_times_rise_with_distance_and_each_segment_is_the_difference_of_two_marks() {
        let (setup, runs) = three_mules();
        let report = build(setup, runs);
        assert!(!report.splits.is_empty(), "the course is shorter than one split");
        let mut last = 0.0;
        for sp in &report.splits {
            let t = sp.t_s[0].expect("the finishing mule passed every mark");
            assert!(t > last, "mark {} m at {t} s after {last} s", sp.mark_m);
            assert!((t - last - sp.segment_s[0].expect("segment")).abs() < 1e-9, "segment at {} m", sp.mark_m); // const-ok: float round-off
            last = t;
        }
        assert!(markdown(&report).contains("Split times"));
    }
}
