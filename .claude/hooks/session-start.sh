#!/bin/bash
# SessionStart hook for Claude Code cloud sessions: get the dependencies ready so a lane can build and test straight away,
# and remind the session of the two documents it must read first.
#
# The contract (see docs/swarm/GUARDRAILS.md):
#   * It does something only when CLAUDE_CODE_REMOTE=true (a cloud session). Anywhere else it prints nothing and exits.
#   * It NEVER fails the session: there is no `set -e`, every step may fail, errors are logged, the exit status is 0.
#   * It is non-interactive, bounded in time (every network step has a time limit) and idempotent: running it again is
#     harmless, and the second run is fast because it skips what is already installed.
#   * What it prints on stdout becomes part of the session's context, so stdout carries only the short reminder;
#     progress and errors go to stderr and to the log file.
#
# Steps: banner; `cargo fetch --locked` (plain `cargo fetch` when there is no Cargo.lock yet); `npm ci` in tools/viewer
# when it has a package-lock.json and the packages are missing or out of date.

[ "${CLAUDE_CODE_REMOTE:-}" = "true" ] || exit 0

ROOT="${CLAUDE_PROJECT_DIR:-}"
if [ -z "$ROOT" ] || [ ! -d "$ROOT" ]; then
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." 2>/dev/null && pwd)"
fi
if ! cd "$ROOT" 2>/dev/null; then
  echo "[session-start] cannot enter the project folder ($ROOT); skipping setup" >&2
  exit 0
fi

LOG="${TMPDIR:-/tmp}/w5k-session-start.log"
STEP_TIMEOUT="${W5K_SESSION_START_TIMEOUT:-420}" # seconds allowed for each step

say() { echo "[session-start] $*" >&2; }

# run LABEL COMMAND [ARGS...]: run one setup step with a time limit and report the result; a failure is only reported.
run() {
  local label="$1"
  shift
  local cmd=("$@")
  if command -v timeout >/dev/null 2>&1; then
    cmd=(timeout "$STEP_TIMEOUT" "$@")
  fi
  echo "--- $(date -u +%H:%M:%SZ) $label" >>"$LOG" 2>/dev/null
  if "${cmd[@]}" >>"$LOG" 2>&1; then
    say "$label: ok"
  else
    say "$label: FAILED (exit $?; details in $LOG). Continuing without it."
  fi
}

# ---- the reminder (stdout, so it lands in the session's context) ---------------------------------------------------------
lane=""
branch="$(git symbolic-ref --short -q HEAD 2>/dev/null)" # works before the first commit; empty when HEAD is detached
case "$branch" in
  lane/*/*)
    lane="${branch#lane/}"
    lane="${lane%%/*}"
    ;;
esac
echo "Warzone 5000 swarm session. Before you change anything:"
echo "  1. Read CLAUDE.md (the rules every session follows)."
if [ -n "$lane" ] && [ -f "docs/swarm/lanes/$lane.md" ]; then
  echo "  2. Read your lane brief: docs/swarm/lanes/$lane.md (you are on branch $branch)."
else
  echo "  2. Read your lane brief: docs/swarm/lanes/<lane>.md (the lane is the middle part of your branch name lane/<lane>/<topic>)."
fi
echo "  3. Edit only your lane's paths (docs/swarm/ownership.toml); for anything else file an interface request. CI checks this."
echo "What each CI check wants, and how to get an exception: docs/swarm/GUARDRAILS.md"

# ---- Rust dependencies ---------------------------------------------------------------------------------------------------
if [ -f Cargo.toml ]; then
  if command -v cargo >/dev/null 2>&1; then
    if [ -f Cargo.lock ]; then
      run "cargo fetch --locked" cargo fetch --locked
    else
      run "cargo fetch" cargo fetch
    fi
  else
    say "cargo not found; skipping the Rust dependencies"
  fi
fi

# ---- the reference viewer's packages -------------------------------------------------------------------------------------
VIEWER=tools/viewer
if [ -f "$VIEWER/package-lock.json" ]; then
  if ! command -v npm >/dev/null 2>&1; then
    say "npm not found; skipping the viewer packages"
  elif [ -d "$VIEWER/node_modules" ] && [ ! "$VIEWER/package-lock.json" -nt "$VIEWER/node_modules/.package-lock.json" ]; then
    say "viewer packages already installed"
  else
    (cd "$VIEWER" && run "npm ci (tools/viewer)" npm ci --no-audit --no-fund)
  fi
fi

exit 0
