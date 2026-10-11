//! The dashboard: one self-contained HTML page (inline CSS and SVG, no external URL, no script).

use std::fmt::Write;

use crate::capability::{Capability, Entry};
use crate::dossier::{Dossier, Evidence};
use crate::verdict::{Light, Row};

/// What the page needs to tell the owner honestly what was scored.
pub struct Page<'a> {
    pub title: &'a str,
    /// One line: which replay and which design sheet were scored (and whether the model is a stand-in).
    pub subject: &'a str,
    pub dossier: &'a Dossier,
    pub rows: &'a [Row],
}

const METER_WIDTH: f64 = 480.0; // const-ok: pixel width of the provenance meter

/// The page's styles: colour tokens with a dark-mode override, and the layout.
const CSS: &str = ":root{--bg:#fff;--fg:#1d2330;--mute:#5a6275;--line:#d8dce6;--green:#1a7f4b;--amber:#b86e00;--red:#c0392b;--grey:#8a91a3}@media(prefers-color-scheme:dark){:root{--bg:#12151c;--fg:#e6e9f2;--mute:#9aa3b8;--line:#2a3040;--green:#3fb97a;--amber:#e0a030;--red:#ee6a5b;--grey:#7c849a}}body{margin:0 auto;max-width:960px;padding:16px;background:var(--bg);color:var(--fg);font:15px/1.45 system-ui,sans-serif}table{border-collapse:collapse;width:100%}td,th{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top}.n{font-variant-numeric:tabular-nums;text-align:right}small,em{color:var(--mute)}.legend{list-style:none;padding:0;display:flex;gap:16px;flex-wrap:wrap}.sw{display:inline-block;width:10px;height:10px;margin-right:6px}.wrap{overflow-x:auto}";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn colour(l: Light) -> &'static str {
    match l {
        Light::Green => "var(--green)",
        Light::Amber => "var(--amber)",
        Light::Red => "var(--red)",
        Light::NotMeasured => "var(--grey)",
    }
}

/// A traffic light that does not rely on colour alone: the shape differs (circle, triangle, square, dashed ring) and the word follows it.
fn light_svg(l: Light) -> String {
    let c = colour(l);
    let shape = match l {
        Light::Green => format!("<circle cx='9' cy='9' r='7' fill='{c}'/>"),
        Light::Amber => format!("<path d='M9 2 L16 16 H2 Z' fill='{c}'/>"),
        Light::Red => format!("<rect x='2' y='2' width='14' height='14' fill='{c}'/>"),
        Light::NotMeasured => {
            format!("<circle cx='9' cy='9' r='6' fill='none' stroke='{c}' stroke-width='2' stroke-dasharray='3 2'/>")
        }
    };
    format!("<svg width='18' height='18' viewBox='0 0 18 18' role='img' aria-label='{}'>{shape}</svg>", l.word())
}

fn fmt(v: f64) -> String {
    // const-ok: display thresholds for scientific notation
    if v != 0.0 && (v.abs() >= 1e5 || v.abs() < 1e-2) {
        format!("{v:.3e}")
    } else {
        format!("{v:.4}").trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// A stacked bar of the dossier's evidence: primary, secondary (unverified) and open-model cross-checks.
fn meter(d: &Dossier) -> String {
    let n = |e| d.quantities.iter().filter(|q| q.evidence == e).count() as f64;
    let parts = [
        (n(Evidence::Primary), "var(--green)", "primary"),
        (n(Evidence::Secondary), "var(--amber)", "secondary (unverified)"),
        (n(Evidence::Cross), "var(--grey)", "cross-check only"),
    ];
    let total: f64 = parts.iter().map(|p| p.0).sum::<f64>().max(1.0);
    let (mut x, mut bar, mut legend) = (0.0, String::new(), String::new());
    for (count, c, name) in parts {
        let w = METER_WIDTH * count / total;
        let _ = write!(bar, "<rect x='{x:.1}' y='0' width='{w:.1}' height='18' fill='{c}'/>");
        let _ = write!(legend, "<li><span class='sw' style='background:{c}'></span>{name}: {count}</li>");
        x += w;
    }
    format!("<svg width='480' height='18' viewBox='0 0 480 18' role='img' aria-label='provenance meter'>{bar}</svg><ul class='legend'>{legend}</ul>")
}

fn light_of(word: &str) -> Light {
    match word {
        "green" => Light::Green,
        "amber" => Light::Amber,
        "red" => Light::Red,
        _ => Light::NotMeasured,
    }
}

fn cell(e: &Option<Entry>, applicable: bool) -> String {
    match e {
        Some(e) => {
            let note = if e.note.is_empty() { String::new() } else { format!("<br><small>{}</small>", esc(&e.note)) };
            format!(
                "<td>{} {} <span class='n'>{}</span> {}{}</td>",
                light_svg(light_of(e.light)),
                esc(e.light),
                fmt(e.value),
                esc(e.unit),
                note
            )
        }
        None if !applicable => "<td><small>does not apply</small></td>".to_string(),
        None => "<td><small>missing</small></td>".to_string(),
    }
}

/// The per-vehicle by per-test grid of the proving ground, from the capability tables: one cell per (vehicle, test) with its light,
/// value, unit and what limited it. A red cell is a model result that failed its physics check, exported as it is.
pub fn render_grid(title: &str, tables: &[Capability]) -> String {
    let mut rows = String::new();
    for t in tables {
        let na = |test: &str| !t.not_applicable.iter().any(|n| n == test);
        let _ = write!(
            rows,
            "<tr><th>{}</th>{}{}{}{}{}{}</tr>",
            esc(&t.vehicle),
            cell(&t.braking_distance_m, na("braking_50kmh")),
            cell(&t.time_0_48kmh_s, na("accel_0_48kmh")),
            cell(&t.max_grade_ratio, na("gradeability")),
            cell(&t.max_side_slope_rad, na("side_slope_rollover")),
            cell(&t.max_lateral_accel_m_s2, na("skidpad")),
            cell(&t.max_step_m, na("step_climb")),
        );
    }
    format!(
        "<!doctype html><html lang='en'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><title>{title}</title><style>{css}</style></head><body>\
<h1>{title}</h1><p>Each cell is a proving-ground test run on the vehicle: its traffic light (shape and word), the measured value, and what limited it. A red cell is a model result that failed its physics check (an oracle or a bound), shown as it is. Nothing is tuned.</p>\
<div class='wrap'><table><thead><tr><th>vehicle</th><th>braking from 50 km/h</th><th>0-48 km/h</th><th>max grade (rise/run)</th><th>side slope (rad)</th><th>skidpad lateral accel</th><th>step climb</th></tr></thead><tbody>{rows}</tbody></table></div>\
<p><small>\"does not apply\": skidpad and step climb are not run on a tracked vehicle (no steered wheel; a belt is a different obstacle problem). \"not scored\" means measured, with no oracle yet.</small></p></body></html>",
        title = esc(title),
        css = CSS,
    )
}

pub fn render(p: &Page) -> String {
    let count = |l| p.rows.iter().filter(|r| r.light == l).count();
    let mut body = String::new();
    for r in p.rows {
        let sim = r.simulated.map_or("n/a".to_string(), fmt);
        let band = if (r.band.1 - r.band.0).abs() <= f64::EPSILON * r.published.abs() {
            String::new()
        } else {
            format!(" (band {} to {})", fmt(r.band.0), fmt(r.band.1))
        };
        let prov = if r.provisional { " <em>provisional: published figure is UNVERIFIED</em>" } else { "" };
        let _ = write!(
            body,
            "<tr><td>{}</td><td class='n'>{}{}</td><td class='n'>{}</td><td>{}</td><td>{} {}{}<br><small>{}</small></td></tr>",
            esc(&r.id),
            fmt(r.published),
            esc(&band),
            sim,
            esc(&r.unit),
            light_svg(r.light),
            r.light.word(),
            prov,
            esc(&r.note)
        );
    }
    format!(
        "<!doctype html><html lang='en'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><title>{title}</title><style>\
{css}</style></head><body>\
<h1>{title}</h1><p>{subject}</p>\
<p><b>{g}</b> green, <b>{a}</b> amber, <b>{r}</b> red, <b>{u}</b> not measured yet. Nothing here has been tuned; reds stay red until the model or a card changes.</p>\
<h2>Published versus simulated: {name}</h2><div class='wrap'><table><thead><tr><th>Quantity</th><th>Published</th><th>Simulated</th><th>Unit</th><th>Verdict</th></tr></thead><tbody>{body}</tbody></table></div>\
<h2>How well the published figures are evidenced</h2>{meter}\
<p><small>v0: one nominal run, no Monte Carlo envelope yet; the published band stands in for it (PROVISIONAL). Tolerances: ADR-0007.</small></p>\
</body></html>",
        css = CSS,
        title = esc(p.title),
        subject = esc(p.subject),
        name = esc(&p.dossier.name),
        g = count(Light::Green),
        a = count(Light::Amber),
        r = count(Light::Red),
        u = count(Light::NotMeasured),
        meter = meter(p.dossier),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_is_self_contained() {
        let d = Dossier::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/dossier/m998.ron"))
            .expect("m998");
        let q = d.scored().next().expect("a quantity");
        let rows = vec![
            crate::verdict::not_measured(q, "<b>&"),
            crate::verdict::judge(q, f64::NAN, crate::verdict::Class::Static),
        ];
        let html = render(&Page { title: "Validation", subject: "a <test>", dossier: &d, rows: &rows });
        for banned in ["http://", "https://", "<script", "<link", "<img", "src=", "href=", "@import", "url("] {
            assert!(!html.contains(banned), "page contains {banned}");
        }
        assert!(html.contains("&lt;b&gt;&amp;") && html.contains("a &lt;test&gt;"), "text is escaped");
        assert!(html.contains("aria-label='red'"), "a non-finite result is red and says so in words");
    }

    #[test]
    fn the_proving_grid_is_self_contained_and_says_does_not_apply_for_a_tracked_vehicle() {
        let mut tracked = Capability { vehicle: "carrier <x>".into(), ..Capability::default() };
        tracked.not_applicable = vec!["skidpad".into(), "step_climb".into()];
        tracked.braking_distance_m = Some(Entry {
            value: 22.3,
            unit: "m",
            test: "braking_50kmh".into(),
            light: "red",
            note: "brakes set the distance".into(),
        });
        let html = render_grid("Proving ground", &[tracked]);
        for banned in ["http://", "https://", "<script", "<link", "<img", "src=", "href=", "@import", "url("] {
            assert!(!html.contains(banned), "page contains {banned}");
        }
        assert!(
            html.contains("does not apply") && html.contains("aria-label='red'") && html.contains("carrier &lt;x&gt;")
        );
        assert!(html.contains("<small>missing</small>"), "an applicable test with no value says missing");
    }
}
