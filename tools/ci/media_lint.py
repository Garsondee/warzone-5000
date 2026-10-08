"""Media lint: keep the git repository small.

    python3 -I tools/ci/media_lint.py --base origin/integration --head HEAD

Looks at the files a pull request adds or modifies (BASE...HEAD; deletions and pure renames are free) and fails when:
  * a PNG, JPG/JPEG or WEBP is over 400 KB, or a GIF is over 800 KB;
  * a file has a forbidden extension: mp4 mov avi mkv zip 7z tar gz exe dll so dylib pdf;
  * any other file is over 1 MB;
  * the files added or modified in the pull request add up to more than 8 MB.
(KB and MB here are 1024 and 1024*1024 bytes.)

Clips are CI artifacts, not git objects: render video in CI and download it from the workflow run. Commit only small
still images (docs/lanes/<lane>/media/).

A path listed in tools/ci/large_files.allow (one repo-relative path per line, `#` starts a comment; read from the HEAD
revision) is exempt from every rule above and from the total. Only ARCH edits that file.

Exit status: 0 = fine, 1 = a rule is broken, 2 = the setup is broken.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, annotate, check_rev, diff_changes, printable, run_git, try_git  # noqa: E402

KB = 1024
IMAGE_LIMITS = {"png": 400 * KB, "jpg": 400 * KB, "jpeg": 400 * KB, "webp": 400 * KB, "gif": 800 * KB}
FORBIDDEN_EXTENSIONS = frozenset("mp4 mov avi mkv zip 7z tar gz exe dll so dylib pdf".split())
OTHER_LIMIT = 1024 * KB
TOTAL_LIMIT = 8 * 1024 * KB
ALLOW_FILE = "tools/ci/large_files.allow"
TITLE = "Media lint"
CLIPS_HINT = (
    "Clips are CI artifacts, not git objects: render video in CI and download it from the workflow run. "
    "Commit only small still images under docs/lanes/<lane>/media/."
)


def extension(path: str) -> str:
    name = path.rsplit("/", 1)[-1]
    return name.rsplit(".", 1)[-1].lower() if "." in name else ""


def kb(size: int) -> str:
    return f"{size / KB:.0f} KB"


def parse_allow_list(text: str) -> set[str]:
    allowed = set()
    for raw in text.splitlines():
        line = raw.split(" #", 1)[0].strip()
        if line and not line.startswith("#"):
            allowed.add(line)
    return allowed


def blob_sizes(repo, head: str, paths: list[str]) -> dict[str, int | None]:
    """Size in bytes of each path at `head` (None for anything that is not a regular blob, e.g. a submodule)."""
    check_rev(head)
    sizes: dict[str, int | None] = {}
    plain = [p for p in paths if "\n" not in p]
    if plain:
        request = "".join(f"{head}:{p}\n" for p in plain).encode("utf-8")
        proc = subprocess.run(["git", "-C", str(repo), "cat-file", "--batch-check"], input=request, capture_output=True)
        if proc.returncode != 0:
            raise CiError(f"git cat-file failed: {proc.stderr.decode('utf-8', 'replace').strip()}")
        for path, line in zip(plain, proc.stdout.decode("utf-8", "replace").splitlines()):
            fields = line.split()
            sizes[path] = int(fields[2]) if len(fields) == 3 and fields[1] == "blob" else None
    for path in paths:
        if "\n" in path:  # cannot be sent through the line-based batch interface
            sizes[path] = int(run_git(repo, "cat-file", "-s", f"{head}:{path}").strip())
    return sizes


def check_files(sizes: dict[str, int | None], allowed: set[str]) -> tuple[list[tuple[str, str]], int, int]:
    """Return ([(path, problem)], total counted bytes, number of exempt files)."""
    problems: list[tuple[str, str]] = []
    total = exempt = 0
    for path, size in sizes.items():
        if size is None:
            continue
        if path in allowed:
            exempt += 1
            continue
        total += size
        ext = extension(path)
        if ext in FORBIDDEN_EXTENSIONS:
            problems.append((path, f"has the forbidden extension .{ext} ({kb(size)}). {CLIPS_HINT}"))
        elif ext in IMAGE_LIMITS:
            if size > IMAGE_LIMITS[ext]:
                problems.append((path, f"is {kb(size)}, over the {kb(IMAGE_LIMITS[ext])} limit for .{ext} images. "
                                       "Shrink it (crop, fewer colours, WebP) or commit a smaller still. " + CLIPS_HINT))
        elif size > OTHER_LIMIT:
            problems.append((path, f"is {kb(size)}, over the {kb(OTHER_LIMIT)} limit for ordinary files. If it really must "
                                   f"be in git, ask ARCH to list it in {ALLOW_FILE}."))
    return problems, total, exempt


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="the revision the pull request is going into, e.g. origin/integration")
    ap.add_argument("--head", required=True, help="the pull request's last commit, e.g. HEAD")
    ap.add_argument("--repo", default=".", help="the git repository (default: the current directory)")
    ap.add_argument("--allow", help=f"read the allow-list from this file instead of {ALLOW_FILE} at --head")
    args = ap.parse_args(argv)

    try:
        changes = diff_changes(args.repo, args.base, args.head)
        # Deleted files and pure renames (same content, new name) add nothing to the repository.
        touched = [c.path for c in changes if c.status != "D" and not (c.status == "R" and c.score == 100)]
        if args.allow:
            with open(args.allow, encoding="utf-8") as fh:
                allowed = parse_allow_list(fh.read())
        else:
            raw = try_git(args.repo, "show", f"{args.head}:{ALLOW_FILE}")
            allowed = parse_allow_list(raw.decode("utf-8", "replace")) if raw is not None else set()
        sizes = blob_sizes(args.repo, args.head, touched)
    except (CiError, OSError) as exc:
        print(f"media lint: setup problem: {exc}", file=sys.stderr)
        return 2

    problems, total, exempt = check_files(sizes, allowed)
    for path, problem in problems:
        print(f"{printable(path)} {problem}")
        annotate("error", f"{path} {problem}", file=path, title=TITLE)
    if total > TOTAL_LIMIT:
        msg = (f"this pull request adds or modifies {kb(total)} of files, over the {kb(TOTAL_LIMIT)} total limit. "
               "Split it up, and remember: " + CLIPS_HINT)
        print(msg)
        annotate("error", msg, title=TITLE)
        problems.append(("(total)", msg))
    note = f", {exempt} exempt via {ALLOW_FILE}" if exempt else ""
    if problems:
        print(f"media lint: FAIL, {len(problems)} problem(s); {len(sizes)} file(s), {kb(total)} counted{note}")
        return 1
    print(f"media lint: OK ({len(sizes)} file(s) added or modified, {kb(total)} of {kb(TOTAL_LIMIT)}{note})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
