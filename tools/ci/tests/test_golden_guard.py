"""Tests for golden_guard.py."""

import tempfile
import unittest
from pathlib import Path

import ci_testlib  # noqa: F401
import golden_guard
from ci_testlib import TempRepo, run_main, run_script

GOLDEN = "crates/w5k_sim/tests/golden/first_light.json"


class IsGolden(unittest.TestCase):
    def test_paths_under_tests_golden(self):
        for path in (GOLDEN, "crates/a/tests/golden/deep/er/x.bin", "tests/golden/top.json", "a/tests/golden/x"):
            self.assertTrue(golden_guard.is_golden(path), path)

    def test_golden_named_files_anywhere(self):
        for path in ("content/fixtures/hmmwv.golden.ron", "x.golden.json", "crates/a/tests/data/run.golden.bin"):
            self.assertTrue(golden_guard.is_golden(path), path)

    def test_lookalikes_are_not_goldens(self):
        for path in ("crates/a/tests/goldens/x.json", "crates/a/tests/golden.rs", "crates/a/src/golden/x.rs",
                     "docs/golden.md", "content/fixtures/hmmwv.ron", "crates/a/tests/mygolden/x", "golden.json"):
            self.assertFalse(golden_guard.is_golden(path), path)


class GoldenReason(unittest.TestCase):
    def test_a_real_reason(self):
        self.assertEqual(golden_guard.golden_reason("Fixes the damper.\n\nGolden-Change: damper rate now from the dossier\n"),
                         "damper rate now from the dossier")

    def test_key_is_case_insensitive_and_whitespace_tolerant(self):
        self.assertEqual(golden_guard.golden_reason("  golden-change:    spring rate fix   \n"), "spring rate fix")

    def test_must_start_the_line(self):
        self.assertIsNone(golden_guard.golden_reason("see Golden-Change: reason\n- not Golden-Change: reason"))

    def test_empty_or_placeholder_reasons_are_refused(self):
        for body in ("Golden-Change:", "Golden-Change:   \n", "Golden-Change: <reason>", "Golden-Change: <why the numbers changed>",
                     "Golden-Change: TODO", "Golden-Change: tbd", "Golden-Change: none", "Golden-Change: n/a", "Golden-Change: ..."):
            self.assertIsNone(golden_guard.golden_reason(body), body)

    def test_html_comments_do_not_count(self):
        self.assertIsNone(golden_guard.golden_reason("<!--\nGolden-Change: hidden in a template comment\n-->\n"))
        self.assertIsNone(golden_guard.golden_reason("<!-- Golden-Change: x -->"))
        self.assertEqual(golden_guard.golden_reason("<!-- hint -->\nGolden-Change: real reason"), "real reason")

    def test_first_usable_line_wins(self):
        self.assertEqual(golden_guard.golden_reason("Golden-Change: <reason>\nGolden-Change: the real one"), "the real one")

    def test_crlf_bodies(self):
        # GitHub stores a description typed in the web editor with \r\n line endings.
        self.assertEqual(golden_guard.golden_reason("Title\r\n\r\nGolden-Change: windows line endings\r\n"), "windows line endings")
        self.assertIsNone(golden_guard.golden_reason("Golden-Change: <reason>\r\n"))
        self.assertIsNone(golden_guard.golden_reason("Golden-Change: TODO\r\n"))
        self.assertEqual(golden_guard.golden_reason("Golden-Change: old mac line endings\rnext"), "old mac line endings")


class GoldenGuardCase(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.repo.write(GOLDEN, '{"hash": "aaaa"}\n')
        self.repo.write("crates/w5k_sim/src/lib.rs", "// sim\n")
        self.repo.commit("base")
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)

    def pr(self, files, body="", env=None, body_exists=True):
        self.repo.checkout("main")
        self.repo.branch(f"topic-{len(self.repo.git('branch').splitlines())}")
        for path, content in files.items():
            if content is None:
                self.repo.remove(path)
            else:
                self.repo.write(path, content)
        self.repo.commit("work")
        body_file = Path(self._tmp.name) / "body.txt"
        if body_exists:
            body_file.write_text(body)
        argv = ["--base", "main", "--head", "HEAD", "--body-file", str(body_file), "--repo", str(self.repo.path)]
        return run_main(golden_guard.main, argv, env)


class Decisions(GoldenGuardCase):
    def test_no_golden_change_needs_no_reason(self):
        code, out, _ = self.pr({"crates/w5k_sim/src/lib.rs": "// changed\n"})
        self.assertEqual(code, 0)
        self.assertIn("no golden file changed", out)

    def test_changed_golden_without_a_reason_fails(self):
        code, out, _ = self.pr({GOLDEN: '{"hash": "bbbb"}\n'}, body="Tuned the damper.")
        self.assertEqual(code, 1)
        self.assertIn(GOLDEN, out)
        self.assertIn("goldens are regenerated only deliberately, never to make a failing test pass", out)
        self.assertIn("Golden-Change: <", out)

    def test_changed_golden_with_a_reason_passes(self):
        code, out, _ = self.pr({GOLDEN: '{"hash": "bbbb"}\n'}, body="Tuned.\n\nGolden-Change: damper ratio 0.3 to 0.35 per dossier\n")
        self.assertEqual(code, 0, out)
        self.assertIn("damper ratio 0.3 to 0.35 per dossier", out)

    def test_empty_reason_fails(self):
        code, _, _ = self.pr({GOLDEN: '{"hash": "bbbb"}\n'}, body="Golden-Change:\n")
        self.assertEqual(code, 1)

    def test_added_deleted_and_renamed_goldens_all_count(self):
        self.assertEqual(self.pr({"crates/w5k_sim/tests/golden/new.json": "{}\n"})[0], 1)
        self.assertEqual(self.pr({GOLDEN: None})[0], 1)
        self.repo.checkout("main")
        self.repo.branch("mover")
        self.repo.move(GOLDEN, "crates/w5k_sim/tests/data/first_light.json")
        self.repo.commit("move out of the golden folder")
        code, out, _ = run_main(golden_guard.main, ["--base", "main", "--head", "HEAD", "--body-file", "/nonexistent", "--repo", str(self.repo.path)])
        self.assertEqual(code, 1, out)  # the old path is a golden that disappeared

    def test_rename_into_the_golden_folder_counts(self):
        self.repo.write("crates/w5k_sim/tests/data/run.json", "some content long enough to be seen as a rename\n" * 5)
        self.repo.commit("base2")
        self.repo.checkout("main")
        self.repo.branch("into")
        self.repo.move("crates/w5k_sim/tests/data/run.json", "crates/w5k_sim/tests/golden/run.json")
        self.repo.commit("move into golden folder")
        code, out, _ = run_main(golden_guard.main, ["--base", "main", "--head", "HEAD", "--body-file", "/nonexistent", "--repo", str(self.repo.path)])
        self.assertEqual(code, 1, out)
        self.assertIn("crates/w5k_sim/tests/golden/run.json", out)

    def test_golden_named_files_in_fixtures(self):
        self.assertEqual(self.pr({"content/fixtures/hmmwv.golden.ron": "()\n"})[0], 1)
        self.assertEqual(self.pr({"content/fixtures/hmmwv.ron": "()\n"})[0], 0)

    def test_every_golden_is_listed(self):
        code, out, _ = self.pr({GOLDEN: "{}\n", "crates/a/tests/golden/b.json": "{}\n"})
        self.assertEqual(code, 1)
        self.assertIn("changes 2 golden file(s)", out)

    def test_missing_body_file_counts_as_an_empty_description(self):
        code, _, _ = self.pr({GOLDEN: "{}\n"}, body_exists=False)
        self.assertEqual(code, 1)

    def test_an_unneeded_reason_is_harmless(self):
        code, _, _ = self.pr({"crates/w5k_sim/src/lib.rs": "// c\n"}, body="Golden-Change: not needed\n")
        self.assertEqual(code, 0)


class Output(GoldenGuardCase):
    def test_github_annotation_names_the_file(self):
        _, out, _ = self.pr({GOLDEN: "{}\n"}, env={"GITHUB_ACTIONS": "true"})
        self.assertIn(f"::error file={GOLDEN},title=Golden guard::golden file changed without a Golden-Change line", out)

    def test_bad_revision_is_a_setup_error(self):
        code, _, err = run_main(golden_guard.main, ["--base", "nope", "--head", "HEAD", "--body-file", "x", "--repo", str(self.repo.path)])
        self.assertEqual(code, 2)
        self.assertIn("setup problem", err)

    def test_runs_under_python_dash_I(self):
        self.repo.branch("x")
        self.repo.write(GOLDEN, "{}\n")
        self.repo.commit("w")
        body = Path(self._tmp.name) / "b.txt"
        body.write_text("Golden-Change: intended\n")
        proc = run_script("golden_guard.py", "--base", "main", "--head", "HEAD", "--body-file", str(body), "--repo", str(self.repo.path))
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        body.write_text("nothing\n")
        proc = run_script("golden_guard.py", "--base", "main", "--head", "HEAD", "--body-file", str(body), "--repo", str(self.repo.path))
        self.assertEqual(proc.returncode, 1)
        self.assertEqual(run_script("golden_guard.py", "--help").returncode, 0)


if __name__ == "__main__":
    unittest.main()
