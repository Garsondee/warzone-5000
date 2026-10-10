//! Writes an SVG of the stand-in dart's drag-bent arc over its drag-free parabola: `cargo run -p w5k_combat --example trajectory -- out.svg`.
use w5k_combat::ballistics::{step, Ballistics, Flyer, Particle};
use w5k_math::{scalar, Vec3};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "trajectory.svg".into());
    let b = Ballistics::from_ron(include_str!("../../../content/combat/ballistics.ron")).expect("ballistics.ron");
    let f = Flyer::new(&b.air, &b.projectiles[0]);
    let (v, th) = (1650.0, scalar::deg_to_rad(4.0));
    let mut s = Particle { pos_m: Vec3::ZERO, vel_m_s: Vec3::new(v * scalar::cos(th), v * scalar::sin(th), 0.0) };
    let (mut drag, mut free) = (String::new(), String::new());
    let range_free = v * v * scalar::sin(2.0 * th) / scalar::G;
    let mut t = 0.0;
    while s.pos_m.y >= 0.0 && t < 20.0 {
        drag += &format!("{:.1},{:.1} ", 60.0 + s.pos_m.x * 0.02, 300.0 - s.pos_m.y * 0.25);
        let (x, y) = (v * scalar::cos(th) * t, v * scalar::sin(th) * t - 0.5 * scalar::G * t * t);
        if y >= 0.0 {
            free += &format!("{:.1},{:.1} ", 60.0 + x * 0.02, 300.0 - y * 0.25);
        }
        s = step(&f, s, 1.0e-3);
        t += 1.0e-3;
    }
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' style='display:block' width='900' height='340' font-family='sans-serif' font-size='13'><rect width='100%' height='100%' fill='white'/><text x='60' y='24' font-weight='bold' font-size='15'>Stand-in dart, 1650 m/s at 4 degrees: drag-free parabola (blue) and the drag-bent arc (red)</text><polyline fill='none' stroke='#2980b9' stroke-width='2' points='{free}'/><polyline fill='none' stroke='#c0392b' stroke-width='2' points='{drag}'/><text x='60' y='326'>range {range_free:.0} m drag-free, {:.0} m with drag; 1 px = 50 m along, 4 m up</text></svg>",
        s.pos_m.x
    );
    std::fs::write(out, svg).expect("write svg");
}
