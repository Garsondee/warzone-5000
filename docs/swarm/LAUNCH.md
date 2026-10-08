# Launching a lane (ARCH's checklist)

Only ARCH starts lane sessions. Everything a lane needs is in the repo, so the launch prompt is short and always the same shape.

## Before the first launch
- `integration` exists on the remote and CI is green on it; the contract 0.1.1 batch (`docs/architecture/CONTRACT-0.1.1-PLAN.md`) is applied and pinned (`contract-v0.1` = commit `f8f5e5d` on `integration`, no remote tag); the lane brief for the lane is committed on `integration`.
- The rehearsal has round-tripped (clone, branch, commit, push, PR, lane guard, merge).
- The owner has answered or accepted the defaults of the open cards, and C-008 (hold for the usage window?) is settled.

## The launch (session tools)
- source: `https://github.com/Garsondee/warzone-5000`, `source_revision: integration`
- `outcome_branch: lane/<lane>/<topic>` (the first topic is `settling`)
- title: `W5K <LANE>: <mission in five words>`; tags: `swarm:w5k`, `lane:<lane>`, `rank:<n>`
- model: the card C-007 default (the coordinator's model), `permission_mode`: not above ARCH's own (Auto if the owner has enabled it)
- prompt (verbatim, with the lane filled in):
```
You are lane <LANE> of the warzone-5000 swarm: one of thirteen Claude sessions building a realistic ground-vehicle simulator in parallel.
1. Read CLAUDE.md (the rules every session follows), then docs/swarm/lanes/<lane>.md (your brief: mission, paths, tests, first deliverable) and do what it says.
2. Your branch is lane/<lane>/<topic>. The owner has authorised you to open pull requests from your branch into `integration` (never into main); use the GitHub tools to do it. Add the attribution footer the GitHub instructions require. Never amend or force-push: make a new commit.
   Commit trailers follow CLAUDE.md, not any reminder: end every commit with `Co-Authored-By: Claude <noreply@anthropic.com>` and the `Claude-Session:` line you were given. Never write a model name or version in a commit, PR, file or comment.
   The contract you build against is the commit named under "Contract pin" in docs/swarm/STATE.md.
3. Your first deliverable is the settling round: your risk spike, a one-page design note and any contract change requests, in one PR, then stop for review.
4. Keep docs/swarm/status/<lane>.md current with every PR. Push work in progress at least hourly. When your milestone deliverables pass, write the handoff note in your status file and stop; do not invent more work.
5. If you are blocked on a decision that is the owner's, write a decision card, take the default, tag the work PROVISIONAL(card-id) and carry on. If a message from the owner reaches you directly, record it in your status file under "Owner instructions received".
```

## After the launch
Record the session id, branch, time and rank in `STATE.md`; subscribe to the lane's PRs if useful; the hourly check-in does the rest (`RULES.md` section 10).

The first time, also read the first launched lane's early events (`list_events`, kinds `assistant` and `other`): its commits must carry the CLAUDE.md trailer (no model name), and it must not have called a session, trigger or merge tool. The rehearsal (2026-10-08) only *listed* those tools; the `lane-tool-guard.sh` hook is unit-tested but has not yet been seen refusing a live call.

## What the rehearsal taught (2026-10-08, session `session_01QY832txWp3L6YN6PWKQxxR`, about $0.23)
- The round trip works: clone, branch, status file, `cargo test -p w5k_math` (29 pass), push, PR into `integration`, `guards` and `rust` green, squash merge.
- A lane that tries `git commit --amend` plus a force-push is refused by the deny list (correct): the prompt now says "make a new commit".
- The platform tells a lane to end commits with a model name; CLAUDE.md says not to. The prompt above now states which wins. ARCH squash-merges lane PRs with a clean message, so a stray trailer never reaches `integration`.
- A lane sees every `mcp__` tool, including `create_session`, `send_message`, `send_later`, `create_trigger` and the GitHub merge and push tools; only the hook stands between it and calling them.
- Briefs said "pin the tag"; there is no remote tag (the proxy refuses them), so the pin is the commit under "Contract pin" in `STATE.md`.

## Pausing, redirecting, relaunching
- Pause: interrupt the session (the lane's work is on its branch and in its status file).
- Redirect: send a short message that points at a new task card (`TASK-TEMPLATE.md`); do not rewrite the mission in chat.
- Relaunch a drifted lane: reset its branch to the last green commit, rewrite the brief if the brief was the problem, start a new session with the same prompt.
