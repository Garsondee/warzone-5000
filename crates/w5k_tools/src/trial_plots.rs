//! Charts of a set of time trials: who finished and how fast (`results.png`) and every vehicle's speed along the course
//! (`traces.png`). Both are drawn with the software rasteriser, like the other charts, so they need nothing installed.

use std::path::Path;

use w5k_forge::raster::{Image, Rgb};
use w5k_sim::replay::run_state;
use w5k_sim::{Course, Outcome, Run, HZ};

use crate::plot::{hex, Axis, Plot, BG};

/// One vehicle in a chart.
pub struct Lane<'a> {
    pub name: &'a str,
    /// The kind of running gear ("tracks", "legs"...): the colour of its line.
    pub class: &'a str,
    pub run: &'a Run,
    /// Seconds on the same course with every surface rigid, when known: the bar is split into that and what the soil cost.
    pub dry_s: Option<f64>,
}

const TEXT: Rgb = [0.8, 0.82, 0.86];
const DIM: Rgb = [0.5, 0.52, 0.58];
const GRID: Rgb = [0.03, 0.033, 0.045];
const BAD: Rgb = [0.95, 0.2, 0.3];
/// The colour of what the soft ground cost.
const EARTH: Rgb = [0.55, 0.33, 0.14];

/// The colour of a kind of running gear (the same as the viewer's).
pub fn class_colour(class: &str) -> Rgb {
    hex(match class {
        "tracks" | "halftracks" => "#ffb454",
        "wheels" => "#a3e635",
        "legs" => "#ff5c8a",
        "rail" => "#cbd5e1",
        "air cushion" => "#38bdf8",
        "anti-gravity" => "#c084fc",
        "rotor" => "#34d399",
        _ => "#8e97aa",
    })
}

fn classes(lanes: &[Lane]) -> Vec<(String, Rgb)> {
    let mut names: Vec<&str> = Vec::new();
    for l in lanes {
        if !names.contains(&l.class) {
            names.push(l.class);
        }
    }
    names.sort();
    names.into_iter().map(|n| (n.to_string(), class_colour(n))).collect()
}

fn plain(x: f64) -> String {
    format!("{x:.0}")
}

/// Speed (km/h) against distance for every vehicle that started, with the soft ground shaded and the profile along the foot.
pub fn traces_png(lanes: &[Lane], course: &Course, path: &Path) {
    let running: Vec<&Lane> = lanes.iter().filter(|l| !l.run.frames.is_empty()).collect();
    let vmax = running
        .iter()
        .flat_map(|l| l.run.frames.iter().map(|f| f.v_cms as f64 * 0.036))
        .fold(40.0f64, f64::max);
    let ytop = ((vmax * 1.12) / 20.0).ceil() * 20.0;
    let finish = course.finish().to_f64();
    let xhi = (finish * 1.2 / 50.0).ceil() * 50.0;
    let mut p = Plot::new(
        1500,
        760,
        &format!("{}: speed along the course", course.name),
        "Every vehicle's speed (km/h) against distance from the start line. Soft earth is shaded; the profile of the course runs along the foot of the chart.",
        Axis::lin(0.0, xhi, "distance from the start line (m)", plain),
        Axis::lin(0.0, ytop, "speed (km/h)", plain),
    );
    // Soft ground and the profile.
    let (heights, surf) = course.profile(0.5);
    let s0 = course.s_min().to_f64();
    let (hmin, hmax) = heights.iter().fold((f64::MAX, f64::MIN), |(a, b), &h| (a.min(h), b.max(h)));
    let foot = |h: f64| (h - hmin) / (hmax - hmin).max(1e-6) * 0.1 * ytop;
    let soft = course.surface_id("soft_earth");
    for (i, &h) in heights.iter().enumerate() {
        let s = s0 + i as f64 * 0.5;
        if s < 0.0 || s > finish {
            continue;
        }
        if Some(surf[i]) == soft {
            p.seg(s, 0.0, s, ytop, [0.36, 0.22, 0.1], 0.35);
        }
        p.seg(s, 0.0, s, foot(h), [0.3, 0.33, 0.4], 0.6);
    }
    for g in course.checkpoints() {
        p.seg(g.to_f64(), 0.0, g.to_f64(), ytop, [0.15, 0.5, 0.65], 0.45);
    }
    // The lines: what is plotted is the running part of each run (not the braking after the finish).
    let mut ends: Vec<(f64, String, Rgb)> = Vec::new();
    for l in &running {
        let c = class_colour(l.class);
        let fr: Vec<_> = l.run.frames.iter().filter(|f| f.run_state() == run_state::RUNNING || f.run_state() == run_state::FINISHED).collect();
        for w in fr.windows(2) {
            if w[0].run_state() == run_state::FINISHED {
                break;
            }
            let (a, b) = (&w[0], &w[1]);
            for off in [0.0, 0.12] {
                p.seg(a.s_mm as f64 / 1000.0, a.v_cms as f64 * 0.036 + off * ytop / 60.0, b.s_mm as f64 / 1000.0, b.v_cms as f64 * 0.036 + off * ytop / 60.0, c, 0.92);
            }
        }
        if let Some(last) = fr.iter().rev().find(|f| f.run_state() == run_state::RUNNING) {
            let (x, y) = (last.s_mm as f64 / 1000.0, last.v_cms as f64 * 0.036);
            match &l.run.outcome {
                Outcome::Finished { .. } => p.dot(x, y, 3.0, c, 1.0),
                _ => {
                    p.dot(x, y, 4.0, BAD, 1.0);
                    p.dot(x, y, 2.0, c, 1.0);
                }
            }
            ends.push((y, l.name.to_string(), c));
        }
    }
    // Names at the right of the finish, spread out so they do not overlap.
    ends.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut next_free = f64::MIN;
    let top = p.y(ytop);
    for (v, name, c) in &ends {
        let py = p.y(*v).max(next_free).max(top + 2.0);
        p.text_px(p.x(finish) + 10.0, py - 4.0, name, *c);
        next_free = py + 11.0;
    }
    p.legend(&classes(lanes));
    p.notes(&[
        "A dot marks where a run ended; a red ring means it did not finish.",
        "Light vertical lines are the timing gates.",
        &format!("The lap is {:.0} m long, from a standing start.", finish),
    ]);
    if let Err(e) = p.img.save_png(path) {
        eprintln!("error: {e}");
    }
}

/// Finish times as bars, fastest first, with those that did not finish (and where and why) after them.
pub fn results_png(lanes: &[Lane], course: &Course, path: &Path) {
    let mut order: Vec<&Lane> = lanes.iter().collect();
    let key = |l: &Lane| match &l.run.outcome {
        Outcome::Finished { ticks, .. } => (0, *ticks as i64),
        Outcome::Dnf { s_mm, .. } => (1, -(*s_mm as i64)),
        Outcome::Dns { .. } => (2, 0),
    };
    order.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.name.cmp(b.name)));
    let limit_s = course.time_limit_ticks() as f64 / HZ as f64;
    let slowest = order.iter().filter_map(|l| l.run.outcome.time_s()).fold(10.0f64, f64::max).max(order.iter().filter_map(|l| l.dry_s).fold(0.0, f64::max));
    let xmax = ((slowest * 1.1) / 20.0).ceil().max(2.0) * 20.0;
    let (row, left, right) = (28i64, 270i64, 60i64);
    let (w, h) = (1500usize, (130 + row * order.len() as i64 + 40) as usize);
    let mut img = Image::new(w, h, BG);
    img.text(24, 18, 2, &format!("{}: time to complete the course", course.name), [0.9, 0.91, 0.94]);
    let split = lanes.iter().any(|l| l.dry_s.is_some());
    let sub = if split {
        format!("Seconds from a standing start to the finish line, fastest first. Each bar: the time on concrete, then what the soft earth added (brown). Clock stops at {limit_s:.0} s.")
    } else {
        format!("Seconds from the standing start to the finish line, fastest first. The clock stops at {limit_s:.0} s.")
    };
    img.text(24, 44, 1, &sub, DIM);
    let (x0, bw) = (left, w as i64 - left - right);
    let top = 84i64;
    let xs = |t: f64| x0 as f64 + t / xmax * bw as f64;
    let mut t = 0.0;
    while t <= xmax + 1e-9 {
        img.line(xs(t), top as f64 - 6.0, xs(t), (top + row * order.len() as i64) as f64, GRID, 1.0);
        let s = format!("{t:.0}");
        img.text(xs(t) as i64 - 4 * s.len() as i64, top + row * order.len() as i64 + 8, 1, &s, DIM);
        t += 20.0;
    }
    img.text(x0 + bw / 2 - 40, top + row * order.len() as i64 + 28, 1, "time (s)", TEXT);
    for (i, l) in order.iter().enumerate() {
        let y = top + row * i as i64;
        let c = class_colour(l.class);
        img.fill_rect(24, y + 6, 10, 10, c, 1.0);
        img.text(44, y + 7, 1, l.name, TEXT);
        img.text(44 + 8 * (l.name.len() as i64 + 1), y + 7, 1, l.class, DIM);
        match &l.run.outcome {
            Outcome::Finished { ticks, .. } => {
                let t = *ticks as f64 / HZ as f64;
                match l.dry_s {
                    // What it would take on concrete, then what the soft earth added.
                    Some(dry) if dry < t - 0.05 => {
                        img.fill_rect(x0, y + 3, (xs(dry) - x0 as f64) as i64, row - 8, c, 0.9);
                        img.fill_rect(xs(dry) as i64, y + 3, (xs(t) - xs(dry)) as i64, row - 8, EARTH, 0.95);
                        img.text(xs(t) as i64 + 8, y + 7, 1, &format!("{t:.1} s   (+{:.1} s of soft earth)", t - dry), TEXT);
                    }
                    _ => {
                        img.fill_rect(x0, y + 3, (xs(t) - x0 as f64) as i64, row - 8, c, 0.9);
                        img.text(xs(t) as i64 + 8, y + 7, 1, &format!("{t:.1} s"), TEXT);
                    }
                }
            }
            Outcome::Dnf { cause, s_mm, ticks, .. } => {
                let t = *ticks as f64 / HZ as f64;
                if let Some(dry) = l.dry_s {
                    // The ghost of the dry run: how quickly it would have finished.
                    img.fill_rect(x0, y + 3, (xs(dry) - x0 as f64) as i64, row - 8, c, 0.12);
                }
                img.fill_rect(x0, y + 3, (xs(t.min(xmax)) - x0 as f64) as i64, row - 8, c, 0.3);
                img.fill_rect(xs(t.min(xmax)) as i64 - 3, y + 1, 6, row - 4, BAD, 1.0);
                img.text(xs(t.min(xmax)) as i64 + 12, y + 7, 1, &format!("DID NOT FINISH: {cause:?} at {:.0} m, after {t:.0} s", *s_mm as f64 / 1000.0), BAD);
            }
            Outcome::Dns { cause } => {
                img.text(x0 + 10, y + 7, 1, &format!("DID NOT START: {cause}"), [1.0, 0.7, 0.3]);
            }
        }
    }
    if let Err(e) = img.save_png(path) {
        eprintln!("error: {e}");
    }
}
