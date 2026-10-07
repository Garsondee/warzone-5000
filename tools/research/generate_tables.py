#!/usr/bin/env python3
"""Regenerate the derived Warzone 2100 data tables under docs/research/warzone-2100/data/.

Usage (from anywhere):
    python3 -I tools/research/generate_tables.py <wz_checkout> <output_dir>

Example:
    python3 -I tools/research/generate_tables.py /path/to/warzone2100 docs/research/warzone-2100/data

Needs: python3 and git. Only parses JSON from the checkout; never executes anything in it.
"""
import datetime
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

HEADER = """<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`{ruleset}` stats), snapshot commit `{commit}`
> ({date}). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generator: `{cmd}`.

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

    jobs = [
        ("bodies.md", "# Bodies (designable)\n\n", [comp, wz, "bodies"], "wz_components.py bodies"),
        ("propulsion.md", "# Propulsion (designable)\n\n", [comp, wz, "propulsion"], "wz_components.py propulsion"),
        ("weapons.md", "# Weapons (designable)\n\n", [comp, wz, "weapons"], "wz_components.py weapons"),
        ("turrets.md", "# Non-weapon turrets (designable)\n\n", [comp, wz, "turrets"], "wz_components.py turrets"),
        ("modifiers.md", "# Damage modifier matrices\n\n", [comp, wz, "modifiers"], "wz_components.py modifiers"),
        ("terrain.md", "# Terrain speed factors\n\n", [comp, wz, "terrain"], "wz_components.py terrain"),
        ("timeline.md", "# Component unlock timeline (by research critical path)\n\n", [comp, wz, "timeline"], "wz_components.py timeline"),
        ("dominance.md", "# Body dominance analysis\n\n", [comp, wz, "dominance"], "wz_components.py dominance"),
        ("structures.md", "# Key structures\n\n", [comp, wz, "structures"], "wz_components.py structures"),
        ("research-tree-analysis.md", "", [graph, wz, "mp"], "wz_research_graph.py mp"),
        ("research-tree-full-list.md", "# Research tree: full list in depth order\n\n", [graph, wz, "mp", "--list"], "wz_research_graph.py mp --list"),
    ]
    for fname, title, args, cmd in jobs:
        body = run(args)
        if fname == "research-tree-full-list.md":
            # keep only the list part of the report
            body = body[body.index("## Full list in depth order"):].split("\n", 2)[2]
        with open(os.path.join(outdir, fname), "w", encoding="utf-8", newline="\n") as f:
            cmd = "{} <wz_checkout> {}".format(os.path.basename(args[0]), " ".join(args[2:]))
            header = HEADER.format(ruleset="mp", commit=commit, date=date, cmd="python3 -I tools/research/" + cmd)
            if not title:  # the report brings its own title: put the provenance block under it
                first, body = body.split("\n", 1)
                title = first + "\n\n"
                body = body.lstrip("\n")
            f.write(title)
            f.write(header)
            f.write(body.rstrip() + "\n")
        print("wrote", fname)


if __name__ == "__main__":
    main()
