# Ownership

The machine-readable truth is [`ownership.toml`](ownership.toml) (the CI lane guard reads it). This page is the plain version.

**One writer per path.** A lane edits only what is listed for it, plus its own status file, interface requests, theory note, media directory and spikes. Everything else is read-only to it.

| Lane | Crates and directories it owns |
|---|---|
| ARCH | `crates/w5k_math`, `w5k_contract`, `w5k_vehicle`, `w5k_sim`; `w5k_tools` dispatcher; root manifests; `.github`, `.claude`, `tools/ci`; `docs/{brief,architecture,decisions,swarm,research,archive}`; `reference/`; `content/fixtures` |
| CHASSIS | `crates/w5k_chassis`, `content/physics/chassis` |
| DRIVE | `crates/w5k_drive`, `content/physics/drive` |
| TRACKS | `crates/w5k_terramech`, `content/physics/tracks` |
| WORLD | `crates/w5k_world`, `content/world` |
| FORGE | `crates/w5k_forge`, `content/parts`, `content/vehicles` |
| GEOMETRY | `crates/w5k_geo` |
| LOOK | `assets/**`, `docs/art` |
| VIEWER | `crates/w5k_replay`, `tools/viewer` |
| GODOT | `crates/w5k_godot`, `game/` |
| VALIDATION | `crates/w5k_validate`, `content/dossier`, `docs/validation` |
| COMBAT | `crates/w5k_combat`, `content/combat` |
| AI | `crates/w5k_ai` |

Each lane also owns `crates/w5k_tools/src/cmd/<lane>.rs` (its slice of the `w5k` command line).

**Shared by rule, not by file:** `Cargo.lock` (regenerated, never hand-merged). **Nobody but ARCH** edits `Cargo.toml` at the root, the contract, CI, ownership, or another lane's files.

**Need something from another lane?** File an interface request (`RULES.md` section 5). **Think a contract is wrong?** File a CCR (`CONTRACTS.md`).

**Line budgets** per crate live in `budgets.toml`; exceeding one is a card, not a quiet addition.
