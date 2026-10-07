# Determinism Rules

Online multiplayer uses **deterministic lockstep**: players exchange only commands, every machine simulates the whole game,
and the results must match bit for bit. These rules apply to **every crate that affects gameplay** (`w5k_math`, `w5k_sim`, and
any part of `w5k_forge` whose output the simulation consumes at runtime).

## The rules

1. **No floating point in gameplay state or gameplay arithmetic.** Use `w5k_math::Fx` (Q32.32 fixed point: a 64-bit integer
   with 32 fractional bits) and `FxVec3`. Floats are allowed only in presentation, tooling and baked offline content.
2. **Fixed tick.** The simulation advances in fixed steps (50 ms; 60 Hz sub-steps where stated). It never reads a clock.
3. **Seeded randomness only.** All randomness comes from `w5k_math::Pcg32` streams derived from the match seed plus a
   documented stream id (for example `hash(seed, player, draft_index)`). No thread-local or OS randomness.
4. **Ordered iteration.** Never iterate a `HashMap`/`HashSet` in gameplay code; use `Vec` with stable ids or `BTreeMap`.
5. **No parallel reductions whose order can vary.** Parallelism is allowed only where each result is independent and merged in a
   fixed order.
6. **No camera-dependent simulation.** Expensive updates may be staggered by entity id and tick, never by what a player looks at.
7. **AIs are players.** AI decisions become commands that go through the same path as human input.
8. **Hash everything.** The simulation state can be hashed each tick (`w5k_math::StateHasher`, FNV-1a 64). Clients exchange
   hashes to detect desyncs; replays record commands and expected hashes.
9. **Golden hash tests.** Each crate keeps a test that runs a fixed scenario and compares its final hash with a constant
   written in the test. If a change, compiler or platform alters the arithmetic, the test fails.

## Why fixed point rather than careful floats
Floating-point arithmetic is deterministic for the basic operations on one machine, but across machines it can differ through
library functions (`sin`, `exp`, `pow`), fused multiply-add, flush-to-zero settings set by the host program, and compiler
choices. Fixed point removes all of these at once: every operation is integer arithmetic, defined exactly by the Rust
language. The cost is that we write our own square root and trigonometry, and we must respect the range: Q32.32 covers
about plus or minus 2.1 billion with a resolution of about 0.23 billionths, far more than an 8 km map in metres needs.

> **Fallback recorded in the decision log:** `Fx` is used through a type name, so if fixed point became too costly the code
> could move to floats under a strict contract (the `libm` crate only, no SIMD, guarded FTZ/DAZ flags), but only if a
> cross-build hash gate passes.

## How to check
- `cargo test -p w5k_math` includes the golden hash test.
- From M3, every simulation scenario test records its final hash; CI compares builds on different platforms.
