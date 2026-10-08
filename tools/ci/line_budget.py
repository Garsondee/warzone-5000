"""Line budget: every crate has a cap on its lines of code.

    python3 -I tools/ci/line_budget.py [--root DIR] [--budgets FILE]

Counts the non-blank, non-comment lines of crates/<crate>/src/**/*.rs (a line holding code plus a trailing comment
counts; tests inside src count too) and compares each crate with its budget from docs/swarm/budgets.toml:

    [default]
    max_lines = 8000
    [crates.w5k_math]
    max_lines = 2500

Prints a table of usage. A crate at 90% of its budget or more is marked WARN; over budget fails the check.

Exit status: 0 = every crate is within budget, 1 = a crate is over budget, 2 = the setup is broken.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, annotate, load_toml_file  # noqa: E402
from rust_lex import count_code_lines  # noqa: E402

DEFAULT_BUDGETS = "docs/swarm/budgets.toml"
WARN_FRACTION = 0.9
TITLE = "Line budget"


def read_budgets(path: Path) -> tuple[int, dict[str, int]]:
    """Return (default max_lines, {crate: max_lines})."""
    data = load_toml_file(path, f"the budgets file {path}")

    def limit(value, where: str) -> int:
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise CiError(f"{path}: {where} must be a positive whole number, got {value!r}")
        return value

    default = limit(data.get("default", {}).get("max_lines"), "[default].max_lines")
    overrides = {}
    for name, table in data.get("crates", {}).items():
        if not isinstance(table, dict):
            raise CiError(f"{path}: [crates.{name}] must be a table with max_lines")
        overrides[name] = limit(table.get("max_lines"), f"[crates.{name}].max_lines")
    return default, overrides


def count_crate(crate_dir: Path) -> int:
    total = 0
    for rs in sorted((crate_dir / "src").glob("**/*.rs")):
        total += count_code_lines(rs.read_text(encoding="utf-8", errors="replace"))
    return total


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=".", help="the repository root (default: the current directory)")
    ap.add_argument("--budgets", help=f"the budgets file (default: {DEFAULT_BUDGETS} under --root)")
    args = ap.parse_args(argv)
    root = Path(args.root)
    budgets_path = Path(args.budgets) if args.budgets else root / DEFAULT_BUDGETS

    try:
        default, overrides = read_budgets(budgets_path)
        crates_dir = root / "crates"
        if not crates_dir.is_dir():
            raise CiError(f"{crates_dir} does not exist; run from the repository root or pass --root")
        crates = sorted(p for p in crates_dir.iterdir() if (p / "Cargo.toml").is_file())
        rows = [(p.name, count_crate(p), overrides.get(p.name, default)) for p in crates]
    except CiError as exc:
        print(f"line budget: setup problem: {exc}", file=sys.stderr)
        return 2

    for name in sorted(set(overrides) - {n for n, _, _ in rows}):
        msg = f"{budgets_path} gives a budget to crate {name}, which does not exist (typo, or a crate that was removed)"
        print(f"warning: {msg}", file=sys.stderr)
        annotate("warning", msg, file=str(budgets_path), title=TITLE)

    width = max([len(n) for n, _, _ in rows] + [len("crate")])
    print(f"{'crate'.ljust(width)}  {'lines':>6}  {'budget':>6}  {'used':>5}  status")
    over = []
    for name, lines, budget in rows:
        status = "ok"
        if lines > budget:
            status = "OVER"
            over.append((name, lines, budget))
        elif lines >= WARN_FRACTION * budget:
            status = "WARN"
        print(f"{name.ljust(width)}  {lines:>6}  {budget:>6}  {100 * lines // budget:>4}%  {status}")
    print(f"{'total'.ljust(width)}  {sum(r[1] for r in rows):>6}")

    for name, lines, budget in over:
        msg = (f"crate {name} has {lines} lines of code, over its budget of {budget} ({budgets_path.as_posix()}). "
               "Split it into smaller crates, delete dead code, or ask ARCH for a decision card to raise the budget.")
        print(f"error: {msg}")
        annotate("error", msg, file=f"crates/{name}/Cargo.toml", title=TITLE)
    if over:
        print(f"line budget: FAIL, {len(over)} crate(s) over budget")
        return 1
    print("line budget: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
