//! The design audit (`w5k.audit.v1`): every derived number of a compiled design with its unit, provenance, band and a because-line, in the
//! shape VIEWER's Workshop reads (`docs/swarm/requests/viewer-design-audit-shape.md`): `{ consequences: [rows], flags: [flags] }`, extras
//! (provenance, band, group) welcome. Consequences are read-only: the player chooses, the physics answers. Design note section 3:
//! `docs/lanes/forge/mass-budget-note.md`. Flags arrive in the next PR (thresholds as data in `content/parts/envelope.ron`).

use serde::Serialize;
use w5k_contract::def::VehicleDef;
use w5k_contract::param::{Param, Provenance};
use w5k_contract::rig::StationDef;
use w5k_math::scalar::{self, G};

use crate::compile::Compiled;
use crate::extras::Extras;

pub const SCHEMA: &str = "w5k.audit.v1";
const PA_PER_KPA: f64 = 1e3; // const-ok: unit conversion
const KG_PER_TONNE: f64 = 1e3; // const-ok: unit conversion
const PERCENT: f64 = 100.0; // const-ok: unit conversion

/// One read-only number: `value` (the first of several when `text` is set), its unit, the one-line reason, and where it comes from.
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub id: String,
    pub group: &'static str,
    pub label: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub unit: &'static str,
    pub because: String,
    /// The weakest provenance among the inputs the row used: ESTIMATE, TUNED, MEASURED or SPEC.
    pub provenance: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lo: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hi: Option<f64>,
}

/// An envelope flag (the next PR fills them): amber or red, the cause, and the slider that most affects it.
#[derive(Clone, Debug, Serialize)]
pub struct Flag {
    pub id: String,
    pub level: &'static str,
    pub text: String,
    pub because: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lever: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Audit {
    pub schema: &'static str,
    pub vehicle: String,
    pub consequences: Vec<Row>,
    pub flags: Vec<Flag>,
}

/// The weakest provenance among `params` (ESTIMATE is weaker than TUNED, MEASURED, SPEC).
pub fn weakest(params: &[&Param]) -> &'static str {
    let rank = |p: Provenance| match p {
        Provenance::Estimate => 0,
        Provenance::Tuned => 1,
        Provenance::Measured => 2,
        Provenance::Spec => 3,
    };
    match params.iter().map(|p| p.prov).min_by_key(|p| rank(*p)) {
        Some(Provenance::Spec) => "SPEC",
        Some(Provenance::Measured) => "MEASURED",
        Some(Provenance::Tuned) => "TUNED",
        _ => "ESTIMATE",
    }
}

struct Rows(Vec<Row>);

impl Rows {
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        group: &'static str,
        id: &str,
        label: &str,
        value: f64,
        unit: &'static str,
        because: String,
        inputs: &[&Param],
    ) {
        let (lo, hi) = (None, None);
        self.0.push(Row {
            id: id.into(),
            group,
            label: label.into(),
            value,
            text: None,
            unit,
            because,
            provenance: weakest(inputs),
            lo,
            hi,
        });
    }
    fn text(&mut self, text: String) {
        if let Some(r) = self.0.last_mut() {
            r.text = Some(text);
        }
    }
    fn band(&mut self, lo: f64, hi: f64) {
        if let Some(r) = self.0.last_mut() {
            r.lo = Some(lo);
            r.hi = Some(hi);
        }
    }
}

/// The audit of a compiled design: `def`, its extras and the `Compiled` result.
pub fn audit(def: &VehicleDef, ex: &Extras, c: &Compiled) -> Audit {
    let rig = &c.rig;
    let mut r = Rows(Vec::new());
    let (h, su, pt) = (&def.hull, &def.suspension, &def.powertrain);
    let (total, sprung) = (rig.total_mass_kg(), rig.hull.mass_kg);
    let weight = total * G;

    // ---- mass, by system
    r.add(
        "mass",
        "mass.total",
        "Total mass",
        total,
        "kg",
        format!("sprung {sprung:.0} kg + unsprung wheels and the belt's ground run {:.0} kg", total - sprung),
        &[&h.mass_kg],
    );
    r.add(
        "mass",
        "mass.sprung",
        "Sprung mass",
        sprung,
        "kg",
        "everything the suspension carries: structure plus the parts below".into(),
        &[&h.mass_kg],
    );
    if let Some(b) = &ex.mass_budget {
        let en = &pt.engine;
        for m in &c.mass_items {
            let (because, inputs): (String, Vec<&Param>) = match m.name.as_str() {
                "structure" => ("authored: hull, body, armour and everything not listed here".into(), vec![&b.structure_mass_kg]),
                "engine" => (format!("{:.1} kg/kW x {:.0} kW peak power", b.engine_kg_per_kw.v, en.peak_power_w.v / 1e3), vec![&b.engine_kg_per_kw, &en.peak_power_w]), // const-ok: W to kW
                "transmission" => (format!("{:.2} kg/(N m) x {:.0} N m peak torque", b.transmission_kg_per_nm.v, en.peak_torque_nm.v), vec![&b.transmission_kg_per_nm, &en.peak_torque_nm]),
                "fuel" => (
                    format!("{:.0} l x ({:.2} kg/l x {:.1} fill + {:.2} kg/l tank)", b.tank_litres.v, b.fuel_density_kg_l.v, if b.loading == crate::extras::Loading::Curb { b.fill_curb.v } else { b.fill_combat.v }, b.tank_kg_per_litre.v),
                    vec![&b.tank_litres, &b.fuel_density_kg_l, &b.tank_kg_per_litre],
                ),
                "crew" => (format!("{:.0} x {:.0} kg with kit", b.crew_count.v, b.crew_kg_each.v), vec![&b.crew_count, &b.crew_kg_each]),
                _ => (format!("{:.3} kg/(N m) x the torque this drive carries (peak torque x first gear x final drive / drives)", b.final_drive_kg_per_nm.v), vec![&b.final_drive_kg_per_nm, &en.peak_torque_nm]),
            };
            r.add(
                "mass",
                &format!("mass.{}", m.name),
                &format!("Mass: {}", m.name.replace('_', " ")),
                m.mass_kg,
                "kg",
                because,
                &inputs,
            );
        }
    }

    // ---- balance
    let ground_y = rig.ground_y_m();
    let com_up = rig.hull.com_m.y - ground_y;
    let com_from_front = rig.hull.com_m.z + 0.5 * h.length_m.v;
    r.add("balance", "balance.com", "Centre of mass", com_up, "m", "mass-weighted mean of the parts' positions (the structure's own is authored): height above the ground, distance back from the nose, offset to the right".into(), &[&h.com_height_m, &h.com_from_front_m]);
    r.text(format!("{com_up:.2} m up, {com_from_front:.2} m from the front, {:.2} m right", rig.hull.com_m.x));
    let sprung_stations: Vec<&StationDef> = rig.stations.iter().filter(|s| s.suspension.preload_n > 0.0).collect();
    let zs: Vec<f64> = sprung_stations.iter().map(|s| s.rest_pos_m.z).collect();
    let z_mid = zs.iter().sum::<f64>() / zs.len().max(1) as f64;
    let front: f64 = sprung_stations.iter().filter(|s| s.rest_pos_m.z < z_mid).map(|s| s.suspension.preload_n).sum();
    let all: f64 = sprung_stations.iter().map(|s| s.suspension.preload_n).sum();
    r.add(
        "balance",
        "balance.front_share",
        "Front share of the sprung weight",
        PERCENT * front / all,
        "%",
        "spring preloads of the stations ahead of the middle over all preloads (statics: the COM position decides)"
            .into(),
        &[&h.com_from_front_m],
    );
    r.text(format!("{:.0} / {:.0}", PERCENT * front / all, PERCENT * (1.0 - front / all)));
    let (m, i) = (rig.hull.mass_kg, &rig.hull.inertia_kg_m2.m);
    let k = |v: f64| scalar::sqrt(v / m);
    r.add(
        "balance",
        "balance.gyration",
        "Radii of gyration (yaw)",
        k(i[1][1]),
        "m",
        "sqrt(I / m) from the structure box plus the parts as point masses (parallel-axis theorem), about the COM"
            .into(),
        &[&h.mass_kg],
    );
    r.text(format!("roll {:.2} m, pitch {:.2} m, yaw {:.2} m", k(i[2][2]), k(i[0][0]), k(i[1][1])));

    // ---- power
    let (p, t) = (pt.engine.peak_power_w.v, pt.engine.peak_torque_nm.v);
    r.add(
        "power",
        "power.to_weight",
        "Power to weight",
        p / PA_PER_KPA / (total / KG_PER_TONNE),
        "kW/t",
        format!("{:.0} kW peak power / {total:.0} kg total mass", p / PA_PER_KPA),
        &[&pt.engine.peak_power_w, &h.mass_kg],
    );
    r.add(
        "power",
        "power.torque_to_weight",
        "Torque to weight",
        t / (total / KG_PER_TONNE),
        "N m/t",
        format!("{t:.0} N m peak torque / {total:.0} kg total mass"),
        &[&pt.engine.peak_torque_nm, &h.mass_kg],
    );
    let radius = rig.stations.iter().find(|s| s.drive_output.is_some()).map_or(0.0, |s| s.wheel.radius_m);
    let first = pt.gearbox.forward_ratios.first().map_or(0.0, |p| p.v);
    let transfer = pt.transfer_case_ratio.as_ref().map_or(1.0, |p| p.v);
    // A torque converter multiplies torque at stall (the launch case this row describes); a clutch does not.
    let stall = match &pt.coupling {
        w5k_contract::def::CouplingSliders::TorqueConverter { stall_ratio, .. } => stall_ratio.v,
        w5k_contract::def::CouplingSliders::Clutch => 1.0,
    };
    let thrust =
        t * stall * first * transfer * pt.final_drive_ratio.v * pt.driveline_efficiency.v * pt.gearbox.efficiency.v
            / radius;
    r.add("power", "power.thrust_to_weight", "Thrust to weight, first gear", thrust / weight, "", format!("{t:.0} N m x converter stall {stall:.1} x {first:.2} x {:.2} x efficiencies / {radius:.3} m wheel radius, over the weight (before traction limits)", pt.final_drive_ratio.v), &[&pt.engine.peak_torque_nm, &pt.final_drive_ratio]);

    // ---- ride
    let sag = |f: f64| G / ((scalar::TAU * f) * (scalar::TAU * f));
    r.add("ride", "ride.frequency", "Ride frequency", su.front_ride_frequency_hz.v, "Hz", "a slider: the sprung mass on its suspension (in series with the tyre on wheels); the spring rates follow the sprung mass".into(), &[&su.front_ride_frequency_hz, &su.rear_ride_frequency_hz]);
    r.text(format!("{:.2} / {:.2}", su.front_ride_frequency_hz.v, su.rear_ride_frequency_hz.v));
    r.band(su.front_ride_frequency_hz.band().0, su.front_ride_frequency_hz.band().1);
    r.add(
        "ride",
        "ride.damping_ratio",
        "Damping ratio (mean)",
        su.damping_ratio.v,
        "",
        "a slider: c = 2 zeta sqrt(k m), split bump and rebound".into(),
        &[&su.damping_ratio],
    );
    r.band(su.damping_ratio.band().0, su.damping_ratio.band().1);
    let worst_sag = sag(su.front_ride_frequency_hz.v.min(su.rear_ride_frequency_hz.v));
    r.add(
        "ride",
        "ride.static_sag",
        "Static sag",
        worst_sag,
        "m",
        "g / omega^2 for the softer end: how far a spring is compressed by the weight it carries".into(),
        &[&su.front_ride_frequency_hz, &su.rear_ride_frequency_hz],
    );
    r.add("ride", "ride.bump_capacity", "Bump capacity", su.bump_travel_m.v / worst_sag, "g", format!("bump travel {:.2} m / static sag {worst_sag:.2} m: the vertical acceleration the travel can absorb before the stops", su.bump_travel_m.v), &[&su.bump_travel_m, &su.front_ride_frequency_hz]);

    Audit { schema: SCHEMA, vehicle: rig.id.clone(), consequences: r.0, flags: vec![] }
}
