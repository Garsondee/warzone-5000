#!/usr/bin/env python3
"""Regenerate the derived Warzone 2100 data tables under docs/research/warzone-2100/data/.

Usage (from anywhere):
    python3 -I tools/research/generate_tables.py <wz_checkout> <output_dir>

Example:
    python3 -I tools/research/generate_tables.py /path/to/warzone2100 docs/research/warzone-2100/data

Needs: python3 and git. Only parses JSON from the checkout; never executes anything in it.
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

HEADER = """<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `{commit}`
> ({date}). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generated with: {cmds}
"""


def run(args):
    return subprocess.run([sys.executable, "-I"] + args, check=True, capture_output=True, text=True).stdout


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    wz, outdir = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
    os.makedirs(outdir, exist_ok=True)
    commit = subprocess.run(["git", "-C", wz, "rev-parse", "--short=10", "HEAD"], capture_output=True, text=True).stdout.strip() or "unknown"
    date = subprocess.run(["git", "-C", wz, "log", "-1", "--format=%cs"], capture_output=True, text=True).stdout.strip() or "unknown"
    comp = os.path.join(HERE, "wz_components.py")
    graph = os.path.join(HERE, "wz_research_graph.py")
    speed = os.path.join(HERE, "wz_speed_model.py")
    combat = os.path.join(HERE, "wz_combat.py")

    # (file, title, [(script, extra args, optional sub-heading)])
    jobs = [
        ("bodies.md", "# Bodies (designable)", [(comp, ["bodies"], None)]),
        ("propulsion.md", "# Propulsion (designable)", [(comp, ["propulsion"], None)]),
        ("weapons.md", "# Weapons (designable)", [(comp, ["weapons"], None)]),
        ("turrets.md", "# Non-weapon turrets (designable)", [(comp, ["turrets"], None)]),
        ("modifiers.md", "# Damage modifier matrices", [(comp, ["modifiers"], None)]),
        ("terrain.md", "# Terrain speed factors", [(comp, ["terrain"], None)]),
        ("timeline.md", "# Component unlock timeline (by research critical path)", [(comp, ["timeline"], None)]),
        ("dominance.md", "# Body dominance analysis", [(comp, ["dominance"], None)]),
        ("structures.md", "# Key structures", [(comp, ["structures"], None)]),
        ("research-tree-analysis.md", None, [(graph, ["mp"], None)]),
        ("research-tree-full-list.md", "# Research tree: full list in depth order", [(graph, ["mp", "--list"], None)]),
        ("speed-table.md", "# Flat-ground speed by body, propulsion and turret weight", [(speed, ["table"], None)]),
        ("speed-saturation.md", "# How often the speed cap hides the engine", [(speed, ["saturation"], "## Cap saturation"), (speed, ["engine"], "## What the engine research line changes")]),
        ("speed-by-weapon.md", "# Speed penalty of each ground weapon (Cobra on tracks)", [(speed, ["weapons"], None)]),
        ("design-space.md", "# Size of the nominal design space", [(speed, ["designspace"], None)]),
        ("weapons-dps.md", "# Nominal weapon DPS (ground weapons)", [(combat, ["dps"], None)]),
        ("damage-vs-targets.md", "# Per-hit damage against representative targets", [(combat, ["vs"], None), (combat, ["floor"], "## Weapons pinned to the minimum-damage floor")]),
        ("upgrade-race.md", "# Weapon-damage research versus armour research", [(combat, ["race"], None)]),
        ("chassis-frontier-medium-cannon.md", "# Chassis efficiency frontier (Medium Cannon)", [(combat, ["chassis", "Cannon2A-TMk1"], None), (combat, ["chassis", "Cannon2A-TMk1", "--abs-cost"], "## Same analysis with absolute cost as a fourth dimension (summary lines)")]),
        ("chassis-frontier-machinegun.md", "# Chassis efficiency frontier (Machinegun)", [(combat, ["chassis", "MG1Mk1"], None), (combat, ["chassis", "MG1Mk1", "--abs-cost"], "## Same analysis with absolute cost as a fourth dimension (summary lines)")]),
    ]
    for fname, title, parts in jobs:
        chunks, cmds = [], []
        for script, extra, sub in parts:
            body = run([script, wz] + extra)
            if fname == "research-tree-full-list.md":
                body = body[body.index("## Full list in depth order"):].split("\n", 2)[2]
            if sub and "--abs-cost" in extra:
                # keep only the header line and the summary line of the extra variant
                lines = [ln for ln in body.splitlines() if ln.strip()]
                body = "\n".join([lines[0], lines[-1]]) + "\n"
            chunks.append((sub, body.rstrip() + "\n"))
            cmds.append("`{} <wz_checkout> {}`".format(os.path.basename(script), " ".join(extra)))
        if title is None:  # the report brings its own title
            first, rest = chunks[0][1].split("\n", 1)
            title, chunks[0] = first, (None, rest.lstrip("\n"))
        with open(os.path.join(outdir, fname), "w", encoding="utf-8", newline="\n") as f:
            f.write(title + "\n\n")
            f.write(HEADER.format(commit=commit, date=date, cmds="; ".join(cmds)) + "\n")
            for sub, body in chunks:
                if sub:
                    f.write(sub + "\n\n")
                f.write(body + "\n")
        print("wrote", fname)


if __name__ == "__main__":
    main()
