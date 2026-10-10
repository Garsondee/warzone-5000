"""Tests for .claude/hooks/lane-tool-guard.sh, the PreToolUse hook that keeps lane sessions away from the tools only ARCH's own
session may use (creating sessions, triggers and schedules, merging, writing to GitHub through the API), and for the way
.claude/settings.json registers it.

The hook was also run through the real Claude Code engine (headless, with fake MCP servers named github and claude-code-remote)
when it was written: on a lane branch the six blocked GitHub tools and every claude-code-remote tool were refused with the
hook's message, the other GitHub tools ran, and on arch/..., claude/... and integration branches nothing was blocked."""

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
HOOK = ROOT / ".claude/hooks/lane-tool-guard.sh"
SETTINGS = ROOT / ".claude/settings.json"

SEND = "mcp__claude-code-remote__send_message"
BLOCKED_GITHUB = ["merge_pull_request", "enable_pr_auto_merge", "disable_pr_auto_merge", "push_files", "create_or_update_file", "delete_file"]
BLOCKED = [f"mcp__github__{t}" for t in BLOCKED_GITHUB] + [
    f"mcp__claude-code-remote__{t}" for t in
    "create_session create_trigger send_later send_message update_trigger delete_trigger fire_trigger archive_session "
    "unarchive_session interrupt_session set_session_tags set_session_title add_repo register_repo_root".split()
]
# Read-only (or harmless) tools of the same server: lanes keep them. read_documentation matters most: a lane blocked by its
# environment uses it to tell the owner which setting to change.
REMOTE_ALLOWED = [
    f"mcp__claude-code-remote__{t}" for t in
    "list_sessions get_session list_events get_event list_environments list_repos read_documentation list_triggers get_trigger "
    "subscribe_pr_activity unsubscribe_pr_activity watch_url unwatch_url".split()
]
ALLOWED_GITHUB = ("create_pull_request update_pull_request pull_request_read list_pull_requests add_issue_comment get_file_contents "
                  "list_commits get_commit list_branches search_code actions_get actions_list get_job_logs get_check_run issue_read").split()
OTHER = ["Bash", "Edit", "Write", "Read", "Glob", "Grep", "Task", "WebFetch", "mcp__other__merge_pull_request"] + [
    f"mcp__github__{t}" for t in ALLOWED_GITHUB]


def payload(tool, **tool_input):
    return json.dumps({"session_id": "s1", "hook_event_name": "PreToolUse", "tool_name": tool, "tool_input": tool_input})


@unittest.skipUnless(HOOK.is_file() and shutil.which("bash") and shutil.which("git"), "the hook, bash or git is not available")
class LaneToolGuard(unittest.TestCase):
    _repos = {}
    _tmp = None

    @classmethod
    def setUpClass(cls):
        cls._tmp = tempfile.TemporaryDirectory(prefix="w5k-lane-guard-")

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    @classmethod
    def repo(cls, branch, commits=True, detached=False):
        """A git repository whose current branch is `branch` (cached per branch)."""
        key = (branch, commits, detached)
        if key not in cls._repos:
            path = Path(cls._tmp.name) / f"repo{len(cls._repos)}"
            path.mkdir()
            env = {**os.environ, "GIT_CONFIG_GLOBAL": os.devnull}
            subprocess.run(["git", "init", "-q", "-b", branch, str(path)], check=True, env=env)
            if commits:
                (path / "a.txt").write_text("a\n")
                git = ["git", "-C", str(path), "-c", "user.name=t", "-c", "user.email=t@t"]
                subprocess.run([*git, "add", "-A"], check=True, env=env)
                subprocess.run([*git, "commit", "-q", "-m", "base"], check=True, env=env)
                if detached:
                    subprocess.run([*git, "checkout", "-q", "--detach"], check=True, env=env)
            cls._repos[key] = path
        return cls._repos[key]

    def run_hook(self, stdin, repo=None, *, env=None, cwd=None, path=None):
        repo = repo or self.repo("lane/chassis/x")
        full_env = {"PATH": path or os.environ["PATH"], "HOME": str(repo), "GIT_CONFIG_GLOBAL": os.devnull, "CLAUDE_PROJECT_DIR": str(repo)}
        full_env.update(env or {})
        started = time.monotonic()
        proc = subprocess.run([str(HOOK)], input=stdin, capture_output=True, text=not isinstance(stdin, bytes),
                              cwd=cwd or repo, env=full_env, timeout=30)
        self.elapsed = time.monotonic() - started
        return proc

    # ---- the four cases from the brief --------------------------------------------------------------------------------------
    def test_lane_branch_and_a_blocked_tool_is_blocked_with_status_2(self):
        for tool in BLOCKED:
            with self.subTest(tool=tool):
                proc = self.run_hook(payload(tool, anything=1))
                self.assertEqual(proc.returncode, 2, proc.stderr)
                self.assertEqual(proc.stdout, "")

    def test_lane_branch_and_any_other_tool_is_allowed(self):
        for tool in OTHER:
            with self.subTest(tool=tool):
                proc = self.run_hook(payload(tool, command="ls"))
                self.assertEqual((proc.returncode, proc.stdout, proc.stderr), (0, "", ""))

    def test_lane_branch_keeps_the_read_only_session_tools(self):
        for tool in REMOTE_ALLOWED:
            with self.subTest(tool=tool):
                proc = self.run_hook(payload(tool), self.repo("lane/drive/engine-map"))
                self.assertEqual(proc.returncode, 0, proc.stderr)

    # ---- the one exception: a lane may message ARCH's own session and nobody else ---------------------------------------------
    ARCH_ID = "session_01ArchArchArchArchArchArc"

    def test_a_lane_may_send_a_message_to_arch_and_to_no_other_session(self):
        env = {"W5K_ARCH_SESSION": self.ARCH_ID}
        ok = self.run_hook(payload(SEND, session_id=self.ARCH_ID, message="blocked on X"), env=env)
        self.assertEqual((ok.returncode, ok.stderr), (0, ""))
        for target in ("session_01SomeOtherLane", "", "@parent", None):
            with self.subTest(target=target):
                kwargs = {"message": "hi"} if target is None else {"session_id": target, "message": "hi"}
                proc = self.run_hook(payload(SEND, **kwargs), env=env)
                self.assertEqual(proc.returncode, 2, proc.stderr)
                self.assertIn("ARCH", proc.stderr)

    def test_the_arch_session_id_can_come_from_the_tracked_file_and_a_missing_id_blocks_everyone(self):
        repo = self.repo("lane/world/messaging-file")
        hooks = repo / ".claude" / "hooks"
        hooks.mkdir(parents=True, exist_ok=True)
        (hooks / "arch-session-id").write_text(self.ARCH_ID + "\n")
        self.assertEqual(self.run_hook(payload(SEND, session_id=self.ARCH_ID), repo).returncode, 0)
        self.assertEqual(self.run_hook(payload(SEND, session_id="session_01Other"), repo).returncode, 2)
        (hooks / "arch-session-id").write_text("\n")  # an empty file means there is no ARCH to talk to
        self.assertEqual(self.run_hook(payload(SEND, session_id=self.ARCH_ID), repo).returncode, 2)
        (hooks / "arch-session-id").unlink()
        self.assertEqual(self.run_hook(payload(SEND, session_id=self.ARCH_ID), repo).returncode, 2)

    def test_the_messaging_exception_opens_no_other_tool(self):
        env = {"W5K_ARCH_SESSION": self.ARCH_ID}
        for tool in BLOCKED:
            if tool == SEND:
                continue
            with self.subTest(tool=tool):
                proc = self.run_hook(payload(tool, session_id=self.ARCH_ID), env=env)
                self.assertEqual(proc.returncode, 2, proc.stderr)

    def test_arch_claude_and_integration_branches_are_never_blocked(self):
        for branch in ("arch/guard-update", "arch/a/b", "claude/sharp-babbage-d702f7", "integration", "main"):
            for tool in BLOCKED:
                with self.subTest(branch=branch, tool=tool):
                    proc = self.run_hook(payload(tool), self.repo(branch))
                    self.assertEqual((proc.returncode, proc.stderr), (0, ""))

    def test_garbage_input_is_allowed(self):
        garbage = ["", "\n", "not json at all", "[]", "{}", "null", "42", '"mcp__github__merge_pull_request"',
                   '{"tool_name": 5}', '{"tool_name": null}', '{"tool_name": ["mcp__github__merge_pull_request"]}',
                   '{"tool_input": {"tool_name": "mcp__github__merge_pull_request"}}',
                   '{"tool_name": "mcp__github__merge_pull_request', "{'tool_name': 'mcp__github__merge_pull_request'}",
                   b"\xff\xfe\x00\x01 binary \x80", "x" * 2_000_000]
        for text in garbage:
            with self.subTest(text=(text[:40] if isinstance(text, str) else text[:12])):
                proc = self.run_hook(text)
                self.assertEqual(proc.returncode, 0, proc.stderr)

    # ---- details ------------------------------------------------------------------------------------------------------------
    def test_the_message_says_why_and_what_to_do_instead(self):
        proc = self.run_hook(payload("mcp__claude-code-remote__create_session"), self.repo("lane/drive/engine-map"))
        err = proc.stderr
        for needle in ("mcp__claude-code-remote__create_session", "lane/drive/engine-map", "lane drive", "ARCH", "interface request",
                       "docs/swarm/requests/drive-<topic>.md", "decision card", "docs/swarm/status/drive.md", "pull request"):
            self.assertIn(needle, err)
        self.assertLess(len(err.splitlines()), 12)

    def test_only_lane_branches_with_a_lane_and_a_topic_count(self):
        for branch in ("lane", "lane/chassis", "archive/lane/x", "feature/lane/x", "lanes/a/b", "Lane/a/b", "laneX/a/b"):
            with self.subTest(branch=branch):
                try:
                    repo = self.repo(branch)
                except subprocess.CalledProcessError:
                    self.skipTest(f"git refuses the branch name {branch}")
                self.assertEqual(self.run_hook(payload(BLOCKED[0]), repo).returncode, 0)
        self.assertEqual(self.run_hook(payload(BLOCKED[0]), self.repo("lane/a/b/c/d")).returncode, 2)  # deep topics are lane branches

    def test_a_lane_branch_without_commits_is_still_a_lane_branch(self):
        self.assertEqual(self.run_hook(payload(BLOCKED[0]), self.repo("lane/chassis/fresh", commits=False)).returncode, 2)

    def test_a_detached_head_is_allowed(self):
        self.assertEqual(self.run_hook(payload(BLOCKED[0]), self.repo("lane/chassis/x", detached=True)).returncode, 0)

    def test_outside_a_git_repository_is_allowed(self):
        with tempfile.TemporaryDirectory() as plain:
            self.assertEqual(self.run_hook(payload(BLOCKED[0]), Path(plain)).returncode, 0)

    def test_without_the_project_dir_variable_the_current_directory_is_used(self):
        repo = self.repo("lane/chassis/x")
        env = {"CLAUDE_PROJECT_DIR": "/nonexistent/dir"}
        self.assertEqual(self.run_hook(payload(BLOCKED[0]), repo, env=env, cwd=repo).returncode, 2)
        with tempfile.TemporaryDirectory() as plain:
            self.assertEqual(self.run_hook(payload(BLOCKED[0]), repo, env=env, cwd=plain).returncode, 0)

    def test_no_stdin_at_all(self):
        repo = self.repo("lane/chassis/x")
        env = {**os.environ, "CLAUDE_PROJECT_DIR": str(repo), "GIT_CONFIG_GLOBAL": os.devnull}
        proc = subprocess.run([str(HOOK)], stdin=subprocess.DEVNULL, capture_output=True, text=True, cwd=repo, env=env)
        self.assertEqual(proc.returncode, 0)

    def test_a_decoy_tool_name_inside_tool_input_does_not_count(self):
        decoy = json.dumps({"tool_name": "Bash", "tool_input": {"command": "echo", "note": '"tool_name": "mcp__github__merge_pull_request"'}})
        self.assertEqual(self.run_hook(decoy).returncode, 0)

    def test_it_is_quick(self):
        self.run_hook(payload(BLOCKED[0]))
        self.assertLess(self.elapsed, 3)

    # ---- without python3 -----------------------------------------------------------------------------------------------------
    def test_works_without_python3_through_the_grep_fallback(self):
        bin_dir = Path(self._tmp.name) / "nopython-bin"
        bin_dir.mkdir(exist_ok=True)
        for tool in ("bash", "git", "grep", "sed", "head", "cat"):
            real = shutil.which(tool)
            self.assertTrue(real, tool)
            if not (bin_dir / tool).exists():
                (bin_dir / tool).symlink_to(real)
        self.assertFalse(shutil.which("python3", path=str(bin_dir)))
        for tool in BLOCKED:
            self.assertEqual(self.run_hook(payload(tool), path=str(bin_dir)).returncode, 2, tool)
        for tool in OTHER:
            self.assertEqual(self.run_hook(payload(tool), path=str(bin_dir)).returncode, 0, tool)
        for text in ("", "not json", "[]", '{"tool_name": 5}', b"\xff\xfe\x00"):
            self.assertEqual(self.run_hook(text, path=str(bin_dir)).returncode, 0, text)
        decoy = json.dumps({"tool_name": "Bash", "tool_input": {"note": '"tool_name": "mcp__github__merge_pull_request"'}})
        self.assertEqual(self.run_hook(decoy, path=str(bin_dir)).returncode, 0)

    # ---- the script itself ---------------------------------------------------------------------------------------------------
    def test_script_is_executable_has_valid_syntax_and_never_uses_set_e(self):
        self.assertTrue(HOOK.stat().st_mode & stat.S_IXUSR)
        self.assertEqual(subprocess.run(["bash", "-n", str(HOOK)]).returncode, 0)
        code = "\n".join(l for l in HOOK.read_text().splitlines() if not l.lstrip().startswith("#"))
        self.assertNotRegex(code, r"(?m)^\s*set\s+-[a-z]*e")  # a failing command must fall through to 'allow'
        self.assertNotIn("jq", code)  # no jq dependency


@unittest.skipUnless(SETTINGS.is_file() and HOOK.is_file(), ".claude/settings.json or the hook not found")
class Registration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.settings = json.loads(SETTINGS.read_text())
        (cls.group,) = cls.settings["hooks"]["PreToolUse"]
        cls.matcher = re.compile(cls.group["matcher"])

    def test_registered_as_a_command_hook_for_the_script(self):
        (hook,) = self.group["hooks"]
        self.assertEqual(hook["type"], "command")
        self.assertEqual(hook["command"], '"$CLAUDE_PROJECT_DIR"/.claude/hooks/lane-tool-guard.sh')
        self.assertGreater(hook["timeout"], 0)

    def test_the_matcher_is_exactly_what_ARCH_asked_for(self):
        self.assertEqual(
            self.group["matcher"],
            "mcp__claude-code-remote__.*|mcp__github__(merge_pull_request|enable_pr_auto_merge|disable_pr_auto_merge|push_files|create_or_update_file|delete_file)",
        )

    def test_the_matcher_matches_every_blocked_tool_however_it_is_anchored(self):
        for tool in BLOCKED:
            with self.subTest(tool=tool):
                self.assertTrue(self.matcher.fullmatch(tool))
                self.assertTrue(self.matcher.search(tool))

    def test_the_matcher_matches_no_other_tool_even_unanchored(self):
        for tool in OTHER:
            with self.subTest(tool=tool):
                self.assertIsNone(self.matcher.search(tool))

    def test_the_script_and_the_matcher_agree_on_the_github_tools(self):
        in_script = set(re.findall(r"mcp__github__[a-z_]+", HOOK.read_text()))
        in_matcher = {f"mcp__github__{t}" for t in re.search(r"\(([^)]*)\)", self.group["matcher"]).group(1).split("|")}
        self.assertEqual(in_script, in_matcher)
        self.assertIn("mcp__claude-code-remote__", HOOK.read_text())

    def test_the_allow_list_gives_lanes_the_pull_request_tools_but_none_of_the_blocked_ones(self):
        allow = self.settings["permissions"]["allow"]
        for tool in ALLOWED_GITHUB:
            self.assertIn(f"mcp__github__{tool}", allow)
        for tool in BLOCKED:
            if tool == SEND:
                continue  # allowed by the owner's instruction (2026-10-10): the hook still limits its target to ARCH's session
            self.assertNotIn(tool, allow)
        for broad in ("mcp__github", "mcp__claude-code-remote", "mcp__github__*", "mcp__*"):
            self.assertNotIn(broad, allow)


if __name__ == "__main__":
    unittest.main()
