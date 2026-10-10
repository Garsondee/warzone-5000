# TRACKS design note (settling round)

Contract pin: 0.2.0. Spikes: [S3](spike-s3.md), [S4](spike-s4.md); both pass their kill criteria. This note is the whole design on one page, plus the CCRs.

## Geometry: samples that follow the ground
A track has `samples` (12 for the reference tank) cell centres along its ground run, in a track frame (x forward, origin at the contact centre). The glue gives, per road wheel, the penetration of the belt bottom into the undeformed ground (a `WorldQuery` ground height under the wheel, minus the belt thickness). A sample's penetration is the **linear interpolation of its two neighbouring wheels minus the belt's sag between them** (`m' g s^2 / 8T` from the belt weight and tension; 7.5 mm for the reference span). Pressure therefore peaks under the wheels on firm ground and evens out as soft ground lets them sink: the road-wheel load becomes a *distribution along the footprint*, not a uniform guess. The soil's reaction goes back to the wheels by the same interpolation weights (partition of unity), so Newton's third law holds exactly and the suspension sees a smooth wheel force.

## Vertical: a series spring, so sinkage is explainable
The wheel contact (`TrackDef::wheel_contact`, N/m, its share per sample) is in series with the soil: `k_s (delta - z) = b dx (kc/b + kphi) z^n`. `sinkage_m` is `z`. On rigid ground it is just `k_s delta`. Compaction resistance is the work of deepening the rut: each sample pays `b integral p dz` for the depth beyond the sample *ahead* in the direction of travel (the leading sample pays the whole `b k z^(n+1)/(n+1)`, a rut already made costs nothing); it is smoothly switched by `v/(|v| + u0)`, so a parked tank has none. It is a hull load: it does **not** go through the belt to the sprocket.

## Belly drag
When the hull sinks past the ground clearance the belly bears on the soil. The glue gives the belly plate's penetration `c`; the plate carries load by the same Bekker law over its area, scaled by `smoothstep(0, ramp, c)` (`belly_ramp_m`, 3 cm), and drags at `tau_max`-limited shear: a smooth ramp, never a cliff. (Built in the next PR.)

## Shear and the sprocket
Each sample owns a shear displacement vector `j` (hull frame) integrated from the slip velocity and carried along the belt (upwind from the neighbouring sample); stress `tau_max (1 - e^(-|j|/K))` against it, with `tau_max = c + p tan(phi)` on soil and `mu_peak * shoe_mu_scale * p` on firm ground (a short `K`, `firm_shear_k_m`), and a damping of the fast part of the slip velocity so neither a parked nor a driven tank rings on its shear spring (S3). `shoe_mu_scale_soft` scales soil strength for grousers (PROVISIONAL: a rubber pad on soil uses the soil's own strength). The sprocket reaction torque is `r (sum of shear thrust + c0 N sign(v_belt))`: the engine pays the shear thrust plus the internal running resistance; the compaction and belly drag load the hull directly. Power balance: sprocket power = useful `F v` + slip loss `F (v_belt - v)`, which is why a pivot costs 10x a straight run.

## The object the glue calls
`TrackedRunningGear::new(GearConfig::from_track_def(&def, wheel_x_m, sprocket_radius_m), tuning)`, one per track; each substep `step(&GearInput { wheel_penetration_m, wheel_penetration_rate_m_s, vel_long_m_s, vel_lat_m_s, yaw_rate_rad_s, sprocket_omega_rad_s, ground: &[&Material], dt_s }) -> GearTotals`, then `outputs()` (per-sample `ContactOutput`, for the ledger and the viewer), `wheel_force_n()` (vertical force back on each road wheel) and `GearTotals::shaft_reaction_nm`. A single sample is also a plain `ContactElement` (`TrackSample`), and `soil::*` are free functions, so a tyre can use the soil laws today.

## CCRs expected (text only; ARCH settles into contract v0.3)
1. **`TrackDef::grouser_height_m`** (default 0): effective extra shear depth on soft ground; replaces the vague `shoe_mu_scale_soft` once validated. Not used yet.
2. **`TrackDef::belt_stiffness_n_m`** (default: derive from `tension_n`): optional; today the sag comes from tension and belt weight only.
3. **`ContactInput`: no change needed.** `penetration_m`, the two velocities and `surface_speed_m_s` suffice because the gear owns the neighbour coupling. We do ask that the glue pass the hull yaw rate and the sample-ahead ordering that `GearInput` documents (they are not contract fields).
4. **`WheelContact` per road wheel is enough**; no per-sample stiffness. A `BellyDef { clearance_m, width_m, length_m }` on the hull is wanted (the glue needs it to compute belly penetration): put it in `VehicleDef`/`PhysRig`, FORGE authors it.
5. **Soil table**: `Material.soil` already carries every Bekker-Wong parameter; the rigid-wheel formula needs only `n, kc, kphi`. No change.

## Not done in this round
Persistent ruts and multi-pass soil memory (non-goal), unloading curve (a lifted sample forgets its sinkage), track-link dynamics. Theory: [`docs/theory/tracks.md`](../../theory/tracks.md).
