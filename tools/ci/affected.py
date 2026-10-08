"""Affected crates: which crates should `cargo test` run for this pull request?

    python3 -I tools/ci/affected.py --base origin/integration --head HEAD

Prints ONE line on stdout, ready to paste after `cargo test`:
    -p w5k_chassis -p w5k_vehicle -p w5k_sim     the changed crates plus every crate that depends on them
    --workspace                                  something that can affect everything changed (see below), or we cannot tell
    (empty line)                                 nothing that cargo tests cares about changed

How it decides (changed files = BASE...HEAD, a rename counts both sides):
  * crates/<name>/...  -> that crate, then every crate that depends on it, directly or through others (normal, dev and
    build dependencies), from `cargo metadata --no-deps --format-version 1`;
  * Cargo.toml, Cargo.lock, clippy.toml, rustfmt.toml, rust-toolchain(.toml), deny.toml, anything under .cargo/,
    .github/ or tools/ci/  -> --workspace (they change how every crate builds or is checked);
  * anything under content/  -> --workspace (the tests read the RON content, and no crate owns it);
  * a crates/<unknown>/ folder, or any problem running git or cargo  -> --workspace (when in doubt, test everything);
  * anything else (docs, assets, game, tools/viewer, spikes, README...) does not affect cargo tests.
The first-light scenario test is run by CI separately, whatever this prints.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import PurePosixPath, PureWindowsPath

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, diff_changes  # noqa: E402

WORKSPACE = "--workspace"
ROOT_FILES = frozenset(
    ["Cargo.toml", "Cargo.lock", "clippy.toml", ".clippy.toml", "rustfmt.toml", ".rustfmt.toml", "rust-toolchain",
     "rust-toolchain.toml", "deny.toml"]
)
WORKSPACE_PREFIXES = (".cargo/", ".github/", "tools/ci/", "content/")
SAFE_NAME = re.compile(r"[A-Za-z0-9_-]+")


class Undecidable(Exception):
    """We cannot tell what is affected; the safe answer is --workspace."""


def package_dirs(metadata: dict) -> dict[str, str]:
    """{repo-relative directory: package name} for every workspace package in `cargo metadata` output."""
    root = metadata.get("workspace_root")
    dirs: dict[str, str] = {}
    for pkg in metadata.get("packages", []):
        name = pkg["name"]
        manifest = pkg.get("manifest_path")
        if manifest and root:
            parent = PurePosixPath(PureWindowsPath(manifest).as_posix()).parent
            try:
                dirs[parent.relative_to(PureWindowsPath(root).as_posix()).as_posix()] = name
                continue
            except ValueError:
                pass
        dirs[f"crates/{name}"] = name  # no usable manifest path: the workspace convention is crates/<name>
    return dirs


def reverse_closure(metadata: dict, start: set[str]) -> set[str]:
    """`start` plus every workspace package that depends on one of them, however indirectly."""
    names = {pkg["name"] for pkg in metadata.get("packages", [])}
    dependents: dict[str, set[str]] = {n: set() for n in names}
    for pkg in metadata.get("packages", []):
        for dep in pkg.get("dependencies", []):
            if dep["name"] in names:  # a workspace (path) dependency; external crates cannot be affected by us
                dependents[dep["name"]].add(pkg["name"])
    result = set(start)
    queue = list(start)
    while queue:
        for parent in dependents.get(queue.pop(), ()):
            if parent not in result:
                result.add(parent)
                queue.append(parent)
    return result


def affected_args(paths: list[str], metadata: dict) -> str:
    """The `cargo test` selection for these changed paths: '-p a -p b', '--workspace' or '' (nothing)."""
    dirs = package_dirs(metadata)
    crates: set[str] = set()
    for path in paths:
        parts = path.split("/")
        if path in ROOT_FILES or path.startswith(WORKSPACE_PREFIXES):
            return WORKSPACE
        if parts[0] == "crates" and len(parts) >= 3:
            name = dirs.get(f"crates/{parts[1]}")
            if name is None:
                return WORKSPACE  # a crate folder cargo does not know (new, deleted or misnamed)
            crates.add(name)
    if not crates:
        return ""
    chosen = sorted(reverse_closure(metadata, crates))
    if not all(SAFE_NAME.fullmatch(n) for n in chosen):
        return WORKSPACE
    return " ".join(f"-p {n}" for n in chosen)


def cargo_metadata(repo: str) -> dict:
    proc = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=repo, capture_output=True, text=True
    )
    if proc.returncode != 0:
        raise Undecidable(f"cargo metadata failed: {proc.stderr.strip()}")
    return json.loads(proc.stdout)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="the revision the pull request is going into, e.g. origin/integration")
    ap.add_argument("--head", required=True, help="the pull request's last commit, e.g. HEAD")
    ap.add_argument("--repo", default=".", help="the git repository (default: the current directory)")
    ap.add_argument("--metadata-file", help="read `cargo metadata --no-deps --format-version 1` output from this file "
                                            "('-' for stdin) instead of running cargo (tests)")
    args = ap.parse_args(argv)

    try:
        try:
            paths = [p for change in diff_changes(args.repo, args.base, args.head) for p in change.paths]
        except CiError as exc:
            raise Undecidable(str(exc)) from exc
        if args.metadata_file:
            try:
                raw = sys.stdin.read() if args.metadata_file == "-" else open(args.metadata_file, encoding="utf-8").read()
                metadata = json.loads(raw)
            except (OSError, ValueError) as exc:
                raise Undecidable(f"cannot read the cargo metadata: {exc}") from exc
        else:
            try:
                metadata = cargo_metadata(args.repo)
            except (OSError, ValueError) as exc:
                raise Undecidable(f"cannot run cargo metadata: {exc}") from exc
        result = affected_args(paths, metadata)
    except Undecidable as exc:
        print(f"affected: cannot tell what is affected ({exc}); testing the whole workspace", file=sys.stderr)
        result = WORKSPACE
        paths = []

    print(f"affected: {len(paths)} changed path(s) -> {result or 'no crates to test'}", file=sys.stderr)
    print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main())
