# Tools

Small helper scripts for this project. Python 3 standard library only; no installs needed.

## `tools/research/` — Warzone 2100 data analysis

These scripts read the **stats data of a local checkout of Warzone 2100** and turn it into tables, statistics and figures for the
research documents. They only *parse JSON*; they never execute anything from the checkout.

| Script | What it does |
|---|---|
| `wz_components.py` | Tabulates bodies, propulsion, weapons, turrets, damage modifiers, terrain speeds, unlock timeline, body dominance, key structures |
| `wz_research_graph.py` | Builds the research prerequisite graph; reports shape, composition, costs, upgrade chains, family coupling, which lines gate which bodies |
| `wz_speed_model.py` | The engine / weight / speed model: speed tables, cap saturation, engine-research effect, nominal design-space size |
| `wz_combat.py` | Nominal DPS, per-hit damage against representative targets, weapons pinned to the damage floor, weapon-vs-armour upgrade race, chassis efficiency frontier |
| `generate_tables.py` | Runs the above and writes every table into `docs/research/warzone-2100/data/` with a provenance header |
| `make_figures.py` | Draws the three SVG figures in `docs/assets/` from the data (validated palette, light surface) |

## `tools/explorer/`: possibility-space explorer

Builds the interactive explorer page and prints the summary statistics from a sampled `space_sample.csv`
(see [tools/explorer/README.md](explorer/README.md)). Python standard library for the build; Node and Playwright for the optional helpers.

### Getting the Warzone 2100 data (read-only, outside this repo)

```bash
git clone --depth 1 https://github.com/Warzone2100/warzone2100 /some/path/warzone2100
```

Keep the checkout **outside this repository**. Warzone 2100 is GPL-2.0-or-later; we study it but do not copy its code or assets
into this project. (On Windows, any folder outside this repo works, for example next to it.)

### Regenerating everything

```bash
python3 -I tools/research/generate_tables.py /some/path/warzone2100 docs/research/warzone-2100/data
python3 -I tools/research/make_figures.py   /some/path/warzone2100 docs/assets
```

The generated files record the Warzone 2100 commit they were built from. The current set was built from `d7ce18df8d`
(master, 2026-10-07). On Windows use `python` or `py` instead of `python3`.

### Running one report

```bash
python3 -I tools/research/wz_components.py    /some/path/warzone2100 bodies   # also: propulsion weapons turrets modifiers terrain timeline dominance structures
python3 -I tools/research/wz_research_graph.py /some/path/warzone2100 mp       # or: base (campaign); add --list for the full item list
python3 -I tools/research/wz_speed_model.py   /some/path/warzone2100 saturation   # also: table weapons engine designspace
python3 -I tools/research/wz_combat.py        /some/path/warzone2100 race         # also: dps vs floor chassis [weapon_id] [--abs-cost]
```

`-I` (isolated mode) is deliberate: it stops Python from importing anything from the current directory, which matters when the
data you are analysing is untrusted.

### Model notes
The speed and combat scripts encode formulas **confirmed by reading the engine source** (documented in
`docs/research/warzone-2100/unit-design/04-engine-and-speed.md` and `06-damage-and-counters.md`). If Warzone 2100 changes those
formulas in a future release, regenerate and re-verify before trusting the tables.
