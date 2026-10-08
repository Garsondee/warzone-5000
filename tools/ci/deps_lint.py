"""Dependency lint: crates may only use dependencies that ARCH has put in the workspace.

    python3 -I tools/ci/deps_lint.py [--root DIR]

Every entry in [dependencies], [dev-dependencies] and [build-dependencies] (also under [target.'cfg(..)'.*]) of every
crates/*/Cargo.toml must be inherited from the root manifest:

    serde.workspace = true            # or: serde = { workspace = true, features = ["derive"] }

and the name must exist in the root [workspace.dependencies]. A crate cannot name its own version, path, git URL or
renamed package, because that is how an unapproved dependency would slip in. Only `features`, `optional` and
`default-features` may sit next to `workspace = true`.

Exit status: 0 = clean, 1 = violations, 2 = the setup is broken (no root manifest, unreadable TOML).
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, annotate, load_toml_file, printable  # noqa: E402

MESSAGE = "new dependencies need an approved decision card; ask ARCH"
TITLE = "Dependency lint"
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies", "dev_dependencies", "build_dependencies")
ALLOWED_NEXT_TO_WORKSPACE = {"workspace", "features", "optional", "default-features", "default_features"}


@dataclass
class Violation:
    manifest: str  # repo-relative path of the Cargo.toml
    table: str  # e.g. "dependencies" or "target.'cfg(windows)'.dependencies"
    name: str
    problem: str
    line: int | None = None


def workspace_dependency_names(root: Path) -> set[str]:
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        raise CiError(f"{manifest} does not exist; run from the repository root or pass --root")
    deps = load_toml_file(manifest).get("workspace", {}).get("dependencies", {})
    return set(deps)


def dependency_tables(manifest: dict):
    """Yield (table name, {dependency: spec}) for every dependency table in a Cargo.toml."""
    for key in DEPENDENCY_TABLES:
        if isinstance(manifest.get(key), dict):
            yield key, manifest[key]
    for cfg, table in manifest.get("target", {}).items():
        if isinstance(table, dict):
            for key in DEPENDENCY_TABLES:
                if isinstance(table.get(key), dict):
                    yield f"target.{cfg}.{key}", table[key]


def find_line(text: str, table: str, name: str) -> int | None:
    """Best-effort 1-based line of a dependency in the manifest text (for the annotation); None if not found."""
    last_part = table.rsplit(".", 1)[-1]
    in_table = False
    for number, line in enumerate(text.splitlines(), 1):
        header = re.match(r"\s*\[([^\]]+)\]", line)
        if header:
            title = header.group(1).replace(" ", "")
            if title.rstrip("'\"").endswith(f"{last_part}.{name}") or title.endswith(f".{name}"):
                return number  # [dependencies.name] sub-table
            in_table = title.endswith(last_part)
            continue
        if in_table and re.match(rf"\s*[\"']?{re.escape(name)}[\"']?\s*[.=]", line):
            return number
    return None


def check_manifest(rel: str, text: str, manifest: dict, allowed: set[str]) -> list[Violation]:
    found: list[Violation] = []
    for table, deps in dependency_tables(manifest):
        for name, spec in deps.items():
            where = dict(manifest=rel, table=table, name=name, line=find_line(text, table, name))
            if not (isinstance(spec, dict) and spec.get("workspace") is True):
                found.append(Violation(problem=f"names its own version/path/git; write `{name}.workspace = true`", **where))
                continue
            extra = sorted(set(spec) - ALLOWED_NEXT_TO_WORKSPACE)
            if extra:
                found.append(Violation(problem=f"sets {', '.join(extra)} next to `workspace = true`", **where))
            if name not in allowed:
                found.append(Violation(problem="is not listed in the root [workspace.dependencies]", **where))
    return found


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=".", help="the repository root (default: the current directory)")
    args = ap.parse_args(argv)
    root = Path(args.root)
    try:
        allowed = workspace_dependency_names(root)
        violations: list[Violation] = []
        manifests = sorted(root.glob("crates/*/Cargo.toml"))
        for path in manifests:
            rel = path.relative_to(root).as_posix()
            text = path.read_text(encoding="utf-8", errors="replace")
            violations += check_manifest(rel, text, load_toml_file(path, rel), allowed)
    except CiError as exc:
        print(f"deps lint: setup problem: {exc}", file=sys.stderr)
        return 2

    for v in violations:
        where = f"{v.manifest}:{v.line}" if v.line else v.manifest
        print(f"{printable(where)}: [{v.table}] `{printable(v.name)}` {v.problem}")
        annotate("error", f"[{v.table}] `{v.name}` {v.problem}. {MESSAGE}", file=v.manifest, line=v.line, title=TITLE)
    if violations:
        print()
        print(f"deps lint: FAIL, {len(violations)} problem(s) in {len({v.manifest for v in violations})} manifest(s).")
        print(MESSAGE)
        return 1
    print(f"deps lint: OK ({len(manifests)} manifest(s), {len(allowed)} workspace dependencies)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
