# Layers and crates

Each crate has exactly **one owning lane** (`docs/swarm/ownership.toml`). A crate may depend only on crates **above** it in the table. A lane crate
depends on `w5k_contract` and `w5k_math` and on nothing else: lanes meet only through the contract. The exceptions, marked *, are FORGE's use of
GEOMETRY's generators and the **composition roots** (`w5k_vehicle`, `w5k_sim`, `w5k_validate`, `w5k_tools`, `w5k_godot`), whose job is to wire lanes together.

| Crate | Owner | What it is | Depends on |
|---|---|---|---|
| `w5k_math` | ARCH | `f64` + `libm` maths: `Vec3`, `Quat`, `Mat3`, `Transform`, scalar helpers, `Pcg32`, `StateHasher` | `libm`, `serde` |
| `w5k_contract` | ARCH | data, traits, ports and test doubles (`testing`): see `CONTRACTS.md` | `w5k_math` |
| `w5k_chassis` | CHASSIS | hull integrator, suspension, tyres, steering, aero | contract, math |
| `w5k_drive` | DRIVE | engine, clutch or converter, gearbox, diffs, tracked steering units, brakes, fuel | contract, math |
| `w5k_terramech` | TRACKS | soil laws (Bekker, Janosi-Hanamoto), track contact, skid-steer | contract, math |
| `w5k_vehicle` | ARCH | *glue*: assembles a `PhysRig` into a stepping `VehicleModel`; force ledger; calls the articulation port | contract, math, chassis*, drive*, terramech*, combat* (the servo and recoil module only) |
| `w5k_world` | WORLD | heightfield, materials and soil layers, props, procedural courses; implements `WorldQuery` | contract, math |
| `w5k_geo` | GEOMETRY | geometry generators: hull lofts, turrets, wheels, track links, node tagging | contract, math |
| `w5k_forge` | FORGE | `VehicleDef` to `PhysRig` + `RenderRig` compile, mass and inertia kernel, reference garage | contract, math, geo* |
| `w5k_replay` | VIEWER | replay v2 writer and reader | contract, math |
| `w5k_validate` | VALIDATION | dossiers, harness, dashboard, impact matrix, fuzz | contract, math, sim*, forge* |
| `w5k_combat` | COMBAT | ballistics, penetration, damage, targets, turret and gun servos and recoil (implements the articulation port) | contract, math |
| `w5k_ai` | AI | perception, capability table, planner, driver, gunner, tactician | contract, math |
| `w5k_sim` | ARCH | scheduler, scenarios, the integration spine; steps projectiles and feeds the AI | contract, math, replay, vehicle*, world*, forge*, combat*, ai* |
| `w5k_godot`, `game/` | GODOT | Godot 4.6 front end over a thin bridge; no logic | contract, replay, sim |
| `w5k_tools` | ARCH (dispatcher) + each lane's `cmd/<lane>.rs` | the `w5k` command line | everything |

## Rules
1. **Only the composition roots wire lanes together** (ARCH's `w5k_vehicle` for the physics lanes, `w5k_sim` for the whole simulation). CHASSIS, DRIVE, TRACKS and COMBAT's servos each implement ports from the contract and are tested against stand-ins; they never import each other. `w5k_ai` never sees `w5k_sim`, `w5k_vehicle` or any ground truth: it receives an `Observation` and returns a `Command`.
2. **Data flows through the contract.** Anything two lanes both need to know (a field, a unit, a sign) is in `w5k_contract` or does not exist.
3. **No cycles; no upward dependencies.** If you need something from a crate below you, you need a port or an interface request.
4. **Dependencies are declared once**, in the root `[workspace.dependencies]` (ARCH). Crates say `name.workspace = true`; CI rejects anything else.
5. **Presentation is not logic.** The Godot project and the three.js viewer read replays (and, in drive mode, send `Command`s); they never compute physics.
6. **Spikes** live in `spikes/<lane>/`, are never imported, and are archived once their finding note is written.

## Picture
```
 layer 0   w5k_math
              |
 layer 1   w5k_contract              data, traits (ports), test stand-ins
              |
 layer 2   LANE CRATES: each knows only layers 0 and 1 (forge also uses geo)
           chassis  drive  terramech  combat  world  geo  forge  ai  replay
              |       |        |        |       |     |     |    |     |
 layer 3   COMPOSITION ROOTS: wire lanes together
           w5k_vehicle  <- chassis, drive, terramech, combat (servos)
              |
           w5k_sim      <- vehicle, world, forge, combat, ai, replay   (the scheduler and the first-light spine)
              |
 layer 4   w5k_validate (sim, forge)    w5k_tools (everything)    w5k_godot (replay, sim) -> game/
```
