"""Tests for .claude/settings.json (the permission lists) and .claude/hooks/session-start.sh (the SessionStart hook).

The command lists in SettingsDecisions were also run through the real Claude Code permission engine (headless
`claude -p --permission-prompts none`) when the file was written: every "denied" command was refused and every "allowed"
one ran. These tests keep that behaviour from drifting. They use a small matcher of the documented rule syntax:
`Bash(prefix *)` matches the bare command too, and `*` matches any run of characters (spaces and slashes included)."""

import json
import os
import re
import shutil
import stat
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

import ci_testlib

ROOT = ci_testlib.ROOT
SETTINGS = ROOT / ".claude/settings.json"
HOOK = ROOT / ".claude/hooks/session-start.sh"


def bash_rule_matches(rule: str, command: str) -> bool:
    inner = re.fullmatch(r"Bash\((.*)\)", rule, re.DOTALL).group(1)
    pattern = re.escape(inner).replace(r"\*", ".*")
    if inner.endswith(" *"):  # `git status *` also matches plain `git status`
        pattern = re.escape(inner[:-2]).replace(r"\*", ".*") + r"(?: .*)?"
    return re.fullmatch(pattern, command, re.DOTALL) is not None


def decision(settings: dict, command: str) -> str:
    """deny beats allow beats 'ask' (no rule: an unattended session is refused)."""
    perms = settings["permissions"]
    if any(bash_rule_matches(r, command) for r in perms.get("deny", []) if r.startswith("Bash(")):
        return "deny"
    if any(bash_rule_matches(r, command) for r in perms.get("allow", []) if r.startswith("Bash(")):
        return "allow"
    return "ask"


@unittest.skipUnless(SETTINGS.is_file(), ".claude/settings.json not found")
class SettingsFile(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.settings = json.loads(SETTINGS.read_text())

    def test_rules_are_well_formed_and_unique(self):
        for kind in ("allow", "deny"):
            rules = self.settings["permissions"][kind]
            self.assertEqual(len(rules), len(set(rules)), f"duplicate {kind} rules")
            for rule in rules:
                self.assertRegex(rule, r"^[A-Za-z_][\w]*(\(.+\))?$", rule)
                self.assertEqual(rule.count("("), rule.count(")") - 0 if "(" in rule else 0, rule)

    def test_allow_list_has_every_command_the_brief_names(self):
        allow = self.settings["permissions"]["allow"]
        wanted = (
            [f"cargo {c}" for c in "build check test clippy fmt run metadata fetch tree".split()]
            + [f"git {c}" for c in "status diff log show add commit push fetch checkout switch branch merge rebase restore stash".split()]
            + ["python3 -I tools/ci/", "python3 -I tools/", "node ", "npm ci", "npm run ", "ffmpeg "]
            + [f"{c} " for c in "ls cat head tail wc grep find mkdir cp mv sed awk sort uniq diff".split()]
        )
        for command in wanted:
            self.assertTrue(any(r.startswith(f"Bash({command}") for r in allow), f"no allow rule for: {command}")

    def test_deny_list_has_every_command_the_brief_names(self):
        deny = " ".join(self.settings["permissions"]["deny"])
        for needle in ("git push --force", "git push -f", "git push --force-with-lease", "git reset --hard", "git clean", "rm -r"):
            self.assertIn(needle, deny)

    GITHUB_TOOLS = ("create_pull_request update_pull_request pull_request_read list_pull_requests add_issue_comment get_file_contents "
                    "list_commits get_commit list_branches search_code actions_get actions_list get_job_logs get_check_run issue_read").split()

    def test_github_tools_a_lane_needs_to_open_and_read_pull_requests_are_allowed(self):
        mcp = [rule for rule in self.settings["permissions"]["allow"] if rule.startswith("mcp__")]
        self.assertEqual(sorted(mcp), sorted(f"mcp__github__{t}" for t in self.GITHUB_TOOLS))

    def test_merging_and_writing_to_github_by_api_and_the_session_tools_are_not_pre_approved(self):
        allow = self.settings["permissions"]["allow"]
        for tool in ("merge_pull_request", "enable_pr_auto_merge", "disable_pr_auto_merge", "push_files", "create_or_update_file", "delete_file",
                     "create_branch", "issue_write", "pull_request_review_write", "run_secret_scanning"):
            self.assertNotIn(f"mcp__github__{tool}", allow)
        self.assertFalse([r for r in allow if r.startswith("mcp__claude-code-remote")])

    def test_no_permission_rule_is_a_deny_rule_for_the_lane_tools(self):
        # The settings file is shared with ARCH's own session, so the lane restrictions are a hook (lane-tool-guard.sh), not deny rules.
        deny = self.settings["permissions"]["deny"]
        self.assertFalse([r for r in deny if r.startswith("mcp__")])

    def test_the_allow_list_stays_narrow(self):
        allow = self.settings["permissions"]["allow"]
        for broad in ("Bash", "Bash(*)", "Bash(git *)", "Bash(cargo *)", "Bash(rm *)", "Bash(sudo *)", "Bash(curl *)", "Bash(python3 *)",
                      "Bash(npm *)", "Bash(bash *)", "Bash(sh *)", "WebFetch", "Write", "Edit", "mcp__github", "mcp__github__*",
                      "mcp__*", "mcp__claude-code-remote", "mcp__claude-code-remote__*"):
            self.assertNotIn(broad, allow)

    def test_session_start_hook_is_registered_and_points_at_an_executable_script(self):
        (group,) = self.settings["hooks"]["SessionStart"]
        (hook,) = group["hooks"]
        self.assertEqual(hook["type"], "command")
        self.assertIn(".claude/hooks/session-start.sh", hook["command"])
        self.assertGreater(hook["timeout"], 0)
        self.assertTrue(HOOK.is_file())
        self.assertTrue(HOOK.stat().st_mode & stat.S_IXUSR, "the hook script is not executable")

    def test_only_known_top_level_keys(self):
        self.assertLessEqual(set(self.settings), {"$schema", "permissions", "hooks", "env"})


@unittest.skipUnless(SETTINGS.is_file(), ".claude/settings.json not found")
class SettingsDecisions(unittest.TestCase):
    """Verified against the real permission engine; see the module docstring."""

    @classmethod
    def setUpClass(cls):
        cls.settings = json.loads(SETTINGS.read_text())

    DENIED = [
        "git push --force origin x", "git push origin x --force", "git push -f origin x", "git push origin x -f",
        "git push --force-with-lease origin x", "git push origin +x", "git push -uf origin x", "git push --force",
        "git push -f", "git push --force-with-lease", "git reset --hard HEAD", "git reset HEAD --hard", "git reset --hard",
        "git clean -fd", "git clean", "rm -rf victim", "rm -fr victim", "rm -r victim", "rm victim -rf", "rm -f -r victim",
        "rm --recursive victim", "rm -Rf victim", "rm -R victim", "find victim -name x -delete", "find victim -exec rm {} +",
    ]
    ALLOWED = [
        "git push -u origin lane/chassis/force-model", "git push origin lane/chassis/fix", "git push", "git reset --soft HEAD",
        "git reset HEAD file.rs", "git pull --rebase origin integration", "git status", "git diff", "git log --oneline -1",
        "git commit -m 'fix the force model'", "git commit -m 'never git push --force'", "git add -A", "find victim -name x",
        "find . -name '*.rs'", "cargo build --release", "cargo test -p w5k_sim --test first_light",
        "cargo clippy --workspace --all-targets -- -D warnings", "cargo fmt --all -- --check",
        "cargo run --release -p w5k_tools --bin w5k -- scenario first-light", "cargo metadata --no-deps --format-version 1",
        "python3 -I tools/ci/lane_guard.py --help", "python3 -I tools/viewer/build.py x y",
        "python3 -I -m unittest discover -s tools/ci/tests", "node tools/viewer/capture.js", "npm ci", "npm run build",
        "ffmpeg -version", "ls -la", "cat a.txt", "grep -rn foo crates", "diff a b", "sed -n 1p a.txt", "mkdir -p out/x", "cp a b",
        "mv a b", "git rm old.rs", "git mv a.rs b.rs", "cargo update -w", "cargo update --workspace",
        "W5K_BLESS=1 cargo test -p w5k_sim", "export RUSTFLAGS=-Dwarnings", "touch a.txt",
    ]
    # Not caught by any deny rule even though they look similar (whether they are allowed is a separate question).
    NOT_DENIED = ["rm -f victim/x", "rm victim/y", "rm -f victim/build-report", "rm -f /tmp/x", "git push -u origin lane/x/force-fix"]
    ASKED = [  # nothing allows these: an unattended session is refused, on purpose
        "cargo add serde", "cargo install ripgrep", "cargo update", "cargo update -p serde", "cargo update -w -p serde", "curl https://example.com", "wget https://example.com",
        "rm victim/x", "chmod +x a.txt", "sudo ls", "git config user.name x", "git remote add o http://example.com/r.git",
        "python3 -c 'print(1)'", "python3 other.py", "npm install left-pad", "pip install requests", "bash -c 'echo hi'",
    ]

    def test_denied(self):
        for command in self.DENIED:
            with self.subTest(command=command):
                self.assertEqual(decision(self.settings, command), "deny")

    def test_allowed(self):
        for command in self.ALLOWED:
            with self.subTest(command=command):
                self.assertEqual(decision(self.settings, command), "allow")

    def test_not_covered(self):
        for command in self.ASKED:
            with self.subTest(command=command):
                self.assertEqual(decision(self.settings, command), "ask")

    def test_look_alikes_are_not_denied(self):
        for command in self.NOT_DENIED:
            with self.subTest(command=command):
                self.assertNotEqual(decision(self.settings, command), "deny")


def make_project(files=(), branch=None):
    """A temporary git project with the hook's inputs, plus a bin/ folder of fake commands that log their calls."""
    tmp = tempfile.TemporaryDirectory(prefix="w5k-hook-")
    base = Path(tmp.name)
    project = base / "project"
    project.mkdir()
    for rel in files:
        (project / rel).parent.mkdir(parents=True, exist_ok=True)
        (project / rel).write_text("x\n")
    env = {**os.environ, "GIT_CONFIG_GLOBAL": os.devnull}
    subprocess.run(["git", "init", "-q", "-b", branch or "main", str(project)], check=True, env=env)
    return tmp, base, project


def fake_command(bin_dir: Path, name: str, exit_code: int = 0, sleep: float = 0):
    """An executable `name` that appends 'name args... (in <cwd>)' to calls.log and then exits with exit_code."""
    path = bin_dir / name
    path.write_text(f'#!/bin/bash\necho "{name} $* (in ${{PWD##*/}})" >> "{bin_dir}/calls.log"\n'
                    + (f"sleep {sleep}\n" if sleep else "") + f"exit {exit_code}\n")
    path.chmod(0o755)


@unittest.skipUnless(HOOK.is_file() and shutil.which("bash"), "the hook script or bash is not available")
class SessionStartHook(unittest.TestCase):
    def setUp(self):
        self.tmp, self.base, self.project = None, None, None

    def tearDown(self):
        if self.tmp:
            self.tmp.cleanup()

    def start(self, files=(), branch=None, cargo=0, npm=0, cargo_sleep=0, remote="true", with_cargo=True, with_npm=True, extra_env=None):
        self.tmp, self.base, self.project = make_project(files, branch)
        self.bin = self.base / "bin"
        self.bin.mkdir()
        if with_cargo:
            fake_command(self.bin, "cargo", cargo, cargo_sleep)
        if with_npm:
            fake_command(self.bin, "npm", npm)
        for tool in ("git", "timeout", "date", "dirname", "bash", "env", "sleep"):  # the few real tools the hook itself needs
            real = shutil.which(tool)
            if real and not (self.bin / tool).exists():
                (self.bin / tool).symlink_to(real)
        env = {"PATH": str(self.bin), "HOME": str(self.base), "TMPDIR": str(self.base), "CLAUDE_PROJECT_DIR": str(self.project),
               "GIT_CONFIG_GLOBAL": os.devnull}
        if remote is not None:
            env["CLAUDE_CODE_REMOTE"] = remote
        env.update(extra_env or {})
        return self.run_hook(env)

    def run_hook(self, env=None):
        started = time.monotonic()
        proc = subprocess.run([str(HOOK)], capture_output=True, text=True, cwd=self.base, env=env or self.last_env, timeout=60)
        self.last_env = env or self.last_env
        self.elapsed = time.monotonic() - started
        return proc

    def calls(self):
        log = self.bin / "calls.log"
        return log.read_text().splitlines() if log.exists() else []

    # ---- what it does ---------------------------------------------------------------------------------------------------
    def test_does_nothing_outside_cloud_sessions(self):
        for remote in (None, "", "false"):
            with self.subTest(remote=remote):
                proc = self.start(files=["Cargo.toml", "Cargo.lock"], remote=remote)
                self.assertEqual((proc.returncode, proc.stdout, proc.stderr), (0, "", ""))
                self.assertEqual(self.calls(), [])
                self.tmp.cleanup()

    def test_fetches_locked_when_there_is_a_lock_file(self):
        proc = self.start(files=["Cargo.toml", "Cargo.lock"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.calls(), ["cargo fetch --locked (in project)"])

    def test_plain_fetch_without_a_lock_file(self):
        proc = self.start(files=["Cargo.toml"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.calls(), ["cargo fetch (in project)"])

    def test_no_cargo_call_without_a_manifest(self):
        self.start(files=["README.md"])
        self.assertEqual(self.calls(), [])

    def test_npm_ci_runs_in_the_viewer_folder_when_packages_are_missing(self):
        self.start(files=["Cargo.toml", "tools/viewer/package-lock.json"])
        self.assertEqual(self.calls(), ["cargo fetch (in project)", "npm ci --no-audit --no-fund (in viewer)"])

    def test_no_npm_without_a_lock_file(self):
        self.start(files=["Cargo.toml", "tools/viewer/package.json"])
        self.assertEqual(self.calls(), ["cargo fetch (in project)"])

    def test_second_run_skips_packages_that_are_already_installed(self):
        self.start(files=["tools/viewer/package-lock.json"])
        (self.project / "tools/viewer/node_modules").mkdir()
        installed = self.project / "tools/viewer/node_modules/.package-lock.json"
        installed.write_text("{}")
        os.utime(installed, (time.time() + 5, time.time() + 5))  # newer than the lock file
        (self.bin / "calls.log").unlink()
        proc = self.run_hook()
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(self.calls(), [])
        self.assertIn("already installed", proc.stderr)

    def test_a_newer_lock_file_reinstalls(self):
        self.start(files=["tools/viewer/package-lock.json"])
        (self.project / "tools/viewer/node_modules").mkdir()
        installed = self.project / "tools/viewer/node_modules/.package-lock.json"
        installed.write_text("{}")
        os.utime(installed, (time.time() - 100, time.time() - 100))  # older than package-lock.json
        (self.bin / "calls.log").unlink()
        self.run_hook()
        self.assertEqual(self.calls(), ["npm ci --no-audit --no-fund (in viewer)"])

    def test_is_idempotent(self):
        first = self.start(files=["Cargo.toml", "Cargo.lock"])
        second = self.run_hook()
        self.assertEqual((first.returncode, second.returncode), (0, 0))
        self.assertEqual(first.stdout, second.stdout)
        self.assertEqual(self.calls(), ["cargo fetch --locked (in project)"] * 2)

    # ---- it never fails the session ---------------------------------------------------------------------------------------
    def test_failing_steps_are_reported_but_the_exit_status_is_zero(self):
        proc = self.start(files=["Cargo.toml", "Cargo.lock", "tools/viewer/package-lock.json"], cargo=101, npm=1)
        self.assertEqual(proc.returncode, 0)
        self.assertIn("cargo fetch --locked: FAILED (exit 101", proc.stderr)
        self.assertIn("npm ci (tools/viewer): FAILED (exit 1", proc.stderr)
        self.assertIn("Read CLAUDE.md", proc.stdout)  # the reminder still came out

    def test_missing_tools_are_skipped(self):
        proc = self.start(files=["Cargo.toml", "tools/viewer/package-lock.json"], with_cargo=False, with_npm=False)
        self.assertEqual(proc.returncode, 0)
        self.assertIn("cargo not found", proc.stderr)
        self.assertIn("npm not found", proc.stderr)

    def test_a_hanging_step_is_stopped_by_the_time_limit(self):
        proc = self.start(files=["Cargo.toml"], cargo_sleep=30, extra_env={"W5K_SESSION_START_TIMEOUT": "1"})
        self.assertEqual(proc.returncode, 0)
        self.assertLess(self.elapsed, 15)
        self.assertIn("FAILED", proc.stderr)

    def test_unusable_project_dir_still_exits_zero(self):
        self.start(files=["Cargo.toml"])
        env = dict(self.last_env, CLAUDE_PROJECT_DIR="/nonexistent/path")
        proc = subprocess.run([str(HOOK)], capture_output=True, text=True, cwd=self.base, env=env)
        self.assertEqual(proc.returncode, 0)

    def test_finds_the_project_from_its_own_location_when_the_variable_is_missing(self):
        self.start(files=["Cargo.toml"])
        env = {k: v for k, v in self.last_env.items() if k != "CLAUDE_PROJECT_DIR"}
        (self.project / ".claude/hooks").mkdir(parents=True)
        shutil.copy(HOOK, self.project / ".claude/hooks/session-start.sh")
        proc = subprocess.run([str(self.project / ".claude/hooks/session-start.sh")], capture_output=True, text=True, cwd=self.base, env=env)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("cargo fetch (in project)", self.calls())

    # ---- the reminder -----------------------------------------------------------------------------------------------------
    def test_reminder_on_stdout_names_claude_md_and_the_lane_brief(self):
        proc = self.start()
        self.assertIn("Read CLAUDE.md", proc.stdout)
        self.assertIn("docs/swarm/lanes/<lane>.md", proc.stdout)
        self.assertIn("docs/swarm/GUARDRAILS.md", proc.stdout)
        self.assertLess(len(proc.stdout.splitlines()), 8)  # short: it lands in the session's context

    def test_reminder_names_the_lane_from_the_branch_when_the_brief_exists(self):
        proc = self.start(files=["docs/swarm/lanes/chassis.md"], branch="lane/chassis/bump-strip")
        self.assertIn("docs/swarm/lanes/chassis.md (you are on branch lane/chassis/bump-strip)", proc.stdout)

    def test_reminder_is_generic_when_the_lane_brief_is_missing(self):
        proc = self.start(branch="lane/chassis/bump-strip")
        self.assertIn("docs/swarm/lanes/<lane>.md", proc.stdout)

    def test_progress_goes_to_stderr_and_the_log_not_stdout(self):
        proc = self.start(files=["Cargo.toml"])
        self.assertNotIn("cargo fetch", proc.stdout)
        self.assertIn("[session-start] cargo fetch: ok", proc.stderr)
        self.assertTrue((self.base / "w5k-session-start.log").exists())

    def test_script_has_valid_bash_syntax_and_is_fast_when_there_is_nothing_to_do(self):
        self.assertEqual(subprocess.run(["bash", "-n", str(HOOK)]).returncode, 0)
        self.start()
        self.assertLess(self.elapsed, 5)


if __name__ == "__main__":
    unittest.main()
