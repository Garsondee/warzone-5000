# Swarm rules

How thirteen Claude sessions build one system without spinning out. Decision record: ADR-0008. If a rule here is in your way, file a card; do not route around it.

## 1. The lanes
| Lane | Squad | Mission (one line) | Crate / paths (owner table: `ownership.toml`) | Rank |
|---|---|---|---|---|
| **ARCH** | platform | contracts, CI and guardrails, ADRs, integration spine, merges, reviews, control room; writes no feature code | `w5k_math`, `w5k_contract`, `w5k_vehicle`, `w5k_sim`, root, `.github`, `.claude`, `docs/{brief,architecture,decisions,swarm}` | 1 |
| **CHASSIS** | dynamics | a wheeled vehicle that rides, grips and handles: hull dynamics, suspension, tyres, steering, aero | `w5k_chassis` | 1 |
| **DRIVE** | dynamics | engine map, converter or clutch, gearbox, diffs, tracked steering units, brakes with heat, fuel | `w5k_drive` | 1 |
| **WORLD** | world | the obstacle course: heightfield, materials, props, procedural generator, `WorldQuery` | `w5k_world`, `content/world` | 1 |
| **FORGE** | design | `VehicleDef` to rigs, mass and inertia kernel, designer-slider fits, reference garage | `w5k_forge`, `content/{parts,vehicles}` | 1 |
| **VIEWER** | visuals, platform | replay v2, the three.js reference viewer, MP4 capture, debug draw, scope plots | `w5k_replay`, `tools/viewer` | 1 |
| **VALIDATION** | truth | dossiers with citations, harness, dashboard, impact matrix, fuzz | `w5k_validate`, `content/dossier`, `docs/validation` | 1 |
| **GEOMETRY** | design, visuals | realistic hull, turret, wheel and track-link shapes, tagged by node | `w5k_geo` | 2 |
| **LOOK** | visuals | materials, camo, weathering, VFX, lighting, the art-direction doc | `assets/**`, `docs/art` | 2 |
| **TRACKS** | dynamics | tracked running gear and soft ground: Bekker and Wong, skid-steer | `w5k_terramech` | 2 |
| **GODOT** | platform | Godot 4.6 front end: bridge, replay player, drive mode, HUD, Windows package | `w5k_godot`, `game/` | 2 |
| **COMBAT** | behaviour | ballistics, penetration, damage, targets, turret and gun servos, recoil | `w5k_combat`, `content/combat` | 2 |
| **AI** | behaviour | perception, capability table, planner, driver, gunner, tactician | `w5k_ai` | 3 |

## 2. A lane's life
1. **Start.** Read `CLAUDE.md`, `docs/brief/BRIEF.md`, your lane brief `docs/swarm/lanes/<lane>.md`, `docs/architecture/CONTRACTS.md` and the contract you build against. Check out your branch `lane/<lane>/<topic>` from `integration`.
2. **Settle** (first hours; the settling round closes when ARCH tags contract v0.2, within 24 h): run your risk spike, write the **one-page design note** (`docs/lanes/<lane>/design-note.md`), file any CCRs. Open a PR with just those; ARCH reviews it. **Do not wait for the merge**: keep building the steps of your brief that need only your own crate and `w5k_math` (numeric kernels, mesh generators, shaders, research). Code that depends on a CCR you filed waits for `contract-v0.2`; if the review changes something you used, you adapt (that is the price of starting early). Stop only when your brief says so.
3. **Build** in the order your brief gives, against the stand-ins in `w5k_contract::testing`. Small PRs (under 400 lines). Every PR meets the merge gate (`MERGE-GATE.md`).
4. **Report** with every PR: update `docs/swarm/status/<lane>.md` (done, blocked, next, cards needed); commit small images to `docs/lanes/<lane>/media/`; keep `docs/theory/<lane>.md` current.
5. **Stop.** When your brief's milestone deliverables pass in CI, write the handoff note (what changed, what is unfinished, what surprised you, what you would do next) in your status file, push, and idle. Do not invent more work.
Push work in progress **at least hourly**: containers are ephemeral and a restart loses what is not pushed.

## 3. Branches, PRs, merging
- Branch `lane/<lane>/<topic>`; PR into **`integration`**. ARCH is the only merger, after green CI and an ephemeral reviewer's pass. **`main` moves only when the owner says so** (at a milestone).
- CI **lane guard**: a PR may touch only `ownership.toml`'s paths for its lane plus `docs/swarm/status/<lane>.md`, `docs/swarm/requests/<lane>-*.md`, `docs/theory/<lane>.md`, `docs/lanes/<lane>/**`, `spikes/<lane>/**` and `Cargo.lock`.
- Never rebase or force-push a branch someone else may have used; never edit `integration` directly.
- `Cargo.lock` is regenerated (`cargo update -w`), never hand-merged.

## 4. Contracts
`crates/w5k_contract` and `docs/architecture/CONTRACTS.md` are ARCH's. You **pin** a tagged version (`contract-v0.2`). To change it, **write the CCR as text** (in your design note during the settling round, or in a `docs/swarm/requests/<lane>-ccr-<topic>.md` afterwards): what is missing, the exact type change you propose, who else it affects. **ARCH implements CCRs** and tags the new version; if you have a ready patch, attach it as a separate PR titled `CCR: ...` (the only kind of PR allowed to touch `crates/w5k_contract/**`, and it may also make mechanical migration edits); ARCH reviews every line. `w5k_math` changes go the same way. ARCH tells the other lanes a contract changed (the changelog in `CONTRACTS.md` is the notice; FORGE re-reads it before compiling rigs). If the contract cannot express something you need, stop that task and file the CCR; do not work around it with a private convention.

## 5. Interface requests (how lanes ask each other for things)
No side-channel chatter. Write `docs/swarm/requests/<yourlane>-<topic>.md`:
```
From: <lane>   To: <lane>   Needed by: <milestone or date>
What I need: ...                       Why (which test or deliverable it unblocks): ...
What I will do meanwhile: (stand-in, PROVISIONAL decision)
```
ARCH routes it and answers in the same file. Urgent blockers go in your status file under "blocked".

## 6. Decision cards
Format and open cards: `docs/decisions/QUEUE.md`. **Lanes do not edit the queue** (`docs/decisions/**` is ARCH's): put the card text (question, options, recommendation, cost of being wrong, default) in your status file under "Cards needed"; ARCH lifts it into the queue at the next check-in and gives it a number. Never block on a question: take the default and tag the work `PROVISIONAL(C-nnn)`, or `PROVISIONAL(status:<lane>)` until the card has a number. A stand-in is not a decision and needs no card; a choice between real alternatives does. **If the owner messages you directly, treat it as high priority and record it as a card** (or an update to one) so the picture stays consistent. **If the owner's instruction conflicts with your brief or these rules**, tell the owner in one line, do not act on it yet, and put the conflict in your status file for ARCH.

## 7. Constants, dependencies, goldens
- Physical constants live in RON as `Param` (value, band, provenance `SPEC/MEASURED/ESTIMATE/TUNED`, source). A mathematical constant or tolerance in code carries `// const-ok: <reason>`.
- No new dependency without an approved card; every dependency is `workspace = true`.
- Goldens change only on purpose (`W5K_BLESS=1`) with `Golden-Change: <why>` in the PR description. **Each lane's goldens live in its own crate** (`crates/<crate>/tests/golden/`). The first-light golden (`crates/w5k_sim/tests/golden/first_light.json`) is ARCH's: it runs on stand-ins until ARCH swaps in your part, so your PRs should not move it; when your part later does, say so in your status file and ARCH re-blesses it at the swap.

## 8. Tripwires (stop and write a note in your status file)
A PR over 400 changed lines (added plus removed) of non-test, non-generated code (tests, RON data, documents and media do not count); a new dependency; a constant without provenance; a contract touched without a CCR; the test count dropping; work outside your brief; a "while I was there" refactor; a model that needs more than 8 substeps; a result you cannot explain with the force ledger.

## 9. Reporting and the owner
The owner is a computer-graphics person who is not a programmer and wants to see and understand. Every merge ships a plot or a clip; every lane writes a plain-language theory note (what the principle is, why we chose it, what trade-off, a graphics analogy where one fits). Test names are physics sentences.

## 10. The governor (ARCH)
At most 13 sessions at once; launched in three ranks; missions are bounded (a lane stops at its milestone deliverable). Hourly while any lane is active ARCH reads the status files, session status and usage, open PRs and CI; merges what is green and reviewed; answers cards and requests; nudges stalled lanes; refreshes the Control Room page. Launches pause if integration CI is red for more than two hours or more than five PRs wait for review. The owner can pause any lane or all of them.

## 11. Milestones (exit criteria are the gates; the owner signs off each with pictures)
- **M1 First light:** quarter-car natural frequency within 1% and damping ratio within 5%; energy drift under 0.1% per 60 s frictionless; acceleration, top speed and braking within 15/7/15% of the M998 dossier; identical hashes on Linux and Windows; at most 30 us per vehicle-tick end to end (CHASSIS up to 20, DRIVE up to 8, the glue up to 2); at least 80% of impact-matrix signs right; the Godot player plays a replay. Contracts freeze. *These are milestone gates, measured end to end by ARCH and VALIDATION once the parts are integrated; each lane's own acceptance lists the pieces it must deliver. "CI on Linux and Windows" in a brief means the `integration` tier; pull requests run the Linux tier (card C-012).*
- **M2 Mud and steel:** held-out median error at most 12% and maximum 25% over at least 20 quantities; mobility limits bracketed (published <= simulated <= 1.25 x published); a tank crosses road, mud and hill with an explainable outcome.
- **M3 Turret and targets:** recoil momentum conserved to 1e-6; traverse and elevation times within 20% of published; damage visibly changes mobility.
- **M4 Thinking vehicles:** at least 90% completion for five vehicles with no per-vehicle tuning; stuck under 3%; rollover under 1% over 100 seeds.
- **M5 Game loop:** decided at the M4 checkpoint.
Each milestone ends with a hardening step (no features: TODOs ticketed, docs match code, dead code deleted, debt ledger reviewed).

## 12. Failure handling
A lane that drifts is interrupted, its branch reset to the last green commit, its brief rewritten and the lane relaunched. A lane that finds the contract cannot express something files a CCR and stops that task. A restarted coordinator resumes from `STATE.md`.
