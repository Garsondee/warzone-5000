"""Helpers shared by the checks in tools/ci (standard library only, no network).

Run every check as `python3 -I tools/ci/<name>.py`. The -I flag keeps the current folder and the script's own folder off
sys.path (so a stray json.py cannot hijack a check), so each script appends its own folder to sys.path before importing
this file. Appending (not inserting) keeps the standard library first.

Exit codes used by all the checks: 0 = pass, 1 = the pull request breaks a rule, 2 = the setup is broken (bad config,
git failed). A check that exits 2 is a bug in CI, not in the pull request.
"""

from __future__ import annotations

import functools
import os
import re
import subprocess
import tomllib
from dataclasses import dataclass


class CiError(Exception):
    """The setup is wrong (unreadable config, git failed), not the pull request."""


# ---- git ---------------------------------------------------------------------------------------------------------------


def run_git(repo, *args: str) -> bytes:
    """Run `git -C repo ARGS...` and return stdout. Raises CiError carrying git's own message when it fails."""
    proc = subprocess.run(["git", "-C", str(repo), *args], capture_output=True)
    if proc.returncode != 0:
        detail = proc.stderr.decode("utf-8", "replace").strip()
        raise CiError(f"`git {' '.join(args[:2])} ...` failed in {repo}: {detail}")
    return proc.stdout


def try_git(repo, *args: str) -> bytes | None:
    """Like run_git but returns None instead of raising (for 'does this file exist at that revision?')."""
    try:
        return run_git(repo, *args)
    except CiError:
        return None


@dataclass(frozen=True)
class Change:
    """One entry of `git diff --name-status`."""

    status: str  # one letter: A added, M modified, D deleted, R renamed, C copied, T type change
    path: str  # the path after the change (the only path for A, M, D, T)
    old_path: str | None = None  # the path before the change, for R and C
    score: int = 100  # similarity percentage for R and C (100 = pure rename)

    @property
    def paths(self) -> tuple[str, ...]:
        """Every path this change touches. A rename touches both sides, so both are checked."""
        return (self.old_path, self.path) if self.old_path else (self.path,)


def _decode(raw: bytes) -> str:
    return raw.decode("utf-8", "replace")


def parse_name_status(raw: bytes) -> list[Change]:
    """Parse the NUL-separated output of `git diff --name-status -z` (safe for spaces, quotes and unicode in names)."""
    parts = raw.split(b"\0")
    if parts and parts[-1] == b"":
        parts.pop()
    changes: list[Change] = []
    i = 0
    try:
        while i < len(parts):
            status = parts[i].decode("ascii", "replace")
            letter = status[:1]
            if letter in ("R", "C"):
                score = int(status[1:] or 100)
                changes.append(Change(letter, _decode(parts[i + 2]), _decode(parts[i + 1]), score))
                i += 3
            else:
                changes.append(Change(letter, _decode(parts[i + 1])))
                i += 2
    except (IndexError, ValueError) as exc:
        raise CiError(f"could not parse `git diff --name-status -z` output: {exc}") from exc
    return changes


def check_rev(rev: str) -> str:
    """A revision must not look like an option (`--output=file`), or git would treat it as one."""
    if not rev or rev.startswith("-"):
        raise CiError(f"not a usable git revision: {rev!r}")
    return rev


def diff_changes(repo, base: str, head: str) -> list[Change]:
    """Files changed on HEAD since it branched from BASE (`BASE...HEAD`), with renames detected (-M)."""
    check_rev(base)
    check_rev(head)
    raw = run_git(
        repo, "diff", "--name-status", "-M", "-z", "--no-color", "--no-ext-diff", f"{base}...{head}", "--"
    )
    return parse_name_status(raw)


# ---- globs -------------------------------------------------------------------------------------------------------------


@functools.lru_cache(maxsize=None)
def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Translate a repo-relative glob into a regex.

    `**` crosses directories (`a/**/b` also matches `a/b`); `*` and `?` never cross `/`; `[abc]` and `[!abc]` are
    character classes; everything else is literal.
    """
    out: list[str] = []
    i, n = 0, len(pattern)
    while i < n:
        c = pattern[i]
        if c == "*":
            j = i
            while j < n and pattern[j] == "*":
                j += 1
            if j - i >= 2:
                if pattern.startswith("/", j):  # `**/` also matches zero directories
                    out.append("(?:.*/)?")
                    j += 1
                else:
                    out.append(".*")
            else:
                out.append("[^/]*")
            i = j
        elif c == "?":
            out.append("[^/]")
            i += 1
        elif c == "[":
            j = i + 1
            if j < n and pattern[j] in "!^":
                j += 1
            if j < n and pattern[j] == "]":
                j += 1
            while j < n and pattern[j] != "]":
                j += 1
            if j >= n:  # no closing bracket: a literal `[`
                out.append(re.escape(c))
                i += 1
            else:
                body = pattern[i + 1 : j]
                negate = body[:1] in ("!", "^")
                if negate:
                    body = body[1:]
                body = re.sub(r"([\\\[\]^])", r"\\\1", body)
                out.append(("[^/" if negate else "[") + body + "]")
                i = j + 1
        else:
            out.append(re.escape(c))
            i += 1
    return re.compile("".join(out), re.DOTALL)


def glob_match(pattern: str, path: str) -> bool:
    """Does the repo-relative `path` match the glob `pattern`?"""
    if pattern.startswith("./"):
        pattern = pattern[2:]
    return glob_to_regex(pattern).fullmatch(path) is not None


def matches_any(patterns, path: str) -> bool:
    return any(glob_match(p, path) for p in patterns)


# ---- output ------------------------------------------------------------------------------------------------------------


def printable(text: str) -> str:
    """Make user-controlled text (file names, PR titles) safe to print: no control characters, so nothing can start a
    line with `::` and be mistaken for a GitHub workflow command."""
    return re.sub(r"[\x00-\x1f\x7f]", "?", text)


def on_github() -> bool:
    return os.environ.get("GITHUB_ACTIONS") == "true"


MAX_ANNOTATIONS_PER_LEVEL = 10  # GitHub shows at most 10 errors and 10 warnings per step, so more would only be noise
_annotations_sent: dict[str, int] = {}


def reset_annotation_counts() -> None:
    _annotations_sent.clear()


def _escape_data(text: str) -> str:
    return text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def _escape_property(text: str) -> str:
    return _escape_data(text).replace(":", "%3A").replace(",", "%2C")


def annotate(level: str, message: str, *, file: str | None = None, line: int | None = None, title: str | None = None):
    """Print a GitHub annotation (`::error file=...::message`), shown on the pull request. Does nothing elsewhere."""
    if not on_github():
        return
    sent = _annotations_sent[level] = _annotations_sent.get(level, 0) + 1
    if sent > MAX_ANNOTATIONS_PER_LEVEL:
        if sent == MAX_ANNOTATIONS_PER_LEVEL + 1:
            print(f"::notice::more {level} annotations are not shown here; see the full list in this step's log")
        return
    # Escaping (%0A for a newline, ...) keeps the annotation on one line, so no user-controlled text can start a new
    # workflow command; that is why file names and messages are escaped here rather than stripped.
    props = []
    if file:
        props.append(f"file={_escape_property(file)}")
    if line:
        props.append(f"line={int(line)}")
    if title:
        props.append(f"title={_escape_property(title)}")
    head = f"::{level} {','.join(props)}" if props else f"::{level}"
    print(f"{head}::{_escape_data(message)}")


# ---- config ------------------------------------------------------------------------------------------------------------


def load_toml_bytes(data: bytes, what: str) -> dict:
    try:
        return tomllib.loads(data.decode("utf-8"))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as exc:
        raise CiError(f"{what} is not valid TOML: {exc}") from exc


def load_toml_file(path, what: str | None = None) -> dict:
    try:
        with open(path, "rb") as fh:
            data = fh.read()
    except OSError as exc:
        raise CiError(f"cannot read {what or path}: {exc}") from exc
    return load_toml_bytes(data, what or str(path))
