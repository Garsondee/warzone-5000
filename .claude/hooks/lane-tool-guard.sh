#!/bin/bash
# PreToolUse hook: keeps LANE sessions away from the tools that only ARCH's own session may use (cost and history control).
#
# A lane session (its git branch is lane/<lane>/<topic>) must never create sessions, triggers or schedules, merge pull
# requests, or write to GitHub through the API. ARCH launches sessions and merges; a lane that needs one of those files an
# interface request or a decision card instead (docs/swarm/GUARDRAILS.md).
#
# The contract:
#   * Claude Code runs this hook before the tools matched in .claude/settings.json and passes the call as JSON on stdin
#     ({"tool_name": "...", "tool_input": {...}}). The matcher in settings.json only decides WHEN the hook runs; the list of
#     blocked tools below is checked again here, so this script is correct on its own.
#   * A call is blocked only when BOTH hold: the tool is one of the blocked ones AND the current git branch is lane/*/*.
#     Blocking means exit status 2 with the reason on stderr (Claude Code shows it to the model).
#   * Everything else is allowed with exit status 0, and so is every error of any kind (no stdin, garbage JSON, not a git
#     repository, ...): this hook must never break ARCH's session. ARCH works on claude/..., arch/... or integration.
#   * No jq. python3 -I parses the JSON; without python3 a plain grep reads the first "tool_name".
#
# One exception, by the owner's instruction (2026-10-10): a lane may call send_message to ARCH's own session, and to nothing else, so
# lanes and ARCH can hold a conversation. ARCH's session id is in .claude/hooks/arch-session-id (or $W5K_ARCH_SESSION); update that
# file when ARCH's session changes. Sending to any other session, or with no session id, stays blocked.
#
# This is a guard against accidents and runaway cost, not a security boundary: a session that deliberately switches branches
# can get around it.

# No `set -e` on purpose: every failure below falls through to "allow".

input="$(cat 2>/dev/null)" || input=""

# ---- which tool is being called? ----------------------------------------------------------------------------------------
tool=""
if command -v python3 >/dev/null 2>&1; then
  tool="$(printf '%s' "$input" | python3 -I -c '
import json, sys
try:
    data = json.load(sys.stdin)
    name = data.get("tool_name") if isinstance(data, dict) else None
    print(name if isinstance(name, str) else "")
except Exception:
    print("")
' 2>/dev/null)" || tool=""
else
  # Fallback: the first "tool_name" in the payload (it comes before tool_input).
  tool="$(printf '%s' "$input" | grep -o '"tool_name"[[:space:]]*:[[:space:]]*"[^"]*"' 2>/dev/null | head -n 1 | sed 's/.*"\([^"]*\)"$/\1/')" || tool=""
fi

# ---- who is a send_message addressed to? (only read for that one tool) -------------------------------------------------------
target=""
if [ "$tool" = "mcp__claude-code-remote__send_message" ] && command -v python3 >/dev/null 2>&1; then
  target="$(printf '%s' "$input" | python3 -I -c '
import json, sys
try:
    data = json.load(sys.stdin)
    ti = data.get("tool_input") if isinstance(data, dict) else None
    sid = ti.get("session_id") if isinstance(ti, dict) else None
    print(sid.strip() if isinstance(sid, str) else "")
except Exception:
    print("")
' 2>/dev/null)" || target=""
fi

# ---- is it one of the blocked tools? ------------------------------------------------------------------------------------
case "$tool" in
  # Starting, steering, renaming or archiving sessions; triggers and schedules; attaching repositories. The read-only tools of the
  # same server (get_session, list_sessions, list_events, read_documentation, subscribe_pr_activity, ...) stay available: a lane
  # that is blocked by its environment needs read_documentation to tell the owner which setting to change.
  mcp__claude-code-remote__create_session | mcp__claude-code-remote__send_message | mcp__claude-code-remote__interrupt_session) ;;
  mcp__claude-code-remote__archive_session | mcp__claude-code-remote__unarchive_session) ;;
  mcp__claude-code-remote__set_session_tags | mcp__claude-code-remote__set_session_title) ;;
  mcp__claude-code-remote__create_trigger | mcp__claude-code-remote__update_trigger | mcp__claude-code-remote__fire_trigger) ;;
  mcp__claude-code-remote__delete_trigger | mcp__claude-code-remote__send_later) ;;
  mcp__claude-code-remote__add_repo | mcp__claude-code-remote__register_repo_root) ;;
  mcp__github__merge_pull_request | mcp__github__enable_pr_auto_merge | mcp__github__disable_pr_auto_merge) ;;
  mcp__github__push_files | mcp__github__create_or_update_file | mcp__github__delete_file) ;;
  *) exit 0 ;;
esac

# ---- is this a lane session? --------------------------------------------------------------------------------------------
dir="${CLAUDE_PROJECT_DIR:-.}"
[ -d "$dir" ] || dir="."
branch="$(git -C "$dir" symbolic-ref --short -q HEAD 2>/dev/null)" || branch=""
if ! [[ "$branch" =~ ^lane/[^/]+/.+$ ]]; then
  exit 0
fi

# The one exception: a message to ARCH's own session (see the header). Its id comes from the environment or the tracked file.
if [ "$tool" = "mcp__claude-code-remote__send_message" ] && [ -n "$target" ]; then
  arch_id="${W5K_ARCH_SESSION:-}"
  if [ -z "$arch_id" ] && [ -r "$dir/.claude/hooks/arch-session-id" ]; then
    arch_id="$(head -n 1 "$dir/.claude/hooks/arch-session-id" 2>/dev/null | tr -d '[:space:]')" || arch_id=""
  fi
  if [ -n "$arch_id" ] && [ "$target" = "$arch_id" ]; then
    exit 0
  fi
fi

lane="${branch#lane/}"
lane="${lane%%/*}"
shown_tool="${tool//[^[:print:]]/?}"
{
  echo "BLOCKED by .claude/hooks/lane-tool-guard.sh: lane sessions may not use $shown_tool."
  echo "You are on branch $branch (lane $lane). Only ARCH's own session starts sessions, triggers and schedules, merges pull requests"
  echo "or writes to GitHub through the API; that keeps the swarm's cost and history under control."
  echo "Exception: send_message to ARCH's own session is allowed (id in .claude/hooks/arch-session-id); to any other session it is not."
  echo "What to do instead:"
  echo "  - Need something merged, or another session started? Push your branch, open a pull request into integration, and say so in"
  echo "    docs/swarm/status/$lane.md. ARCH launches sessions and merges."
  echo "  - Need something from another lane, or a rule changed? File an interface request: docs/swarm/requests/$lane-<topic>.md."
  echo "  - Have a question with options? Put a decision card in your status file under 'Cards needed' and carry on with the default."
} >&2
exit 2
