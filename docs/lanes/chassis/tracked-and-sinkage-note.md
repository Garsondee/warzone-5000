# CHASSIS note: a tracked hull on the same integrator, and tyres that sink (slice 2, stage A)

*One page, for ARCH and TRACKS. TRACKS' design note has not landed yet; the interface below is a proposal to agree with them, and the soil laws are theirs.*

## 1. A tracked hull on `WheeledChassis`'s integrator
**What stays the same.** One 6-DoF hull (angular-momentum state, semi-implicit Euler, the substep floor from the stiffest mode), one station per wheel with a
travel coordinate and its own unsprung mass, the force ledger (body 0 = hull, 1 + i = station i), the `CONTRACTS.md` substep order. Nothing in the hull changes.
**What is new for the stations.**
- **Swinging arms.** Road wheels hang on torsion-bar arms (`StationDef.arm_pivot_m`, refused today). The travel coordinate stays the wheel centre's *vertical rise* `c`
  (the contract's convention); the centre lies on the circle about the pivot, `sin(phi) = sin(phi0) - c / L`, and the strut direction `d` becomes the arm's tangent
  each substep. `SpringKind::Torsion` already returns the vertical wheel force (`suspension.rs`, tested against the arm geometry).
- **Rigid stations.** Sprocket, idler and return rollers are spin-only (no travel); their mass is in the hull (the contract says so).
- **Spin.** Road wheels, idler and rollers do not get their own spin state: the belt links them rigidly to the sprocket, so their inertia is reflected onto it,
  `J_eff = J_sprocket + sum J_i (r_s / r_i)^2 + m_belt r_s^2`, and only the two sprockets integrate `J dw/dt = T_shaft - reaction` (the reaction is the sum of the
  belt's ground shear times the sprocket's pitch radius). Belt speed = sprocket spin x pitch radius (`WheelDef.radius_m` of a sprocket is its pitch radius).
- **Contact.** Road wheels do not touch the ground; the belt does, through `TrackDef.samples` massless contact samples along the footprint (TRACKS' `TrackSample:
  ContactElement`). A sample's height is the belt's lower envelope under the road wheels (interpolated between neighbours, plus `thickness_m`); its vertical force is
  shared back to the two neighbouring road wheels by distance (it loads their travel along `d`); its shear and lateral force go to the hull at the sample point (the same
  across-strut path as a tyre today, with the `r x F` torque, so roll and pitch moments stay right).
- **Belly.** A `ProxyRole::Belly` proxy that sinks below the soil surface gets TRACKS' belly-drag law, booked as `BellyDrag` on the hull.

**Proposed seam (to agree with TRACKS).** CHASSIS owns the bodies and their integration; TRACKS owns what the belt and soil do.
```text
trait RunningGear {                       // implemented by TRACKS' TrackedRunningGear; one call per substep
    fn step(&mut self, i: &GearInput, o: &mut GearOutput);
}
GearInput  { hull: HullMotion, wheel_centres_m: &[Vec3], wheel_vels_m_s: &[Vec3], sprocket_omega_rad_s: [f64; 2], world: &dyn WorldQuery, dt_s }
GearOutput { wheel_forces_n: Vec<Vec3>,           // on each road-wheel centre (world): drives its travel and, across the strut, the hull
             hull_forces: Vec<(Vec3, Vec3)>,      // (point, force) straight onto the hull (shear at the samples, belly drag)
             sprocket_reaction_nm: [f64; 2],      // into the sprocket spin equation
             rows: Vec<(ForceTerm, u16, Vec3, Vec3)> } // ledger rows, so the per-body balance test keeps holding
```
This is a contract addition (a `ports.rs` trait), so it would be a CCR in `contract-v0.3`; until then it lives in `w5k_chassis` and TRACKS implements it there.

**Numbers to check first.** The stiffest mode becomes a road wheel on its torsion bar against the belt's sample stiffness (`TrackDef.wheel_contact`): `w5k chassis modes`
learns this case. Spike S1's kill criterion (more than 8 substeps) is the first thing to report on `box_tank()` and the first 70 t rig.
**Tests.** `torsion_arm_wheel_centre_follows_the_circle`, `reflected_inertia_spins_up_at_t_over_j_eff`, `tank_rests_at_its_design_ride_height_within_5mm`,
`ledger_net_force_equals_mass_times_acceleration` (tracked), and TRACKS' `skid_steer_turn_radius_follows_the_track_speeds` run through this integrator.

## 2. A tyre that sinks (rigid-wheel Bekker, `UNVALIDATED` until TRACKS' S4 cites Wong)
**Today** a tyre on soft ground behaves as on asphalt and `ContactOutput.sinkage_m` is 0. **Plan:** when `Material.soil` is `Some`:
- **Load and sinkage.** The tyre spring and the soil act in series: the penetration splits into tyre deflection `d_t` and soil sinkage `z`, with `k_t d_t = W(z)` and
  `W(z) = b (kc / b + kphi) sqrt(2R) z^(n + 1/2) (3 - n) / 3` (Bekker's rigid wheel; Wong, *Theory of Ground Vehicles*, ch. 2). One monotone scalar equation per substep
  (bisection, a fixed number of iterations: deterministic). No soil memory between passes (multi-pass ruts are NOT-MODELLED).
- **Resistance.** Compaction resistance `R_c = b (kc / b + kphi) z^(n+1) / (n + 1)`, booked as `SoilCompaction`; it replaces the surface rolling coefficient (a soil material carries 0 there).
- **Traction.** The friction cap `mu Fz` becomes the soil's strength `A c + W tan(phi)` (Mohr-Coulomb, `A` = patch area `b * sqrt(2 R z)`), and the patch-stretch
  relaxation length becomes the shear modulus `K` (Janosi-Hanamoto: shear needs displacement before it delivers strength, the same idea as our stretch).
- **Tests.** `rigid_wheel_sinkage_matches_bekker_closed_form`, `compaction_resistance_matches_wong`, `a_tyre_on_firm_ground_does_not_sink`,
  `a_wider_tyre_sinks_less_and_rolls_easier`, `soft_ground_traction_saturates_at_ac_plus_w_tan_phi`.

## 3. Decisions needed (ARCH)
- **Where the soil laws live.** TRACKS writes Bekker, Mohr-Coulomb and Janosi-Hanamoto in `w5k_terramech`. *Default:* `w5k_chassis` takes a workspace dependency on
  `w5k_terramech` and calls those functions (one law, one place) rather than CHASSIS copying them. Needs ARCH's yes (a new internal dependency edge).
- **The `RunningGear` seam** above: agree with TRACKS, then CCR it for `contract-v0.3`. *Default:* prototype it inside `w5k_chassis` behind `PROVISIONAL(CCR-chassis-5)`.
- **Order.** *Default:* the sinking tyre first (it reuses the tyre, small, and gives the mud crossing a real answer now), the tracked hull once TRACKS' samples exist.
