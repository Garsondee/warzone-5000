//! The component mass budget (design note `docs/lanes/forge/mass-budget-note.md`, D2a): sprung mass is the structure plus the parts, each
//! part's mass a function of the choice that sizes it, so mass, COM and inertia follow the configuration. Absent `mass_budget`, the def's
//! `hull.mass_kg` at its COM is the whole sprung mass (the old lump).

use w5k_contract::def::{HullDef, VehicleDef};
use w5k_math::{Mat3, Vec3};

use crate::extras::{Extras, Loading, PartPos};

const W_PER_KW: f64 = 1e3; // const-ok: unit conversion

/// One lump of the sprung mass: a name, its mass, and where it sits in the hull frame (datum at the hull box centre, +Z back).
#[derive(Clone, Debug, PartialEq)]
pub struct MassItem {
    pub name: String,
    pub mass_kg: f64,
    pub pos_m: Vec3,
}

/// The sprung mass of a design: total, centre of mass and the lumps it came from (empty without a budget).
#[derive(Clone, Debug, PartialEq)]
pub struct Sprung {
    pub mass_kg: f64,
    pub com_m: Vec3,
    pub items: Vec<MassItem>,
}

/// `drive_z`: the hull-frame z of each final drive (a driven axle, or a sprocket).
pub(crate) fn sprung(def: &VehicleDef, ex: &Extras, ride_height: f64, drive_z: &[f64]) -> Result<Sprung, String> {
    let h = &def.hull;
    let z_of = |from_front: f64| from_front - 0.5 * h.length_m.v;
    let Some(b) = &ex.mass_budget else {
        let com = Vec3::new(0.0, h.com_height_m.v - ride_height, z_of(h.com_from_front_m.v));
        return Ok(Sprung { mass_kg: h.mass_kg.v, com_m: com, items: vec![] });
    };
    let at = |p: &PartPos| Vec3::new(p.lateral_m.v, p.height_m.v - ride_height, z_of(p.from_front_m.v));
    let (pt, en) = (&def.powertrain, &def.powertrain.engine);
    let first_gear = pt.gearbox.forward_ratios.first().map(|p| p.v).ok_or("the gearbox has no forward ratio")?;
    let drive_torque = en.peak_torque_nm.v * first_gear * pt.final_drive_ratio.v / drive_z.len().max(1) as f64;
    let fill = if b.loading == Loading::Curb { b.fill_curb.v } else { b.fill_combat.v };
    let mut items = vec![
        MassItem { name: "structure".into(), mass_kg: b.structure_mass_kg.v, pos_m: at(&b.structure_pos) },
        MassItem {
            name: "engine".into(),
            mass_kg: b.engine_kg_per_kw.v * en.peak_power_w.v / W_PER_KW,
            pos_m: at(&b.engine_pos),
        }, // const-ok: W to kW
        MassItem {
            name: "transmission".into(),
            mass_kg: b.transmission_kg_per_nm.v * en.peak_torque_nm.v,
            pos_m: at(&b.transmission_pos),
        },
    ];
    for (i, z) in drive_z.iter().enumerate() {
        items.push(MassItem {
            name: format!("final_drive{i}"),
            mass_kg: b.final_drive_kg_per_nm.v * drive_torque,
            pos_m: Vec3::new(0.0, b.final_drive_height_m.v - ride_height, *z),
        });
    }
    let (litres, kg_l) = (b.tank_litres.v, b.fuel_density_kg_l.v);
    items.push(MassItem {
        name: "fuel".into(),
        mass_kg: litres * (kg_l * fill + b.tank_kg_per_litre.v),
        pos_m: at(&b.fuel_pos),
    });
    items.push(MassItem { name: "crew".into(), mass_kg: b.crew_count.v * b.crew_kg_each.v, pos_m: at(&b.crew_pos) });
    if let Some(bad) = items.iter().find(|m| !(m.mass_kg > 0.0 && m.mass_kg.is_finite())) {
        return Err(format!(
            "mass budget: the {} has mass {:.1} kg: every part must have a positive mass",
            bad.name, bad.mass_kg
        ));
    }
    let mass_kg: f64 = items.iter().map(|m| m.mass_kg).sum();
    let com_m = items.iter().fold(Vec3::ZERO, |a, m| a + m.pos_m * m.mass_kg) / mass_kg;
    Ok(Sprung { mass_kg, com_m, items })
}

/// The point-mass term of the parallel-axis theorem: `m (|d|^2 1 - d d^T)`.
fn shift(m: f64, d: Vec3) -> Mat3 {
    Mat3::diagonal(1.0, 1.0, 1.0).scaled(d.dot(d)).add(&Mat3::outer(d, d).scaled(-1.0)).scaled(m)
}

/// The hull body and its box size. Without a budget: one uniform box of the whole sprung mass, moved to the COM. With one: the structure
/// as a uniform box about the hull box centre, plus every other part as a point mass, all about the common COM (parallel-axis theorem).
pub(crate) fn hull_body(h: &HullDef, s: &Sprung) -> (w5k_contract::rig::BodyDef, Vec3) {
    let size = Vec3::new(h.width_m.v, h.height_m.v, h.length_m.v);
    let box_inertia = |m: f64| {
        let k = m / 12.0; // const-ok: solid box inertia 1/12
        Mat3::diagonal(
            k * (size.y * size.y + size.z * size.z),
            k * (size.x * size.x + size.z * size.z),
            k * (size.x * size.x + size.y * size.y),
        )
    };
    let structure_mass = s.items.first().map_or(s.mass_kg, |m| m.mass_kg);
    let mut inertia = box_inertia(structure_mass).add(&shift(structure_mass, s.com_m)); // the box centre is the datum
    for m in s.items.iter().skip(1) {
        inertia = inertia.add(&shift(m.mass_kg, m.pos_m - s.com_m));
    }
    (
        w5k_contract::rig::BodyDef { name: "hull".into(), mass_kg: s.mass_kg, com_m: s.com_m, inertia_kg_m2: inertia },
        size,
    )
}
