//! Mixing and matching: the movement, weapon and sensor families, their physics, and the explorer (roller and
//! auto-fit) that combines them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use w5k_forge::assemble::{hover_power_w, VehicleSheet};
use w5k_forge::explore::{Explorer, Spec, GEARS, HULLS};
use w5k_forge::family::{self, antigrav, beam, legs, missile, rail, rotor, wheel, Family, Values};
use w5k_forge::schema::{Attach, DesignDef, MaterialLibrary};
use w5k_forge::Forge;

fn lib() -> MaterialLibrary {
    let text = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/materials.ron")).unwrap();
    ron::from_str(&text).unwrap()
}

fn attach(socket: &str, family: &str, params: &[(&str, f64)]) -> Attach {
    Attach {
        socket: socket.into(),
        part: String::new(),
        family: Some(family.into()),
        params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect::<BTreeMap<_, _>>(),
        mirror: false,
        spin: 0.0,
        children: vec![],
    }
}

fn design(hull: &str, hull_params: &[(&str, f64)], attaches: Vec<Attach>) -> DesignDef {
    DesignDef {
        id: "t".into(),
        name: "t".into(),
        hull: hull.into(),
        hull_params: hull_params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        palette: None,
        attach: attaches,
        lift_m: 0.0,
    }
}

/// Quick sheet of a family-level design.
fn sheet(d: &DesignDef) -> VehicleSheet {
    let mut forge = Forge::new(lib());
    let inst = forge.instantiate(d).unwrap();
    forge.quick_design(&inst).1
}

fn skiff_hover(armour: f64) -> DesignDef {
    design(
        "hull_skiff",
        &[("length_m", 10.0), ("width_m", 4.4), ("height_m", 1.1), ("front_mm", armour), ("side_mm", armour)],
        vec![attach("belly", "hover", &[("cushion_kpa", 8.0), ("skirt_m", 0.6)]), attach("engine", "engine", &[("power_kw", 5000.0), ("tech", 2.0)])],
    )
}

#[test]
fn wheel_rating_follows_the_contact_patch() {
    // Rated load = pressure x width x (0.35 x diameter) / g.
    let v = wheel::Wheel.with(&[("diameter_m", 1.0), ("width_m", 0.4), ("pressure_kpa", 300.0)]);
    let want = 300e3 * 0.4 * 0.35 * 1.0 / 9.81;
    assert!((wheel::rated_load_kg(&v) - want).abs() < 1e-6 * want);
    let wide = wheel::Wheel.with(&[("diameter_m", 1.0), ("width_m", 0.8), ("pressure_kpa", 300.0)]);
    assert!((wheel::rated_load_kg(&wide) / wheel::rated_load_kg(&v) - 2.0).abs() < 1e-9);
    // The generated tyre touches the ground: lowest point exactly `hip_height` under the mount.
    let mut v = wheel::Wheel.defaults();
    v.insert("ctx.hip_height".into(), 0.7);
    let def = wheel::Wheel.generate(&v, &lib());
    let pieces = w5k_forge::build::build_part(&def);
    let (lo, _) = w5k_forge::voxel::bounds(&pieces);
    assert!((lo.y + 0.7).abs() < 0.03, "{}", lo.y);
}

#[test]
fn walker_legs_follow_the_square_cube_law() {
    let lib = lib();
    let base = legs::Legs.with(&[("stance_m", 3.0), ("thickness", 0.08)]);
    let tall = legs::Legs.with(&[("stance_m", 6.0), ("thickness", 0.08)]);
    let fat = legs::Legs.with(&[("stance_m", 3.0), ("thickness", 0.16)]);
    // Same slenderness, twice the size: a leg that is twice as long (and so twice as thick) carries about four times
    // the crushing load, but a *taller* leg of the same fraction of its length is stronger in absolute terms.
    assert!(legs::rated_load_kg(&tall, &lib) > legs::rated_load_kg(&base, &lib));
    // Thicker walls carry more.
    assert!(legs::rated_load_kg(&fat, &lib) > 2.0 * legs::rated_load_kg(&base, &lib));
    // The hip actuator that holds a heavier leg is heavier.
    assert!(legs::hip_actuator_kg(&tall, &lib) > legs::hip_actuator_kg(&base, &lib));
    // Fitting to a load gives a leg that carries it with a margin, and a bigger load asks for a fatter leg.
    let mut v = base.clone();
    legs::Legs.fit_to_load(&mut v, 8000.0, &lib);
    assert!(legs::rated_load_kg(&v, &lib) >= 8000.0 * 1.2);
    let mut w = base.clone();
    legs::Legs.fit_to_load(&mut w, 30_000.0, &lib);
    assert!(w["thickness"] > v["thickness"] && w["foot_m"] > v["foot_m"]);
}

#[test]
fn rail_rolls_easily_and_is_bound_to_the_track() {
    let d = design(
        "hull_dreadnought",
        &[("length_m", 24.0), ("stations", 4.0), ("turrets", 2.0)],
        vec![attach("keel_*", "rail", &[("axle_load_t", 30.0), ("axles", 3.0)]), attach("engine", "engine", &[("power_kw", 2000.0), ("tech", 1.0)])],
    );
    let s = sheet(&d);
    assert_eq!(s.movement, "rail");
    let part = rail::Rail.generate(&rail::Rail.defaults(), &lib());
    assert!(part.function.rail_bound);
    assert_eq!(part.function.rolling, Some(0.0015));
    // Rating = axles x axle load.
    let v = rail::Rail.with(&[("axles", 3.0), ("axle_load_t", 30.0)]);
    assert!((rail::rated_load_kg(&v) - 90_000.0).abs() < 1.0);
}

#[test]
fn air_cushion_lift_power_grows_as_weight_to_the_three_halves() {
    let (a, b) = (sheet(&skiff_hover(10.0)), sheet(&skiff_hover(80.0)));
    assert!(b.mass_kg > 1.3 * a.mass_kg, "{} -> {}", a.mass_kg, b.mass_kg);
    assert!(a.problems.iter().all(|p| !p.contains("cannot")), "{:?}", a.problems);
    // Same footprint, perimeter and gap: only the weight changes, so power follows W^1.5.
    let ratio = b.lift_kw / a.lift_kw;
    let want = (b.mass_kg / a.mass_kg).powf(1.5);
    assert!((ratio / want - 1.0).abs() < 0.01, "{ratio} vs {want}");
    // The cushion floats it at a few kilopascals.
    assert!(a.ground_pressure_kpa > 0.0 && a.ground_pressure_kpa < 10.0, "{}", a.ground_pressure_kpa);
}

#[test]
fn anti_gravity_costs_power_per_tonne_and_raises_the_hull() {
    let d = design(
        "hull_skiff",
        &[("length_m", 12.0), ("width_m", 5.0), ("height_m", 1.4), ("front_mm", 20.0), ("side_mm", 10.0), ("stations", 2.0)],
        vec![
            attach("station_*", "antigrav", &[("lift_t", 40.0), ("kw_per_t", 30.0), ("ride_m", 10.0)]),
            attach("engine", "engine", &[("power_kw", 12000.0), ("tech", 2.0)]),
        ],
    );
    let s = sheet(&d);
    let want = 30.0 * (s.mass_kg / 1000.0) * (1.0 + 10.0 / 25.0);
    assert!((s.lift_kw / want - 1.0).abs() < 1e-6, "{} vs {want}", s.lift_kw);
    assert_eq!(s.ground_pressure_kpa, 0.0, "nothing touches the ground");
    assert!(s.lift_height_m > 8.0, "the hull rides high: {}", s.lift_height_m);
    // The pod's own rating and power figure.
    assert!((antigrav::hold_kw(10.0, 30.0, 25.0) - 600.0).abs() < 1e-9);
    // A cheaper field needs more coil rings: a heavier pod.
    let light = antigrav::AntiGrav.generate(&antigrav::AntiGrav.with(&[("kw_per_t", 80.0)]), &lib());
    let heavy = antigrav::AntiGrav.generate(&antigrav::AntiGrav.with(&[("kw_per_t", 9.0)]), &lib());
    let mass = |def: w5k_forge::schema::PartDef| {
        let id = def.id.clone();
        Forge::from_parts(lib(), vec![def]).unwrap().build_part(&id).unwrap().mass.mass_kg
    };
    assert!(mass(heavy) > mass(light));
}

#[test]
fn doubling_the_rotor_disc_halves_hover_power() {
    let w = 50_000.0;
    let (p1, p4) = (hover_power_w(w, 10.0), hover_power_w(w, 40.0));
    assert!((p4 / p1 - 0.5).abs() < 1e-9, "radius x2 = area x4: {}", p4 / p1);
    // Weight^1.5.
    assert!((hover_power_w(2.0 * w, 10.0) / p1 - 2f64.powf(1.5)).abs() < 1e-9);
    // Rated lift: thrust at the blade-stall limit grows with disc area, blades and tip speed squared.
    let base = rotor::Rotor.with(&[("radius_m", 2.0), ("blades", 3.0), ("tip_ms", 200.0)]);
    let r = |k: &str, x: f64| {
        let mut v = base.clone();
        v.insert(k.into(), x);
        rotor::rated_lift_kg(&v)
    };
    let l0 = rotor::rated_lift_kg(&base);
    assert!((r("radius_m", 4.0) / l0 - 4.0).abs() < 1e-9);
    assert!((r("blades", 6.0) / l0 - 2.0).abs() < 1e-9);
    assert!((r("tip_ms", 240.0) / l0 - 1.44).abs() < 1e-9);
}

#[test]
fn a_rotor_craft_hovers_when_there_is_power_and_not_when_there_is_not() {
    let make = |power: f64| {
        design(
            "hull_skiff",
            &[("length_m", 7.0), ("width_m", 2.6), ("height_m", 0.8), ("front_mm", 5.0), ("side_mm", 5.0)],
            vec![attach("hub", "rotor", &[("radius_m", 4.0), ("blades", 4.0), ("altitude_m", 10.0)]), attach("engine", "engine", &[("power_kw", power), ("tech", 2.0)])],
        )
    };
    let strong = sheet(&make(900.0));
    assert!(strong.problems.iter().all(|p| !p.contains("cannot hover")), "{:?}", strong.problems);
    assert!(strong.top_speed_kmh > 20.0, "{}", strong.top_speed_kmh);
    let weak = sheet(&make(40.0));
    assert!(weak.problems.iter().any(|p| p.contains("cannot hover")), "{:?}", weak.problems);
    assert_eq!(strong.movement, "air");
}

#[test]
fn missile_physics_trades_punch_for_range() {
    let base = missile::Missile.with(&[("tube_mm", 100.0), ("seeker", 0.0)]);
    let big = missile::Missile.with(&[("tube_mm", 200.0), ("seeker", 0.0)]);
    let (a, b) = (missile::round(&base), missile::round(&big));
    assert!((b.mass_kg / a.mass_kg - 8.0).abs() < 1e-9, "mass goes with the cube of the calibre");
    assert!((a.penetration_mm - 600.0 * (0.3f64 / 0.3).powf(0.3)).abs() < 1e-6, "shaped charge: six calibres");
    let heavy_warhead = missile::round(&missile::Missile.with(&[("tube_mm", 100.0), ("seeker", 0.0), ("warhead", 0.55)]));
    let light_warhead = missile::round(&missile::Missile.with(&[("tube_mm", 100.0), ("seeker", 0.0), ("warhead", 0.15)]));
    assert!(heavy_warhead.range_m < 0.5 * light_warhead.range_m, "{} vs {}", heavy_warhead.range_m, light_warhead.range_m);
    assert!(heavy_warhead.penetration_mm > light_warhead.penetration_mm && heavy_warhead.energy_j > light_warhead.energy_j);
    // A seeker makes it guided, and heavier.
    let guided = missile::round(&missile::Missile.with(&[("tube_mm", 100.0), ("seeker", 1.0)]));
    assert!(guided.guided && guided.mass_kg > a.mass_kg);
}

#[test]
fn beam_aperture_buys_a_tighter_spot_and_deeper_burn() {
    let narrow = beam::shot(&beam::Beam.with(&[("power_mw", 10.0), ("aperture_m", 0.2), ("dwell_s", 0.5)]));
    let wide = beam::shot(&beam::Beam.with(&[("power_mw", 10.0), ("aperture_m", 1.0), ("dwell_s", 0.5)]));
    assert!((narrow.energy_j - 5e6).abs() < 1.0, "energy = power x dwell");
    assert!(wide.spot_1km_m < narrow.spot_1km_m && wide.penetration_mm > 3.0 * narrow.penetration_mm);
    assert!(wide.range_m > narrow.range_m);
    // The capacitor bank grows with the energy stored.
    let big = beam::shot(&beam::Beam.with(&[("power_mw", 100.0), ("aperture_m", 1.0), ("dwell_s", 0.5)]));
    assert!((big.cap_kg / wide.cap_kg - 10.0).abs() < 1e-9);
}

#[test]
fn a_taller_sensor_mast_sees_farther_and_a_repair_rig_draws_power() {
    let make = |h: f64| {
        design(
            "hull_lancer",
            &[("length_m", 7.0), ("width_m", 3.2), ("height_m", 1.1)],
            vec![
                attach("gear_*", "track", &[]),
                attach("engine", "engine", &[("power_kw", 600.0), ("tech", 1.0)]),
                attach("mast_1", "sensor", &[("height_m", h), ("aperture_m", 0.6), ("radar", 0.9)]),
            ],
        )
    };
    let (low, high) = (sheet(&make(1.0)), sheet(&make(12.0)));
    assert!(high.sight_km > low.sight_km + 1.0, "{} vs {}", low.sight_km, high.sight_km);
    assert!(high.eye_height_m > low.eye_height_m + 8.0);
    // Repair rig: 25 kW for each kg/s, summed on the sheet.
    let mut d = make(2.0);
    d.attach.push(attach("mast_2", "repair", &[("reach_m", 5.0), ("rate", 4.0)]));
    let s = sheet(&d);
    assert!((s.repair_kg_s - 4.0).abs() < 1e-9 && (s.repair_reach_m - 5.0).abs() < 1e-9);
    let part = family::repair::Repair.generate(&family::repair::Repair.with(&[("rate", 4.0)]), &lib());
    assert!((part.function.draw_kw - 100.0).abs() < 1e-9);
}

#[test]
fn gun_turrets_report_their_weapon_to_the_sheet() {
    let d = design(
        "hull_lancer",
        &[("length_m", 7.0), ("width_m", 3.0), ("height_m", 1.1)],
        vec![
            attach("gear_*", "track", &[]),
            attach("engine", "engine", &[("power_kw", 600.0), ("tech", 1.0)]),
            attach("turret", "turret_gun", &[("calibre_mm", 75.0), ("armour_mm", 40.0)]),
        ],
    );
    let s = sheet(&d);
    assert_eq!(s.weapons.len(), 1);
    assert!(s.firepower_kw > 100.0 && s.best_pen_mm > 30.0 && s.range_km > 1.0, "{s:?}");
}

#[test]
fn incompatible_mounts_are_refused_with_a_reason() {
    let mut forge = Forge::new(lib());
    // A track unit on a walker's hip.
    let d = design("hull_strider", &[], vec![attach("station_*", "track", &[])]);
    let e = forge.instantiate(&d).unwrap_err();
    assert!(e.contains("does not fit"), "{e}");
    // A gun turret on a mast.
    let d = design("hull_lancer", &[], vec![attach("mast_1", "turret_gun", &[])]);
    assert!(forge.instantiate(&d).unwrap_err().contains("does not fit"));
}

#[test]
fn the_roller_is_deterministic_and_varied() {
    let ex = Explorer::new(lib());
    let spec = Spec::default();
    let a = ex.roll(42, &spec);
    let b = ex.roll(42, &spec);
    assert_eq!(ron::to_string(&a).unwrap(), ron::to_string(&b).unwrap());
    let hulls: std::collections::BTreeSet<String> = (0..40).map(|s| ex.roll(s, &spec).hull).collect();
    assert!(hulls.len() >= 4, "{hulls:?}");
    let gears: std::collections::BTreeSet<String> = (0..80)
        .flat_map(|s| ex.roll(s, &spec).attach.into_iter().filter_map(|a| a.family).filter(|f| GEARS.contains(&f.as_str())))
        .collect();
    assert!(gears.len() >= 6, "{gears:?}");
}

#[test]
fn every_mountable_hull_and_gear_combination_instantiates() {
    let ex = Explorer::new(lib());
    let mut combos = 0;
    for hull in HULLS {
        for gear in GEARS {
            let spec = Spec { hull: Some(hull.into()), gear: Some(gear.into()), ..Default::default() };
            let d = ex.roll(5, &spec);
            if !d.attach.iter().any(|a| a.family.as_deref() == Some(gear)) {
                continue; // this hull has no socket for that gear
            }
            let mut forge = Forge::new(lib());
            forge.instantiate(&d).unwrap_or_else(|e| panic!("{hull} + {gear}: {e}"));
            combos += 1;
        }
    }
    assert!(combos >= 28, "{combos} of 35 combinations have a socket");
}

#[test]
fn auto_fit_makes_most_rolled_designs_valid_and_the_gear_carries_the_weight() {
    let ex = Explorer::new(lib());
    let samples = ex.sample(100, 48, &Spec::default());
    let valid: Vec<_> = samples.iter().filter(|s| s.valid).collect();
    assert!(valid.len() * 10 >= samples.len() * 7, "{} of {} valid", valid.len(), samples.len());
    for s in &valid {
        let h = &s.sheet;
        assert!(h.load_kg == 0.0 || h.mass_kg <= h.load_kg, "{}: {} kg on gear rated {} kg", s.seed, h.mass_kg, h.load_kg);
        assert!(h.power_kw >= h.draw_kw + h.lift_kw, "{}: power {} draw {} lift {}", s.seed, h.power_kw, h.draw_kw, h.lift_kw);
        assert!(h.top_speed_kmh > 0.0 || h.movement == "ground" || h.movement == "rail" || h.movement == "hover" || h.movement == "air", "{}", s.seed);
    }
}

#[test]
fn the_fitter_thins_armour_when_the_gear_cannot_carry_it() {
    let ex = Explorer::new(lib());
    // A thick-skinned Lancer on an air cushion: far too heavy to float as asked.
    let d = design(
        "hull_lancer",
        &[("length_m", 8.0), ("width_m", 3.0), ("height_m", 1.1), ("front_mm", 200.0), ("side_mm", 120.0)],
        vec![attach("belly", "hover", &[]), attach("engine", "engine", &[("power_kw", 500.0), ("tech", 1.0)])],
    );
    let f = ex.fit(&d).unwrap();
    assert!(f.valid(), "{:?}", f.sheet.problems);
    assert!(f.diet < 1.0, "the plating had to go on a diet");
    assert!(f.spec.hull_params["front_mm"] < 200.0);
    // A tracked tank of the same hull keeps all its armour.
    let t = design(
        "hull_lancer",
        &[("length_m", 8.0), ("width_m", 3.0), ("height_m", 1.1), ("front_mm", 200.0), ("side_mm", 120.0)],
        vec![attach("gear_*", "track", &[]), attach("engine", "engine", &[("power_kw", 500.0), ("tech", 1.0)])],
    );
    let f = ex.fit(&t).unwrap();
    assert!(f.valid() && f.diet == 1.0 && f.spec.hull_params["front_mm"] == 200.0);
}

#[test]
fn the_quick_path_agrees_with_the_full_build_after_fitting() {
    let ex = Explorer::new(lib());
    for seed in [3u64, 11, 29] {
        let s = ex.sample_one(seed, &Spec::default());
        if !s.valid {
            continue;
        }
        let (_built, full, _) = ex.build(&s.spec).unwrap();
        let ratio = s.sheet.mass_kg / full.mass_kg;
        assert!((0.8..1.25).contains(&ratio), "seed {seed}: quick {} kg vs full {} kg", s.sheet.mass_kg, full.mass_kg);
    }
}

#[test]
fn fitted_designs_round_trip_through_content_files() {
    let ex = Explorer::new(lib());
    let s = ex.sample_one(8, &Spec { hull: Some("hull_skiff".into()), gear: Some("antigrav".into()), ..Default::default() });
    let text = ron::ser::to_string_pretty(&s.spec, ron::ser::PrettyConfig::default()).unwrap();
    let text = format!("#![enable(implicit_some)]\n{text}");
    let back: DesignDef = ron::from_str(&text).unwrap();
    assert_eq!(ron::to_string(&back).unwrap(), ron::to_string(&s.spec).unwrap());
}

#[test]
fn every_family_declares_where_it_mounts_or_hosts() {
    // Everything but a hull names the sockets it fits; running gear and equipment also name a host hull for sweeps.
    for f in family::all() {
        let id = f.id();
        if id.starts_with("hull_") {
            assert!(f.fits().is_empty() && f.host().is_none(), "{id}");
        } else {
            assert!(!f.fits().is_empty(), "{id} must declare its socket kinds");
            if id != "engine" {
                assert!(f.host().is_some(), "{id} needs a host for sweeps");
            }
        }
    }
    let _: Values = family::all()[0].defaults();
}
