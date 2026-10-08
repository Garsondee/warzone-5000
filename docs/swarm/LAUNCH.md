# Launching a lane (ARCH's checklist)

Only ARCH starts lane sessions. Everything a lane needs is in the repo, so the launch prompt is short and always the same shape.

## Before the first launch
- `integration` exists on the remote and CI is green on it; `contract-v0.1` is tagged; the lane brief for the lane is committed on `integration`.
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
2. Your branch is lane/<lane>/<topic>. The owner has authorised you to open pull requests from your branch into `integration` (never into main); use the GitHub tools to do it. Add the attribution footer the GitHub instructions require.
3. Your first deliverable is the settling round: your risk spike, a one-page design note and any contract change requests, in one PR, then stop for review.
4. Keep docs/swarm/status/<lane>.md current with every PR. Push work in progress at least hourly. When your milestone deliverables pass, write the handoff note in your status file and stop; do not invent more work.
5. If you are blocked on a decision that is the owner's, write a decision card, take the default, tag the work PROVISIONAL(card-id) and carry on. If a message from the owner reaches you directly, record it in your status file under "Owner instructions received".
```

## After the launch
Record the session id, branch, time and rank in `STATE.md`; subscribe to the lane's PRs if useful; the hourly check-in does the rest (`RULES.md` section 10).

## Pausing, redirecting, relaunching
- Pause: interrupt the session (the lane's work is on its branch and in its status file).
- Redirect: send a short message that points at a new task card (`TASK-TEMPLATE.md`); do not rewrite the mission in chat.
- Relaunch a drifted lane: reset its branch to the last green commit, rewrite the brief if the brief was the problem, start a new session with the same prompt.
