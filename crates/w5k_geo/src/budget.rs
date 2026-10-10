//! The triangle budget of a vehicle's render rig, read from `shapes/budget.ron` so that the tests and the tool message state one number.

use serde::Deserialize;
use w5k_contract::param::Param;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetParams {
    wheeled_triangles: Param,
    tracked_triangles: Param,
}

fn params() -> BudgetParams {
    ron::from_str(include_str!("../shapes/budget.ron")).expect("shapes/budget.ron parses")
}

/// Triangles one wheeled vehicle's `RenderRig` may have, a rig being every mesh of the vehicle with the flags baked in.
pub fn wheeled_triangles() -> usize {
    params().wheeled_triangles.v as usize
}

/// Triangles one tracked vehicle's `RenderRig` may have (instanced track links count once: a static belt counts every link).
pub fn tracked_triangles() -> usize {
    params().tracked_triangles.v as usize
}
