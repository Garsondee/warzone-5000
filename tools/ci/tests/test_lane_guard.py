"""Tests for lane_guard.py. Each test builds a throw-away git repository, makes a branch, and runs the guard on it.

The real docs/swarm/ownership.toml is checked too when it is available (set W5K_OWNERSHIP=/path/to/ownership.toml to
point at it when running from a staging tree that does not contain it)."""

import os
import re
import tempfile
import unittest
from pathlib import Path

import ci_testlib
import lane_guard
from ci_common import Change
from ci_testlib import TempRepo, run_main, run_script

OWNERSHIP = """
[unrestricted]
heads = ["claude/sharp-babbage-d702f7", "integration", "arch/*"]

[common]
paths = [
  "docs/swarm/status/{lane}.md",
  "docs/swarm/requests/{lane}-*.md",
  "docs/theory/{lane}.md",
  "docs/lanes/{lane}/**",
  "spikes/{lane}/**",
  "Cargo.lock",
]

[ccr]
paths = ["crates/w5k_contract/**", "docs/architecture/CONTRACTS.md"]

[lanes.arch]
paths = ["crates/w5k_contract/**", "crates/w5k_math/**", "Cargo.toml", ".github/**", "tools/ci/**", "docs/swarm/**",
         "docs/architecture/**"]

[lanes.chassis]
paths = ["crates/w5k_chassis/**", "content/physics/chassis/**", "crates/w5k_tools/src/cmd/chassis.rs"]

[lanes.drive]
paths = ["crates/w5k_drive/**", "content/physics/drive/**", "crates/w5k_tools/src/cmd/drive.rs"]
"""

PROTECTED_TABLE = """
[protected]
paths = [".github/**", ".claude/**", "tools/ci/**", "docs/swarm/ownership.toml", "clippy.toml", "docs/swarm/RULES.md",
         "crates/w5k_chassis/src/frozen.rs"]
"""
OWNERSHIP_WITH_PROTECTED = OWNERSHIP.replace("[lanes.arch]", PROTECTED_TABLE + "\n[lanes.arch]")
assert OWNERSHIP_WITH_PROTECTED != OWNERSHIP

BASE_FILES = {
    "docs/swarm/ownership.toml": OWNERSHIP,
    "crates/w5k_chassis/src/lib.rs": "pub fn chassis() {}\n",
    "crates/w5k_drive/src/lib.rs": "pub fn drive() {}\n" * 20,
    "crates/w5k_contract/src/lib.rs": "pub fn contract() {}\n",
    "crates/w5k_math/src/lib.rs": "pub fn math() {}\n",
    "Cargo.lock": "# lock\n",
}


class LaneGuardCase(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        for path, content in BASE_FILES.items():
            self.repo.write(path, content)
        self.repo.commit("base")

    def pr(self, branch, files, title="Some work", env=None, extra=(), claimed=None):
        """Make `branch` from main with these file changes (content, or None to delete) and run the guard on it.

        `claimed` is the branch name passed to --branch when it differs from the git branch (git refuses to create
        names such as `lane/chassis` next to `lane/chassis/x`, or names with spaces; the guard only sees a string)."""
        self.repo.checkout("main")
        self.repo.branch(branch)
        for path, content in files.items():
            if content is None:
                self.repo.remove(path)
            else:
                self.repo.write(path, content)
        self.repo.commit("work")
        name = claimed if claimed is not None else branch
        argv = ["--base", "main", "--head", branch, "--branch", name, "--title", title, "--repo", str(self.repo.path)]
        return run_main(lane_guard.main, argv + list(extra), env)

    def outside(self, name, content):
        """A file outside the repository (so it is not part of the pull request)."""
        tmp = tempfile.TemporaryDirectory(prefix="w5k-ci-outside-")
        self.addCleanup(tmp.cleanup)
        path = Path(tmp.name) / name
        path.write_text(content)
        return path


class OwnPathsAndCommonPaths(LaneGuardCase):
    def test_own_paths_pass(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {
                "crates/w5k_chassis/src/lib.rs": "pub fn chassis() { /* changed */ }\n",
                "crates/w5k_chassis/src/deep/er/mod.rs": "// new\n",
                "content/physics/chassis/tyre.ron": "()\n",
                "crates/w5k_tools/src/cmd/chassis.rs": "// cmd\n",
            },
        )
        self.assertEqual(code, 0, out)
        self.assertIn("PASS", out)

    def test_common_paths_pass(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {
                "docs/swarm/status/chassis.md": "done: nothing\n",  # the status file
                "docs/theory/chassis.md": "# theory note\n",  # the theory note
                "Cargo.lock": "# lock, updated\n",  # the shared lock file
                "docs/swarm/requests/chassis-bump-strip.md": "# request\n",
                "docs/lanes/chassis/media/shot.png": b"\x89PNG",
                "spikes/chassis/s1/main.rs": "fn main() {}\n",
            },
        )
        self.assertEqual(code, 0, out)

    def test_other_lanes_common_paths_are_not_allowed(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {
                "docs/swarm/status/drive.md": "x\n",
                "docs/theory/drive.md": "x\n",
                "docs/swarm/requests/drive-thing.md": "x\n",
                "docs/lanes/drive/media/a.png": b"x",
            },
        )
        self.assertEqual(code, 1)
        for path in ("docs/swarm/status/drive.md", "docs/theory/drive.md", "docs/swarm/requests/drive-thing.md"):
            self.assertIn(path, out)
        self.assertIn("belongs to lane drive", out)

    def test_empty_pr_passes(self):
        self.repo.branch("lane/chassis/empty")
        code, out, _ = run_main(
            lane_guard.main,
            ["--base", "main", "--head", "HEAD", "--branch", "lane/chassis/empty", "--repo", str(self.repo.path)],
        )
        self.assertEqual(code, 0, out)
        self.assertIn("0 path(s)", out)

    def test_paths_with_spaces_and_unicode(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_chassis/src/café notes.rs": "x\n"})
        self.assertEqual(code, 0, out)


class OtherLanePaths(LaneGuardCase):
    def test_other_lane_path_fails_with_an_actionable_message(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_drive/src/lib.rs": "pub fn drive() { /* hack */ }\n"})
        self.assertEqual(code, 1)
        self.assertIn("your PR touches crates/w5k_drive/src/lib.rs which belongs to lane drive", out)
        self.assertIn("file an interface request in docs/swarm/requests/chassis-x.md instead", out)
        self.assertIn("FAIL", out)

    def test_only_the_bad_files_are_reported(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {"crates/w5k_chassis/src/lib.rs": "// fine\n", "crates/w5k_drive/src/lib.rs": "// not fine\n"},
        )
        self.assertEqual(code, 1)
        self.assertIn("1 path(s) are outside lane 'chassis'", out)
        self.assertNotIn("- crates/w5k_chassis/src/lib.rs", out)

    def test_the_suggested_request_file_is_itself_allowed(self):
        _, out, _ = self.pr("lane/chassis/needs-a-hook", {"crates/w5k_drive/src/lib.rs": "// hack\n"})
        suggested = re.search(r"interface request in (\S+) instead", out).group(1)
        self.assertEqual(suggested, "docs/swarm/requests/chassis-needs-a-hook.md")
        code, out, _ = self.pr("lane/chassis/needs-a-hook-2", {suggested: "# please add a hook\n"})
        self.assertEqual(code, 0, out)

    def test_topic_with_slashes_still_gives_a_valid_request_name(self):
        _, out, _ = self.pr("work", {"crates/w5k_drive/src/lib.rs": "// hack\n"}, claimed="lane/chassis/a/b c")
        suggested = re.search(r"interface request in (\S+) instead", out).group(1)
        self.assertEqual(suggested, "docs/swarm/requests/chassis-a-b-c.md")
        self.assertTrue(lane_guard.matches_any(["docs/swarm/requests/chassis-*.md"], suggested))

    def test_path_no_lane_owns(self):
        code, out, _ = self.pr("lane/chassis/x", {"mystery/file.txt": "x\n"})
        self.assertEqual(code, 1)
        self.assertIn("which no lane owns", out)

    def test_contract_path_without_ccr_is_refused_with_a_hint(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_contract/src/lib.rs": "pub fn contract() { /*x*/ }\n"})
        self.assertEqual(code, 1)
        self.assertIn("belongs to lane arch", out)
        self.assertIn("`CCR:`", out)

    def test_deleting_another_lanes_file_is_refused(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_drive/src/lib.rs": None})
        self.assertEqual(code, 1)
        self.assertIn("crates/w5k_drive/src/lib.rs", out)

    def test_cannot_edit_the_guard_or_the_rules(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {"tools/ci/lane_guard.py": "# weakened\n", ".github/workflows/pr.yml": "# x\n", "docs/swarm/ownership.toml": "# x\n"},
        )
        self.assertEqual(code, 1)
        self.assertEqual(out.count("belongs to lane arch"), 3)

    def test_single_star_in_a_common_pattern_does_not_cross_directories(self):
        code, _, _ = self.pr("lane/chassis/x", {"docs/swarm/requests/chassis-a/b.md": "x\n"})
        self.assertEqual(code, 1)

    def test_double_star_reaches_deep_but_not_a_sibling_prefix(self):
        code, out, _ = self.pr("lane/chassis/x", {"content/physics/chassis/a/b/c/d.ron": "()\n"})
        self.assertEqual(code, 0, out)
        code, out, _ = self.pr("lane/chassis/y", {"content/physics/chassis_extra/d.ron": "()\n"})
        self.assertEqual(code, 1, out)


class Renames(LaneGuardCase):
    def setUp(self):
        super().setUp()
        self.long_text = "a line of text that is long enough for git to see the rename\n" * 10
        self.repo.write("crates/w5k_drive/src/engine.rs", self.long_text)
        self.repo.write("crates/w5k_chassis/src/hull.rs", self.long_text + "// chassis\n")
        self.repo.commit("more base")

    def rename_pr(self, branch, old, new):
        self.repo.checkout("main")
        self.repo.branch(branch)
        self.repo.move(old, new)
        self.repo.commit("move")
        return run_main(
            lane_guard.main,
            ["--base", "main", "--head", branch, "--branch", branch, "--title", "move", "--repo", str(self.repo.path)],
        )

    def test_rename_inside_the_lane_passes(self):
        code, out, _ = self.rename_pr(
            "lane/chassis/a", "crates/w5k_chassis/src/hull.rs", "crates/w5k_chassis/src/body/hull.rs"
        )
        self.assertEqual(code, 0, out)

    def test_moving_a_file_out_of_the_lane_checks_the_new_path(self):
        code, out, _ = self.rename_pr("lane/chassis/b", "crates/w5k_chassis/src/hull.rs", "crates/w5k_drive/src/hull.rs")
        self.assertEqual(code, 1, out)
        self.assertIn("crates/w5k_drive/src/hull.rs", out)
        self.assertNotIn("- crates/w5k_chassis/src/hull.rs", out)

    def test_moving_another_lanes_file_into_the_lane_checks_the_old_path(self):
        code, out, _ = self.rename_pr("lane/chassis/c", "crates/w5k_drive/src/engine.rs", "crates/w5k_chassis/src/engine.rs")
        self.assertEqual(code, 1, out)
        self.assertIn("- crates/w5k_drive/src/engine.rs", out)
        self.assertIn("belongs to lane drive", out)
        self.assertNotIn("- crates/w5k_chassis/src/engine.rs", out)

    def test_rename_is_detected_as_a_rename(self):
        self.repo.checkout("main")
        self.repo.branch("probe")
        self.repo.move("crates/w5k_drive/src/engine.rs", "crates/w5k_chassis/src/engine.rs")
        self.repo.commit("move")
        changes = lane_guard.diff_changes(str(self.repo.path), "main", "probe")
        self.assertEqual([(c.status, c.old_path, c.path) for c in changes],
                         [("R", "crates/w5k_drive/src/engine.rs", "crates/w5k_chassis/src/engine.rs")])


class ContractChangeRequests(LaneGuardCase):
    def test_ccr_title_allows_contract_and_edits_anywhere_with_a_loud_notice(self):
        code, out, _ = self.pr(
            "lane/chassis/ccr",
            {
                "crates/w5k_contract/src/lib.rs": "pub fn contract() { /* new field */ }\n",
                "crates/w5k_drive/src/lib.rs": "pub fn drive() { /* migrated */ }\n",
                "crates/w5k_chassis/src/lib.rs": "// own file\n",
            },
            title="CCR: add a damper field",
        )
        self.assertEqual(code, 0, out)
        self.assertIn("ARCH MUST REVIEW EVERY LINE", out)
        self.assertIn("crates/w5k_contract/src/lib.rs   [contract path]", out)
        self.assertIn("crates/w5k_drive/src/lib.rs   [migration edit in lane drive]", out)
        self.assertNotIn("crates/w5k_chassis/src/lib.rs", out.split("outside lane")[1])
        self.assertIn("2 path(s) are outside", out)

    def test_ccr_must_be_at_the_start_of_the_title(self):
        code, out, _ = self.pr(
            "lane/chassis/x", {"crates/w5k_contract/src/lib.rs": "// x\n"}, title="Fix CCR: something"
        )
        self.assertEqual(code, 1, out)

    def test_ccr_prefix_survives_leading_whitespace_but_not_a_different_case(self):
        code, _, _ = self.pr("lane/chassis/a", {"crates/w5k_contract/src/lib.rs": "// a\n"}, title="  CCR: ok")
        self.assertEqual(code, 0)
        code, _, _ = self.pr("lane/chassis/b", {"crates/w5k_contract/src/lib.rs": "// b\n"}, title="ccr: lower case")
        self.assertEqual(code, 1)

    def test_ccr_without_a_contract_change_warns(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_drive/src/lib.rs": "// x\n"}, title="CCR: sneaky")
        self.assertEqual(code, 0)
        self.assertIn("no contract path changed", out)

    def test_ccr_has_no_effect_on_a_lane_that_stays_home(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_chassis/src/lib.rs": "// x\n"}, title="CCR: nothing outside")
        self.assertEqual(code, 0, out)
        self.assertIn("0 path(s) are outside", out)


class Heads(LaneGuardCase):
    def test_unrestricted_heads_may_change_anything(self):
        for branch in ("integration", "arch/launch-kit", "arch/deep/er/name", "claude/sharp-babbage-d702f7"):
            with self.subTest(branch=branch):
                code, out, _ = self.pr(
                    branch,
                    {"crates/w5k_drive/src/lib.rs": f"// by {branch}\n", ".github/workflows/pr.yml": "# x\n"},
                )
                self.assertEqual(code, 0, out)
                self.assertIn("unrestricted pattern", out)

    def test_unknown_heads_fail_with_an_explanation(self):
        names = ("feature/foo", "main-fix", "claude/other-session", "lane", "lane/chassis", "lane/chassis/", "archive/x")
        for i, branch in enumerate(names):
            with self.subTest(branch=branch):
                code, out, _ = self.pr(f"probe-{i}", {"docs/theory/chassis.md": "x\n"}, claimed=branch)
                self.assertEqual(code, 1, out)
                self.assertIn("lane/<lane>/<topic>", out)
                self.assertIn("[unrestricted].heads", out)

    def test_unknown_lane_name_fails_and_lists_the_real_lanes(self):
        code, out, _ = self.pr("lane/nope/x", {"docs/theory/chassis.md": "x\n"})
        self.assertEqual(code, 1)
        self.assertIn("unknown lane 'nope'", out)
        self.assertIn("arch, chassis, drive", out)

    def test_lane_names_are_case_sensitive(self):
        code, out, _ = self.pr("lane/Chassis/x", {"docs/theory/chassis.md": "x\n"})
        self.assertEqual(code, 1)
        self.assertIn("unknown lane 'Chassis'", out)

    def test_refs_heads_prefix_is_accepted(self):
        self.repo.branch("lane/chassis/x")
        self.repo.write("crates/w5k_chassis/src/lib.rs", "// x\n")
        self.repo.commit("w")
        code, out, _ = run_main(
            lane_guard.main,
            ["--base", "main", "--head", "HEAD", "--branch", "refs/heads/lane/chassis/x", "--repo", str(self.repo.path)],
        )
        self.assertEqual(code, 0, out)


class RulesAreTrusted(LaneGuardCase):
    def test_a_pr_cannot_grant_itself_rights_by_editing_ownership_toml(self):
        tampered = OWNERSHIP.replace(
            'paths = ["crates/w5k_chassis/**"', 'paths = ["crates/w5k_drive/**", "docs/swarm/**", "crates/w5k_chassis/**"'
        )
        self.assertNotEqual(tampered, OWNERSHIP)
        code, out, _ = self.pr(
            "lane/chassis/x",
            {"docs/swarm/ownership.toml": tampered, "crates/w5k_drive/src/lib.rs": "// stolen\n"},
        )
        self.assertEqual(code, 1, out)
        self.assertIn("crates/w5k_drive/src/lib.rs", out)
        self.assertIn("docs/swarm/ownership.toml", out)

    def test_ownership_option_reads_a_file_from_disk(self):
        alt = self.outside("alt.toml", OWNERSHIP.replace("[lanes.drive]", "[lanes.rover]"))
        code, out, _ = self.pr("lane/rover/x", {"docs/theory/rover.md": "x\n"}, extra=["--ownership", str(alt)])
        self.assertEqual(code, 0, out)
        code, out, _ = self.pr("lane/drive/x", {"docs/theory/drive.md": "x\n"}, extra=["--ownership", str(alt)])
        self.assertEqual(code, 1, out)

    def test_falls_back_to_the_working_tree_when_base_has_no_ownership_file(self):
        repo = TempRepo()
        self.addCleanup(repo.cleanup)
        repo.write("README.md", "hi\n")
        repo.commit("base without rules")
        repo.branch("lane/chassis/x")
        repo.write("docs/swarm/ownership.toml", OWNERSHIP)
        repo.write("crates/w5k_chassis/src/lib.rs", "// x\n")
        repo.commit("bootstrap")
        code, out, _ = run_main(
            lane_guard.main,
            ["--base", "main", "--head", "HEAD", "--branch", "lane/chassis/x", "--repo", str(repo.path)],
        )
        # The bootstrap PR itself edits docs/swarm/ownership.toml, which belongs to ARCH, so it is refused: correct.
        self.assertEqual(code, 1, out)
        self.assertIn("not in main yet", out)

    def test_missing_rules_is_a_setup_error(self):
        repo = TempRepo()
        self.addCleanup(repo.cleanup)
        repo.write("a.txt", "a\n")
        repo.commit("base")
        repo.branch("lane/chassis/x")
        repo.write("b.txt", "b\n")
        repo.commit("work")
        code, _, err = run_main(
            lane_guard.main, ["--base", "main", "--head", "HEAD", "--branch", "lane/chassis/x", "--repo", str(repo.path)]
        )
        self.assertEqual(code, 2)
        self.assertIn("setup problem", err)

    def test_broken_toml_is_a_setup_error(self):
        bad = self.outside("bad.toml", "this is = = not toml")
        code, _, err = self.pr("lane/chassis/x", {"docs/theory/chassis.md": "x\n"}, extra=["--ownership", str(bad)])
        self.assertEqual(code, 2)
        self.assertIn("not valid TOML", err)

    def test_unknown_base_revision_is_a_setup_error(self):
        code, _, err = run_main(
            lane_guard.main,
            ["--base", "nope", "--head", "main", "--branch", "lane/chassis/x", "--repo", str(self.repo.path),
             "--ownership", str(self.outside("o.toml", OWNERSHIP))],
        )
        self.assertEqual(code, 2)
        self.assertIn("setup problem", err)


class Output(LaneGuardCase):
    def test_github_annotations_for_each_violation(self):
        code, out, _ = self.pr(
            "lane/chassis/x",
            {"crates/w5k_drive/src/lib.rs": "// x\n", "crates/w5k_math/src/lib.rs": "// y\n"},
            env={"GITHUB_ACTIONS": "true"},
        )
        self.assertEqual(code, 1)
        errors = [line for line in out.splitlines() if line.startswith("::error ")]
        self.assertEqual(len(errors), 2)
        self.assertTrue(errors[0].startswith("::error file=crates/w5k_drive/src/lib.rs,title=Lane guard::your PR touches"))
        self.assertIn("belongs to lane drive", errors[0])

    def test_no_annotations_outside_github(self):
        _, out, _ = self.pr("lane/chassis/x", {"crates/w5k_drive/src/lib.rs": "// x\n"})
        self.assertNotIn("::error", out)

    def test_rejected_head_annotation_has_no_file(self):
        _, out, _ = self.pr("feature/x", {"docs/theory/chassis.md": "x\n"}, env={"GITHUB_ACTIONS": "true"})
        self.assertTrue(any(line.startswith("::error title=Lane guard::") for line in out.splitlines()), out)

    def test_ccr_gets_a_warning_annotation(self):
        _, out, _ = self.pr(
            "lane/chassis/x", {"crates/w5k_contract/src/lib.rs": "// c\n"}, title="CCR: x", env={"GITHUB_ACTIONS": "true"}
        )
        self.assertIn("::warning title=CCR pull request::CCR pull request: ARCH must review every line", out)

    def test_hostile_file_and_title_text_cannot_start_a_workflow_command(self):
        # A file name containing a newline followed by '::error' must not produce a line that starts with '::'.
        code, out, _ = self.pr(
            "lane/chassis/x", {"crates/w5k_drive/evil\n::error::pwned.rs": "x\n"}, title="x\n::set-output name=a::b",
            env={"GITHUB_ACTIONS": "true"},
        )
        self.assertEqual(code, 1)
        for line in out.splitlines():
            if line.startswith("::"):
                self.assertTrue(line.startswith("::error file=") and "pwned" in line and "%0A" in line, line)
        self.assertNotIn("\n::set-output", out)


class BigPullRequests(LaneGuardCase):
    def test_a_pr_over_hundreds_of_foreign_files_prints_a_bounded_report(self):
        files = {f"crates/w5k_drive/src/gen/file{i}.rs": "// x\n" for i in range(120)}
        code, out, _ = self.pr("lane/chassis/x", files, env={"GITHUB_ACTIONS": "true"})
        self.assertEqual(code, 1)
        self.assertIn("120 path(s) are outside lane 'chassis'", out)
        self.assertEqual(out.count("\n  - crates/"), lane_guard.MAX_LISTED)
        self.assertIn(f"... and {120 - lane_guard.MAX_LISTED} more path(s)", out)
        self.assertEqual(len([l for l in out.splitlines() if l.startswith("::error ")]), 10)


class ProtectedPaths(LaneGuardCase):
    """[protected]: no lane branch may change these paths, not even with a CCR: title; unrestricted heads still may."""

    PROTECTED_SAMPLES = [".github/workflows/pr.yml", ".claude/settings.json", "tools/ci/lane_guard.py", "docs/swarm/ownership.toml",
                         "clippy.toml", "docs/swarm/RULES.md"]
    MESSAGE = ("is a protected path (CI, settings, ownership, the rules): only ARCH's own branches may change it; "
               "file an interface request")

    def setUp(self):
        super().setUp()
        self.repo.checkout("main")
        self.repo.write("docs/swarm/ownership.toml", OWNERSHIP_WITH_PROTECTED)
        for path in self.PROTECTED_SAMPLES[:-1] + ["crates/w5k_chassis/src/frozen.rs"]:
            if path != "docs/swarm/ownership.toml":
                self.repo.write(path, "# base\n")
        self.repo.commit("base with a [protected] table")

    def test_a_lane_pr_touching_a_protected_path_fails_with_the_message(self):
        code, out, _ = self.pr("lane/chassis/x", {".github/workflows/pr.yml": "# loosened\n"})
        self.assertEqual(code, 1, out)
        self.assertIn(f".github/workflows/pr.yml {self.MESSAGE} in docs/swarm/requests/chassis-x.md instead", out)
        self.assertIn("of these, 1 PROTECTED: no lane branch may change a protected path", out)
        self.assertIn("Protected (only ARCH's own branches may change these, CCR or not): .github/**, .claude/**, tools/ci/**", out)
        self.assertNotIn("belongs to lane", out)

    def test_every_protected_path_is_refused_for_every_lane(self):
        for lane in ("chassis", "drive", "arch"):  # arch included: its protected files change from ARCH's own branches only
            for i, path in enumerate(self.PROTECTED_SAMPLES):
                with self.subTest(lane=lane, path=path):
                    code, out, _ = self.pr(f"lane/{lane}/p{i}", {path: "# changed\n"})
                    self.assertEqual(code, 1, out)
                    self.assertIn(f"{path} {self.MESSAGE}", out)

    def test_only_the_protected_files_of_a_mixed_pull_request_are_flagged_as_protected(self):
        code, out, _ = self.pr("lane/chassis/x", {
            "crates/w5k_chassis/src/lib.rs": "// own file\n", "tools/ci/lane_guard.py": "# weakened\n",
            "crates/w5k_drive/src/lib.rs": "// other lane\n"})
        self.assertEqual(code, 1)
        self.assertIn("2 path(s) are outside lane 'chassis'", out)
        self.assertIn("of these, 1 PROTECTED: no lane branch may change a protected path", out)
        self.assertIn(f"tools/ci/lane_guard.py {self.MESSAGE}", out)
        self.assertIn("crates/w5k_drive/src/lib.rs which belongs to lane drive", out)
        self.assertNotIn("crates/w5k_chassis/src/lib.rs which", out)

    def test_protected_beats_the_lanes_own_paths(self):
        code, out, _ = self.pr("lane/chassis/x", {"crates/w5k_chassis/src/frozen.rs": "// frozen\n"})
        self.assertEqual(code, 1, out)
        self.assertIn(f"crates/w5k_chassis/src/frozen.rs {self.MESSAGE}", out)

    def test_a_ccr_pull_request_may_not_touch_protected_paths_either(self):
        code, out, _ = self.pr(
            "lane/chassis/ccr",
            {
                "crates/w5k_contract/src/lib.rs": "pub fn contract() { /* new field */ }\n",
                "crates/w5k_drive/src/lib.rs": "pub fn drive() { /* migrated */ }\n",
                ".github/workflows/pr.yml": "# 'migration edit'\n",
                "tools/ci/constants_lint.py": "# 'migration edit'\n",
            },
            title="CCR: add a damper field",
        )
        self.assertEqual(code, 1, out)
        self.assertIn("ARCH MUST REVIEW EVERY LINE", out)  # the notice is still printed ...
        self.assertIn("crates/w5k_contract/src/lib.rs   [contract path]", out)  # ... the CCR part is fine ...
        self.assertIn("crates/w5k_drive/src/lib.rs   [migration edit in lane drive]", out)
        self.assertIn("2 path(s) are outside lane 'chassis'", out)  # ... but the two protected files fail it
        self.assertIn(f".github/workflows/pr.yml {self.MESSAGE}", out)
        self.assertIn(f"tools/ci/constants_lint.py {self.MESSAGE}", out)
        self.assertIn("of these, 2 PROTECTED: no lane branch may change a protected path, not even with a 'CCR:' title", out)
        self.assertIn("with CCR:     : also the contract paths, and migration edits anywhere except the protected paths", out)
        self.assertNotIn("PASS", out)
        self.assertNotIn("crates/w5k_contract/src/lib.rs which", out)

    def test_a_ccr_pull_request_without_protected_paths_still_passes(self):
        code, out, _ = self.pr(
            "lane/chassis/ccr",
            {"crates/w5k_contract/src/lib.rs": "// c\n", "crates/w5k_drive/src/lib.rs": "// migrated\n"},
            title="CCR: ok",
        )
        self.assertEqual(code, 0, out)
        self.assertIn("PASS (with the review obligation above)", out)

    def test_unrestricted_heads_may_still_change_protected_paths(self):
        for branch in ("integration", "arch/guard-update", "claude/sharp-babbage-d702f7"):
            with self.subTest(branch=branch):
                files = {path: f"# by {branch}\n" for path in self.PROTECTED_SAMPLES}
                code, out, _ = self.pr(branch, files)
                self.assertEqual(code, 0, out)
                self.assertIn("unrestricted pattern", out)

    def test_a_lane_arch_branch_is_a_lane_branch_like_any_other(self):
        code, out, _ = self.pr("lane/arch/topic", {"crates/w5k_math/src/lib.rs": "// math\n"})
        self.assertEqual(code, 0, out)  # its own paths are fine ...
        code, out, _ = self.pr("lane/arch/other", {"tools/ci/lane_guard.py": "# x\n"})
        self.assertEqual(code, 1, out)  # ... but protected paths wait for ARCH's own (unrestricted) branches
        self.assertIn(self.MESSAGE, out)

    def test_renames_are_checked_on_both_sides(self):
        long_text = "a line of text that is long enough for git to see the rename\n" * 10
        self.repo.checkout("main")
        self.repo.write("tools/ci/helper.py", long_text)
        self.repo.write("crates/w5k_chassis/src/hull.rs", long_text + "// chassis\n")
        self.repo.commit("more base")
        for branch, old, new in (("lane/chassis/out", "tools/ci/helper.py", "crates/w5k_chassis/src/helper.py"),
                                 ("lane/chassis/in", "crates/w5k_chassis/src/hull.rs", ".github/hull.rs")):
            with self.subTest(old=old, new=new):
                self.repo.checkout("main")
                self.repo.branch(branch)
                self.repo.move(old, new)
                self.repo.commit("move")
                code, out, _ = run_main(lane_guard.main, ["--base", "main", "--head", branch, "--branch", branch, "--title", "move",
                                                          "--repo", str(self.repo.path)])
                self.assertEqual(code, 1, out)
                self.assertIn(f"{old if 'tools' in old else new} {self.MESSAGE}", out)

    def test_deleting_a_protected_file_is_refused(self):
        code, out, _ = self.pr("lane/chassis/x", {"clippy.toml": None})
        self.assertEqual(code, 1, out)
        self.assertIn(f"clippy.toml {self.MESSAGE}", out)

    def test_a_pr_cannot_remove_the_protected_table_to_free_itself(self):
        weakened = OWNERSHIP  # the same rules without [protected]
        code, out, _ = self.pr("lane/chassis/x", {"docs/swarm/ownership.toml": weakened, ".github/workflows/pr.yml": "# x\n"})
        self.assertEqual(code, 1, out)
        self.assertIn(f".github/workflows/pr.yml {self.MESSAGE}", out)
        self.assertIn(f"docs/swarm/ownership.toml {self.MESSAGE}", out)

    def test_github_annotation_for_a_protected_path(self):
        _, out, _ = self.pr("lane/chassis/x", {".github/workflows/pr.yml": "# x\n"}, env={"GITHUB_ACTIONS": "true"})
        self.assertIn(f"::error file=.github/workflows/pr.yml,title=Lane guard::.github/workflows/pr.yml {self.MESSAGE}", out)

    def test_a_malformed_table_is_a_setup_error(self):
        bad = self.outside("bad.toml", OWNERSHIP + '\n[protected]\npaths = ".github/**"\n')
        code, _, err = self.pr("lane/chassis/x", {"docs/theory/chassis.md": "x\n"}, extra=["--ownership", str(bad)])
        self.assertEqual(code, 2)
        self.assertIn("[protected].paths must be a list of strings", err)


class ProtectedPathsAreOptional(unittest.TestCase):
    """No [protected] table (an older ownership.toml) means no protected paths and the old behaviour."""

    def changes(self, *paths):
        return [Change("M", p) for p in paths]

    def setUp(self):
        import tomllib

        self.data = lane_guard.check_ownership(tomllib.loads(OWNERSHIP))

    def test_missing_table_behaves_as_before(self):
        verdict = lane_guard.evaluate(self.data, "lane/chassis/x", "work", self.changes(".github/workflows/pr.yml"))
        self.assertEqual(len(verdict.violations), 1)
        self.assertFalse(verdict.violations[0].protected)
        self.assertIn("belongs to lane arch", verdict.violations[0].message)
        self.assertEqual(verdict.protected_hits, 0)

    def test_missing_table_ccr_may_still_edit_anywhere(self):
        verdict = lane_guard.evaluate(self.data, "lane/chassis/x", "CCR: x", self.changes(".github/workflows/pr.yml", "crates/w5k_contract/a.rs"))
        self.assertTrue(verdict.ok)
        self.assertEqual(verdict.outside, [".github/workflows/pr.yml", "crates/w5k_contract/a.rs"])

    def test_an_empty_table_is_the_same_as_a_missing_one(self):
        data = dict(self.data, protected={"paths": []})
        self.assertTrue(lane_guard.evaluate(data, "lane/chassis/x", "CCR: x", self.changes("tools/ci/x.py")).ok)

    def test_protected_beats_common_and_own_paths_in_the_pure_function(self):
        data = dict(self.data, protected={"paths": ["docs/swarm/status/chassis.md", "crates/w5k_chassis/locked/**"]})
        verdict = lane_guard.evaluate(data, "lane/chassis/x", "CCR: x",
                                      self.changes("docs/swarm/status/chassis.md", "crates/w5k_chassis/locked/a.rs", "crates/w5k_chassis/ok.rs"))
        self.assertEqual([v.path for v in verdict.violations], ["docs/swarm/status/chassis.md", "crates/w5k_chassis/locked/a.rs"])
        self.assertTrue(all(v.protected for v in verdict.violations))

    def test_unrestricted_heads_ignore_the_table(self):
        data = dict(self.data, protected={"paths": [".github/**"]})
        self.assertEqual(lane_guard.evaluate(data, "arch/x", "t", self.changes(".github/a.yml")).mode, "unrestricted")


class CommandLine(LaneGuardCase):
    def test_runs_under_python_dash_I_and_reports_a_bad_pr_end_to_end(self):
        self.repo.checkout("main")
        self.repo.branch("lane/chassis/x")
        self.repo.write("crates/w5k_drive/src/lib.rs", "pub fn drive() { /* hack */ }\n")
        self.repo.commit("work")
        proc = run_script(
            "lane_guard.py", "--base", "main", "--head", "lane/chassis/x", "--branch", "lane/chassis/x",
            "--title", "Chassis: sneaky", "--repo", str(self.repo.path),
        )
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn("your PR touches crates/w5k_drive/src/lib.rs which belongs to lane drive", proc.stdout)

    def test_passing_pr_exits_zero(self):
        self.repo.branch("lane/drive/x")
        self.repo.write("crates/w5k_drive/src/lib.rs", "pub fn drive() { /* ok */ }\n")
        self.repo.commit("work")
        proc = run_script(
            "lane_guard.py", "--base", "main", "--head", "HEAD", "--branch", "lane/drive/x", "--repo", str(self.repo.path)
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_help_documents_the_rules(self):
        proc = run_script("lane_guard.py", "--help")
        self.assertEqual(proc.returncode, 0)
        for word in ("--base", "--head", "--branch", "--title", "--ownership", "--repo", "CCR:"):
            self.assertIn(word, proc.stdout)

    def test_missing_required_argument_is_a_usage_error(self):
        proc = run_script("lane_guard.py", "--base", "main")
        self.assertEqual(proc.returncode, 2)


# ---- the real rules -------------------------------------------------------------------------------------------------------


def real_ownership_path():
    for candidate in (os.environ.get("W5K_OWNERSHIP"), ci_testlib.ROOT / "docs/swarm/ownership.toml"):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    return None


@unittest.skipUnless(real_ownership_path(), "docs/swarm/ownership.toml not available (set W5K_OWNERSHIP)")
class RealOwnershipFile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from ci_common import load_toml_file

        cls.data = lane_guard.check_ownership(load_toml_file(real_ownership_path()))
        cls.lanes = sorted(cls.data["lanes"])

    def violations(self, lane, paths, title="work"):
        changes = [Change("M", p) for p in paths]
        return lane_guard.evaluate(self.data, f"lane/{lane}/topic", title, changes).violations

    def test_the_thirteen_lanes_of_the_plan_exist(self):
        self.assertEqual(
            self.lanes,
            sorted("arch chassis drive tracks world forge geometry look viewer godot validation combat ai".split()),
        )

    def test_every_lane_can_write_its_own_status_theory_and_request_files_and_the_lock(self):
        for lane in self.lanes:
            paths = [f"docs/swarm/status/{lane}.md", f"docs/theory/{lane}.md", f"docs/swarm/requests/{lane}-x.md",
                     f"docs/lanes/{lane}/media/a.png", f"spikes/{lane}/s1/main.rs", "Cargo.lock"]
            self.assertEqual(self.violations(lane, paths), [], lane)

    GUARD_FILES = [  # what ARCH put (or intends to put) in [protected]
        ".github/workflows/pr.yml", ".claude/settings.json", "tools/ci/lane_guard.py", "docs/swarm/ownership.toml", "clippy.toml",
        "rustfmt.toml", "CLAUDE.md", "Cargo.toml", "docs/swarm/RULES.md", "docs/swarm/MERGE-GATE.md", "docs/swarm/GUARDRAILS.md",
    ]

    def test_no_lane_branch_can_change_the_guardrails_but_unrestricted_heads_can(self):
        if not self.data.get("protected", {}).get("paths"):
            self.skipTest("this ownership.toml has no [protected] table yet")
        for lane in self.lanes:  # arch's own lane branch included: ARCH changes these from its unrestricted arch/* branches
            self.assertEqual(len(self.violations(lane, self.GUARD_FILES)), len(self.GUARD_FILES), lane)
            self.assertEqual(len(self.violations(lane, self.GUARD_FILES, title="CCR: sneaky")), len(self.GUARD_FILES), lane)
        for head in ("integration", "arch/launch-kit"):
            verdict = lane_guard.evaluate(self.data, head, "t", [Change("M", p) for p in self.GUARD_FILES])
            self.assertEqual((verdict.mode, verdict.violations), ("unrestricted", []))

    def test_other_arch_files_are_still_owned_by_arch_only(self):
        arch_only = ["docs/swarm/budgets.toml", "crates/w5k_tools/src/main.rs", "crates/w5k_tools/src/cmd/mod.rs", "crates/w5k_sim/src/lib.rs"]
        self.assertEqual(self.violations("arch", arch_only), [])
        for lane in self.lanes:
            if lane != "arch":
                self.assertEqual(len(self.violations(lane, arch_only)), len(arch_only), lane)

    def test_every_protected_path_exists_as_a_real_guard_file_pattern(self):
        for pattern in self.data.get("protected", {}).get("paths", []):
            self.assertTrue(lane_guard.matches_any([pattern], pattern.replace("**", "x/y")), pattern)

    def test_each_lane_owns_exactly_its_own_example_paths(self):
        """Turn every lane glob into a concrete path; only that lane (and, for the contract, ARCH) may own it."""
        for lane, table in self.data["lanes"].items():
            for pattern in table["paths"]:
                sample = pattern.replace("**", "x/y").replace("*", "z")
                owners = lane_guard.owners_of(sample, self.data)
                self.assertEqual(owners, [lane], f"{pattern} (sample {sample}) is owned by {owners}")

    def test_chassis_cannot_touch_drive_but_can_touch_its_own_command_file(self):
        self.assertEqual(len(self.violations("chassis", ["crates/w5k_drive/src/lib.rs"])), 1)
        self.assertEqual(self.violations("chassis", ["crates/w5k_tools/src/cmd/chassis.rs"]), [])
        self.assertEqual(len(self.violations("chassis", ["crates/w5k_tools/src/cmd/drive.rs"])), 1)

    def test_the_unrestricted_heads_of_the_file(self):
        for head in self.data["unrestricted"]["heads"]:
            sample = head.replace("*", "anything")
            self.assertEqual(lane_guard.evaluate(self.data, sample, "t", [Change("M", ".github/x")]).mode, "unrestricted")


if __name__ == "__main__":
    unittest.main()
