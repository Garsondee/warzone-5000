"""Run the shell scripts that live inside .github/workflows/pr.yml, the way GitHub Actions would, in throw-away repositories.

GitHub hands a workflow's expressions to a script as environment variables; these tests set those variables directly and
run the real `run:` text of the step. They show that hostile text in a pull-request title, description or branch name
stays plain text, and that the "checks come from the target branch" step cannot be fooled by the pull request."""

import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

import ci_testlib
from ci_testlib import TempRepo, run_blocks

PR_YML = ci_testlib.ROOT / ".github/workflows/pr.yml"
CI_DIR = ci_testlib.CI_DIR

OWNERSHIP = """
[unrestricted]
heads = ["integration", "arch/*"]
[common]
paths = ["docs/swarm/status/{lane}.md", "Cargo.lock"]
[ccr]
paths = ["crates/w5k_contract/**"]
[lanes.arch]
paths = ["crates/w5k_contract/**", "tools/ci/**"]
[lanes.chassis]
paths = ["crates/w5k_chassis/**"]
[lanes.drive]
paths = ["crates/w5k_drive/**"]
"""


def step_script(name_prefix):
    """The `run:` text of the step whose name starts with `name_prefix`, dedented, exactly as committed."""
    text = PR_YML.read_text()
    for chunk in text.split("\n      - ")[1:]:
        if chunk.startswith(f"name: {name_prefix}"):
            (_, body), *_ = run_blocks(chunk)
            return textwrap.dedent(body).strip("\n") + "\n"
    raise AssertionError(f"no step named {name_prefix!r} in pr.yml")


def run_step(script, cwd, env):
    full_env = {"PATH": os.environ["PATH"], "HOME": str(cwd), "GIT_CONFIG_GLOBAL": os.devnull, **env}
    return subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script], cwd=cwd, env=full_env, capture_output=True, text=True)


@unittest.skipUnless(PR_YML.is_file(), ".github/workflows/pr.yml not found")
class TakeTheChecksFromTheTargetBranch(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.runner = tempfile.TemporaryDirectory()
        self.addCleanup(self.runner.cleanup)
        self.github_env = Path(self.runner.name) / "github_env"
        self.script = step_script("Take the checks from the target branch")

    def run_it(self):
        self.repo.git("update-ref", "refs/remotes/origin/integration", "integration")
        return run_step(self.script, self.repo.path, {
            "BASE_REF": "integration", "RUNNER_TEMP": self.runner.name, "GITHUB_ENV": str(self.github_env),
            "GITHUB_WORKSPACE": str(self.repo.path)})

    def ci_dir(self):
        line = [l for l in self.github_env.read_text().splitlines() if l.startswith("CI_DIR=")]
        self.assertEqual(len(line), 1)
        return Path(line[0].removeprefix("CI_DIR="))

    def test_uses_the_target_branchs_copy_not_the_pull_requests(self):
        self.repo.write("tools/ci/lane_guard.py", "# TARGET BRANCH VERSION\n")
        self.repo.write("tools/ci/ci_common.py", "# helper\n")
        self.repo.commit("base")
        self.repo.git("branch", "-M", "integration")
        self.repo.branch("lane/chassis/tamper")
        self.repo.write("tools/ci/lane_guard.py", "# TAMPERED: always passes\n")  # the PR rewrites its own judge
        self.repo.commit("tamper")
        proc = self.run_it()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.ci_dir(), Path(self.runner.name) / "checks-from-integration/tools/ci")
        self.assertEqual((self.ci_dir() / "lane_guard.py").read_text(), "# TARGET BRANCH VERSION\n")
        self.assertTrue((self.ci_dir() / "ci_common.py").exists())
        self.assertIn("Using the checks from origin/integration", proc.stdout)

    def test_falls_back_to_the_pull_requests_copy_when_the_target_branch_has_none_yet(self):
        self.repo.write("README.md", "hi\n")
        self.repo.commit("base")
        self.repo.git("branch", "-M", "integration")
        self.repo.branch("lane/arch/bootstrap")
        self.repo.write("tools/ci/lane_guard.py", "# first version\n")
        self.repo.commit("add the checks")
        proc = self.run_it()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.ci_dir(), self.repo.path / "tools/ci")
        self.assertIn("::notice::origin/integration has no tools/ci yet", proc.stdout)

    def test_a_target_branch_with_only_a_placeholder_counts_as_having_none(self):
        self.repo.write("tools/ci/.gitkeep", "")
        self.repo.commit("base")
        self.repo.git("branch", "-M", "integration")
        self.repo.branch("lane/arch/bootstrap")
        self.repo.write("tools/ci/lane_guard.py", "# first version\n")
        self.repo.commit("add the checks")
        proc = self.run_it()
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.ci_dir(), self.repo.path / "tools/ci")


@unittest.skipUnless(PR_YML.is_file(), ".github/workflows/pr.yml not found")
class LaneGuardStep(unittest.TestCase):
    HOSTILE_TITLES = [
        '"; touch PWNED; echo "', "$(touch PWNED)", "`touch PWNED`", "--base evil", "-x", "--title=other", "a' ; touch PWNED ; '",
        "$IFS$(touch PWNED)", "line one\\nCCR: not a real prefix", "*", "~", "${HOME}",
    ]

    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.repo.write("docs/swarm/ownership.toml", OWNERSHIP)
        self.repo.write("crates/w5k_chassis/src/lib.rs", "// chassis\n")
        self.repo.write("crates/w5k_drive/src/lib.rs", "// drive\n")
        self.repo.commit("base")
        self.repo.git("branch", "-M", "integration")
        self.repo.git("update-ref", "refs/remotes/origin/integration", "integration")
        self.script = step_script("Lane guard")

    def run_it(self, title="Chassis: work", branch="lane/chassis/x", pr_repo="Garsondee/warzone-5000"):
        return run_step(self.script, self.repo.path, {
            "PR_TITLE": title, "PR_BRANCH": branch, "PR_REPO": pr_repo, "THIS_REPO": "Garsondee/warzone-5000",
            "CI_DIR": str(CI_DIR), "BASE": "origin/integration"})

    def make_pr(self, branch, path="crates/w5k_chassis/src/lib.rs"):
        self.repo.checkout("integration")
        self.repo.branch(branch)
        self.repo.write(path, f"// changed on {branch}\n")
        self.repo.commit("work")

    def test_a_good_pull_request_passes(self):
        self.make_pr("lane/chassis/x")
        proc = self.run_it()
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_a_bad_pull_request_fails(self):
        self.make_pr("lane/chassis/x", "crates/w5k_drive/src/lib.rs")
        proc = self.run_it()
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn("belongs to lane drive", proc.stdout)

    def test_hostile_titles_stay_plain_text(self):
        self.make_pr("lane/chassis/x")
        for title in self.HOSTILE_TITLES:
            with self.subTest(title=title):
                proc = self.run_it(title=title)
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)  # the title is irrelevant to this good PR
                self.assertFalse((self.repo.path / "PWNED").exists(), "the title was executed as a command")
                self.assertNotIn("setup problem", proc.stderr)

    def test_the_title_reaches_the_guard_unchanged(self):
        self.make_pr("lane/chassis/x", "crates/w5k_drive/src/lib.rs")
        self.assertEqual(self.run_it(title="Fix the CCR: thing").returncode, 1)
        proc = self.run_it(title="CCR: really a contract change")
        self.assertEqual(proc.returncode, 0, proc.stdout)
        self.assertIn("CCR PULL REQUEST", proc.stdout)

    def test_a_branch_from_a_fork_cannot_pass_as_integration(self):
        self.make_pr("integration-fork-copy", "crates/w5k_drive/src/lib.rs")
        trusted = self.run_it(branch="integration")
        self.assertEqual(trusted.returncode, 0, trusted.stdout)  # same repository: an unrestricted head
        forked = self.run_it(branch="integration", pr_repo="mallory/warzone-5000")
        self.assertEqual(forked.returncode, 1, forked.stdout)
        self.assertIn("fork/integration", forked.stdout)
        deleted_fork = self.run_it(branch="integration", pr_repo="")
        self.assertEqual(deleted_fork.returncode, 1)


@unittest.skipUnless(PR_YML.is_file(), ".github/workflows/pr.yml not found")
class GoldenGuardStep(unittest.TestCase):
    BODIES = [
        "plain text\n", "", "Golden-Change: tuned $(touch PWNED) the damper\n", "`touch PWNED`\nGolden-Change: a `reason`\n",
        "100% sure %s %d \\n \\x41 end", "-n\n-e\nGolden-Change: dashes", "line1\r\nGolden-Change: windows\r\n", "'quotes' \"double\" $HOME ${HOME}",
        "unicode é中 \U0001f600\nGolden-Change: ok",
    ]

    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.golden = "crates/w5k_sim/tests/golden/first_light.json"
        self.repo.write(self.golden, "{}\n")
        self.repo.commit("base")
        self.repo.git("branch", "-M", "integration")
        self.repo.git("update-ref", "refs/remotes/origin/integration", "integration")
        self.repo.branch("lane/x/y")
        self.runner = tempfile.TemporaryDirectory()
        self.addCleanup(self.runner.cleanup)
        self.script = step_script("Golden guard")

    def run_it(self, body):
        return run_step(self.script, self.repo.path, {
            "PR_BODY": body, "RUNNER_TEMP": self.runner.name, "CI_DIR": str(CI_DIR), "BASE": "origin/integration"})

    def test_the_description_is_written_to_the_file_byte_for_byte(self):
        self.repo.write("crates/a/src/lib.rs", "//\n")
        self.repo.commit("work")
        for body in self.BODIES:
            with self.subTest(body=body):
                proc = self.run_it(body)
                self.assertEqual(proc.returncode, 0, proc.stderr)
                written = (Path(self.runner.name) / "pr_body.txt").read_bytes()
                self.assertEqual(written, body.encode("utf-8"))
                self.assertFalse((self.repo.path / "PWNED").exists())

    def test_verdict_for_a_changed_golden(self):
        self.repo.write(self.golden, '{"hash": "new"}\n')
        self.repo.commit("work")
        self.assertEqual(self.run_it("no reason given").returncode, 1)
        self.assertEqual(self.run_it("").returncode, 1)
        proc = self.run_it("Tuned the damper.\r\n\r\nGolden-Change: damper ratio changed on purpose\r\n")
        self.assertEqual(proc.returncode, 0, proc.stdout)


@unittest.skipUnless(PR_YML.is_file(), ".github/workflows/pr.yml not found")
class AffectedStep(unittest.TestCase):
    def test_writes_the_selection_to_the_step_output(self):
        repo = TempRepo()
        self.addCleanup(repo.cleanup)
        repo.write("docs/readme.md", "x\n")
        repo.commit("base")
        repo.git("branch", "-M", "integration")
        repo.git("update-ref", "refs/remotes/origin/integration", "integration")
        repo.branch("lane/x/y")
        repo.write("docs/readme.md", "y\n")
        repo.commit("work")
        out = tempfile.NamedTemporaryFile("w+", delete=False)
        self.addCleanup(os.unlink, out.name)
        script = step_script("Which crates does this pull request affect?").replace("tools/ci/affected.py", f"{CI_DIR}/affected.py")
        proc = run_step(script, repo.path, {"BASE": "origin/integration", "GITHUB_OUTPUT": out.name})
        self.assertEqual(proc.returncode, 0, proc.stderr)
        # no Cargo workspace in this throw-away repository, so affected.py cannot tell and says: test everything
        self.assertEqual(Path(out.name).read_text(), "args=--workspace\n")
        self.assertIn("cargo test selection: --workspace", proc.stdout)


if __name__ == "__main__":
    unittest.main()
