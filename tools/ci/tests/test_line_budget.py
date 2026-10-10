"""Tests for line_budget.py."""

import tempfile
import unittest
from pathlib import Path

import ci_testlib
import line_budget
from ci_common import CiError
from ci_testlib import run_main, run_script

BUDGETS = """
[default]
max_lines = 10

[crates.w5k_small]
max_lines = 3
"""


def rust_lines(n):
    """A source file with exactly n lines of code, padded with blank lines and comments that must not count."""
    body = "\n".join(f"let v{i} = {i};" for i in range(n))
    return f"// header comment\n\n/// doc\n{body}\n\n/* trailing\n   block */\n"


class LineBudgetCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.write("docs/swarm/budgets.toml", BUDGETS)

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def crate(self, name, **files):
        self.write(f"crates/{name}/Cargo.toml", f'[package]\nname = "{name}"\n')
        for rel, text in files.items():
            self.write(f"crates/{name}/src/{rel.replace('__', '/')}", text)

    def run_lint(self, *extra, env=None):
        return run_main(line_budget.main, ["--root", str(self.root), *extra], env)


class Counting(LineBudgetCase):
    def test_counts_code_lines_only(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(4)})
        code, out, _ = self.run_lint()
        self.assertEqual(code, 0, out)
        self.assertRegex(out, r"w5k_a\s+4\s+10\s+40%\s+ok")

    def test_counts_every_file_under_src_recursively(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(2), "a__b__deep.rs": rust_lines(3), "c.rs": rust_lines(1)})
        self.assertRegex(self.run_lint()[1], r"w5k_a\s+6\s+10")

    def test_ignores_non_rust_files_and_files_outside_src(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(1), "data.ron": "(a: 1)\n(b: 2)\n", "notes.md": "x\ny\n"})
        self.write("crates/w5k_a/tests/big.rs", rust_lines(50))
        self.write("crates/w5k_a/build.rs", rust_lines(50))
        self.assertRegex(self.run_lint()[1], r"w5k_a\s+1\s+10")

    def test_code_with_a_trailing_comment_counts_and_comment_markers_in_strings_are_code(self):
        self.crate("w5k_a", **{"lib.rs": 'let a = 1; // trailing\n// only a comment\nlet url = "http://x"; /* c */\n/* a\n b */\n'})
        self.assertRegex(self.run_lint()[1], r"w5k_a\s+2\s+10")

    def test_inline_tests_count(self):
        self.crate("w5k_a", **{"lib.rs": "fn f() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n"})
        self.assertRegex(self.run_lint()[1], r"w5k_a\s+6\s+10")

    def test_crate_without_src_counts_zero(self):
        self.write("crates/w5k_a/Cargo.toml", "[package]\nname = \"w5k_a\"\n")
        self.assertRegex(self.run_lint()[1], r"w5k_a\s+0\s+10\s+0%\s+ok")

    def test_directories_without_a_manifest_are_not_crates(self):
        self.write("crates/stray/src/lib.rs", rust_lines(50))
        self.assertNotIn("stray", self.run_lint()[1])


class Budgets(LineBudgetCase):
    def test_exactly_at_the_budget_passes_one_over_fails(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(10)})
        code, out, _ = self.run_lint()
        self.assertEqual(code, 0, out)
        self.assertRegex(out, r"w5k_a\s+10\s+10\s+100%\s+WARN")
        self.crate("w5k_a", **{"lib.rs": rust_lines(11)})
        code, out, _ = self.run_lint()
        self.assertEqual(code, 1)
        self.assertRegex(out, r"w5k_a\s+11\s+10\s+110%\s+OVER")
        self.assertIn("crate w5k_a has 11 lines of code, over its budget of 10", out)
        self.assertIn("ask ARCH for a decision card", out)

    def test_per_crate_override_beats_the_default(self):
        self.crate("w5k_small", **{"lib.rs": rust_lines(4)})  # default would allow 10, the override allows 3
        self.crate("w5k_other", **{"lib.rs": rust_lines(4)})
        code, out, _ = self.run_lint()
        self.assertEqual(code, 1)
        self.assertRegex(out, r"w5k_small\s+4\s+3\s+133%\s+OVER")
        self.assertRegex(out, r"w5k_other\s+4\s+10\s+40%\s+ok")

    def test_warn_starts_at_ninety_percent(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(8)})
        self.crate("w5k_b", **{"lib.rs": rust_lines(9)})
        out = self.run_lint()[1]
        self.assertRegex(out, r"w5k_a\s+8\s+10\s+80%\s+ok")
        self.assertRegex(out, r"w5k_b\s+9\s+10\s+90%\s+WARN")

    def test_table_has_a_header_total_and_sorted_rows(self):
        self.crate("w5k_b", **{"lib.rs": rust_lines(2)})
        self.crate("w5k_a", **{"lib.rs": rust_lines(3)})
        lines = self.run_lint()[1].splitlines()
        self.assertRegex(lines[0], r"crate\s+lines\s+budget\s+used\s+status")
        self.assertTrue(lines[1].startswith("w5k_a"))
        self.assertTrue(lines[2].startswith("w5k_b"))
        self.assertRegex(lines[3], r"total\s+5")

    def test_budget_for_a_missing_crate_warns_but_does_not_fail(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(1)})
        code, out, err = self.run_lint()
        self.assertEqual(code, 0, out)
        self.assertIn("gives a budget to crate w5k_small, which does not exist", err)

    def test_budgets_option_reads_another_file(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(5)})
        self.write("other/budgets.toml", "[default]\nmax_lines = 4\n")
        code, out, _ = self.run_lint("--budgets", str(self.root / "other/budgets.toml"))
        self.assertEqual(code, 1)
        self.assertIn("over its budget of 4", out)

    def test_github_annotation(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(11)})
        _, out, _ = self.run_lint(env={"GITHUB_ACTIONS": "true"})
        self.assertIn("::error file=crates/w5k_a/Cargo.toml,title=Line budget::crate w5k_a has 11 lines", out)


class SetupProblems(LineBudgetCase):
    def test_missing_budgets_file(self):
        (self.root / "docs/swarm/budgets.toml").unlink()
        self.crate("w5k_a", **{"lib.rs": ""})
        code, _, err = self.run_lint()
        self.assertEqual(code, 2)
        self.assertIn("cannot read", err)

    def test_budgets_that_are_not_positive_whole_numbers(self):
        self.crate("w5k_a", **{"lib.rs": ""})
        for text in ("[default]\nmax_lines = 0\n", "[default]\nmax_lines = 1.5\n", '[default]\nmax_lines = "8000"\n', "",
                     "[default]\nmax_lines = 5\n[crates.w5k_a]\n", "[default]\nmax_lines = 5\n[crates.w5k_a]\nmax_lines = -1\n"):
            with self.subTest(text=text):
                self.write("docs/swarm/budgets.toml", text)
                code, _, err = self.run_lint()
                self.assertEqual(code, 2)
                self.assertIn("positive whole number", err)

    def test_wrong_root(self):
        code, _, err = run_main(line_budget.main, ["--root", str(self.root / "nowhere"), "--budgets", str(self.root / "docs/swarm/budgets.toml")])
        self.assertEqual(code, 2)
        self.assertIn("does not exist", err)

    def test_runs_under_python_dash_I(self):
        self.crate("w5k_a", **{"lib.rs": rust_lines(11)})
        proc = run_script("line_budget.py", "--root", str(self.root))
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertEqual(run_script("line_budget.py", "--help").returncode, 0)


class TheRealBudgetsFile(unittest.TestCase):
    """docs/swarm/budgets.toml, when it is available, has the numbers the plan asks for."""

    def test_values(self):
        path = ci_testlib.ROOT / "docs/swarm/budgets.toml"
        if not path.is_file():
            import os

            path = Path(os.environ.get("W5K_BUDGETS", "/nonexistent"))
        if not path.is_file():
            self.skipTest("docs/swarm/budgets.toml not available (set W5K_BUDGETS)")
        default, overrides = line_budget.read_budgets(path)
        self.assertEqual(default, 8000)
        self.assertEqual(overrides, {"w5k_math": 2500, "w5k_contract": 7000, "w5k_sim": 5000, "w5k_tools": 9500})


if __name__ == "__main__":
    unittest.main()
