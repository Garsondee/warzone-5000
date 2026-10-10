//! A tiny software canvas for report images: filled rectangles, lines and text in a 3x5 pixel font. It returns raw RGB; PNG encoding is
//! the tools' job (image output stays out of simulation crates). The font is the kind of thing a graphics person would call a bitmap font
//! atlas, here as data: each glyph is five rows of three pixels.

/// Glyphs: the character and its five rows (`#` = lit).
const GLYPHS: &[(char, [&str; 5])] = &[
    ('A', [".#.", "#.#", "###", "#.#", "#.#"]),
    ('B', ["##.", "#.#", "##.", "#.#", "##."]),
    ('C', [".##", "#..", "#..", "#..", ".##"]),
    ('D', ["##.", "#.#", "#.#", "#.#", "##."]),
    ('E', ["###", "#..", "##.", "#..", "###"]),
    ('F', ["###", "#..", "##.", "#..", "#.."]),
    ('G', [".##", "#..", "#.#", "#.#", ".##"]),
    ('H', ["#.#", "#.#", "###", "#.#", "#.#"]),
    ('I', ["###", ".#.", ".#.", ".#.", "###"]),
    ('J', ["..#", "..#", "..#", "#.#", ".#."]),
    ('K', ["#.#", "#.#", "##.", "#.#", "#.#"]),
    ('L', ["#..", "#..", "#..", "#..", "###"]),
    ('M', ["#.#", "###", "###", "#.#", "#.#"]),
    ('N', ["##.", "#.#", "#.#", "#.#", "#.#"]),
    ('O', [".#.", "#.#", "#.#", "#.#", ".#."]),
    ('P', ["##.", "#.#", "##.", "#..", "#.."]),
    ('Q', [".#.", "#.#", "#.#", "##.", ".##"]),
    ('R', ["##.", "#.#", "##.", "#.#", "#.#"]),
    ('S', [".##", "#..", ".#.", "..#", "##."]),
    ('T', ["###", ".#.", ".#.", ".#.", ".#."]),
    ('U', ["#.#", "#.#", "#.#", "#.#", "###"]),
    ('V', ["#.#", "#.#", "#.#", "#.#", ".#."]),
    ('W', ["#.#", "#.#", "###", "###", "#.#"]),
    ('X', ["#.#", "#.#", ".#.", "#.#", "#.#"]),
    ('Y', ["#.#", "#.#", ".#.", ".#.", ".#."]),
    ('Z', ["###", "..#", ".#.", "#..", "###"]),
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('1', [".#.", "##.", ".#.", ".#.", "###"]),
    ('2', ["##.", "..#", ".#.", "#..", "###"]),
    ('3', ["##.", "..#", ".#.", "..#", "##."]),
    ('4', ["#.#", "#.#", "###", "..#", "..#"]),
    ('5', ["###", "#..", "##.", "..#", "##."]),
    ('6', [".##", "#..", "###", "#.#", "###"]),
    ('7', ["###", "..#", ".#.", ".#.", ".#."]),
    ('8', ["###", "#.#", "###", "#.#", "###"]),
    ('9', ["###", "#.#", "###", "..#", "##."]),
    ('.', ["...", "...", "...", "...", ".#."]),
    (',', ["...", "...", "...", ".#.", "#.."]),
    ('-', ["...", "...", "###", "...", "..."]),
    ('+', ["...", ".#.", "###", ".#.", "..."]),
    ('%', ["#.#", "..#", ".#.", "#..", "#.#"]),
    (':', ["...", ".#.", "...", ".#.", "..."]),
    ('/', ["..#", "..#", ".#.", "#..", "#.."]),
    ('(', [".#.", "#..", "#..", "#..", ".#."]),
    (')', [".#.", "..#", "..#", "..#", ".#."]),
    ('=', ["...", "###", "...", "###", "..."]),
    ('<', ["..#", ".#.", "#..", ".#.", "..#"]),
    ('>', ["#..", ".#.", "..#", ".#.", "#.."]),
    ('_', ["...", "...", "...", "...", "###"]),
    ('^', [".#.", "#.#", "...", "...", "..."]),
];

/// Width of one character cell at scale 1, pixels (three of glyph and one of gap).
pub const CHAR_W: usize = 4;
/// Height of one text line at scale 1, pixels (five of glyph and two of gap).
pub const LINE_H: usize = 7;

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub rgb: Vec<u8>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, bg: [u8; 3]) -> Canvas {
        let mut rgb = Vec::with_capacity(w * h * 3);
        for _ in 0..w * h {
            rgb.extend_from_slice(&bg);
        }
        Canvas { w, h, rgb }
    }

    pub fn put(&mut self, x: i64, y: i64, c: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = (y as usize * self.w + x as usize) * 3;
            self.rgb[i..i + 3].copy_from_slice(&c);
        }
    }

    pub fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, c);
            }
        }
    }

    /// Line between two pixels (Bresenham).
    pub fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Text at pixel `(x, y)` (top left), upper-cased, `scale` pixels per font pixel.
    pub fn text(&mut self, x: i64, y: i64, s: &str, scale: i64, c: [u8; 3]) {
        let mut cx = x;
        for ch in s.chars() {
            let up = ch.to_ascii_uppercase();
            if let Some((_, rows)) = GLYPHS.iter().find(|(g, _)| *g == up) {
                for (r, row) in rows.iter().enumerate() {
                    for (k, b) in row.bytes().enumerate() {
                        if b == b'#' {
                            self.rect(cx + k as i64 * scale, y + r as i64 * scale, scale, scale, c);
                        }
                    }
                }
            }
            cx += CHAR_W as i64 * scale;
        }
    }
}
