# COMBAT design note (one page)

*Slice 2 builds only S6 and the ballistics kernel; everything below is the design the later slice follows. Numbers are UNVALIDATED until VALIDATION lands sources.*

**Scheme and seam.** Articulation = one reduced-coordinate solve of hull + turret/cradle/barrel (`M(q)` from Jacobians, one small direct solve, RK4), proven by S6
(`spike-s6.md`): momentum with the ejecta to 1e-6 from 2 substeps. COMBAT implements `ArticulationPort`; `w5k_vehicle` keeps the substep order and never depends on `w5k_combat`.
The port must take the other loads on the hull and return the hull state (CCR-C1); a wrench applied a substep late diverges.

**Servo and stabiliser in SI.** `kp_si` N m/rad (N/m), `kd_si` N m s/rad, `max_effort_si` the saturation, `max_rate_si` and `max_accel_si` shape a trapezoid
(`alpha = min(max_accel, max_effort/I)`), `latency_s` delays the effort. FORGE derives, never authors: `kp = I wn^2`, `kd = 2 z I wn`, `I` = held-hull subtree inertia,
`wn` from the dossier traverse rate and acceleration (slew lag = latency + alpha/(wn^2 w)); a stabiliser's `bandwidth_hz` is the *achieved* one,
`f_b <= 1/(32 pi (t_servo + t_stab))` (CCR-C2, C3).

**The shot.** An impulse `J = factor x m x v` along the bore at the muzzle line, through the recoil joint (the barrel is the body pushed); shell and gas are ejecta carrying `+J`
and `r x J` out, booked in the ledger as the `Recoil` row. The shell starts with `v_bore + v_hull + w x r` (the muzzle point's velocity).

**Who steps projectiles.** `w5k_sim` owns the world and calls COMBAT's pure kernels (`ballistics::step`, `hits`) at fixed substeps with swept segment tests: 1.6 km/s is
27 m per tick, so every substep tests the segment, never the point. Drag is `Cd(Mach)` from RON; wind is out of scope.

**Armour.** Closed solids per body (convex slabs and wedges first); line-of-sight thickness is a chord integral with the even-odd rule. A thin-shell variant does not help
GEOMETRY: it cannot say which side is solid. Modules (engine, tracks, ring, gun, ammunition, crew) are volumes on the same shape vocabulary. Per-vehicle thickness is authored as
sliders on `VehicleDef` compiled by FORGE onto GEOMETRY's closed shells into `combat.armour`; COMBAT owns the material table, penetration kernels and `degrade()`.

**Damage.** A pure `degrade(&PhysRig, &Health) -> PhysRig` (health scales a rig parameter: engine torque curve, a thrown track zeroes its side) followed by `swap_rig` and a new
`rig_hash`; full health returns the original hash. Target health lives in the sim (the world is immutable). No damage bypasses penetration; every shot returns a `ShotReport`.

**Weapons.** A muzzle names its weapon by `catalogue_id` in `content/combat/`; the rig carries only the dynamics numbers (mass, velocity, impulse factor, cycle).

**CCRs expected** (text in the S6 finding): C1 port integrates the hull; C2 pin the servo law; C3 FORGE derives gains. Later: armour solids and module volumes on `PhysRig`,
`PropRef.ballistic` and a ray exit distance with WORLD, `Frame.projectiles` with VIEWER.
