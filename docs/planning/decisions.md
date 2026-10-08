# Decision Log

Short records of decisions that are expensive to reverse. Newest first. Each entry: what, why, alternatives, consequences.

## D10 (2026-10-08, proposed): Trade-offs come from price and physics gates, not from mass alone
- **What:** the sampled possibility space ([../design/07-possibility-space.md](../design/07-possibility-space.md)) shows that at
  equal mass firepower, armour and speed are nearly independent (rank correlations -0.17, +0.01, +0.05); speed is almost free
  (the engine is roughly 2-15 % of the vehicle and each kind of running gear tops out at its rating); anti-gravity dominates
  (98 % valid, fast at every size, no ground pressure, power linear in the weight); sight is flat (median 3.0 km, 7 % above 4 km).
  The proposal is that every part carries a price in command points that is **not** just its mass: speed and lift are priced by
  power, anti-gravity is a late-tier gate with an upkeep or a core that does not scale down, and sensors differentiate sight.
- **Why:** a constraint only shapes decisions where it binds, and mass binds weakly. "Physics keeps titans honest" already works
  (walkers crawl, rotors stop near 100 t); "balance is found by the player" needs a second scarce resource to find it with.
- **Alternatives:** tighten the mass budget (does not touch speed, which is cheap in mass); hard caps by rule (throws away the
  physics story); do nothing and let the meta find it (the dominant choice then wins every match).
- **Consequences:** lands in M6 (command-point pricing). Until then the sample is the regression test: the correlations and the
  corners of the space should move when a price or a gate is added. Awaiting the owner's decision.

## D9 (2026-10-08): The possibility space is sampled, not argued
- **What:** `w5k space` rolls thousands of auto-fitted designs (every hull with every gear, many sizes) with the quick
  evaluation path and charts them: speed against mass, armour against firepower per tonne, sight against range, ground pressure,
  which mixes survive, the trade triangle, the corners, and what one budget buys
  ([../design/07-possibility-space.md](../design/07-possibility-space.md)).
- **Why:** with eighteen families and continuous sliders nobody can reason about the whole space; sampling shows where it is
  empty, where it is crowded and which constants are wrong. It is also the first balance instrument.
- **Alternatives:** hand-picked archetypes only (miss the edges); full voxel builds for every sample (a hundred times slower).
- **Consequences:** results depend on the fitter's assumptions (the sample is stratified over hull x gear, power per tonne is drawn
  from a range, armour is sampled thin) and on constants that are hypotheses; the quick path is within about 10-20 % of the full
  build. The charts are for finding problems, not for pricing.

## D8 (2026-10-08): Auto-fit sizes designs; a diet fixes what gear cannot carry
- **What:** `Explorer::fit` sizes running gear to its share of the weight and the engine to the vehicle, iterating to a fixed point;
  when the gear cannot carry the vehicle it thins the armour in steps (a "diet") before giving up. Fitter hints that are not
  sliders (`margin`, `kw_per_t`) live in the attachment parameters.
- **Why:** designs must be coherent to be compared; the same function is the AI designer's repair step and the player's "fit this
  gear to my vehicle" button.
- **Alternatives:** leave every design to hand sizing (rejects most random mixes); size by lookup tables (loses the physics).
- **Consequences:** the fitter can only fix what the sliders can reach; a design that still fails reports the reason (overloaded,
  cannot hover, engine does not fit). Viability charts therefore show both "valid" and "how much armour it had to give up".

## D7 (2026-10-08): One mounting vocabulary; physics gates combinations
- **What:** hulls publish standard sockets (`gear`, `station`, `keel`, `belly`, `turret`, `mast`, `hub`, `engine`) with sizing
  hints; families declare the socket kinds they fit. Running gear that lifts the hull (`ride_height_m`) raises it (`lift_m`).
- **Why:** the owner wants to mix and match components in fun ways; rules about which combinations are "allowed" would cut the
  possibility space and need constant upkeep. Physics (load rating, lift power, engine bay, ring size) already says which ones work.
- **Alternatives:** a compatibility matrix (brittle); per-hull bespoke gear (no mixing).
- **Consequences:** every new family must publish a `host()` for sweeps and the sheet must check what it adds; odd mixes (a
  walker on rails' keel, anti-gravity on a tank) are legal and are shown, with their problems, rather than hidden.

## D6 (2026-10-08): Components are parametric families, written as generators
- **What:** each component (turret, hull, engine, locomotion, sensor) is a *family* with sliders. A family is Rust code that
  turns slider values into an ordinary part tree; the forge then measures the real geometry. Sliders are free, budgeted
  (coupled through the part's mass by a solver that respects locks) or physically opposed. See
  [../design/04-parametric-components.md](../design/04-parametric-components.md).
- **Why:** the owner's revised concept: fewer tech items, a far larger design space, and balance found by the player.
  Generators keep stats honest (measured, not tabulated) and can follow real scaling laws.
- **Alternatives:** fixed part libraries (rejected: tech bloat, no design space); expressions inside RON part files
  (simpler for modders, too weak for sizing loops and conditional geometry); an embedded scripting language such as Rhai
  (worth revisiting when modding matters; the family interface is designed so a scripted family can implement it).
- **Consequences:** a design is (family id, slider values) per component, plus attachments; the fixed RON parts remain for
  hulls and props until their families exist. Interactive use needs a fast mass estimate (done) and background
  measurement.

## D5 (2026-10-08): An async auto-battler; the deterministic simulation stays, lockstep does not lead
- **What:** the game is a design-first async auto-battler ([../design/05-game-loop.md](../design/05-game-loop.md)). A battle
  is a pure function of (roster A, roster B, map, seed). The simulation stays deterministic (fixed point, seeded RNG,
  ordered iteration) for replays, server verification and pricing benchmarks. Live lockstep PvP is no longer a milestone.
- **Why:** owner's revised loop: players design, test against bots, then send rosters into hands-off battles against
  other players' snapshots.
- **Alternatives:** keep live RTS control with lockstep (dropped by the owner); a non-deterministic simulation with
  recorded replays (bigger replays, no cheap verification, no reproducible pricing).
- **Consequences:** the battle AI becomes the central risk and moves earlier in the roadmap; a backend for snapshot pools
  and verification comes late and is small; D3's fixed-point rules still apply.

## D4 (2026-10-07): Parts are authored as primitive trees and simulated as voxels
- **What:** parts are written as trees of parametric convex primitives (box, wedge, cylinder, frustum..., with chamfer and taper)
  plus mirror/array/radial operators. For statistics they are voxelised. A `blocks` grid primitive (shaped voxels) will be added
  for the future player part editor.
- **Why:** an AI author places parametric shapes reliably, which it does not do for raw voxel grids; convex pieces give clean,
  faceted geometry without fragile mesh booleans; voxelising gives mass, armour and volume from the actual shape.
- **Alternatives:** pure voxel blocks (blocky look, hard to author from text); signed-distance-field modelling (heavier meshes,
  artefacts); general mesh booleans (a large, fragile project).
- **Consequences:** concave shapes are made by combining convex pieces ("kitbashing"); stats are baked at content-build time.

## D3 (2026-10-07): Online multiplayer is core; deterministic lockstep with fixed point (amended by D5)
- **What:** the gameplay simulation is deterministic: Q32.32 fixed point, fixed tick, seeded RNG, ordered iteration, state
  hashes. See [../design/02-determinism-rules.md](../design/02-determinism-rules.md).
- **Why:** owner's decision that multiplayer is core from day one; lockstep is the standard RTS approach and needs bit-identical
  simulation; fixed point removes cross-machine floating-point differences.
- **Alternatives:** careful floats with a cross-build hash gate (kept as a fallback); server-authoritative state sync (too much
  bandwidth for thousands of units).
- **Consequences:** engine physics (Jolt) is cosmetic only; ropes, legs and toppling are simulated by our own code.

## D2 (2026-10-07): Rust for the simulation and content core
- **What:** `w5k_math`, `w5k_forge`, `w5k_sim`, `w5k_tools` in Rust, loaded into Godot through a GDExtension (`w5k_godot`).
- **Why:** installed in cloud sessions (crates.io reachable) so Claude can build and test everything there; fast enough for
  thousands of units; full control of arithmetic.
- **Alternatives:** GDScript (too slow, float-based), C# (.NET unavailable in cloud sessions, no determinism by itself), C++
  (less safe, slower iteration).
- **Consequences:** Windows builds of the extension come from CI or a local Rust install; the owner mostly works with data files
  and Godot scenes.

## D1 (2026-10-07): Godot 4.6 as the engine
- **What:** Godot 4.6.x for rendering, UI, input, audio and network transport.
- **Why:** owner's choice; open source; plain-text project files; runs headless in cloud sessions (the 4.6.2 editor downloads
  there); Jolt physics and IK features available for cosmetic use.
- **Alternatives:** Unity (cannot run in cloud sessions; would need Claude Code on the owner's PC), Bevy (immature UI, frequent
  breaking changes), a web client (lower performance ceiling).
- **Consequences:** pin Godot 4.6.x and the matching `godot` crate API level; keep the bridge thin.
