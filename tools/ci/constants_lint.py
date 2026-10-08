"""Constants lint: no bare physical constants in Rust code.

    python3 -I tools/ci/constants_lint.py [--root DIR]

Physical numbers (a spring rate, a friction coefficient, gravity) live in RON files with provenance, not in Rust. This
check scans crates/*/src/**/*.rs for float literals (`0.001`, `9.81`, `1e-3`, `2.5f64`, `1f64`) and fails on every one
that is not allowed. Allowed without a comment:
  * the values 0.0, 0.5, 1.0, 2.0, 3.0, 4.0 and 0.25 (however they are spelled: 1e0, 2f64, 0.50, ...);
  * tolerances: any literal smaller than 1e-5 in size, i.e. with a decimal exponent of -6 or less (1e-6, 5e-9, 0.0000001).
Anything else needs `// const-ok: <reason>` on the same line or the line above (a mathematical constant, a tolerance, ...).

Not scanned: comments and string literals; code inside #[cfg(test)] items and #[test] functions; directories named
`tests` or `benches` and files named `tests.rs` under src; the crates in EXEMPT_CRATES; the test doubles in
crates/w5k_contract/src/testing.rs.

Exit status: 0 = clean, 1 = violations, 2 = the setup is broken (no crates folder).
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in tools/ci: lanes run `git add -A` after running the checks
sys.path.append(os.path.dirname(os.path.abspath(__file__)))  # python3 -I does not add the script's folder
from ci_common import CiError, annotate, printable  # noqa: E402
from rust_lex import lex  # noqa: E402

EXEMPT_CRATES = ("w5k_math",)  # the maths library is where the mathematical constants live
SKIP_PATHS = ("crates/w5k_contract/src/testing.rs", "crates/w5k_contract/src/testing/")  # the test doubles
ALLOWED_VALUES = frozenset(Decimal(v) for v in ("0.0", "0.5", "1.0", "2.0", "3.0", "4.0", "0.25"))
TOLERANCE_EXPONENT = -6  # a literal whose leading digit is at 10^-6 or below is a tolerance
MESSAGE = (
    "physical constants live in RON with provenance (see docs/architecture/CONTRACTS.md, Param); "
    "if this is a mathematical constant or a tolerance add `// const-ok: <reason>`"
)
TITLE = "Constants lint"
MAX_LISTED = 100  # print this many violations in full; the count line always shows the real total

_FLOAT_SUFFIX = re.compile(r"(?:f32|f64)$")
_CONST_OK = re.compile(r"const-ok:(.*)", re.DOTALL)

# Attributes that mark the item after them as test-only code (token texts; `inner` = applies to the whole file).
_TEST_ATTRIBUTES = (
    (("#", "[", "cfg", "(", "test", ")", "]"), False),
    (("#", "[", "test", "]"), False),
    (("#", "!", "[", "cfg", "(", "test", ")", "]"), True),
)


@dataclass
class Violation:
    line: int
    literal: str
    hint: str = ""


def float_value(text: str) -> Decimal | None:
    """The exact value of a float literal such as `1_000.5`, `2.5f64`, `1e-3` (None if it cannot be read)."""
    try:
        return Decimal(_FLOAT_SUFFIX.sub("", text.replace("_", "")))
    except InvalidOperation:
        return None


def is_allowed(value: Decimal) -> bool:
    if value in ALLOWED_VALUES:
        return True
    return value != 0 and value.adjusted() <= TOLERANCE_EXPONENT


def _matching(texts: list[str], start: int, opener: str, closer: str) -> int:
    depth = 0
    for j in range(start, len(texts)):
        if texts[j] == opener:
            depth += 1
        elif texts[j] == closer:
            depth -= 1
            if depth == 0:
                return j
    return len(texts) - 1  # unbalanced: treat the rest of the file as the item


def _item_end(texts: list[str], i: int) -> int:
    """`i` is the first token after a test attribute. Return the index just past the item it applies to: a braced
    block ({...}) or a statement ending in `;`, whichever comes first outside (...) and [...]."""
    n = len(texts)
    while i < n and texts[i] == "#":  # more attributes, e.g. #[cfg(test)] #[allow(dead_code)] mod tests { ... }
        j = i + 1
        if j < n and texts[j] == "!":
            j += 1
        if j < n and texts[j] == "[":
            i = _matching(texts, j, "[", "]") + 1
        else:
            break
    depth = 0
    while i < n:
        text = texts[i]
        if text in ("(", "["):
            depth += 1
        elif text in (")", "]"):
            depth -= 1
        elif text == "{" and depth <= 0:
            return _matching(texts, i, "{", "}") + 1
        elif text == ";" and depth <= 0:
            return i + 1
        i += 1
    return n


def find_test_code(tokens) -> bytearray | None:
    """Mark the tokens that are test-only code. Returns None when the whole file is (`#![cfg(test)]`)."""
    texts = [t.text if t.kind in ("punct", "ident") else "" for t in tokens]  # strings and numbers never match
    skip = bytearray(len(tokens))
    i, n = 0, len(texts)
    while i < n:
        if texts[i] == "#":
            for pattern, inner in _TEST_ATTRIBUTES:
                if tuple(texts[i : i + len(pattern)]) == pattern:
                    if inner:
                        return None
                    end = _item_end(texts, i + len(pattern))
                    skip[i:end] = b"\x01" * (end - i)
                    i = end - 1
                    break
        i += 1
    return skip


def const_ok_lines(comments) -> tuple[set[int], set[int]]:
    """Lines covered by a `const-ok: reason` comment, and lines whose `const-ok:` has no reason."""
    with_reason: set[int] = set()
    without_reason: set[int] = set()
    for comment in comments:
        m = _CONST_OK.search(comment.text)
        if not m:
            continue
        reason = m.group(1).strip()
        if reason.endswith("*/"):
            reason = reason[:-2].strip()
        (with_reason if reason else without_reason).update(range(comment.line, comment.end_line + 1))
    return with_reason, without_reason


def scan_source(src: str) -> list[Violation]:
    """Every float literal in `src` that is neither allowed nor marked const-ok, in order."""
    tokens, comments = lex(src)
    skip = find_test_code(tokens)
    if skip is None:
        return []
    marked, marked_without_reason = const_ok_lines(comments)
    found: list[Violation] = []
    for index, tok in enumerate(tokens):
        if tok.kind != "float" or skip[index]:
            continue
        value = float_value(tok.text)
        if value is not None and is_allowed(value):
            continue
        line = tok.line
        if line in marked or line - 1 in marked:
            continue
        hint = ""
        if line in marked_without_reason or line - 1 in marked_without_reason:
            hint = "the `// const-ok:` comment needs a reason after the colon"
        found.append(Violation(line, tok.text, hint))
    return found


def source_files(root: Path):
    """Yield (path, repo-relative path) for every file the lint covers, in a stable order."""
    for path in sorted(root.glob("crates/*/src/**/*.rs")):
        rel = path.relative_to(root).as_posix()
        parts = rel.split("/")  # crates / <crate> / src / ... / file.rs
        if parts[1] in EXEMPT_CRATES:
            continue
        if any(rel == skip or (skip.endswith("/") and rel.startswith(skip)) for skip in SKIP_PATHS):
            continue
        if any(part in ("tests", "benches") for part in parts[3:-1]) or parts[-1] == "tests.rs":
            continue
        yield path, rel


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--root", default=".", help="the repository root (default: the current directory)")
    args = ap.parse_args(argv)
    root = Path(args.root)
    try:
        if not (root / "crates").is_dir():
            raise CiError(f"{root / 'crates'} does not exist; run from the repository root or pass --root")
    except CiError as exc:
        print(f"constants lint: setup problem: {exc}", file=sys.stderr)
        return 2

    scanned = 0
    count = 0
    files_with_hits = 0
    for path, rel in source_files(root):
        scanned += 1
        hits = scan_source(path.read_text(encoding="utf-8", errors="replace"))
        files_with_hits += bool(hits)
        for v in hits:
            count += 1
            if count > MAX_LISTED:
                continue
            suffix = f"  ({v.hint})" if v.hint else ""
            print(f"{printable(rel)}:{v.line}: float literal `{v.literal}`{suffix}")
            annotate("error", f"float literal `{v.literal}`: {MESSAGE}", file=rel, line=v.line, title=TITLE)
    if count:
        if count > MAX_LISTED:
            print(f"... and {count - MAX_LISTED} more not listed (run the check locally for the full list)")
        print()
        print(f"constants lint: FAIL, {count} bare float literal(s) in {files_with_hits} file(s).")
        print(MESSAGE)
        print("Allowed without a comment: 0.0 0.5 1.0 2.0 3.0 4.0 0.25 and anything smaller than 1e-5 (tolerances).")
        return 1
    print(f"constants lint: OK ({scanned} file(s) scanned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
