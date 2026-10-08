"""Tests for deps_lint.py."""

import tempfile
import unittest
from pathlib import Path

import ci_testlib  # noqa: F401
import deps_lint
from ci_testlib import run_main, run_script

ROOT_MANIFEST = """
[workspace]
members = ["crates/*"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
libm = "0.2"
ron = "0.12"
w5k_math = { path = "crates/w5k_math" }
"""


class DepsLintCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.write("Cargo.toml", ROOT_MANIFEST)

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def crate(self, name, body):
        self.write(f"crates/{name}/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.1.0"\n\n{body}\n')

    def run_lint(self, env=None):
        return run_main(deps_lint.main, ["--root", str(self.root)], env)


class Accepted(DepsLintCase):
    def test_dotted_form(self):
        self.crate("w5k_a", "[dependencies]\nserde.workspace = true\nlibm.workspace = true\nw5k_math.workspace = true")
        code, out, _ = self.run_lint()
        self.assertEqual(code, 0, out)
        self.assertIn("OK (1 manifest(s), 4 workspace dependencies)", out)

    def test_inline_table_form_and_features(self):
        self.crate("w5k_a", '[dependencies]\nserde = { workspace = true, features = ["rc"] }\nlibm = { workspace = true, optional = true }')
        self.assertEqual(self.run_lint()[0], 0)

    def test_dev_and_build_dependencies(self):
        self.crate("w5k_a", "[dev-dependencies]\nron.workspace = true\n\n[build-dependencies]\nlibm.workspace = true")
        self.assertEqual(self.run_lint()[0], 0)

    def test_sub_table_form(self):
        self.crate("w5k_a", "[dependencies.serde]\nworkspace = true")
        self.assertEqual(self.run_lint()[0], 0)

    def test_no_dependencies_at_all(self):
        self.crate("w5k_a", "")
        self.assertEqual(self.run_lint()[0], 0)

    def test_default_features_next_to_workspace(self):
        self.crate("w5k_a", "[dependencies]\nserde = { workspace = true, default-features = false }")
        self.assertEqual(self.run_lint()[0], 0)


class Rejected(DepsLintCase):
    def assert_fails(self, body, *needles):
        self.crate("w5k_a", body)
        code, out, _ = self.run_lint()
        self.assertEqual(code, 1, out)
        self.assertIn("new dependencies need an approved decision card; ask ARCH", out)
        for needle in needles:
            self.assertIn(needle, out)
        return out

    def test_own_version_string(self):
        self.assert_fails('[dependencies]\nrand = "0.8"', "crates/w5k_a/Cargo.toml:6: [dependencies] `rand` names its own version")

    def test_own_version_even_for_a_name_the_workspace_has(self):
        self.assert_fails('[dependencies]\nserde = "1"', "`serde` names its own version", "serde.workspace = true")

    def test_inline_table_with_version(self):
        self.assert_fails('[dev-dependencies]\nproptest = { version = "1", features = ["x"] }', "[dev-dependencies] `proptest`")

    def test_path_and_git_dependencies(self):
        out = self.assert_fails(
            '[dependencies]\nw5k_other = { path = "../w5k_other" }\nfoo = { git = "https://example.com/foo" }',
            "`w5k_other`", "`foo`")
        self.assertEqual(out.count("names its own version/path/git"), 2)

    def test_workspace_true_but_name_not_in_the_workspace(self):
        self.assert_fails("[dependencies]\nrand.workspace = true", "`rand` is not listed in the root [workspace.dependencies]")

    def test_build_dependencies_are_checked(self):
        self.assert_fails('[build-dependencies]\ncc = "1"', "[build-dependencies] `cc`")

    def test_target_specific_tables_are_checked(self):
        self.assert_fails("[target.'cfg(windows)'.dependencies]\nwinapi = \"0.3\"", "`winapi`")
        self.assert_fails("[target.'cfg(unix)'.dev-dependencies]\nlibc.workspace = true", "`libc` is not listed")

    def test_deprecated_underscore_table_names_are_checked(self):
        self.assert_fails('[dev_dependencies]\nrand = "0.8"', "`rand`")

    def test_a_renamed_package_next_to_workspace_true(self):
        self.assert_fails('[dependencies]\nserde = { workspace = true, package = "evil" }', "sets package next to `workspace = true`")

    def test_workspace_false(self):
        self.assert_fails("[dependencies]\nserde = { workspace = false }", "`serde`")

    def test_each_crate_is_checked(self):
        self.crate("w5k_a", "[dependencies]\nserde.workspace = true")
        self.crate("w5k_b", '[dependencies]\nrand = "0.8"')
        self.crate("w5k_c", '[dev-dependencies]\nproptest = "1"')
        code, out, _ = self.run_lint()
        self.assertEqual(code, 1)
        self.assertIn("w5k_b/Cargo.toml", out)
        self.assertIn("w5k_c/Cargo.toml", out)
        self.assertNotIn("w5k_a/Cargo.toml:", out)
        self.assertIn("2 problem(s) in 2 manifest(s)", out)

    def test_github_annotation(self):
        self.crate("w5k_a", '[dependencies]\nrand = "0.8"')
        _, out, _ = self.run_lint({"GITHUB_ACTIONS": "true"})
        self.assertIn("::error file=crates/w5k_a/Cargo.toml,line=6,title=Dependency lint::[dependencies] `rand`", out)


class SetupProblems(DepsLintCase):
    def test_missing_root_manifest(self):
        (self.root / "Cargo.toml").unlink()
        code, _, err = self.run_lint()
        self.assertEqual(code, 2)
        self.assertIn("does not exist", err)

    def test_broken_member_manifest(self):
        self.write("crates/w5k_a/Cargo.toml", "[package\nname = ")
        code, _, err = self.run_lint()
        self.assertEqual(code, 2)
        self.assertIn("not valid TOML", err)

    def test_workspace_without_a_dependencies_table_rejects_every_dependency(self):
        self.write("Cargo.toml", '[workspace]\nmembers = ["crates/*"]\n')
        self.crate("w5k_a", "[dependencies]\nserde.workspace = true")
        code, out, _ = self.run_lint()
        self.assertEqual(code, 1)
        self.assertIn("is not listed", out)

    def test_runs_under_python_dash_I(self):
        self.crate("w5k_a", '[dependencies]\nrand = "0.8"')
        proc = run_script("deps_lint.py", "--root", str(self.root))
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertEqual(run_script("deps_lint.py", "--help").returncode, 0)


if __name__ == "__main__":
    unittest.main()
