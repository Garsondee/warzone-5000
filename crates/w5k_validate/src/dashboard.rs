//! The dashboard: one self-contained HTML page (inline CSS and SVG, no external URL, no script).

use std::fmt::Write;

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
:root{{--bg:#fff;--fg:#1d2330;--mute:#5a6275;--line:#d8dce6;--green:#1a7f4b;--amber:#b86e00;--red:#c0392b;--grey:#8a91a3}}\
@media(prefers-color-scheme:dark){{:root{{--bg:#12151c;--fg:#e6e9f2;--mute:#9aa3b8;--line:#2a3040;--green:#3fb97a;--amber:#e0a030;--red:#ee6a5b;--grey:#7c849a}}}}\
body{{margin:0 auto;max-width:960px;padding:16px;background:var(--bg);color:var(--fg);font:15px/1.45 system-ui,sans-serif}}\
table{{border-collapse:collapse;width:100%}}td,th{{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top}}\
.n{{font-variant-numeric:tabular-nums;text-align:right}}small,em{{color:var(--mute)}}.legend{{list-style:none;padding:0;display:flex;gap:16px;flex-wrap:wrap}}\
.sw{{display:inline-block;width:10px;height:10px;margin-right:6px}}.wrap{{overflow-x:auto}}</style></head><body>\
<h1>{title}</h1><p>{subject}</p>\
<p><b>{g}</b> green, <b>{a}</b> amber, <b>{r}</b> red, <b>{u}</b> not measured yet. Nothing here has been tuned; reds stay red until the model or a card changes.</p>\
<h2>Published versus simulated: {name}</h2><div class='wrap'><table><thead><tr><th>Quantity</th><th>Published</th><th>Simulated</th><th>Unit</th><th>Verdict</th></tr></thead><tbody>{body}</tbody></table></div>\
<h2>How well the published figures are evidenced</h2>{meter}\
<p><small>v0: one nominal run, no Monte Carlo envelope yet; the published band stands in for it (PROVISIONAL). Tolerances: ADR-0007.</small></p>\
</body></html>",
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
}
