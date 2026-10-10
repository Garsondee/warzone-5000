# Interface request: CHASSIS -> TRACKS, the soil functions CHASSIS will call

**From:** CHASSIS · **To:** TRACKS · **Date:** 2026-10-10 · **Approved edge (ARCH, 17:33Z):** `w5k_chassis` may depend on `w5k_terramech`'s *soil module only*
(pure functions); `w5k_terramech` never depends on `w5k_chassis`.

## What
A `w5k_terramech::soil` module with pure functions of `w5k_contract::SoilParams`, at least:
```text
pub fn pressure_pa(soil: &SoilParams, plate_width_m: f64, sinkage_m: f64) -> f64          // Bekker: (kc / b + kphi) z^n
pub fn shear_strength_pa(soil: &SoilParams, normal_pressure_pa: f64) -> f64               // Mohr-Coulomb: c + p tan(phi)
pub fn shear_stress_pa(soil: &SoilParams, normal_pressure_pa: f64, shear_displacement_m: f64) -> f64   // Janosi-Hanamoto
```
CHASSIS builds the rigid-wheel relations on top (load-sinkage `W(z)` and compaction resistance, `crates/w5k_chassis/src/soil_wheel.rs`) and today carries a private
copy of `pressure_pa` marked PROVISIONAL, to be replaced by yours the day it lands.

## Why
One soil law in one place: a tyre and a track on the same mud must agree. Your S4 citations (Wong's worked examples) then validate both.

## Default if no answer
CHASSIS keeps its private `bekker_pressure_pa` (same formula, tagged PROVISIONAL) and switches when `w5k_terramech::soil` exists.
