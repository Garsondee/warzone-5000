# Slice 2 proposal: "The Design Loop" (change a design, measure it, explain it, drive it)

*ARCH, 2026-10-10. A proposal for the owner (card C-016). Nothing in it is launched; new lanes cost money, so it waits for "go".*

## 1. Where slice 1 actually left us (honest scorecard)
Slice 1 was a demo and it worked: three wheeled vehicles drive the same 520 m course, you can watch it, and your son can drive it. The M1 *gates*
(the numeric exit criteria in the plan) are only partly met, and a plan for slice 2 has to start from that.

| Claim the project makes | State now |
|---|---|
| Design choices change outcomes you can measure (the Design Impact Matrix is the north-star test) | **Never run.** The runner does not exist. We have only seen it informally (the Scout rides worse than the Mule over the whoops, and the reason is a resonance we can explain). |
| The vehicle model is checked against real vehicles | One dossier (M998), every figure `UNVERIFIED`: lane sessions cannot reach army.mil, DTIC, archive.org or Wikipedia. Today we judge against closed-form physics (braking `v^2/2 mu g`, tipping `atan(t/2h)`), not against published numbers. |
| Terrain roughly matches the real world | At last check every roughness spectrum fell as `n^-4` where ISO 8608 says `n^-2`: **all red**. WORLD owns the fix. |
| Soft ground matters | **Only as friction.** The tyre code reports `sinkage_m: 0.0`: mud and sand change grip and rolling resistance, but nothing sinks or bogs. The Bekker numbers are data nobody consumes yet. |
| Tracks, turrets, guns | Not started. TRACKS and COMBAT were never launched, so spikes **S3 (track contact), S4 (soil) and S6 (turret and recoil)** have not run. The plan says those three gate the contract freeze. |
| Handling | CHASSIS lists its own known gaps: no tyre load sensitivity (load transfer never costs grip), a linear tyre curve with no slide drop, single-ray contact. |
| Speed | Fine. Three vehicles x two runs x ~70 s of driving took about 2 s of wall clock: roughly 150 to 200 times faster than real time per vehicle, plotting included. Twenty vehicles live is not a worry yet. |

## 2. The objective
> **Change a design lever, watch a real measurement move, and read why, for a truck and for the first tracked vehicle, on firm ground and in mud. Then drive the result.**

That is the product thesis in one sentence, and it makes every active lane do something it has not done before.

**What you would see at the end** (in the order of the story):
1. **A tank.** A tracked carrier (M113 archetype, fictional name, about 11 t) on the course; your son can drive it with the arrow keys (skid steering).
2. **Mud decides.** A mud pit. The trucks sink and bog where the carrier floats. A "ladder" chart: add a tonne at a time (or narrow the tracks) and watch sinkage climb to the bog point, next to the number the soil theory predicts.
3. **The scoreboard.** One page: every vehicle x every proving-ground test (braking, 0-48 km/h, gradeability, skidpad, side-slope rollover, step, ride), each with an oracle or a source and a traffic light, plus the terrain lights.
4. **The tornado.** The Impact Matrix running for real: nudge each lever +-10%, bars for how far each benchmark moved, dead levers named honestly.
5. **The Workshop.** Sliders on the page (wheelbase, tyre size, spring rate, engine power, mass, track width). The body reshapes, the scoreboard numbers update in seconds, and a DRIVE button takes that exact vehicle onto the course.

## 3. Why this, and not guns, AI or Godot
The reasoning matters more than the pick, so here it is.
- **Test the hardest shot first.** In a film pipeline you render the hair or the water shot before building the other hundred shots, because if the pipeline cannot do it you must know before everything depends on it. Our "hair shot" is tracks on soft ground (S3, S4) and a turret that kicks the hull (S6). They are the two parts of the contract no code has touched. Finding out after the contract freezes means a version bump that every lane migrates through. Finding out now costs a spike.
- **Prove the base before stacking on it.** Guns and AI sit on top of "vehicle designs change outcomes". That claim has never been measured. Adding complexity above an unmeasured base is how projects spin outwards (your fear from the start).
- **The cheap parts are already built.** The proving-ground runner exists (braking and acceleration merged, rollover, step, skidpad and gradeability in progress); the Impact Matrix runner is mostly "run it twice with one number changed". GEOMETRY already makes bodies from parameters; the Workshop is mostly plumbing.
- **It sets up what comes next.** The AI needs a *measured* capability table and tanks to drive; guns need a tracked hull to recoil into; Godot wants stable rigs to present. Each is easier after this slice.
- **It is fun.** A tank, mud, and building your own truck are things a five-year-old and a vehicle designer both enjoy.

Analogies: a lever is a slider and a benchmark is the pixel difference between two renders; a *dead lever* is a slider that changes nothing, an *orphan effect* is a difference nothing can cause. Soil is a spring (Bekker): ground pressure is weight divided by footprint area, like spreading the same paint over a bigger brush, so wide tracks float where narrow tyres dig in.

## 4. What each lane does
| Lane | Slice 2 job | Status |
|---|---|---|
| TRACKS | S3 and S4 spikes, then soil laws (Bekker, Mohr-Coulomb, Janosi-Hanamoto), track contact, skid steering, the ladder bench | **new lane** |
| COMBAT | Spike S6 only (turret and recoil on a free hull) and the ballistics kernel; no integration | **new, spike-only** |
| CHASSIS | Tracked hull on the same integrator; tyre load sensitivity; tyres that sink (rigid-wheel Bekker, `UNVALIDATED`) | continues |
| DRIVE | Tracked drive hook, crawl torque, fuel and range bench | continues |
| WORLD | ISO 8608 spectrum fix; mud pit with cited soil parameters; proving-ground fixtures; a per-vehicle **mobility map** (go, slow, no-go over the course) | continues |
| FORGE | Tracked `VehicleDef` compile; a lever API for the impact runner; designer sliders | continues |
| GEOMETRY | Carrier hull and track run on the module kernel; skins that follow `VehicleDef` live | continues |
| VIEWER | Track animation, sinkage and ground-pressure overlay, ladder and tornado charts, the Workshop page | continues |
| VALIDATION | Terramechanics oracles (Wong), the remaining scorers, the Impact Matrix runner, the dashboard grid | continues |
| ARCH | Glue for tracked vehicles, proving results into the contract's `CapabilityTable`, contract v0.3, the package | |
| LOOK | Camouflage and weathering on the carrier and the Workshop (optional) | idle until needed |
| AI, GODOT | Stay parked. AI starts next slice from the mobility map and the capability table. | parked |

Eleven of thirteen lanes do real work; two stay parked on purpose.

## 5. Stages (each ends with something visible; you can stop after any of them)
- **A, Measure honestly.** Finish the proving battery for the three trucks; terrain spectrum fix; tyre load sensitivity; Impact Matrix runner on the wheeled levers; TRACKS spikes S3 and S4 with the soil kernel and Wong's textbook cases; COMBAT S6. *You see:* the dashboard with lights, the tornado chart, plate-sinkage curves.
- **B, Mud and steel.** Tracked running gear integrated; carrier skin; mud pit; trucks sink; skid steering in kid mode; the ladder; the mobility map. *You see:* a clip of a truck bogging while the carrier floats, and package v3 with the carrier in the picker.
- **C, Workshop** (starts in parallel with B on the wheeled trucks, the carrier joins when B lands). *You see:* you edit a vehicle, the numbers move, you drive it.

## 6. Acceptance (named as physics sentences; the numbers are starting values)
1. `the_carrier_finishes_the_course_and_the_mud_pit_unaided`, while the Mule and Hauler bog at a ground pressure within 25% of what `z = (p / (kc/b + kphi))^(1/n)` predicts.
2. `pivot_turn_moment_is_within_25_percent_of_mu_W_L_over_4` (the S3 kill criterion) and the track-speed turn radius equals the kinematic value.
3. Impact Matrix: at least 80% of expected signs right; every dead lever and orphan effect listed with a written reason.
4. The proving battery runs for every vehicle with an oracle or a cited source behind every light; a run that ends early says why.
5. Road and ground roughness fall inside their ISO 8608 class bands.
6. Workshop: change any of six levers and the body and scoreboard update in under 10 s, and the changed vehicle can be driven.
7. Hash chains identical on Linux and Windows; `integration` green; the Windows package downloads and runs.

## 7. Pace and cost
Slice 1 took about 14 hours and roughly $80 of lane spend with eight lanes. This adds two lanes (one full, one spike-only) and keeps the existing ones working, so it is bigger: plan for about two five-hour windows, with the same hourly check-ins and the same rule that I stop launching if the window runs low. Active sessions would be 11 including me (under the limit of 13). If the budget is tight, drop stage C first, then the COMBAT spike.

## 8. What I need from you (cards with defaults; work proceeds on the default)
- **C-016, go for slice 2 as proposed?** Default: yes, but no new lane launches until you say go in chat.
- **C-017, validation sources.** Real validation needs published numbers, and lane sessions cannot reach them. (a) Allow the sites in the environment's Network access settings (cloud environment menu in the session title bar, Edit, Allowed domains); (b) drop PDFs into `content/dossier/sources/`; (c) carry on with `UNVERIFIED` secondary figures. Default: (c).
- **C-018, oracle tolerance class.** Exact physics oracles need their own band: green within 10%, amber within 20%. Default: that.
- **Your son's first drive.** The speed cap, steering and braking help numbers are my estimates. A note of what he found hard is worth more than any tuning I can do blind.

## 9. Risks I am watching
- **Tracks are the hard physics.** If S3 cannot get a pivot within 25% of theory without tuning, we change the track model before any vehicle depends on it. That is what a spike is for.
- **Review load.** More lanes means more PRs; I keep merge stewards short-lived and PRs under 400 lines.
- **Fiction versus validation.** The carrier is fictional; its numbers come from an archetype. Until sources are reachable, "validated" means "consistent with first-principles physics", and the dashboard says so.
