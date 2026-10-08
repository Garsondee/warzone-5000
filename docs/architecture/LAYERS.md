# Layers and crates

Each crate has exactly **one owning lane** (`docs/swarm/ownership.toml`). A crate may depend only on crates **above** it in the table; lane crates
never depend on each other except through the contract, with the four exceptions marked *.

| Crate | Owner | What it is | Depends on |
|---|---|---|---|
| `w5k_math` | ARCH | `f64` + `libm` maths: `Vec3`, `Quat`, `Mat3`, `Transform`, scalar helpers, `Pcg32`, `StateHasher` | `libm`, `serde` |
| `w5k_contract` | ARCH | data, traits, ports and test doubles (`testing`): see `CONTRACTS.md` | `w5k_math` |
| `w5k_chassis` | CHASSIS | hull integrator, suspension, tyres, steering, aero | contract, math |
| `w5k_drive` | DRIVE | engine, clutch or converter, gearbox, diffs, tracked steering units, brakes, fuel | contract, math |
| `w5k_terramech` | TRACKS | soil laws (Bekker, Janosi-Hanamoto), track contact, skid-steer | contract, math |
| `w5k_vehicle` | ARCH | *glue*: assembles a `PhysRig` into a stepping `VehicleModel`; force ledger; articulation coupling | contract, math, chassis*, drive*, terramech* |
| `w5k_world` | WORLD | heightfield, materials and soil layers, props, procedural courses; implements `WorldQuery` | contract, math |
| `w5k_geo` | GEOMETRY | geometry generators: hull lofts, turrets, wheels, track links, node tagging | contract, math |
| `w5k_forge` | FORGE | `VehicleDef` to `PhysRig` + `RenderRig` compile, mass and inertia kernel, reference garage | contract, math, geo* |
| `w5k_replay` | VIEWER | replay v2 writer and reader | contract, math |
| `w5k_validate` | VALIDATION | dossiers, harness, dashboard, impact matrix, fuzz | contract, math, sim* |
| `w5k_combat` | COMBAT | ballistics, penetration, damage, targets, turret and gun servos | contract, math |
| `w5k_ai` | AI | perception, capability table, planner, driver, gunner, tactician | contract, math |
| `w5k_sim` | ARCH | scheduler, scenarios, the integration spine | contract, math, replay, vehicle |
| `w5k_godot`, `game/` | GODOT | Godot 4.6 front end over a thin bridge; no logic | contract, replay, sim |
| `w5k_tools` | ARCH (dispatcher) + each lane's `cmd/<lane>.rs` | the `w5k` command line | everything |

## Rules
1. **Only ARCH's `w5k_vehicle` wires the physics lanes together.** CHASSIS, DRIVE and TRACKS each implement ports from the contract and are tested against stand-ins; they never import each other.
2. **Data flows through the contract.** Anything two lanes both need to know (a field, a unit, a sign) is in `w5k_contract` or does not exist.
3. **No cycles; no upward dependencies.** If you need something from a crate below you, you need a port or an interface request.
4. **Dependencies are declared once**, in the root `[workspace.dependencies]` (ARCH). Crates say `name.workspace = true`; CI rejects anything else.
5. **Presentation is not logic.** The Godot project and the three.js viewer read replays (and, in drive mode, send `Command`s); they never compute physics.
6. **Spikes** live in `spikes/<lane>/`, are never imported, and are archived once their finding note is written.

## Picture
```
                           w5k_math
                              |
                         w5k_contract  (+ testing stand-ins)
        .---------.-----------+-----------.----------.--------------.
   w5k_chassis w5k_drive w5k_terramech  w5k_world  w5k_geo -> w5k_forge   w5k_combat  w5k_ai   w5k_replay
        '---------'-----------'                                              (read the contract only)
                 w5k_vehicle (ARCH glue)                                         |
                       |                                                         |
                    w5k_sim  (scheduler, integration spine) -----------------------'
                       |
        w5k_validate   w5k_tools (CLI)   w5k_godot -> game/ (Godot)
```
