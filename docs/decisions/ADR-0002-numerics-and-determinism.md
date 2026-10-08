# ADR-0002: Plain f64 with strict discipline

**Status:** Accepted (owner decision, 2026-10-08). Supersedes D3 (deterministic lockstep on Q32.32 fixed point).

## Context
D3 chose fixed-point arithmetic so a lockstep multiplayer simulation would be bit-identical. Multiplayer is off the table, and fixed point had
already bent the physics: the soil model was forced to an exponent of 1 and the speed governor to a power of 32 computed by repeated
squaring, because no `pow` existed; every new solver would be an overflow exercise. Floats are what the owner and the sessions read best.
We still want **reproducible** runs: golden tests, replays you can verify, and bugs you can reproduce.

## Decision
1. Simulation state is **`f64`**, SI units (ADR-0006).
2. **Reproducibility contract (level 2):** the same scenario produces bit-identical state hashes on Linux and Windows CI. Enforced by golden hash chains (`tests/golden/`) compared across both.
3. Rules that make it true:
   - fixed outer tick (60 Hz) and an integer substep count per vehicle, set when the rig is baked;
   - every transcendental from **`libm`** (via `w5k_math::scalar`); clippy `disallowed-methods` bans `f64/f32` `sin cos tan asin acos atan atan2 sinh cosh tanh exp exp2 exp_m1 ln ln_1p log log2 log10 powf powi hypot cbrt mul_add sin_cos` and `std::time::Instant::now` / `SystemTime::now`; `sqrt abs min max floor ceil round` are exact and allowed;
   - no fast-math or reassociation (Rust's default; do not enable);
   - ordered iteration: no `HashMap` or `HashSet` in simulation code (clippy `disallowed-types`);
   - a vehicle's step reads only its own state and an immutable world snapshot; no threads inside a step;
   - NaN or infinity is a fatal bug with a state dump (`StateHasher::write_f64` panics in debug builds);
   - tiny values are flushed on purpose (a host process can change the FPU's denormal mode), not by luck;
   - random numbers only from seeded `Pcg32` streams.
4. **Fallback, by decision card only:** if a compiler or CPU change breaks cross-platform identity in a way we cannot fix, goldens drop to "same-platform hash plus tolerance trajectory". Never silently.

## Consequences
- Q32.32 (`Fx`) and its fixed-point trigonometry are dropped. Soil exponents other than 1 and any smooth governor are now affordable.
- CI runs every golden on Linux and Windows. Spike S2 checks this on day one.
- Tolerance-based comparisons (not equality) are the norm *between* implementations (sim vs published data); bit-equality is only for regression of the same code.

## Alternatives considered
Keep fixed point (rejected above); "same-build reproducibility only" (weaker than what we can afford: Linux vs Windows identity catches non-portable calls early).
