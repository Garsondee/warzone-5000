//! The tracked carrier: the hull module (`shapes/carrier_tracked.ron`, the same shell-and-anchors machinery as the trucks, with no wheel arches)
//! with one `Station` socket per wheel and a `Run` socket per side, the wheel modules on the stations and a belt module on each run.
//! Hull frame as everywhere: origin at the hull box centre, +Y up, -Z forward, +X right; the ground is `ride_height_m` below the origin.

use crate::module::Assembly;
use crate::module::Module;
use crate::track::{belt_module, link_module, run_sockets, wheel_module, LinkSpec, RunSpec};
use crate::truck::{truck_hull, TruckKind, UtilityDims};

/// `shapes/placeholder_carrier_tracked.ron`, `PROVISIONAL(C-002)`: the stand-in hull box until FORGE's definition supplies it.
pub fn placeholder_dims() -> UtilityDims {
    ron::from_str(include_str!("../shapes/placeholder_carrier_tracked.ron"))
        .expect("placeholder_carrier_tracked.ron parses")
}

/// The carrier's hull module: the shell and its fittings, the roof `Ring` socket, and the sockets for the running gear of `run`.
pub fn carrier_hull(d: &UtilityDims, run: &RunSpec, detail: u8) -> Module {
    let mut hull = truck_hull(TruckKind::Carrier, d, &[], detail);
    hull.sockets.extend(run_sockets(run));
    hull
}

/// The carrier: its hull with a wheel on every station socket of both sides and a belt on each run. An error names a wheel the belt does not touch.
pub fn tracked_assembly(d: &UtilityDims, run: &RunSpec, link: &LinkSpec, detail: u8) -> Result<Assembly, String> {
    tracked_assembly_with(d, run, link, detail, false)
}

/// `tracked_assembly`, with the belt of each side as ONE link (`instanced`, for a viewer that repeats it along the path) or as every link.
pub fn tracked_assembly_with(
    d: &UtilityDims,
    run: &RunSpec,
    link: &LinkSpec,
    detail: u8,
    instanced: bool,
) -> Result<Assembly, String> {
    let mut asm = Assembly::new(carrier_hull(d, run, detail));
    let (belt, name) = if instanced { (link_module(run, link)?, "link") } else { (belt_module(run, link)?, "belt") };
    for tag in ["r", "l"] {
        for (i, w) in run.wheels.iter().enumerate() {
            let module = wheel_module(w, run, detail);
            asm.attach(&format!("station.{i}.{tag}"), &module, 0.0, &format!("{}.{i}.{tag}", module.name))
                .map_err(|e| e.to_string())?;
        }
        asm.attach(&format!("run.{tag}"), &belt, 0.0, &format!("{name}.{tag}")).map_err(|e| e.to_string())?;
    }
    Ok(asm)
}
