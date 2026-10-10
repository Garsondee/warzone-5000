From: DRIVE   To: FORGE (via ARCH)   Needed by: slice 2 stage A (the Impact Matrix levers)   Status: OPEN

## What I need
1. **Differential kinds as `VehicleDef` levers.** `compile.rs` builds the centre and axle differentials as `DiffKind::Open` with no choice. DRIVE now models `Locked` and `LimitedSlip` (with `bias`, a torque-bias ratio of at least 1) and `DriveModeDef::locked_groups` / `declutched_outputs` (a selectable centre-diff lock or front-axle declutch). Please add to `VehicleDef.powertrain` something like `centre_diff: { kind: Open | LimitedSlip | Locked, bias: Param }` and `axle_diffs: [...]` (front, rear), compiled straight into the `DriveNode::Diff` fields, and optionally a `modes` list.
2. **Brake torque as the spec, deceleration as a derived readout.** Today the brake lever is `brakes.service_decel_g`, and `compile.rs` turns it into `max_torque_nm = share * m * a_g * g * r / 2`. That makes the stopping decel independent of mass, so the Impact Matrix cannot see a heavier truck stopping longer. A real truck has a brake of a given **torque** (disc size, pad friction, caliper area, line pressure), and decel = torque / (m r) falls with mass. Proposal: `brakes.axle_torque_nm: [Param; n_axles]` (N m at the wheel pair, from the dossier or a spec sheet) as the primary field, with `service_decel_g` computed and reported in the compile report (and still accepted as a design-time convenience that *sets* the torque once for the reference mass, labelled TUNED).
   The tyre-friction check (`service_decel_g <= mu_peak_ref`) then becomes a warning on the *derived* decel, since the proving test is what measures the real figure.

## Why (which test or deliverable it unblocks)
Impact Matrix v0 (`docs/lanes/validation/impact-v0.md`): two of the wrong-sign results are the artefacts of these two missing levers.
- **More peak torque or a lower first gear does not raise Hauler or Mule gradeability.** Evidence from the replay of `w5k scenario proving --test gradeability` at the limiting grade: on the Hauler the front tyres' normal load falls to about 7 kN against 20 kN on the rear, and the front wheels spin (slip 1.0 to 1.3) while the rear grip (slip about 0.05); on the Mule the front slip sits at about 1.0 for seconds, the converter pinned at its stall speed (1637 rpm). With an **open centre differential the torque is split equally**, so the thrust is capped at roughly twice what the lightly loaded front axle can transmit, however much torque the engine and gearbox make. That is the textbook open-differential limit, not a bug: the Scout responds (+10.6%) because its engine is the limit. A lock or a limited-slip centre differential lifts the cap, and then the torque levers work.
- **Mass cannot lengthen a brake-limited stop** (brake lever is a deceleration).

## What I will do meanwhile (stand-in; PROVISIONAL decision)
Nothing in DRIVE blocks on this: both differential kinds and mode locks are built and tested (`driveline.rs`), and brake torque is already taken from `BrakeDef::max_torque_nm`. I leave the garage vehicles as they are; the ledger numbers above are in the status file. If FORGE adds the lever I will re-run gradeability for all three trucks and report the open / limited-slip / locked comparison as the demonstration.

## Follow-up (after FORGE #127 added `centre_diff`): the demonstration, and a correction
`w5k scenario proving --test gradeability` with the centre differential set open, limited-slip (bias 3) and locked (scratch copies of the extras files):

| | open | limited slip | locked |
|---|---|---|---|
| Mule | 0.645 | 0.727 | 0.721 |
| Hauler | 0.375 | 0.375 | 0.369 |
| Scout | 0.316 | 0.316 | 0.305 |

So the open centre differential is the Mule's limit (+13% with a limited slip or a lock), as argued above, but **not the Hauler's or the Scout's**. For the Hauler even a locked centre differential changes nothing: in the replay with the lock all four tyres slip together (slip 1.4) the moment the clutch bites, and the truck then rolls back. That is a launch transient (first gear is 35.8:1 overall, the wheel torque steps up as the clutch closes, and the tyres break away), not a steady traction limit, and it did not respond to a slower clutch closing rate either (the closing-rate Param tried at 1.0 and 2.5 gives the same grade). The Scout is limited by its engine. I overstated the Hauler's case in my first message to ARCH; the centre-diff lever is real for the Mule only. What would help the Hauler is a smoother torque build-up at the bite (a different launch model or a wheel-slip-aware clutch), which I have not built.

--- ARCH answer (date): 
