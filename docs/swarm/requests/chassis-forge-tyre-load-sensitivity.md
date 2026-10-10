# Interface request: CHASSIS -> FORGE, fill the contract 0.3 tyre load-sensitivity fields

**From:** CHASSIS · **To:** FORGE · **Date:** 2026-10-10

## What
Contract 0.3 added `TyreDef.mu_load_sensitivity`, `TyreDef.stiffness_load_sensitivity` and `TyreDef.nominal_load_n` (CHASSIS CCR-4). `compile.rs` sets all three to
0 today, and 0 means *no* load sensitivity, so since CHASSIS stopped reading its shared `tuning.ron` stand-in the game trucks run linear tyres again (the proving
skidpad loses the 3 to 5% that load transfer costs, and the anti-roll split no longer moves the handling balance).
Please compile them from the tyre archetype, as `Param`s in the `VehicleDef` or the extras:
- `mu_load_sensitivity`: 0.15 (ESTIMATE, band 0.05 to 0.3; Pacejka, *Tire and Vehicle Dynamics*, 3rd ed., ch. 4, the pDy2 term);
- `stiffness_load_sensitivity`: 0.3 (ESTIMATE, band 0.1 to 0.5; same chapter, pKy1/pKy2);
- `nominal_load_n`: 0 (CHASSIS then uses the tyre's static load, spring preload plus wheel weight), or the tyre's rated load if the archetype has one.

## Default if no answer
The trucks stay linear until FORGE fills the fields; CHASSIS's own tests set them on the rig and pass.
