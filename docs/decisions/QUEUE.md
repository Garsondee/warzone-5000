# Decision queue

Anything that is the owner's to decide and that no document already settles becomes a **card** here. Nobody blocks on a card: work
continues on the **default** and is tagged `PROVISIONAL(card-id)` in the code, the design note or the PR until the owner answers.
ARCH keeps this file current. Lanes do not edit it: they write the card in their status file under "Cards needed" and ARCH lifts it here and numbers it.

## Card format
```
### C-NNN  <short question>
Asked by: <lane> | Date: <date> | Status: OPEN / ANSWERED (<date>) / SUPERSEDED
Options: (a) ... (b) ... (c) ...
Recommendation: <which and why, in two lines>
Cost of being wrong: <what has to be redone, roughly how much>
Default (what happens if nobody answers): <option>
Answer: <owner's decision, when given>
```

## Open cards

### C-004  Art pipeline
Asked by: ARCH | Status: ANSWERED (2026-10-09, other): parametric models only, built by us; more surface detail (panel lines in logical places, better tracks and skirts); the whole vehicle stays parametric, and so do landscapes, roads, mud, trees and buildings; textures simple and procedural. No hand-modelled assets.
Options: (a) procedural PBR camo and weathering plus a lofted hull-geometry kit; hand-modelled hero assets optional later; (b) hand-modelled assets from the start.
Recommendation: (a); the owner may supply hero assets at any time.
Cost of being wrong: shader and kit work partly redone.
Default: (a).
Answer: Parametric models only. The goal is to make it so that you can build everything yourself and the 'medium tank' proved that you can make something that looks very convincing. You will need to give them some more surface details, panel lines in logical places, better modelling of tracks and the skirts but overall your prototype models were very good. But the entire vehicle must be parametric to support the rest of the game so we might as well build parametric landscapes, roads, mud, trees and buildings and anything else we need along the way. Textures are simple, procedural.

### C-005  Can the owner drive any vehicle?
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a).
Options: (a) yes, keyboard or gamepad in the Godot front end, through the same `Command` as the AI; (b) watch only.
Recommendation: (a): an animator judges suspension by feel, and it is nearly free given `Command`.
Cost of being wrong: low.
Default: (a).
Answer: (a)

### C-006  What happens to the parked sci-fi families and the draft / command-point / shop loop?
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a).
Options: (a) parked in the archive, revisited at M5; (b) deleted.
Recommendation: (a).
Cost of being wrong: low.
Default: (a).
Answer: (a)

### C-008  Hold the fan-out until the weekly usage window resets (about 10 Oct 02:00 UTC)?
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a); the owner said go on 2026-10-10 after the reset.
Options: (a) hold rank 1 until after the reset if the Launch Kit finishes first; (b) launch immediately.
Recommendation: (a): the session metadata shows a seven-day usage warning; a parallel fan-out burns the allowance quickly.
Cost of being wrong: (a) loses hours; (b) risks hitting the limit mid-wave.
Default: (a).
Answer: (a)

### C-009  The old prototype code in the working tree
Asked by: ARCH | Status: DONE as (a) on 2026-10-08 (moved with `git mv`, one directory at a time; deleting stays the owner's word)
Options: (a) moved to `reference/prototype-v0/` (read-only, excluded from the workspace and CI); (b) removed from the tree entirely (history keeps it at `2854baf`).
Recommendation: (a) until the port ledger is done, then (b). Moving or deleting the code needs the owner's explicit word in this environment.
Cost of being wrong: none; both are one command.
Default: (a).

### C-010  When do we build the Windows package?
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a).
Options: (a) at M2, with Godot drive mode; (b) at M1 with the replay player.
Recommendation: (a), with the S8 spike (Godot in cloud sessions, gdext on 4.6) reporting earlier.
Cost of being wrong: the owner sees a playable build later than wanted.
Default: (a).
Answer: (a)

### C-011  Pace of the fan-out
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a).
Options: (a) three ranks over about a day, governed by the burn and CI health; (b) everything at once; (c) rank 1 only until M1.
Recommendation: (a).
Cost of being wrong: (b) burns the allowance and floods review; (c) leaves parallel-safe work idle.
Default: (a).
Answer: (a)

### C-012  CI minutes policy
Asked by: ARCH | Status: ANSWERED (2026-10-09): (a).
Options: (a) PR runs on Linux only (affected crates), Windows and goldens on `integration` and nightly; (b) Windows on every PR.
Recommendation: (a): Windows minutes cost double on private repositories.
Cost of being wrong: a Windows-only break is found at merge, not at PR time.
Default: (a).
Answer: (a)

### C-013  Extra permission rules for unattended lane sessions
Asked by: ARCH | Date: 2026-10-08 | Status: ANSWERED (2026-10-09): (b), applied by the owner in PR 6 (without the plain `rm` and `chmod` allow rules, which the CI settings tests keep on ask).
Options: (a) leave `.claude/settings.json` as the tooling agent wrote it and let lanes ask for a missing rule through an interface request; (b) also add rules to it: deny `git push` to `main` and `integration` (lanes open pull requests, ARCH merges), allow `python3 -I assets/*` and `python3 -B -I *` (the LOOK lane's tests), `cargo bench *` and `cargo doc *`, and plain `rm` plus a few harmless shell commands (`echo`, `printf`, `chmod`, `tee`, `test`, `stat`, `du`, `cut`, `tr`, `basename`, `dirname`, `realpath`, `which`, `date`, `pwd`); recursive `rm` stays refused.
Recommendation: (b). ARCH could not apply it: the auto-mode classifier treats an edit to this session's own permission file as self-modification, so it needs your word in chat ("apply C-013 b"), after which ARCH makes the edit. In Auto mode lanes are rarely stopped by prompts anyway, so the cost of waiting is small; the push deny rules are the part that matters.
Cost of being wrong: (a) a lane could push straight to `integration` (CI and review would still see it afterwards), or stall on a prompt in a non-Auto mode.
Default: (a).
Answer: Done

### C-014  GitHub branch protection on `main` and `integration` (two minutes in your GitHub settings)
Asked by: ARCH | Date: 2026-10-08 | Status: ANSWERED (2026-10-09): (a), the owner set up a GitHub ruleset for `integration` and `main` (pull request, checks `guards` and `rust`, owner bypass) and made `integration` the default branch.
Options: (a) you switch it on: for `main` and `integration` require a pull request and the status checks `guards` and `rust`, and restrict who may push (steps in `docs/swarm/GUARDRAILS.md`, "Limits worth knowing"); (b) leave it off and rely on the lane tool guard, the permission rules and ARCH's review.
Recommendation: (a): it is the only protection that does not depend on a session behaving, and ARCH has no tool to set it.
Cost of being wrong: (b) one stray direct push to `integration` has to be found and reverted by ARCH at the next check-in.
Default: (b).
Answer: Done

### C-015  How does the owner's five-year-old son test-drive a vehicle?
Asked by: ARCH | Date: 2026-10-10 | Status: OPEN, default taken (owner's goal: "Ideal ending point would also include the ability for my five year old son to give this vehicle a test drive"; a local folder `C:\Users\Hivemind\Documents\Warzone 5000` exists on the owner's Windows PC for builds)
Options: (a) a native `w5k drive` program (Windows exe built by CI) that runs the real simulation in real time and serves a browser page; the child drives with arrow keys, a gamepad or big on-screen buttons; kid assists on by default (speed cap, steering smoothing, auto-brake, auto-recover onto the road); (b) the Godot front end (C-005, C-010: M2), a proper game window with the same `Command`; (c) the simulation compiled to WebAssembly so the page needs no install.
Recommendation: (a) now, (b) as the long-term front end. (a) needs no new dependency and no Godot work, reuses the three.js viewer and the real physics, and can be built and tested in the cloud; the owner downloads one zip from GitHub Actions into the local folder and double-clicks. (c) needs a wasm toolchain and likely a new dependency (a card).
Cost of being wrong: (a) is thrown away when Godot lands, but the protocol, the assists and the kid-mode design carry over; low.
Default: (a).
Answer: (pending; the owner asked for the capability but did not choose the route). In practice: the owner downloaded and ran the Windows package on 2026-10-10 ("I love the demo") and asked for the speed limiter to be removed from `START.bat`; `START-KID.bat` keeps it. Route (a) is working.

### C-016  What is slice 2?
Asked by: ARCH | Date: 2026-10-10 | Status: ANSWERED (2026-10-10, owner in chat: "go ahead with slice 2"): (a), The Design Loop. TRACKS launched, COMBAT launched for spike S6 and the ballistics kernel only. (the owner asked for "a good objective ... something that elevates most parts of the systems")
Options: (a) "The Design Loop" (`docs/swarm/SLICE-2.md`): the first tracked vehicle and soft ground that decides who passes, the Design Impact Matrix running for real, and a Workshop page where a design change moves measured numbers and can be driven; TRACKS launched, COMBAT for the S6 spike only; (b) guns and targets first (M3); (c) the AI driver first (M4); (d) the Godot front end first.
Recommendation: (a). It retires the two contract regions no code has touched (tracks and soil S3 and S4, articulation S6) before the freeze, measures the claim the whole project rests on (design changes outcomes) before stacking guns and AI on it, and gives the owner a tank and a workshop. (b), (c) and (d) are easier after it.
Cost of being wrong: (a) is bigger than slice 1 (about two five-hour windows); drop stage C first if the budget is tight. (b) to (d) risk building on an unmeasured base.
Default: (a), but no new lane (TRACKS, COMBAT) launches until the owner says go in chat, because it spends money.
Answer: Go ahead with slice 2 (owner, chat, 2026-10-10 17:10 UTC). Recorded; C-017 and C-018 keep their defaults until the owner answers.

### C-017  Where do validation's published figures come from?
Asked by: VALIDATION (spike S9), numbered by ARCH | Date: 2026-10-10 | Status: OPEN, default taken
Options: (a) the owner allows the source sites in the cloud environment's Network access settings (Edit, Allowed domains: army.mil, apps.dtic.mil, archive.org, Wikipedia and similar); (b) the owner drops manuals and papers into `content/dossier/sources/`; (c) carry on with `UNVERIFIED` secondary figures that are never promoted to verified.
Recommendation: (a) or (b): without published numbers "validated against real vehicles" stays a claim. (b) is the more controlled; (a) is the less work.
Cost of being wrong: (c) leaves every real-vehicle light grey; the closed-form oracles still judge the physics.
Default: (c).
Answer: (pending)

### C-018  Tolerance class for closed-form oracle tests
Asked by: VALIDATION, numbered by ARCH | Date: 2026-10-10 | Status: OPEN, default taken
Options: (a) green within 10%, amber within 20% (an oracle is exact for an idealised vehicle, so the band is for what the idealisation leaves out); (b) the published-figure classes of ADR-0007 (3% to 15%); (c) per-test bands.
Recommendation: (a), tagged `PROVISIONAL(C-018)`: measured stops are +11%, +30% and +76% from the braking oracle for the Mule, Scout and Hauler, which is information, not noise.
Cost of being wrong: lights change colour; no physics changes.
Default: (a).
Answer: (pending)

### C-019  What the mud pit is supposed to prove (which vehicles bog in the published clay)
Asked by: WORLD (`docs/lanes/world/clay-bogging.md`), numbered by ARCH | Date: 2026-10-10 | Status: OPEN, default taken
Options: (a) keep the published Bekker-Wong clay numbers; on level clay none of Scout, Mule, Hauler bogs (margins 0.114 / 0.104 / 0.061, carrier 0.350) and the 10% pit walls separate them (the Hauler bogs, the Mule and Scout cross, the carrier crosses): reword acceptance sentence 1 of SLICE-2.md to say so; (b) add a second, softer cited soil row so the Mule bogs on level ground (needs a source from VALIDATION); (c) deepen or lengthen the pit until the Mule bogs (tuning the course to get a result).
Recommendation: (a), tagged `PROVISIONAL(world-clay-acceptance)`: it is what the physics says with the published figures and the sim agrees with the hand calculation. Never tune soil numbers to get a result. Caution recorded: the clay friction-angle band moves the Mule's margin from 5% to 16%, so the Mule is a knife edge.
Cost of being wrong: the headline demo would show one bogging wheeled truck instead of two; no physics changes.
Default: (a).
Answer: (pending)

### C-021  Defaults for the design model's open questions (armour sliders, overrating, tech level, ranges)
Asked by: ARCH, from the owner's direction C-020 | Date: 2026-10-10 | Status: OPEN, defaults taken
Options: (a) armour as one slider per facing (front, side, rear, top, turret), engine rating as a slider above 100% that adds an aperture share and a durability penalty, parts gated by an era only later, wide slider ranges and the model does the punishing; (b) one armour slider, rating fixed at 100%; (c) tight slider ranges that keep designs sensible.
Recommendation: (a): it follows the owner's words (free to build wacky vehicles that can fail; weak points that cannot be armoured) and costs nothing until COMBAT needs it. Every price is an `ESTIMATE` with a band until sourced (C-017).
Cost of being wrong: low; the sliders and prices are data and can be reshaped without touching the solvers.
Default: (a), tagged `PROVISIONAL(C-021)`.
Answer: (pending)

## Answered cards
*(the owner's four scoping answers are recorded as ADR-0001 to ADR-0004; the cards below were answered in the Control Room on 2026-10-08.)*

### C-001  What era is the game?
Asked by: ARCH | Date: 2026-10-08 | Status: ANSWERED (2026-10-08, other): baseline is very late WW2 and early Cold War tank design, with the technology we model running from there to today and onwards into the futuristic; the long-term single-player fiction is an early-Cold-War military fighting an alien invasion with a rapid vehicle-prototyping programme. Realistic ground vehicles stay the scope now; exotic technology comes later, as data.
Options: (a) modern-ish, c. 1950s to today, mixed-era reference set; (b) WW2 only; (c) both, as separate content packs.
Recommendation: (a). The physics must generalise (leaf springs and manual boxes are a subset of the machinery torsion bars and automatics need), so the reference set mixes eras; the game's era can be narrowed later.
Cost of being wrong: low until M2 (the content catalogue), moderate after.
Default: (a).
Answer: If we ever get to my idea for the single player version of this game try to imagine an early cold war era military which suddenly finds itself fighting off an alien invasion. Players are part of a group who have access to a new program designed to rapidly construct and prototype new vehicles in an effort to defeat opponents who are using exotic materials and technology. So as a rough guide think about tank design from very late WW2, early Cold War but we have to consider the full tech from that point to today and onwards into futuristic tech.

### C-002  Which real vehicles are in the reference garage?
Asked by: ARCH | Status: ANSWERED (2026-10-08, other): a slice of vehicles in every role, not only assault, to exercise the vehicle builder and get semi-realistic results before exotic technology. The game is fiction, not the real Earth: fictional vehicle names, archetypes based on real vehicles for physics grounding, ideally original hulls built on real hull-shape logic. Validation dossiers stay real vehicles.
Options: (a) calibration M998 HMMWV, M113A3, M4A3 Sherman; held out M1A1 Abrams, Leopard 2A5, T-72B, M35 6x6, Tiger II; (b) the owner's own list.
Recommendation: (a): well-documented civilian-adjacent vehicles first, tanks held out.
Cost of being wrong: dossier research is redone for swapped vehicles (days).
Default: (a).
Answer: We should do a whole slice of vehicles from all different roles not just assault but the primary reason for doing this is to fully flex and test the vehicle building and also so that we can test those vehicles and get semi-realistic results before then trying much more exotic or unusual technology. We do not need or want this to be a game about the real earth, it takes place in fiction, so real vehicle names aren't what we need but archetypes based on real world vehicles are ideal to try and give some good physics grounding. Ideally we'd have our own fictional hull designs but those would be based off using the same hull shape logic that you might find in the real world.

### C-003  How are courses authored?
Asked by: ARCH | Status: ANSWERED (2026-10-08, by note): default (a) stands. We build the courses ourselves, so the tooling must be strong (generator, checks, previews); an editor is a very long-term milestone.
Options: (a) data files made by a procedural generator first, an in-Godot editor later; (b) a Godot editor plugin from the start; (c) hand-built in Blender.
Recommendation: (a): testable and reproducible; the editor follows once the format settles.
Cost of being wrong: the editor is built later than wanted.
Default: (a).
Answer: Ideally you'd build them yourself which means you'd need strong tools for helping you to do that. An editor is a very long term milestone.

### C-007  Which model do the lanes run on?
Asked by: ARCH | Status: ANSWERED (2026-10-08, other): ARCH runs on the strongest model tier and every lane on the default tier; a stalled lane is not upgraded without asking. The lane model is passed explicitly at launch (a session otherwise inherits its launcher's model). The exact names are in the owner's note on this card in the Control Room database.
Options: (a) the same as the coordinator for every lane; (b) a stronger model for CHASSIS, TRACKS and ARCH, the default for the rest; (c) the default everywhere, upgraded when a lane stalls twice.
Recommendation: (c): measure first; the burn is reported at every check-in.
Cost of being wrong: money, or time lost to a lane that stalls.
Default: (c).
Answer: See the owner's note on this card in the Control Room database (cards/C-007); the repository carries no model names.

### C-020  How a vehicle design relates to its properties (mass is not a slider)
Asked by: the owner, unprompted, in chat | Date: 2026-10-10 | Status: ANSWERED (owner, chat, 2026-10-10): vehicle weight must come from the configuration, not a slider: a more powerful engine weighs more, more armour weighs more, a bigger weapon weighs more. It must be possible to build a configuration that does not work (a huge weapon makes the vehicle immobile, dangerous to drive or unusable off road); people are free to make wacky vehicles and the model is just good enough that a high-risk high-reward creation has real chances to fail in a real situation. More engine in a recon vehicle is a bigger target and easier to destroy, and past a point extra speed adds risk for little survivability; an engine can go over 100% at the cost of weak points that cannot be armoured. Some things are sliders, others are emergent properties of the parametric design; think about what else is displayed but not a slider.
Reading (ARCH; the owner corrects it here): three kinds of quantity (choices, consequences by design, consequences measured); no slider without a price; compile rejects only what the solver cannot represent, everything else is built, simulated and flagged; vulnerability emerges from component exposure plus apertures that need openings (cooling grows with power) and cannot be armoured. Written up in `docs/architecture/DESIGN-MODEL.md`; the Workshop loses its Mass slider now and FORGE builds a component mass budget (stage D2).
Options: none (owner direction). Open sub-decisions with defaults: C-021.
Answer: (the owner's message above)
