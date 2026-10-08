# Working notes for Claude on warzone-5000

This repo builds a **realistic ground-vehicle simulator** (wheeled and tracked, 1-80 t) that grows into an auto-battler. The physics is
the product: design choices (suspension, gearbox, tracks, hull, turret) must change outcomes you can measure. Several Claude sessions
("lanes") work in parallel; **the guardrails live here and in `docs/`, not in anyone's prompt.** Read this file fully, then your brief.

## Read first
1. `docs/brief/BRIEF.md` (what we are building and why; two pages) and `docs/brief/NON-GOALS.md` (what we are not).
2. **If you were started as a lane: your brief, `docs/swarm/lanes/<lane>.md`.** It is your mission, your paths, your first deliverable and your tests.
3. `docs/swarm/RULES.md` (how lanes work together), `docs/architecture/CONTRACTS.md` (the interfaces you build against).
4. If you are the owner's chat session (ARCH, the coordinator): `docs/swarm/STATE.md` has the live picture.

## How the owner likes to work
- **Show the visuals.** Whenever something visible changes (a vehicle, a course, a plot, a UI), render it and send the image or clip
  (`SendUserFile` in a chat session; a small PNG under `docs/lanes/<lane>/media/` in a PR). Not just a description.
- **Teach the theory.** The owner has a Masters in computer graphics and animation, is not a programmer, and wants to learn. For each
  design or engineering decision explain the principle (the physics, the maths, the trade-off) briefly and concretely; graphics
  analogies are welcome. Each lane writes `docs/theory/<lane>.md` in plain language.
- **Ask with a default.** If you are blocked on a decision that is the owner's, write a decision card (`docs/decisions/QUEUE.md`
  format), take the default, tag the work `PROVISIONAL(card-id)` and carry on.
- Commit trailers carry **no model identifier**: end commits with `Co-Authored-By: Claude <noreply@anthropic.com>` and the
  `Claude-Session:` line you are given.
- Do not copy Warzone 2100 code or assets (GPL); the research notes in `docs/research/` are notes only.

## Conventions (CI enforces most of these)
- **Units are SI** (m, kg, s, N, W, rad). The unit is the field-name suffix: `mass_kg`, `length_m`, `torque_nm`, `power_w`,
  `angle_rad`, `omega_rad_s`, `rate_n_m`. km/h, kW, degrees and rpm exist only at the edges (display, spec sheets, RON authoring).
- **Frame: right-handed, +Y up, -Z forward, +X right** (Godot's). Positive yaw (about +Y) turns the nose **left**; positive pitch raises the
  nose; positive roll lowers the right side. Command `steer` is +1 = right. Wheel spin is positive when rolling forward. Details and
  tests: `docs/architecture/UNITS-AND-FRAMES.md`.
- **Numbers: plain `f64`, strict discipline.** Every transcendental from `libm` via `w5k_math::scalar` (clippy bans `f64::sin` & co);
  no fast-math; fixed time step; ordered iteration (no `HashMap`/`HashSet` in simulation code); NaN or infinity is a fatal bug. Tiny values
  are flushed on purpose, not by luck. See `docs/architecture/DETERMINISM.md`.
- **No bare physical constants in code.** They live in RON as `Param` (value, band, provenance `SPEC/MEASURED/ESTIMATE/TUNED`, source).
  A mathematical constant or tolerance in code carries `// const-ok: <reason>`.
- **Contracts are shared and versioned** (`crates/w5k_contract`). You may read them and build against them; you may not edit them
  except through a contract change request (a PR titled `CCR: ...`, ARCH reviews every line).
- **Goldens** (anything under `tests/golden/`) change only deliberately: `W5K_BLESS=1 cargo test ...`, and the PR description says
  `Golden-Change: <why>`. Never bless to make a failing test pass.
- Every outcome must be explainable: the force ledger records every force term; a run that ends early says why.
- Gameplay-affecting content is RON under `content/<lane's dir>/`; every file must pass `cargo test --workspace`.

## Rules of the road for lanes (full text: `docs/swarm/RULES.md`)
- One writer per path: `docs/swarm/ownership.toml` says who owns what, and the CI **lane guard** rejects a PR that touches
  another lane's files. Need something from another lane? File an *interface request* `docs/swarm/requests/<yourlane>-<topic>.md`.
- Branch `lane/<lane>/<topic>`, PR into **`integration`** (never `main`; `main` moves only at an owner-approved milestone).
- **Your first PR is a one-page design note** (plus your risk spike, plus the CCRs you expect, written as text in the note). Open it, then keep
  building: do not wait for the merge. Start with the steps of your brief that need only your own crate and `w5k_math`; anything that depends on a
  CCR answer waits for `contract-v0.2`, and if the review changes something you used you adapt. Update `docs/swarm/status/<lane>.md` with every PR
  (done, blocked, next, cards needed).
- **Merge gate** (`docs/swarm/MERGE-GATE.md`): an analytic-oracle test or a validation entry (or an `UNVALIDATED` tag with a source),
  a theory note, golden and impact-matrix diffs explained, and an image or clip. Tests are named as physics sentences
  (`braking_distance_matches_v2_over_2mu_g`).
- **No new dependency** without an approved card (CI checks that every dependency is `workspace = true`). PRs stay under 400 lines.
- **Tripwires: stop and write a note in your status file** (the full list is `docs/swarm/RULES.md` section 8): a PR over 400 changed lines of
  non-test code; a new dependency; a constant without provenance; a contract touched without a CCR; deleting tests; work outside your brief;
  a refactor "while you are there"; a result you cannot explain from the force ledger.
- Push work in progress at least hourly (containers are ephemeral). When your mission's deliverables pass, stop, write the handoff note,
  and idle; do not invent more work.

## Build and test
- `cargo test --workspace` (debug profile has overflow checks), `cargo test --workspace --release`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`.
- `cargo run -p w5k_tools --bin w5k -- scenario first-light --out out/first-light` runs the integration spine and writes a replay.
- Python tools: `python3 -I tools/ci/<script>.py` (standard library only). Cloud sessions cannot reach every CDN; npm packages are installed
  by the SessionStart hook, not fetched at run time.

## Where things live
`crates/` one crate per lane (`w5k_contract` and `w5k_math` are ARCH's) · `content/` RON data per lane · `assets/` shaders and materials ·
`tools/viewer/` the three.js reference viewer · `game/` the Godot project · `docs/brief|architecture|decisions|swarm|validation|theory` ·
`docs/archive/` and `reference/` the superseded prototype (read-only; commit `2854baf`) · `spikes/<lane>/` throwaway experiments.
