"""Lane guard: a pull request may only change the files its lane owns.

    python3 -I tools/ci/lane_guard.py --base origin/integration --head HEAD --branch lane/chassis/bump-strip \
        --title "Chassis: quarter-car oracle"

The rules live in docs/swarm/ownership.toml (see the comments at the top of that file). In short:
  * a branch named `lane/<name>/<topic>` with a known lane may change the [lanes.<name>] paths and the [common] paths
    (with {lane} replaced by the lane name);
  * a pull-request title starting `CCR:` (contract change request) may also change the [ccr] paths and make migration
    edits anywhere, and ARCH must then review every line (a loud notice is printed);
  * a path matching the optional [protected] table (CI, settings, ownership, the rules themselves) may NOT be changed by
    any lane branch, not even with a `CCR:` title; a missing table means no protected paths;
  * a head matching one of the [unrestricted].heads patterns (fnmatch) may change anything, protected paths included
    (these are ARCH's own branches);
  * every other head is rejected.
Globs: `**` crosses directories, `*` and `?` do not cross `/`. A rename counts as touching both the old and the new path.

ownership.toml is read from the BASE revision, so a pull request cannot grant itself rights by editing that file. Use
--ownership PATH to read a file from disk instead (tests, local experiments).

Exit status: 0 = allowed, 1 = the pull request touches paths its lane does not own, 2 = the setup is broken.
"""

from __future__ import annotations

import argparse
import fnmatch
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import (  # noqa: E402
    Change,
    CiError,
    annotate,
    diff_changes,
    load_toml_bytes,
    load_toml_file,
    matches_any,
    printable,
    try_git,
)

OWNERSHIP_REL = "docs/swarm/ownership.toml"
LANE_BRANCH = re.compile(r"^lane/([^/]+)/(.+)$")
CCR_PREFIX = "CCR:"
TITLE = "Lane guard"
MAX_LISTED = 50  # a pull request that strays over thousands of files should not print thousands of lines


@dataclass
class Violation:
    path: str
    message: str
    protected: bool = False  # the path is in [protected]: not even a CCR: pull request may change it


@dataclass
class Verdict:
    mode: str  # "lane", "unrestricted" or "rejected"
    lane: str | None = None
    topic: str | None = None
    is_ccr: bool = False
    detail: str = ""  # the matching unrestricted pattern, or why the head was rejected
    violations: list[Violation] = field(default_factory=list)
    outside: list[str] = field(default_factory=list)  # CCR only: paths beyond the lane's own and the common ones
    contract_touched: bool = False

    @property
    def ok(self) -> bool:
        return self.mode != "rejected" and not self.violations

    @property
    def protected_hits(self) -> int:
        return sum(v.protected for v in self.violations)


# ---- the rules -----------------------------------------------------------------------------------------------------------


def _paths(table: dict, key: str, where: str) -> list[str]:
    value = table.get(key, [])
    if not isinstance(value, list) or not all(isinstance(p, str) for p in value):
        raise CiError(f"{OWNERSHIP_REL}: {where} must be a list of strings")
    return value


def check_ownership(data: dict) -> dict:
    """Make sure the ownership file has the shape the guard relies on."""
    lanes = data.get("lanes")
    if not isinstance(lanes, dict) or not lanes:
        raise CiError(f"{OWNERSHIP_REL}: there is no [lanes.<name>] table")
    for name, table in lanes.items():
        if not isinstance(table, dict):
            raise CiError(f"{OWNERSHIP_REL}: [lanes.{name}] must be a table")
        _paths(table, "paths", f"[lanes.{name}].paths")
    _paths(data.get("common", {}), "paths", "[common].paths")
    _paths(data.get("ccr", {}), "paths", "[ccr].paths")
    _paths(data.get("protected", {}), "paths", "[protected].paths")
    _paths(data.get("unrestricted", {}), "heads", "[unrestricted].heads")
    return data


def protected_paths(ownership: dict) -> list[str]:
    return ownership.get("protected", {}).get("paths", [])


def request_file(lane: str, topic: str) -> str:
    """The file name a lane should use for an interface request (it matches [common] docs/swarm/requests/{lane}-*.md)."""
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", topic).strip("-.") or "topic"
    return f"docs/swarm/requests/{lane}-{slug}.md"


def owners_of(path: str, ownership: dict, exclude: str | None = None) -> list[str]:
    """The lanes that own `path`: through their [common] files (their status file, theory note, ...) first, then
    through their own [lanes.<name>] globs."""
    common = ownership.get("common", {}).get("paths", [])
    by_common, by_paths = [], []
    for name, table in ownership["lanes"].items():
        if name == exclude:
            continue
        if matches_any([p.replace("{lane}", name) for p in common], path):
            by_common.append(name)
        elif matches_any(table.get("paths", []), path):
            by_paths.append(name)
    return by_common + by_paths


def explain(path: str, lane: str, topic: str, ownership: dict, is_ccr: bool) -> str:
    """One actionable sentence for a path the lane may not change."""
    request = request_file(lane, topic)
    if matches_any(protected_paths(ownership), path):
        return (
            f"{path} is a protected path (CI, settings, ownership, the rules): only ARCH's own branches may change it; "
            f"file an interface request in {request} instead"
        )
    others = owners_of(path, ownership, exclude=lane)
    contract = matches_any(ownership.get("ccr", {}).get("paths", []), path)
    if others:
        text = f"your PR touches {path} which belongs to lane {', '.join(others)}; file an interface request in {request} instead"
    else:
        text = (
            f"your PR touches {path} which no lane owns; file an interface request in {request} "
            f"asking ARCH to assign it in {OWNERSHIP_REL}"
        )
    if contract and not is_ccr:
        text += "; it is a contract path, so a real contract change must come in a PR whose title starts with `CCR:`"
    return text


def evaluate(ownership: dict, branch: str, title: str, changes: list[Change]) -> Verdict:
    """Decide whether a branch with this title may make these changes."""
    for pattern in ownership.get("unrestricted", {}).get("heads", []):
        if fnmatch.fnmatchcase(branch, pattern):
            return Verdict("unrestricted", detail=pattern)

    match = LANE_BRANCH.match(branch)
    lanes = ownership["lanes"]
    if not match:
        return Verdict(
            "rejected",
            detail=f"branch {branch!r} is not a lane branch (lane/<lane>/<topic>) and is not in [unrestricted].heads",
        )
    lane, topic = match.groups()
    if lane not in lanes:
        return Verdict(
            "rejected",
            lane=lane,
            detail=f"unknown lane {lane!r} in branch {branch!r}; the lanes are: {', '.join(sorted(lanes))}",
        )

    is_ccr = title.lstrip().startswith(CCR_PREFIX)
    ccr_paths = ownership.get("ccr", {}).get("paths", [])
    protected = protected_paths(ownership)
    allowed = [p.replace("{lane}", lane) for p in ownership.get("common", {}).get("paths", [])] + lanes[lane]["paths"]
    verdict = Verdict("lane", lane=lane, topic=topic, is_ccr=is_ccr)

    seen: set[str] = set()
    for change in changes:
        for path in change.paths:
            if path in seen:
                continue
            seen.add(path)
            if matches_any(ccr_paths, path):
                verdict.contract_touched = True
            if matches_any(protected, path):  # beats own paths, common paths and the CCR: exception alike
                verdict.violations.append(Violation(path, explain(path, lane, topic, ownership, is_ccr), protected=True))
                continue
            if matches_any(allowed, path):
                continue
            if is_ccr:
                verdict.outside.append(path)
            else:
                verdict.violations.append(Violation(path, explain(path, lane, topic, ownership, is_ccr)))
    return verdict


# ---- reading the inputs --------------------------------------------------------------------------------------------------


def load_ownership(repo: str, base: str, explicit: str | None) -> tuple[dict, str]:
    """Return (ownership data, where it came from)."""
    if explicit:
        return check_ownership(load_toml_file(explicit)), explicit
    from_base = try_git(repo, "show", f"{base}:{OWNERSHIP_REL}")
    if from_base is not None:
        return check_ownership(load_toml_bytes(from_base, f"{OWNERSHIP_REL} at {base}")), f"{OWNERSHIP_REL} at {base}"
    on_disk = Path(repo) / OWNERSHIP_REL
    if on_disk.is_file():
        return check_ownership(load_toml_file(on_disk)), f"{on_disk} (not in {base} yet)"
    raise CiError(f"{OWNERSHIP_REL} exists neither at {base} nor in {repo}; pass --ownership PATH")


# ---- the report ----------------------------------------------------------------------------------------------------------


def report(verdict: Verdict, ownership: dict, branch: str, title: str, changes: list[Change], source: str, base: str) -> None:
    shown_title = printable(title)
    print(f"lane guard: branch {printable(branch)!r}, title {shown_title!r}")
    print(f"  rules from : {source}")
    touched = sorted({p for c in changes for p in c.paths})
    print(f"  changed    : {len(touched)} path(s)")

    if verdict.mode == "unrestricted":
        print(f"  head       : matches unrestricted pattern {verdict.detail!r}; every path is allowed")
        print("PASS")
        return

    if verdict.mode == "rejected":
        print(f"FAIL: {verdict.detail}")
        print("  Work on a branch named lane/<lane>/<topic>, for example lane/chassis/bump-strip.")
        print("  Only ARCH's own branches (see [unrestricted].heads in docs/swarm/ownership.toml) may change anything.")
        annotate("error", verdict.detail, title=TITLE)
        return

    lane = verdict.lane
    print(f"  head       : lane branch, lane {lane!r}, topic {verdict.topic!r}" + (" (CCR)" if verdict.is_ccr else ""))

    if verdict.is_ccr:
        print()
        print("=" * 100)
        print(" CCR PULL REQUEST: ARCH MUST REVIEW EVERY LINE OF THIS PULL REQUEST")
        print("=" * 100)
        print(f"The title starts with {CCR_PREFIX!r}, so this PR may change contract paths and make migration edits")
        print(f"anywhere. {len(verdict.outside)} path(s) are outside lane {lane!r} and the common paths:")
        ccr_paths = ownership.get("ccr", {}).get("paths", [])
        for path in verdict.outside[:MAX_LISTED]:
            if matches_any(ccr_paths, path):
                note = "contract path"
            else:
                owners = owners_of(path, ownership, exclude=lane)
                note = "migration edit in lane " + ", ".join(owners) if owners else "migration edit (no lane owns it)"
            print(f"  - {printable(path)}   [{note}]")
        if len(verdict.outside) > MAX_LISTED:
            print(f"  ... and {len(verdict.outside) - MAX_LISTED} more path(s)")
        annotate(
            "warning",
            f"CCR pull request: ARCH must review every line. {len(verdict.outside)} path(s) are outside lane {lane}.",
            title="CCR pull request",
        )
        if not verdict.contract_touched:
            msg = "the title starts with 'CCR:' but no contract path changed; drop the prefix if this is not a contract change"
            print(f"warning: {msg}")
            annotate("warning", msg, title="CCR pull request")
        if not verdict.violations:
            print("PASS (with the review obligation above)")
            return

    if verdict.violations:
        print(f"FAIL: {len(verdict.violations)} path(s) are outside lane {lane!r}")
        if verdict.protected_hits:
            print(f"  of these, {verdict.protected_hits} PROTECTED: no lane branch may change a protected path, not even with a 'CCR:' title.")
        for v in verdict.violations[:MAX_LISTED]:
            print(f"  - {printable(v.path)}")
            print(f"      {printable(v.message)}")
            annotate("error", v.message, file=v.path, title=TITLE)
        if len(verdict.violations) > MAX_LISTED:
            print(f"  ... and {len(verdict.violations) - MAX_LISTED} more path(s). The full list: git diff --name-status -M {printable(base)}...HEAD")
        print()
        print(f"Lane {lane!r} may change:")
        print("  its own paths : " + ", ".join(ownership["lanes"][lane]["paths"]))
        print("  common paths  : " + ", ".join(p.replace("{lane}", lane) for p in ownership.get("common", {}).get("paths", [])))
        if verdict.is_ccr:
            print("  with CCR:     : also the contract paths, and migration edits anywhere except the protected paths")
        if verdict.protected_hits:
            print("Protected (only ARCH's own branches may change these, CCR or not): " + ", ".join(protected_paths(ownership)))
        print(f"Undo the changes to the other files (git checkout {printable(base)} -- <path>) and file an interface request.")
        print("See docs/swarm/GUARDRAILS.md, section 'Lane guard'.")
        return
    print("  every changed path is inside the lane's own or the common paths")
    print("PASS")


# ---- command line --------------------------------------------------------------------------------------------------------


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="the revision the pull request is going into, e.g. origin/integration")
    ap.add_argument("--head", required=True, help="the pull request's last commit, e.g. HEAD")
    ap.add_argument("--branch", required=True, help="the pull request's branch name, e.g. lane/chassis/bump-strip")
    ap.add_argument("--title", default="", help="the pull request title (a leading 'CCR:' enables contract changes)")
    ap.add_argument("--ownership", help=f"read the rules from this file instead of {OWNERSHIP_REL} at --base")
    ap.add_argument("--repo", default=".", help="the git repository (default: the current directory)")
    args = ap.parse_args(argv)

    try:
        branch = args.branch.strip().removeprefix("refs/heads/")
        ownership, source = load_ownership(args.repo, args.base, args.ownership)
        changes = diff_changes(args.repo, args.base, args.head)
    except CiError as exc:
        print(f"lane guard: setup problem: {exc}", file=sys.stderr)
        return 2

    verdict = evaluate(ownership, branch, args.title, changes)
    report(verdict, ownership, branch, args.title, changes, source, args.base)
    return 0 if verdict.ok else 1


if __name__ == "__main__":
    sys.exit(main())
