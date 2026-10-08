"""Tests for the small Rust lexer used by constants_lint and line_budget."""

import unittest

import ci_testlib  # noqa: F401
from rust_lex import count_code_lines, lex


def kinds(src, wanted=("float", "int", "str", "char", "lifetime")):
    tokens, _ = lex(src)
    return [(t.kind, t.text) for t in tokens if t.kind in wanted]


def floats(src):
    return [text for kind, text in kinds(src, ("float",))]


class NumberTests(unittest.TestCase):
    def test_float_forms(self):
        self.assertEqual(floats("1.5 1. 1e3 1e-3 2.5E+10 1_000.5 2.5f64 1e-3f32 3f32 1f64 2.0_f64"),
                         ["1.5", "1.", "1e3", "1e-3", "2.5E+10", "1_000.5", "2.5f64", "1e-3f32", "3f32", "1f64", "2.0_f64"])

    def test_integers_are_not_floats(self):
        self.assertEqual(floats("1 100 1_000 0x1e3 0xff 0b101 0o17 1u32 7usize 0xffu8"), [])

    def test_hex_with_an_e_is_not_an_exponent(self):
        self.assertEqual(kinds("0x1e3", ("int", "float")), [("int", "0x1e3")])

    def test_method_call_on_an_integer_is_not_a_float(self):
        self.assertEqual(kinds("1.max(2)", ("int", "float")), [("int", "1"), ("int", "2")])
        self.assertEqual(floats("2.0_f64.sqrt()"), ["2.0_f64"])

    def test_tuple_fields_are_not_floats(self):
        self.assertEqual(floats("let a = t.0.1 + t.2; self.0.0"), [])

    def test_ranges_are_not_floats_but_float_ranges_are(self):
        self.assertEqual(floats("for i in 0..10 {} for j in 0..=3 {}"), [])
        self.assertEqual(floats("let r = 0.5..1.5;"), ["0.5", "1.5"])

    def test_float_after_a_dot_that_ends_a_float(self):
        self.assertEqual(floats("1.0.max(2.0)"), ["1.0", "2.0"])


class StringAndCommentTests(unittest.TestCase):
    def test_floats_in_strings_and_comments_are_ignored(self):
        src = 'let s = "9.81"; // 3.14\n/* 2.71 */ let t = r#"1.23 "q" 4.56"#; /// 7.89\nlet x = 0.1;'
        self.assertEqual(floats(src), ["0.1"])

    def test_block_comments_nest(self):
        self.assertEqual(floats("/* a /* b 1.1 */ still comment 2.2 */ let x = 3.3;"), ["3.3"])

    def test_unterminated_block_comment_runs_to_the_end(self):
        self.assertEqual(floats("let a = 1.5; /* never closed 2.5"), ["1.5"])

    def test_char_literal_holding_a_quote_does_not_open_a_string(self):
        self.assertEqual(floats("let q = '\"'; let g = 9.81;"), ["9.81"])

    def test_lifetimes_are_not_char_literals(self):
        toks = kinds("fn f<'a>(x: &'a str) -> f64 { 9.81 }", ("lifetime", "char", "float"))
        self.assertEqual(toks, [("lifetime", "'a"), ("lifetime", "'a"), ("float", "9.81")])

    def test_escaped_chars(self):
        self.assertEqual([t for k, t in kinds("'\\'' '\\\\' '\\n' '\\u{1F600}' b'x'", ("char",))],
                         ["'\\''", "'\\\\'", "'\\n'", "'\\u{1F600}'", "b'x'"])

    def test_byte_c_and_raw_strings(self):
        src = 'b"1.5" c"2.5" br#"3.5"# r"4.5" r##"5.5 "# 6.5"## 7.5'
        self.assertEqual(floats(src), ["7.5"])

    def test_raw_identifier_is_not_a_string(self):
        self.assertEqual(floats("let r#type = 1.5;"), ["1.5"])

    def test_escaped_quote_inside_a_string(self):
        self.assertEqual(floats(r'let s = "say \"9.81\" now"; let t = 2.5;'), ["2.5"])


class LineNumberTests(unittest.TestCase):
    def test_lines_are_counted_through_comments_and_strings(self):
        tokens, comments = lex("a\n/* two\nlines */ b \"s\nt\" c")
        self.assertEqual([(t.text, t.line, t.end_line) for t in tokens],
                         [("a", 1, 1), ("b", 3, 3), ('"s\nt"', 3, 4), ("c", 4, 4)])
        self.assertEqual([(c.line, c.end_line) for c in comments], [(2, 3)])

    def test_crlf_files(self):
        tokens, _ = lex("a\r\nb\r\n1.5")
        self.assertEqual([t.line for t in tokens], [1, 2, 3])

    def test_byte_order_mark_is_ignored(self):
        tokens, _ = lex("﻿fn main() {}")
        self.assertEqual(tokens[0].text, "fn")


class CountCodeLinesTests(unittest.TestCase):
    def test_blank_and_comment_only_lines_do_not_count(self):
        src = "// header\n\n/// doc\nfn a() {\n    // inside\n    let x = 1; // trailing\n}\n\n/* block\n   comment */\n"
        self.assertEqual(count_code_lines(src), 3)  # `fn a() {`, `let x = 1;`, `}`

    def test_code_after_a_block_comment_on_the_same_line_counts(self):
        self.assertEqual(count_code_lines("/* c */ let x = 1;\n"), 1)

    def test_multi_line_string_counts_every_line(self):
        self.assertEqual(count_code_lines('let s = "one\ntwo\n\nthree";\n'), 4)

    def test_comment_markers_inside_strings_are_code(self):
        self.assertEqual(count_code_lines('let url = "http://example.com"; \nlet g = "/* not a comment */";\n'), 2)

    def test_empty_file(self):
        self.assertEqual(count_code_lines(""), 0)


if __name__ == "__main__":
    unittest.main()
