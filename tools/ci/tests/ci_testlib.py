"""Helpers for the tools/ci tests: throw-away git repositories and a way to run a check's main() and capture its output.

Run all the tests from the repository root with:  python3 -I -m unittest discover -s tools/ci/tests
"""

from __future__ import annotations

import contextlib
import io
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True  # keep tools/ci free of __pycache__
CI_DIR = Path(__file__).resolve().parent.parent  # tools/ci
ROOT = CI_DIR.parent.parent  # the repository root (or the staged tree it is being built in)
if str(CI_DIR) not in sys.path:
    sys.path.append(str(CI_DIR))

# Make git (the tests' own calls and the checks' calls) ignore the machine's git configuration.
os.environ["GIT_CONFIG_GLOBAL"] = os.devnull
os.environ["GIT_CONFIG_NOSYSTEM"] = "1"
os.environ["GIT_TERMINAL_PROMPT"] = "0"
os.environ.setdefault("GIT_AUTHOR_NAME", "test")
os.environ.setdefault("GIT_AUTHOR_EMAIL", "test@example.com")
os.environ.setdefault("GIT_COMMITTER_NAME", "test")
os.environ.setdefault("GIT_COMMITTER_EMAIL", "test@example.com")


class TempRepo:
    """A git repository in a temporary directory, with `main` as the first branch."""

    def __init__(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="w5k-ci-")
        self.path = Path(self._tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "commit.gpgsign", "false")
        # No background housekeeping: `git commit` can start `git maintenance`/`gc` detached, and it may still be writing to .git
        # when the test ends, which made the temp-directory cleanup fail now and then ("Directory not empty: .../.git") on CI.
        self.git("config", "gc.auto", "0")
        self.git("config", "gc.autoDetach", "false")
        self.git("config", "maintenance.auto", "false")

    def git(self, *args: str) -> str:
        proc = subprocess.run(["git", "-C", str(self.path), *args], capture_output=True, text=True)
        if proc.returncode != 0:
            raise AssertionError(f"git {' '.join(args)} failed: {proc.stderr}")
        return proc.stdout.strip()

    def write(self, rel: str, content: str | bytes = "") -> Path:
        target = self.path / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, str):
            content = content.encode("utf-8")
        target.write_bytes(content)
        return target

    def remove(self, rel: str) -> None:
        (self.path / rel).unlink()

    def move(self, old: str, new: str) -> None:
        (self.path / new).parent.mkdir(parents=True, exist_ok=True)
        self.git("mv", old, new)

    def commit(self, message: str = "commit") -> str:
        self.git("add", "-A")
        self.git("commit", "-q", "--allow-empty", "-m", message)
        return self.git("rev-parse", "HEAD")

    def branch(self, name: str) -> None:
        self.git("checkout", "-q", "-b", name)

    def checkout(self, name: str) -> None:
        self.git("checkout", "-q", name)

    def cleanup(self) -> None:
        # Retry: a straggling git process can still be writing into .git for a moment; the directory is only a scratch copy.
        for attempt in range(5):  # const-ok: retry count for a flaky filesystem race, not a physical constant
            try:
                self._tmp.cleanup()
                return
            except OSError:
                time.sleep(0.2 * (attempt + 1))  # const-ok: back-off seconds
        shutil.rmtree(self._tmp.name, ignore_errors=True)


def run_main(main, argv, env: dict | None = None) -> tuple[int, str, str]:
    """Call a check's main(argv) in this process; return (exit code, stdout, stderr).

    GITHUB_ACTIONS is removed from the environment unless the test sets it, so annotation output is opt-in."""
    import ci_common

    ci_common.reset_annotation_counts()
    overrides = {"GITHUB_ACTIONS": None, **(env or {})}
    saved = {k: os.environ.get(k) for k in overrides}
    for key, value in overrides.items():
        if value is None:
            os.environ.pop(key, None)
        else:
            os.environ[key] = value
    out, err = io.StringIO(), io.StringIO()
    try:
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = main(list(argv))
            except SystemExit as exc:  # argparse errors
                code = exc.code if isinstance(exc.code, int) else (0 if exc.code is None else 1)
    finally:
        for key, value in saved.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
    return code, out.getvalue(), err.getvalue()


def run_script(name: str, *args: str, cwd=None, env: dict | None = None) -> subprocess.CompletedProcess:
    """Run tools/ci/<name> exactly the way CI does: `python3 -I script args...` in a separate process."""
    full_env = {k: v for k, v in os.environ.items() if k != "GITHUB_ACTIONS"}
    full_env.update(env or {})
    return subprocess.run(
        [sys.executable, "-I", str(CI_DIR / name), *args], capture_output=True, text=True, cwd=cwd, env=full_env
    )


def run_blocks(text):
    """Yield (line number, text) of every `run:` value, inline or block scalar."""
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        m = re.match(r"^(\s*)(- )?run:\s*(.*)$", lines[i])
        if m:
            key_indent = len(m.group(1)) + (2 if m.group(2) else 0)
            value = m.group(3)
            if value[:1] in ("|", ">"):
                j, body = i + 1, []
                while j < len(lines) and (not lines[j].strip() or len(lines[j]) - len(lines[j].lstrip()) > key_indent):
                    body.append(lines[j])
                    j += 1
                yield i + 1, "\n".join(body)
                i = j
                continue
            yield i + 1, value
        i += 1
