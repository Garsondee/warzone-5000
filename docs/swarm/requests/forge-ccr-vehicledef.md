# CCR (text): `VehicleDef` additions for contract 0.1.2

From lane FORGE, settling round. **All additions are `#[serde(default)]` or new enum variants**, so existing RON still loads; every number is a `Param`. Baselines: `docs/architecture/redteam/sketches/*`; evidence for W1 to W4: `docs/lanes/forge/spike-s5.md`. ARCH owns `def.rs`; this file is the request, not the edit.

## W. Wheeled (spike S5 and the wheeled red-team G2)
- **W1 `TyreSliders` +=** `vertical_stiffness_n_m`, `vertical_damping_ns_m`, `slip_stiffness`, `relaxation_length_m`, `wheel_inertia_kg_m2` (all `Param`). `patch_length_m` is derived (load / (inflation x width)), not a field.
- **W2 `SuspensionSliders` +=** `rebound_to_bump: Param`, `bump_stop_engage_frac: Param`, `bump_stop_rate_n_m: Param`; `SuspensionKind::Volute` (tracked red-team).
  *Semantics to write into the doc comments:* ride frequency is the **ride** frequency (tyre in series); damping ratio is the mean of bump and rebound.
- **W3 `EngineSliders` +=** `idle_torque_frac` (template shape), `drag_const_nm`, `drag_per_rpm_nm`, `response_time_s`, `idle_fuel_kg_s`, `free_turbine: bool` (gas turbines, M2).
- **W4 `AxleDef` +=** `max_steer_deg: Option<Param>`, `ackermann: Option<Param>`, and the red-team's `tyres_per_side: u8`, `dual_spacing_m`, `hub_reduction`, `portal_drop_m`, `layout: AxleLayout { Independent, Solid { spring_track_m }, Tandem { group, pivot_frac } }`, `brake_share`, per-axle `tyre: Option<TyreSliders>`, `unsprung_mass_kg: Option<Param>`. `BrakesDef::front_share` stays as the two-axle default.
- **W5 `HullDef`:** document the datum (hull box centre; +Z back) next to `com_height_m` and `com_from_front_m`.

- **W6 `GearboxSliders` gets a design form** (the brief's "gear spread" slider): `design: Option<GearDesign { gears: u8, launch_grade: Param, top_speed_m_s: Param, top_gear_ratio: Param }>`. When present the compile generates `forward_ratios` and `final_drive_ratio` (design note section 2); when absent the explicit lists stay authoritative (reference vehicles with published ratios).

## T. Tracked (tracked red-team H-13, plan item 4)
- **T1 `TrackedDef` +=** `return_rollers: u8` (+ diameter, spacing), `idler: IdlerDef { diameter_m, from_front_m, tensioner_travel_m, tension_n }`, `belt_thickness_m`, `belt_length_m: Option<Param>` (derived from the loop when absent), `bogies: Vec<BogieDef { stations, pivot_frac, arm_length_m }>`, `arm_length_m`, `damped_stations: Vec<u8>`, `shoe_mu_scale_soft: Option<Param>`, `wheel_contact: { vertical_stiffness_n_m, vertical_damping_ns_m }`, `sprocket_teeth: u8`.
- **T2 `SteeringUnitSliders.law:`** `{ diff_ratio_by_gear, detents, diff_speed_rad_s, works_in_neutral, max_steer_torque_nm }` mirroring rig `SteerLaw`; `BrakesDef.location: BrakeLocation`.

## G. Guns, mounts, turret (weapons red-team)
- **G1 `GunDef` +=** `projectile_mass_kg`, `recoiling_mass_kg`, `recoil_impulse_factor`, `reload_s`, `rounds_carried`, `stabiliser_rejection_yaw`, `catalogue_id: String`.
- **G2 `TurretDef` +=** `ring_from_front_m`, `ring_height_m` (ring position on the hull).
- **G3 `VehicleDef.mounts: Vec<MountDef>`** (coax, cupola, loader, bow and hull MGs): `{ name, parent: Turret|Gun|Hull, pose, weapon: GunDef, aim_channel }`.

## A. Armour sliders (with COMBAT; proposal, COMBAT to confirm)
`VehicleDef.armour: ArmourDef { zones: Vec<ZoneDef { name, thickness_mm: Param, material: String (catalogue id), slope_deg: Param }> }`. FORGE compiles zones onto GEOMETRY's closed shells into `PhysRig.combat.armour`; mass of a zone = shell area x thickness x density, added to the hull mass breakdown. COMBAT owns the materials and kernels.

## M. Mass breakdown (design note section 3)
`HullDef.mass_kg` stays the stated total for a **reference** vehicle; for a *designed* vehicle add `HullDef.mass_items: Vec<MassItemDef { name, mass_kg: Param, position_m: Vec3, kind: Structure|Engine|Transmission|Fuel|Crew|Ammo|Other }>`; when present, the compile sums them and checks the total against `mass_kg` (1%).

## Request to ARCH (not a contract edit)
`dummy_vehicle_def()` has 110 kW at 3500 rpm with a 300 N m torque peak at 2500 rpm: inconsistent (power would still be rising). Use <= 100 kW. (spike F5)
