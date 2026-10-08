//! A tiny scatter-plot toolkit on top of the software rasteriser: log or linear axes, ticks, dots, lines, a legend.

use w5k_forge::preview::nice_step;
use w5k_forge::raster::{Image, Rgb};

pub const BG: Rgb = [0.005, 0.0055, 0.008];
const PANEL: Rgb = [0.009, 0.01, 0.014];
const GRID: Rgb = [0.03, 0.033, 0.045];
const AXIS: Rgb = [0.34, 0.36, 0.42];
const TEXT: Rgb = [0.8, 0.82, 0.86];
const DIM: Rgb = [0.5, 0.52, 0.58];

pub fn hex(s: &str) -> Rgb {
    w5k_forge::raster::hex_linear(s).unwrap_or([1.0, 0.0, 1.0])
}

#[derive(Clone)]
pub struct Axis {
    pub lo: f64,
    pub hi: f64,
    pub log: bool,
    pub label: String,
    /// Tick label for a value.
    pub fmt: fn(f64) -> String,
}

impl Axis {
    pub fn lin(lo: f64, hi: f64, label: &str, fmt: fn(f64) -> String) -> Axis {
        Axis { lo, hi, log: false, label: label.into(), fmt }
    }
    pub fn log(lo: f64, hi: f64, label: &str, fmt: fn(f64) -> String) -> Axis {
        Axis { lo, hi, log: true, label: label.into(), fmt }
    }
    fn t(&self, v: f64) -> f64 {
        if self.log {
            (v.max(self.lo * 0.01).ln() - self.lo.ln()) / (self.hi.ln() - self.lo.ln())
        } else {
            (v - self.lo) / (self.hi - self.lo)
        }
    }
    fn ticks(&self) -> Vec<f64> {
        if self.log {
            // A short log axis (under two decades) would show one tick or none, so it gets 1-2-5 ticks as well.
            let steps: &[f64] = if self.hi / self.lo < 100.0 { &[1.0, 2.0, 5.0] } else { &[1.0] };
            let (a, b) = (self.lo.log10().floor() as i32, self.hi.log10().ceil() as i32);
            (a..=b).flat_map(|k| steps.iter().map(move |s| s * 10f64.powi(k))).filter(|v| *v >= self.lo * 0.999 && *v <= self.hi * 1.001).collect()
        } else {
            let step = nice_step((self.hi - self.lo) / 6.0);
            let mut v = (self.lo / step).ceil() * step;
            let mut out = Vec::new();
            while v <= self.hi + 1e-9 {
                out.push(v);
                v += step;
            }
            out
        }
    }
}

pub struct Plot {
    pub img: Image,
    l: i64,
    t: i64,
    w: i64,
    h: i64,
    xa: Axis,
    ya: Axis,
}

impl Plot {
    pub fn new(width: usize, height: usize, title: &str, subtitle: &str, xa: Axis, ya: Axis) -> Plot {
        let mut img = Image::new(width, height, BG);
        let (l, t, r, b) = (96i64, 78i64, 270i64, 70i64);
        let (w, h) = (width as i64 - l - r, height as i64 - t - b);
        img.text(24, 18, 2, title, [0.9, 0.91, 0.94]);
        img.text(24, 44, 1, subtitle, DIM);
        img.fill_rect(l, t, w, h, PANEL, 1.0);
        let p = Plot { img, l, t, w, h, xa, ya };
        p.frame()
    }

    fn frame(mut self) -> Plot {
        let (l, t, w, h) = (self.l, self.t, self.w, self.h);
        for x in self.xa.ticks() {
            let px = l as f64 + self.xa.t(x) * w as f64;
            self.img.line(px, t as f64, px, (t + h) as f64, GRID, 1.0);
            let s = (self.xa.fmt)(x);
            self.img.text(px as i64 - 4 * s.len() as i64, t + h + 8, 1, &s, DIM);
        }
        for y in self.ya.ticks() {
            let py = (t + h) as f64 - self.ya.t(y) * h as f64;
            self.img.line(l as f64, py, (l + w) as f64, py, GRID, 1.0);
            let s = (self.ya.fmt)(y);
            self.img.text(l - 8 - 8 * s.len() as i64, py as i64 - 4, 1, &s, DIM);
        }
        self.img.line(l as f64, t as f64, l as f64, (t + h) as f64, AXIS, 1.0);
        self.img.line(l as f64, (t + h) as f64, (l + w) as f64, (t + h) as f64, AXIS, 1.0);
        let xl = self.xa.label.clone();
        let yl = self.ya.label.clone();
        self.img.text(l + w / 2 - 4 * xl.len() as i64, t + h + 34, 1, &xl, TEXT);
        self.img.text(l - 40, t - 18, 1, &yl, TEXT);
        self
    }

    pub fn x(&self, v: f64) -> f64 {
        self.l as f64 + self.xa.t(v) * self.w as f64
    }

    pub fn y(&self, v: f64) -> f64 {
        (self.t + self.h) as f64 - self.ya.t(v) * self.h as f64
    }

    fn inside(&self, px: f64, py: f64) -> bool {
        px >= self.l as f64 && px <= (self.l + self.w) as f64 && py >= self.t as f64 && py <= (self.t + self.h) as f64
    }

    pub fn dot(&mut self, x: f64, y: f64, r: f64, c: Rgb, a: f32) {
        let (px, py) = (self.x(x), self.y(y));
        if !self.inside(px, py) {
            return;
        }
        let ri = r.ceil() as i64 + 1;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
                if d <= r + 0.5 {
                    let edge = ((r + 0.5 - d) as f32).clamp(0.0, 1.0);
                    self.img.blend(px as i64 + dx, py as i64 + dy, c, a * edge);
                }
            }
        }
    }

    /// A line in data coordinates, clipped to the plot panel (Liang-Barsky).
    pub fn seg(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb, a: f32) {
        let (ax, ay, bx, by) = (self.x(x0), self.y(y0), self.x(x1), self.y(y1));
        let (xmin, xmax, ymin, ymax) = (self.l as f64, (self.l + self.w) as f64, self.t as f64, (self.t + self.h) as f64);
        let (dx, dy) = (bx - ax, by - ay);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q) in [(-dx, ax - xmin), (dx, xmax - ax), (-dy, ay - ymin), (dy, ymax - ay)] {
            if p == 0.0 {
                if q < 0.0 {
                    return;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    if r > t1 {
                        return;
                    }
                    t0 = t0.max(r);
                } else {
                    if r < t0 {
                        return;
                    }
                    t1 = t1.min(r);
                }
            }
        }
        self.img.line(ax + t0 * dx, ay + t0 * dy, ax + t1 * dx, ay + t1 * dy, c, a);
    }

    /// Text at a pixel position inside the panel's data frame (for label placement that avoids overlaps).
    pub fn text_px(&mut self, px: f64, py: f64, s: &str, c: Rgb) {
        self.img.text(px as i64, py as i64, 1, s, c);
    }

    pub fn label(&mut self, x: f64, y: f64, s: &str, c: Rgb) {
        let (px, py) = (self.x(x), self.y(y));
        self.img.text(px as i64, py as i64, 1, s, c);
    }

    pub fn legend(&mut self, items: &[(String, Rgb)]) {
        let x = self.l + self.w + 24;
        let mut y = self.t;
        for (name, c) in items {
            let (cx, cy) = (x as f64 + 5.0, y as f64 + 5.0);
            for dy in -5i64..=5 {
                for dx in -5i64..=5 {
                    if ((dx * dx + dy * dy) as f64).sqrt() <= 5.0 {
                        self.img.blend(cx as i64 + dx, cy as i64 + dy, *c, 1.0);
                    }
                }
            }
            self.img.text(x + 18, y, 1, name, TEXT);
            y += 20;
        }
    }

    /// Notes under the legend, word-wrapped to the margin. `lines` stack upwards from the bottom of the plot.
    pub fn notes(&mut self, notes: &[&str]) {
        let x = self.l + self.w + 24;
        let max = 27usize;
        let mut rows: Vec<String> = Vec::new();
        for n in notes {
            let mut cur = String::new();
            for word in n.split_whitespace() {
                if !cur.is_empty() && cur.len() + 1 + word.len() > max {
                    rows.push(std::mem::take(&mut cur));
                }
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
            }
            rows.push(cur);
            rows.push(String::new());
        }
        let mut y = self.t + self.h - 14 * rows.len() as i64;
        for r in rows {
            self.img.text(x, y, 1, &r, DIM);
            y += 14;
        }
    }
}
