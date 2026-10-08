"""Tests for constants_lint.py: which float literals are flagged, which are not, and which files are scanned."""

import tempfile
import unittest
from pathlib import Path

import ci_testlib  # noqa: F401
import constants_lint
from ci_testlib import run_main, run_script
from constants_lint import MESSAGE, scan_source


def hits(src):
    return [(v.line, v.literal) for v in scan_source(src)]


class WhichLiteralsAreFlagged(unittest.TestCase):
    def test_the_allowed_values_pass_in_any_spelling(self):
        src = "let a = [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 0.25, 1e0, 2f64, 0.50, 1., 4.0f32, 2.0_f64, 1_0e-1, 0e5];"
        self.assertEqual(hits(src), [])

    def test_everything_else_is_flagged(self):
        src = "let g = 9.81; let mu = 0.1; let five = 5.0; let ten = 10.0; let t = 1_000.0; let k = 0.75;"
        self.assertEqual([lit for _, lit in hits(src)], ["9.81", "0.1", "5.0", "10.0", "1_000.0", "0.75"])

    def test_exponent_and_suffix_forms_from_the_brief(self):
        self.assertEqual([lit for _, lit in hits("let a = 1e-3; let b = 2.5f64; let c = 1e3; let d = 7f32;")],
                         ["1e-3", "2.5f64", "1e3", "7f32"])

    def test_tolerances_with_exponent_minus_six_or_less_are_allowed(self):
        src = "let eps = [1e-6, 1e-9, 2.5e-7, 9.9e-6, 1e-12, 5E-8, 0.000001, 0.0000001, 1e-6f64];"
        self.assertEqual(hits(src), [])

    def test_the_boundary_just_above_a_tolerance_is_flagged(self):
        self.assertEqual([lit for _, lit in hits("let a = 1e-5; let b = 15e-6; let c = 0.00001; let d = 1.5e-5;")],
                         ["1e-5", "15e-6", "0.00001", "1.5e-5"])

    def test_negative_numbers_are_judged_by_size(self):
        self.assertEqual(hits("let a = -1.0; let b = -0.5; let c = -9.81;"), [(1, "9.81")])

    def test_integers_and_hex_are_not_floats(self):
        self.assertEqual(hits("let a = 100; let b = 0x1e3; let c = 1_000u32; let d = 0b101; let e = 7usize;"), [])

    def test_a_float_that_looks_like_an_integer_with_a_suffix_is_still_a_float(self):
        self.assertEqual(hits("let g = 9f64; let one = 1f32;"), [(1, "9f64")])

    def test_tuple_fields_ranges_and_method_calls_are_not_floats(self):
        src = "let a = t.0.1 + p.2; for i in 0..10 {} let m = 7.max(3); let r = 0.0..=1.0; let s = x.0.0;"
        self.assertEqual(hits(src), [])

    def test_line_numbers(self):
        src = "fn f() {\n    let a = 0.0;\n    let b = 9.81;\n\n    let c = /* two\n lines */ 3.3;\n}\n"
        self.assertEqual(hits(src), [(3, "9.81"), (6, "3.3")])


class CommentsAndStringsAreIgnored(unittest.TestCase):
    def test_floats_in_comments_and_strings(self):
        src = (
            'let a = 0.0; // gravity is 9.81\n'
            '/* mu = 0.7 */ let b = "9.81 m/s2";\n'
            '/// doc comment 3.14\n'
            "let c = r#\"6.67e-11 and 'quotes' \"#;\n"
            "let d = '\"'; let e = 0.0;\n"
            "/* nested /* 1.23 */ still a comment 4.56 */ let f = 1.0;\n"
        )
        self.assertEqual(hits(src), [])

    def test_a_float_after_a_char_literal_holding_a_quote_is_still_seen(self):
        self.assertEqual(hits("let q = '\"'; let g = 9.81;"), [(1, "9.81")])

    def test_lifetimes_do_not_confuse_the_scanner(self):
        self.assertEqual(hits("fn f<'a>(x: &'a str) -> f64 { 9.81 }"), [(1, "9.81")])


class ConstOkMarker(unittest.TestCase):
    def test_marker_on_the_same_line(self):
        self.assertEqual(hits("let g = 9.81; // const-ok: standard gravity, SPEC\n"), [])

    def test_marker_on_the_line_above(self):
        self.assertEqual(hits("// const-ok: degrees in a half turn\nlet d = 180.0;\n"), [])

    def test_marker_two_lines_above_does_not_count(self):
        self.assertEqual(hits("// const-ok: nope\n\nlet d = 180.0;\n"), [(3, "180.0")])

    def test_a_marker_covers_its_own_line_and_the_line_below_only(self):
        src = (
            "let v = [0.1, 0.2, 0.3]; // const-ok: test vector\n"  # line 1: every literal on the marker's line
            "let w = 0.4;\n"  # line 2: the line below the marker
            "let x = 0.6;\n"  # line 3: two lines below, not covered
        )
        self.assertEqual(hits(src), [(3, "0.6")])

    def test_marker_needs_a_reason(self):
        found = scan_source("let g = 9.81; // const-ok:\n")
        self.assertEqual(len(found), 1)
        self.assertIn("needs a reason", found[0].hint)
        self.assertEqual(hits("let g = 9.81; // const-ok:    \n"), [(1, "9.81")])

    def test_block_comment_marker(self):
        self.assertEqual(hits("let g = 9.81; /* const-ok: gravity */\n"), [])
        self.assertEqual(hits("let g = 9.81; /* const-ok: */\n"), [(1, "9.81")])

    def test_marker_inside_a_string_does_not_count(self):
        self.assertEqual(hits('let s = "// const-ok: lie"; let g = 9.81;\n'), [(1, "9.81")])


class TestCodeIsSkipped(unittest.TestCase):
    def test_cfg_test_module(self):
        src = (
            "pub fn f() -> f64 { 9.81 }\n"
            "#[cfg(test)]\n"
            "mod tests {\n"
            "    use super::*;\n"
            "    #[test]\n"
            "    fn t() { assert!((f() - 9.81).abs() < 0.01); let s = \"}\"; let c = '}'; }\n"
            "    fn helper() { if true { let x = 3.7; } }\n"
            "}\n"
            "pub fn g() -> f64 { 7.7 }\n"
        )
        self.assertEqual(hits(src), [(1, "9.81"), (9, "7.7")])

    def test_cfg_test_function_const_use_and_impl(self):
        src = (
            "#[cfg(test)]\nfn only_in_tests() -> f64 { 1.5 }\n"
            "#[cfg(test)]\nconst G: f64 = 9.81;\n"
            "#[cfg(test)]\nstatic TABLE: [f64; 3] = [1.5, 2.5, 3.5];\n"
            "#[cfg(test)]\nimpl Foo { fn x() -> f64 { 6.6 } }\n"
            "#[cfg(test)]\npub(crate) fn p() -> f64 { 8.8 }\n"
            "fn real() -> f64 { 5.5 }\n"
        )
        self.assertEqual(hits(src), [(11, "5.5")])

    def test_stacked_attributes(self):
        src = "#[cfg(test)]\n#[allow(dead_code)]\nmod tests { fn t() { let x = 9.9; } }\nfn real() { let y = 8.8; }\n"
        self.assertEqual(hits(src), [(4, "8.8")])

    def test_test_attribute_outside_a_cfg_test_module(self):
        src = "#[test]\nfn lonely_test() { assert_eq!(1.5 * 2.0, 3.0); }\nfn real() { let y = 8.8; }\n"
        self.assertEqual(hits(src), [(3, "8.8")])

    def test_cfg_not_test_is_not_skipped(self):
        src = "#[cfg(not(test))]\nfn real() { let y = 8.8; }\n"
        self.assertEqual(hits(src), [(2, "8.8")])

    def test_cfg_test_with_other_conditions_is_not_skipped(self):
        src = '#[cfg(all(test, feature = "x"))]\nfn maybe() { let y = 8.8; }\n'
        self.assertEqual(hits(src), [(2, "8.8")])

    def test_inner_cfg_test_skips_the_whole_file(self):
        self.assertEqual(hits("#![cfg(test)]\nfn t() { let y = 8.8; }\n"), [])

    def test_code_after_the_test_module_is_checked_again(self):
        src = "mod a { #[cfg(test)] mod t { fn f() { let x = 1.1; } } fn g() { let y = 2.2; } }\n"
        self.assertEqual(hits(src), [(1, "2.2")])

    def test_a_cfg_test_mod_declared_in_another_file_is_a_statement(self):
        src = "#[cfg(test)]\nmod tests;\nfn real() { let y = 8.8; }\n"
        self.assertEqual(hits(src), [(3, "8.8")])


class WhichFilesAreScanned(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)

    def write(self, rel, text="pub fn f() -> f64 { 9.81 }\n"):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def files(self):
        return [rel for _, rel in constants_lint.source_files(self.root)]

    def test_scans_src_of_every_crate_recursively(self):
        self.write("crates/w5k_chassis/src/lib.rs")
        self.write("crates/w5k_chassis/src/deep/er/mod.rs")
        self.write("crates/w5k_drive/src/lib.rs")
        self.assertEqual(self.files(), [
            "crates/w5k_chassis/src/deep/er/mod.rs", "crates/w5k_chassis/src/lib.rs", "crates/w5k_drive/src/lib.rs"])

    def test_skips_exempt_crates_tests_benches_and_the_test_doubles(self):
        self.write("crates/w5k_math/src/lib.rs")  # EXEMPT_CRATES
        self.write("crates/w5k_chassis/tests/mixing.rs")  # outside src
        self.write("crates/w5k_chassis/benches/b.rs")
        self.write("crates/w5k_chassis/src/tests/helper.rs")  # a tests directory inside src
        self.write("crates/w5k_chassis/src/benches/b.rs")
        self.write("crates/w5k_chassis/src/tests.rs")  # the file behind `#[cfg(test)] mod tests;`
        self.write("crates/w5k_chassis/src/sub/tests.rs")
        self.write("crates/w5k_contract/src/testing.rs")  # the test doubles
        self.write("crates/w5k_contract/src/testing/doubles.rs")
        self.write("crates/w5k_contract/src/lib.rs")  # but the rest of the contract crate is scanned
        self.write("crates/w5k_chassis/src/contests.rs")  # a name that merely ends in 'tests' is scanned
        self.write("crates/w5k_chassis/src/attests/mod.rs")
        self.assertEqual(self.files(), [
            "crates/w5k_chassis/src/attests/mod.rs", "crates/w5k_chassis/src/contests.rs",
            "crates/w5k_contract/src/lib.rs"])

    def test_exempt_crates_constant(self):
        self.assertEqual(constants_lint.EXEMPT_CRATES, ("w5k_math",))

    def test_only_rust_files(self):
        self.write("crates/w5k_chassis/src/data.ron", "(g: 9.81)")
        self.write("crates/w5k_chassis/src/notes.md", "9.81")
        self.assertEqual(self.files(), [])


class CommandLine(WhichFilesAreScanned):
    def test_clean_tree_passes(self):
        self.write("crates/w5k_chassis/src/lib.rs", "pub fn f() -> f64 { 0.0 }\n")
        code, out, _ = run_main(constants_lint.main, ["--root", str(self.root)])
        self.assertEqual(code, 0)
        self.assertIn("OK (1 file(s) scanned)", out)

    def test_violation_is_reported_with_file_line_and_the_message(self):
        self.write("crates/w5k_chassis/src/lib.rs", "fn f() {\n    let g = 9.81;\n}\n")
        code, out, _ = run_main(constants_lint.main, ["--root", str(self.root)])
        self.assertEqual(code, 1)
        self.assertIn("crates/w5k_chassis/src/lib.rs:2: float literal `9.81`", out)
        self.assertIn(MESSAGE, out)
        self.assertEqual(
            MESSAGE,
            "physical constants live in RON with provenance (see docs/architecture/CONTRACTS.md, Param); "
            "if this is a mathematical constant or a tolerance add `// const-ok: <reason>`",
        )

    def test_github_annotation_has_file_and_line(self):
        self.write("crates/w5k_chassis/src/lib.rs", "fn f() {\n    let g = 9.81;\n}\n")
        _, out, _ = run_main(constants_lint.main, ["--root", str(self.root)], {"GITHUB_ACTIONS": "true"})
        self.assertIn("::error file=crates/w5k_chassis/src/lib.rs,line=2,title=Constants lint::float literal `9.81`", out)

    def test_a_flood_of_violations_prints_a_bounded_list_and_the_true_total(self):
        self.write("crates/w5k_chassis/src/lib.rs", "".join(f"const C{i}: f64 = 7.{i};\n" for i in range(150)))
        code, out, _ = run_main(constants_lint.main, ["--root", str(self.root)])
        self.assertEqual(code, 1)
        self.assertEqual(sum(1 for l in out.splitlines() if l.startswith("crates/")), constants_lint.MAX_LISTED)
        self.assertIn("... and 50 more not listed", out)
        self.assertIn("150 bare float literal(s)", out)

    def test_wrong_root_is_a_setup_error_not_a_silent_pass(self):
        code, _, err = run_main(constants_lint.main, ["--root", str(self.root / "nowhere")])
        self.assertEqual(code, 2)
        self.assertIn("does not exist", err)

    def test_runs_under_python_dash_I(self):
        self.write("crates/w5k_chassis/src/lib.rs", "fn f() { let g = 9.81; }\n")
        proc = run_script("constants_lint.py", "--root", str(self.root))
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn("lib.rs:1:", proc.stdout)
        self.assertEqual(run_script("constants_lint.py", "--help").returncode, 0)


if __name__ == "__main__":
    unittest.main()
