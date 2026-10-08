# Determinism rules

The promise (ADR-0002): **the same scenario gives bit-identical state hashes on Linux and Windows CI.** That is what lets a golden test mean something, a
replay be verified, and a bug be reproduced. Floats are allowed; surprises are not.

## Why floats can differ, and what we do about each
| Source of difference | Our rule |
|---|---|
| `sin`, `cos`, `exp`, `ln`, `pow`... call the platform's maths library, which differs in the last bit between systems | Use `w5k_math::scalar::{sin, cos, atan2, exp, ln, pow, ...}` (they call `libm`, pure Rust). Clippy bans the `f64`/`f32` methods |
| Fused multiply-add can change rounding | `mul_add` is banned; Rust does not fuse implicitly, and we never enable fast-math |
| Summation order changes the result | Iterate in a fixed order (indices, `Vec`, `BTreeMap`); no `HashMap`/`HashSet` in simulation code; no parallel reductions |
| A different step gives a different trajectory | Fixed outer tick (60 Hz), an integer substep count per vehicle set at bake time; no wall clock in the simulation |
| Denormal numbers behave differently when a host process sets flush-to-zero | Decaying state is flushed on purpose with `scalar::flush_tiny` at the end of a step |
| NaN and infinity propagate silently and hash differently | A non-finite value is a fatal bug; `StateHasher::write_f64` panics in debug builds; frames are checked with `is_finite()` |
| Random numbers | Only seeded `Pcg32` streams (`Pcg32::derive(seed, &[labels])`); never the OS or the clock |
| Threads | No threads inside a vehicle's step; a vehicle reads only its own state and an immutable world snapshot |

Exactly rounded operations are fine everywhere: `+ - * /`, `sqrt`, `abs`, `min`, `max`, `floor`, `ceil`, `round`, comparisons.

## Goldens
- A golden is a hash chain of the simulation state, one entry per simulated second (`ReplayHeader::state_hashes`), stored in `crates/<crate>/tests/golden/*.json`; each lane owns the goldens in its own crate, ARCH owns the first-light golden in `w5k_sim`.
- CI compares the **same file** on Linux and Windows. A mismatch is a regression or a non-portable call; find the call (look for a banned method or an unordered iteration).
- A golden changes **only on purpose**: `W5K_BLESS=1 cargo test -p <crate> --test <name>`, and the PR description contains `Golden-Change: <why>`. CI's golden guard
  rejects a PR that touches a golden without that line. Never bless to make a failing test pass.
- Hash state with `StateHasher` (`write_f64` normalises `-0.0`; hash vectors, quaternions and matrices with the provided helpers). Hash everything that evolves; do not hash derived values.

## When tolerance, not equality
Between *implementations* (the simulation vs a published figure, vehicle A vs vehicle B, a refactor that reorders arithmetic) compare with a tolerance and a band, never bit-equality.
Bit-equality is for regression of the same code on different machines.

## Fallback
If a compiler or CPU change breaks cross-platform identity and cannot be fixed, a decision card may drop goldens to "same-platform hash plus tolerance trajectory". Never silently.
