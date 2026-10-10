# Interface request: CHASSIS -> TRACKS, report how the sprocket reaction answers the slip

**From:** CHASSIS · **To:** TRACKS · **Date:** 2026-10-10

## What
Add to `GearTotals` the derivative of `shaft_reaction_nm` with respect to the slip speed over one step, e.g.
`shaft_reaction_rate_nm_s_rad: f64` = `d(shaft_reaction_nm) / d(sprocket_omega)` at fixed hull speed (N m s/rad), from each sample's own response
(the shear damping `p kappa` while unsaturated, the shear spring over the step, 0 for a saturated sample).

## Why
Your shear damping is sized to critically damp the tank's mass; on the sprocket (about 200 kg m^2 on `box_tank`, 1.3 t at the belt) the same damping
is about twice what an explicit spin update can take (`c dt / J` near 2): the sprocket ran away into a saturated slip limit cycle. CHASSIS now
integrates the spin implicitly in the slip (`crates/w5k_chassis/src/tracked.rs`), with `dR/dw` *estimated* from your public tuning
(`2 zeta sqrt(mu / (g K))` per sample plus `mu dt / K`), marked PROVISIONAL. Your gear knows the exact value, including saturation.

## Default if no answer
CHASSIS keeps the estimate (an upper bound when samples saturate, which only damps the spin a little more).
