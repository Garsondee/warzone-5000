# Non-goals

Things that look attractive and that we are **not** doing now. A lane that finds itself building one of these has drifted: stop and write a note.

| Not doing | Why | Revisit when |
|---|---|---|
| Multiplayer, netcode, lockstep | The owner: "not for a long time". Determinism is kept for tests and replays, not for synchronisation | The owner reopens it |
| Sci-fi locomotion (legs, hover, anti-gravity, rotors, rail) and beams | Scope pivot to realistic ground vehicles. The code is parked in the archive, not deleted | After M5, by owner decision |
| The draft / command-point / shop / three-lives run loop | Product order: simulator first | M5 |
| Machine-learning AI | "Adapts" means tactics and measured capability, which we can explain and debug | After M4, if the rule-based AI plateaus |
| A general-purpose physics engine (Rapier, Bullet, Godot physics) | We must own and be able to edit the dynamics. Queries only (`parry3d-f64`), by card | Never for the solver |
| A constraint solver / impulse-based contacts | Penalty contacts are transparent (every force is a plottable spring) and what vehicle simulators use | If the S1 spike shows penalty contacts cannot meet the step budget |
| Air, naval, infantry, economy, campaign | Out of the brief | Owner decision |
| Destruction, fracture, deformable terrain with persistent ruts | Large cost; damage degrades mobility parameters instead | After M3 |
| LOD for far vehicles; 1,000-unit battles | Up to about 20 vehicles at full fidelity | The S7 scale spike says so |
| New component families beyond the reference garage | Calibration first; breadth later | After M2 |
| GDExtension beyond what the M1 replay player needs (poses, interpolation, telemetry) | Godot is presentation; the sim is Rust, and the reference viewer is three.js. Drive mode, the exported Windows package and an in-Godot course editor are M2 | M2 |
| Hand-modelled hero assets; final hull art | Procedural geometry and shaders first | After M2, if the owner supplies assets |
| Tuning a model to individual vehicles | Only global constants are tuned, on the calibration set; the held-out set stays honest | Never |
| Fun overrides that quietly depart from reality | Allowed only as logged, owner-signed deviations | When the owner asks for one |
