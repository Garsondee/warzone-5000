"""Static checks of the GitHub workflows: they say what the brief says, use only known actions, and cannot be hijacked by
the text of a pull-request title. No YAML library is needed (PyYAML is used for one extra parse if it happens to be
installed); the checks work on the text, which is enough because the files are written in a plain, regular style."""

import re
import unittest
from pathlib import Path

import ci_testlib
from ci_testlib import run_blocks

WORKFLOWS = ci_testlib.ROOT / ".github/workflows"
ALLOWED_ACTIONS = {
    "actions/checkout@v4",
    "dtolnay/rust-toolchain@stable",
    "Swatinem/rust-cache@v2",
    "actions/setup-python@v5",
    "actions/upload-artifact@v4",
}
# Contexts whose value other people control. They may only be handed over as an environment variable.
UNTRUSTED = re.compile(
    r"github\.event\.pull_request\.(title|body|head\.ref|head\.label|head\.repo\.[\w.]+)|github\.head_ref|github\.event\.head_commit\.message|"
    r"github\.event\.(issue|comment|review|review_comment)\.[\w.]+|github\.event\.commits|github\.event\.pages"
)
ENV_LINE = re.compile(r"^\s+[A-Z][A-Z0-9_]*:\s*\$\{\{[^}]*\}\}\s*$")


def read(name):
    return (WORKFLOWS / name).read_text()


@unittest.skipUnless(WORKFLOWS.is_dir(), ".github/workflows not found")
class EveryWorkflow(unittest.TestCase):
    NAMES = ["pr.yml", "integration.yml", "nightly.yml"]

    def test_the_three_workflows_exist(self):
        for name in self.NAMES:
            self.assertTrue((WORKFLOWS / name).is_file(), name)

    def test_starts_with_a_plain_english_header_comment(self):
        for name in self.NAMES:
            with self.subTest(workflow=name):
                lines = read(name).splitlines()
                header = []
                for line in lines:
                    if not line.startswith("#"):
                        break
                    header.append(line)
                self.assertGreaterEqual(len(header), 6, "the header comment should explain the purpose in plain English")
                self.assertIn("In plain English", "\n".join(header))

    def test_read_only_permissions_everywhere(self):
        for name in self.NAMES:
            with self.subTest(workflow=name):
                text = read(name)
                self.assertRegex(text, r"(?m)^permissions:\n  contents: read\n")
                self.assertNotRegex(text, r":\s*write\b")

    def test_only_known_actions_pinned_to_a_version(self):
        for name in self.NAMES:
            for used in re.findall(r"(?m)^\s*(?:- )?uses:\s*(\S+)", read(name)):
                self.assertIn(used, ALLOWED_ACTIONS, f"{name} uses {used}")

    def test_dangerous_triggers_are_not_used(self):
        for name in self.NAMES:
            text = re.sub(r"(?m)^\s*#.*$", "", read(name))
            self.assertNotIn("pull_request_target", text)
            self.assertNotIn("workflow_run", text)

    def test_no_expression_is_pasted_into_a_shell_script(self):
        for name in self.NAMES:
            for number, body in run_blocks(read(name)):
                self.assertNotIn("${{", body, f"{name}:{number}: a ${{{{ }}}} expression inside a run: script")

    def test_untrusted_text_only_travels_in_environment_variables(self):
        for name in self.NAMES:
            for number, line in enumerate(read(name).splitlines(), 1):
                if "${{" in line and UNTRUSTED.search(line) and not line.lstrip().startswith("#"):
                    self.assertRegex(line, ENV_LINE, f"{name}:{number}: untrusted context outside an env: entry: {line.strip()}")

    def test_cargo_commands_refuse_to_change_the_lock_file(self):
        for name in self.NAMES:
            for number, body in run_blocks(read(name)):
                for line in body.splitlines():
                    if re.match(r"\s*cargo (test|clippy|run|build)\b", line):
                        self.assertIn("--locked", line, f"{name}:{number}: {line.strip()}")

    def test_optional_yaml_parse(self):
        try:
            import yaml
        except ImportError:
            self.skipTest("PyYAML is not installed")
        for name in self.NAMES:
            doc = yaml.safe_load(read(name))
            self.assertIn("jobs", doc)
            self.assertEqual(doc["permissions"], {"contents": "read"})
            for job in doc["jobs"].values():
                for step in job["steps"]:
                    self.assertTrue("uses" in step or "run" in step, step)


@unittest.skipUnless(WORKFLOWS.is_dir(), ".github/workflows not found")
class PullRequestWorkflow(unittest.TestCase):
    def setUp(self):
        self.text = read("pr.yml")

    def test_triggers_and_concurrency(self):
        self.assertRegex(self.text, r"(?m)^on:\n  pull_request:\n    branches: \[integration, main\]\n")
        self.assertRegex(self.text, r"types: \[opened, synchronize, reopened, edited\]")
        self.assertRegex(self.text, r"(?ms)^concurrency:\n  group: pr-\$\{\{ github\.event\.pull_request\.number \}\}\n  cancel-in-progress: true")

    def test_two_jobs(self):
        self.assertRegex(self.text, r"(?m)^  guards:\n")
        self.assertRegex(self.text, r"(?m)^  rust:\n")
        self.assertEqual(len(re.findall(r"runs-on: ubuntu-latest", self.text)), 2)

    def test_guards_job_checks_out_full_history_and_runs_every_check(self):
        guards = self.text.split("  rust:\n")[0]
        self.assertIn("fetch-depth: 0", guards)
        for script in ("lane_guard", "media_lint", "deps_lint", "line_budget", "constants_lint", "golden_guard"):
            self.assertIn(f'python3 -I "$CI_DIR/{script}.py"', guards)
        self.assertIn("python3 -I -m unittest discover -s tools/ci/tests", guards)

    def test_pull_request_title_and_body_go_through_the_environment(self):
        self.assertRegex(self.text, r"(?m)^          PR_TITLE: \$\{\{ github\.event\.pull_request\.title \}\}$")
        self.assertRegex(self.text, r"(?m)^          PR_BODY: \$\{\{ github\.event\.pull_request\.body \}\}$")
        self.assertIn('--title="$PR_TITLE"', self.text)
        self.assertIn("--branch=\"$branch\"", self.text)
        self.assertIn("printf '%s' \"$PR_BODY\" >", self.text)
        self.assertNotIn("echo \"$PR_BODY\"", self.text)
        self.assertEqual(self.text.count("github.event.pull_request.title"), 1)

    def test_a_branch_from_a_fork_cannot_pass_for_a_trusted_head(self):
        self.assertIn('branch="fork/$PR_BRANCH"', self.text)

    def test_every_guard_runs_even_when_an_earlier_one_failed(self):
        guards = self.text.split("  rust:\n")[0]
        steps = guards.split("      - name: ")[1:]
        for step in steps[1:]:
            # `if: ${{ !cancelled() }}`, optionally narrowed by a condition (the LOOK lane's Python tests run only when assets/tests exists)
            self.assertRegex(step, r"if: \$\{\{ !cancelled\(\)( && [^}]+)? \}\}", step.splitlines()[0])

    def test_rust_job_runs_the_commands_the_brief_names(self):
        rust = self.text.split("  rust:\n")[1]
        self.assertIn("cargo fmt --all -- --check", rust)
        self.assertIn("cargo clippy --workspace --all-targets --locked -- -D warnings", rust)
        self.assertIn('python3 -I tools/ci/affected.py --base "$BASE" --head HEAD', rust)
        self.assertIn("cargo test --locked $AFFECTED_ARGS", rust)
        self.assertIn("cargo test --locked -p w5k_sim --test first_light", rust)
        self.assertIn("components: rustfmt, clippy", rust)

    def test_the_affected_selection_reaches_cargo_through_an_environment_variable(self):
        self.assertRegex(self.text, r"(?m)^          AFFECTED_ARGS: \$\{\{ steps\.affected\.outputs\.args \}\}$")

    def test_the_checks_come_from_the_target_branch(self):
        self.assertIn('git archive "origin/$BASE_REF" tools/ci', self.text)
        self.assertIn("CI_DIR=", self.text)


@unittest.skipUnless(WORKFLOWS.is_dir(), ".github/workflows not found")
class IntegrationWorkflow(unittest.TestCase):
    def setUp(self):
        self.text = read("integration.yml")

    def test_triggers(self):
        self.assertRegex(self.text, r"(?m)^on:\n  push:\n    branches: \[integration, main\]\n  workflow_dispatch:\n")

    def test_matrix_and_commands(self):
        self.assertIn("os: [ubuntu-latest, windows-latest]", self.text)
        self.assertIn("fail-fast: false", self.text)
        self.assertIn("run: cargo test --workspace --locked\n", self.text)
        self.assertIn("run: cargo test --workspace --release --locked\n", self.text)
        self.assertIn("cargo run --release --locked -p w5k_tools --bin w5k -- scenario first-light --out out/first-light", self.text)

    def test_first_light_output_is_uploaded(self):
        self.assertRegex(self.text, r"uses: actions/upload-artifact@v4\n        with:\n          name: first-light-\$\{\{ matrix\.os \}\}\n          path: out/first-light\n")

    def test_godot_package_step_is_a_clearly_marked_placeholder(self):
        self.assertIn("TODO(lane GODOT)", self.text)
        self.assertIn("hashFiles('game/project.godot') != ''", self.text)
        self.assertIn("matrix.os == 'windows-latest'", self.text)
        self.assertNotIn("godot --", self.text.lower())
        self.assertNotIn("--export", self.text)


@unittest.skipUnless(WORKFLOWS.is_dir(), ".github/workflows not found")
class NightlyWorkflow(unittest.TestCase):
    def setUp(self):
        self.text = read("nightly.yml")

    def test_triggers(self):
        self.assertRegex(self.text, r'(?m)^on:\n  schedule:\n    - cron: "\d+ \d+ \* \* \*"')
        self.assertIn("  workflow_dispatch:\n", self.text)

    def test_both_placeholders_call_the_validation_command_and_may_fail(self):
        steps = [s for s in self.text.split("      - name: ")[1:] if "validation nightly" in s]
        self.assertEqual(len(steps), 2)
        for step in steps:
            self.assertIn("continue-on-error: true", step)
            self.assertIn("# remove continue-on-error when lane VALIDATION lands the command", step)
            self.assertIn("-p w5k_tools --bin w5k -- validation nightly", step)

    def test_the_owner_is_told_about_the_default_branch_rule(self):
        self.assertIn("DEFAULT branch", self.text)


if __name__ == "__main__":
    unittest.main()
