# Tools

Small helper scripts for this project. Python 3 standard library only; no installs needed.

## `tools/research/` — Warzone 2100 data analysis

These scripts read the **stats data of a local checkout of Warzone 2100** and turn it into tables and statistics
for the research documents. They only *parse JSON*; they never execute anything from the checkout.

| Script | What it does |
|---|---|
| `wz_components.py` | Tabulates bodies, propulsion, weapons, turrets, damage modifiers, terrain speeds, unlock timeline, body dominance, key structures |
| `wz_research_graph.py` | Builds the research prerequisite graph and reports its shape, composition, costs, upgrade chains, family coupling |
| `generate_tables.py` | Runs the above and writes every table into `docs/research/warzone-2100/data/` with a provenance header |

### Getting the Warzone 2100 data (read-only, outside this repo)

```bash
git clone --depth 1 https://github.com/Warzone2100/warzone2100 /some/path/warzone2100
```

Keep the checkout **outside this repository**. Warzone 2100 is GPL-2.0-or-later; we study it but do not copy its code
or assets into this project.

### Regenerating the tables

```bash
python3 -I tools/research/generate_tables.py /some/path/warzone2100 docs/research/warzone-2100/data
```

The generated files record the Warzone 2100 commit they were built from. The current set was built from
`d7ce18df8d` (master, 2026-10-07).

### Running one report

```bash
python3 -I tools/research/wz_components.py /some/path/warzone2100 bodies        # also: propulsion weapons turrets modifiers terrain timeline dominance structures
python3 -I tools/research/wz_research_graph.py /some/path/warzone2100 mp        # or: base (campaign); add --list for the full item list
```

`-I` (isolated mode) is deliberate: it stops Python from importing anything from the current directory, which
matters when the data you are analysing is untrusted.
