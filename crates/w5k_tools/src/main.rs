//! The `w5k` command line. Each lane owns exactly one module in `cmd/`; this dispatcher is ARCH's.

mod cmd;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("arch") => cmd::arch::run(&args[1..]),
        Some("chassis") => cmd::chassis::run(&args[1..]),
        Some("drive") if cmd::arch_drive::wants(&args[1..]) => cmd::arch_drive::run(&args[1..]),
        Some("drive") => cmd::drive::run(&args[1..]),
        Some("tracks") => cmd::tracks::run(&args[1..]),
        Some("world") => cmd::world::run(&args[1..]),
        Some("forge") => cmd::forge::run(&args[1..]),
        Some("geometry") => cmd::geometry::run(&args[1..]),
        Some("look") => cmd::look::run(&args[1..]),
        Some("viewer") => cmd::viewer::run(&args[1..]),
        Some("godot") => cmd::godot::run(&args[1..]),
        Some("validation") => cmd::validation::run(&args[1..]),
        Some("combat") => cmd::combat::run(&args[1..]),
        Some("ai") => cmd::ai::run(&args[1..]),
        Some("scenario") => scenario(&args[1..]),
        _ => Err(format!("usage: w5k <lane|scenario> ... (lanes: {})", LANES.join(", "))),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

const LANES: [&str; 13] = [
    "arch",
    "chassis",
    "drive",
    "tracks",
    "world",
    "forge",
    "geometry",
    "look",
    "viewer",
    "godot",
    "validation",
    "combat",
    "ai",
];

/// `w5k scenario first-light --out DIR`: run the integration spine on the stand-ins (see w5k_sim).
/// `w5k scenario mule-course --out DIR`: the Mule with DRIVE's real powertrain on WORLD's slice course (ARCH glue).
/// `w5k scenario course-compare --course FILE --vehicles a.ron,b.ron --out DIR`: N vehicles over one course, one replay, a report.
/// `w5k scenario proving --vehicle a.ron[,b.ron] --test <name|all> [--out PATH]`: scripted proving-ground tests, one result JSON each.
fn scenario(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("mule-course") => cmd::arch_course::mule_course(&args[1..]),
        Some("course-compare") => cmd::arch_compare::course_compare(&args[1..]),
        Some("proving") => cmd::arch_proving::proving(&args[1..]),
        _ => w5k_sim::cli::scenario(args),
    }
}
