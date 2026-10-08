"""A small Rust lexer: just enough to tell code from comments and strings (standard library only).

It is used by constants_lint.py (find float literals in real code) and line_budget.py (count lines of real code). It is
not a parser. It understands:
  * comments: `// ...` and nested `/* ... */`
  * strings: "..." with escapes, raw r"..." / r#"..."#, byte b"...", C c"..." and raw variants
  * char literals ('x', '\\n', '"') versus lifetimes and labels ('a, 'static)
  * numbers: integers (decimal, 0x, 0o, 0b) and floats (1.5, 1., 1e-3, 2.5f64, 1f64), with `_` separators and suffixes
  * identifiers (including r#raw) and single-character punctuation
Everything else is ignored.
"""

from __future__ import annotations

import re
from typing import NamedTuple


class Token(NamedTuple):
    kind: str  # ident | int | float | str | char | lifetime | punct
    text: str  # the source text of the token, exactly as written
    line: int  # 1-based line where the token starts
    end_line: int  # line where it ends (only multi-line strings differ from `line`)


class Comment(NamedTuple):
    text: str
    line: int
    end_line: int


_TOKEN = re.compile(
    r"""
    (?P<ws>\s+)
  | (?P<line_comment>//[^\n]*)
  | (?P<block_start>/\*)
  | (?P<raw_str>(?:br|cr|r)(?P<hashes>\#*)")
  | (?P<str>[bc]?")
  | (?P<byte_char>b'(?:\\(?:x[0-9a-fA-F]{2}|[^\n])|[^\\'\n])')
  | (?P<char>'(?:\\(?:u\{[0-9a-fA-F_]*\}|x[0-9a-fA-F]{2}|[^\n])|[^\\'\n])')
  | (?P<lifetime>'[^\W\d]\w*)
  | (?P<number>[0-9])
  | (?P<ident>(?:r\#)?[^\W\d]\w*)
  | (?P<punct>.)
    """,
    re.VERBOSE | re.DOTALL,
)

# A number, tried at a digit. The decimal branch follows the Rust reference: `1.5`, `1.` (a dot not followed by another
# dot, an underscore or a letter), `1e-3`, `1.5E+3`, optionally followed by a suffix such as f32, f64 or u8.
_NUMBER = re.compile(
    r"""
    0[xX][0-9a-fA-F_]+(?:[iu](?:8|16|32|64|128|size))?
  | 0[oO][0-7_]+(?:[iu](?:8|16|32|64|128|size))?
  | 0[bB][01_]+(?:[iu](?:8|16|32|64|128|size))?
  | (?P<int>[0-9][0-9_]*)
    (?:(?P<frac>\.[0-9][0-9_]*)|(?P<bare>\.(?![.\w])))?
    (?P<exp>[eE][+-]?_*[0-9][0-9_]*)?
    (?P<suffix>[A-Za-z_][A-Za-z0-9_]*)?
    """,
    re.VERBOSE,
)
_TUPLE_INDEX = re.compile(r"[0-9]+")
_BLOCK_DELIMITER = re.compile(r"/\*|\*/")
_STRING_REST = re.compile(r'[^"\\]*(?:\\.[^"\\]*)*"?', re.DOTALL)


def _end_of_block_comment(src: str, pos: int) -> int:
    """`pos` is just after an opening /*. Rust block comments nest."""
    depth = 1
    while depth:
        m = _BLOCK_DELIMITER.search(src, pos)
        if m is None:
            return len(src)  # unterminated: the comment runs to the end of the file
        depth += 1 if m.group() == "/*" else -1
        pos = m.end()
    return pos


def _end_of_string(src: str, pos: int) -> int:
    return _STRING_REST.match(src, pos).end()


def _end_of_raw_string(src: str, pos: int, hashes: int) -> int:
    closing = '"' + "#" * hashes
    found = src.find(closing, pos)
    return len(src) if found < 0 else found + len(closing)


def _lex_number(src: str, pos: int) -> tuple[int, str]:
    """Return (end, kind) for the number starting at `pos`; kind is "int" or "float"."""
    if pos >= 1 and src[pos - 1] == "." and (pos < 2 or src[pos - 2] != "."):
        # `t.0` and `t.0.1` are tuple fields, not the floats 0 and 0.1 (a range like `0..1.5` has two dots).
        return _TUPLE_INDEX.match(src, pos).end(), "int"
    m = _NUMBER.match(src, pos)
    if m.group("int") is None:  # 0x.., 0o.., 0b..
        return m.end(), "int"
    is_float = m.group("frac") or m.group("bare") or m.group("exp") or m.group("suffix") in ("f32", "f64")
    return m.end(), "float" if is_float else "int"


def lex(src: str) -> tuple[list[Token], list[Comment]]:
    """Split Rust source into code tokens and comments. Whitespace is dropped."""
    if src.startswith("﻿"):
        src = src[1:]
    tokens: list[Token] = []
    comments: list[Comment] = []
    pos, line, size = 0, 1, len(src)
    match = _TOKEN.match
    while pos < size:
        m = match(src, pos)
        group = m.lastgroup
        end = m.end()
        if group == "ws":
            line += src.count("\n", pos, end)
            pos = end
            continue
        if group == "line_comment":
            comments.append(Comment(m.group(), line, line))
            pos = end
            continue
        if group == "block_start":
            end = _end_of_block_comment(src, end)
            newlines = src.count("\n", pos, end)
            comments.append(Comment(src[pos:end], line, line + newlines))
            line += newlines
            pos = end
            continue
        newlines = 0
        if group == "raw_str":
            end = _end_of_raw_string(src, end, len(m.group("hashes")))
            kind = "str"
        elif group == "str":
            end = _end_of_string(src, end)
            kind = "str"
        elif group in ("byte_char", "char"):
            kind = "char"
        elif group == "lifetime":
            kind = "lifetime"
        elif group == "number":
            end, kind = _lex_number(src, pos)
        elif group == "ident":
            kind = "ident"
        else:
            kind = "punct"
        if kind == "str":
            newlines = src.count("\n", pos, end)
        tokens.append(Token(kind, src[pos:end], line, line + newlines))
        line += newlines
        pos = end
    return tokens, comments


def count_code_lines(src: str) -> int:
    """Lines that hold at least one token of real code. Blank lines and comment-only lines do not count; a line with
    code followed by a trailing comment does; every line of a multi-line string counts."""
    tokens, _ = lex(src)
    lines: set[int] = set()
    for tok in tokens:
        lines.update(range(tok.line, tok.end_line + 1))
    return len(lines)
