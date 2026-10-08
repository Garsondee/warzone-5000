"""Golden guard: golden files change only on purpose.

    python3 -I tools/ci/golden_guard.py --base origin/integration --head HEAD --body-file pr_body.txt

A golden is a recorded answer that tests compare against: every file under a `tests/golden/` folder (the per-scenario
hash chains and trajectories), and any file named `*.golden.*` anywhere. Regenerating a golden is the easiest way to make
a failing test pass without fixing the bug, so goldens are regenerated only deliberately, never to make a failing test
pass. When a pull request adds, changes, deletes or renames a golden, its description (--body-file) must contain a line

    Golden-Change: <why the numbers changed on purpose>

with a real reason (not empty, not an unfilled <placeholder>, not TODO/none/n-a). Lines inside <!-- html comments -->
do not count.

Exit status: 0 = fine, 1 = a golden changed without a reason, 2 = the setup is broken.
"""

from __future__ import annotations

import argparse
import fnmatch
import os
import re
import sys

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, annotate, diff_changes, printable  # noqa: E402

RULE = "goldens are regenerated only deliberately, never to make a failing test pass"
TITLE = "Golden guard"
_HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
_REASON_LINE = re.compile(r"^[ \t]*Golden-Change:[ \t]*(\S.*?)[ \t]*$", re.IGNORECASE | re.MULTILINE)
_PLACEHOLDERS = {"todo", "tbd", "n/a", "na", "none", "-", "...", "…", "?"}


def is_golden(path: str) -> bool:
    basename = path.rsplit("/", 1)[-1]
    return "/tests/golden/" in "/" + path or fnmatch.fnmatchcase(basename, "*.golden.*")


def golden_reason(body: str) -> str | None:
    """The reason given on a `Golden-Change:` line, or None if there is no usable one."""
    body = body.replace("\r\n", "\n").replace("\r", "\n")  # descriptions edited on github.com use Windows line endings
    for match in _REASON_LINE.finditer(_HTML_COMMENT.sub("", body)):
        reason = match.group(1)
        if re.fullmatch(r"<[^>]*>", reason) or reason.lower() in _PLACEHOLDERS:
            continue  # an unfilled template line
        return reason
    return None


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="the revision the pull request is going into, e.g. origin/integration")
    ap.add_argument("--head", required=True, help="the pull request's last commit, e.g. HEAD")
    ap.add_argument("--body-file", required=True, help="a file holding the pull request description")
    ap.add_argument("--repo", default=".", help="the git repository (default: the current directory)")
    args = ap.parse_args(argv)

    try:
        changes = diff_changes(args.repo, args.base, args.head)
    except CiError as exc:
        print(f"golden guard: setup problem: {exc}", file=sys.stderr)
        return 2
    goldens = sorted({p for c in changes for p in c.paths if is_golden(p)})
    if not goldens:
        print("golden guard: OK (no golden file changed)")
        return 0

    try:
        with open(args.body_file, encoding="utf-8", errors="replace") as fh:
            body = fh.read()
    except OSError:
        body = ""  # no description file: treat as an empty description
    reason = golden_reason(body)
    if reason:
        print(f"golden guard: OK, {len(goldens)} golden file(s) changed on purpose.")
        for path in goldens:
            print(f"  - {printable(path)}")
        print(f"  Golden-Change: {printable(reason)}")
        return 0

    print(f"golden guard: FAIL, this pull request changes {len(goldens)} golden file(s):")
    for path in goldens:
        print(f"  - {printable(path)}")
        annotate("error", f"golden file changed without a Golden-Change line in the PR description. {RULE}", file=path, title=TITLE)
    print()
    print(f"Rule: {RULE}.")
    print("If the new numbers are right because the physics or the content changed on purpose, say why: add a line")
    print("to the pull request description (not the commit message) and push again or edit the description:")
    print("    Golden-Change: <what changed on purpose and why the new numbers are correct>")
    print("If a test fails and you regenerated the golden to make it pass, undo that and fix the code instead.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
