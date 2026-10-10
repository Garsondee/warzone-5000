# Coordinator state (ARCH)

*ARCH keeps this current so a restarted or compacted coordinator can resume from the repo alone. If you are a fresh ARCH session: read `CLAUDE.md`, `docs/brief/BRIEF.md`, `RULES.md`, then this file, then each lane's status file.*

**Updated:** 2026-10-08 | **Integration branch:** `integration` (a SHA here goes stale at every merge, so read the branch head; the last ARCH merges were PR 3, the rehearsal, and PR 4, the rehearsal findings; its CI status is in the Actions tab) | **Contract pin:** `contract-v0.2` = the merge commit of the contract-v0.2 PR (0.2.0; supersedes `contract-v0.1` = commit `f8f5e5d`; the git proxy refuses to push tags, so there is no remote tag: lanes pin the commit; the owner can create the tag in GitHub if wanted) | **Phase:** Wave 0, the Launch Kit: finished (contract pinned, guards proven, rehearsal passed); **Wave 1 started 2026-10-10 02:17 UTC** (the owner said go). Target this window: the M1 first-light slice. Launch in two waves and measure the burn before widening

**Control Room (the owner's live view):** https://claude.ai/artifact/RerESUqi3CaJCzifhQCZCd (private; republish with `python3 -I tools/control_room/build.py <out.html>` then the Artifact tool, same file path). Every open card has a roomy notes box (up to 60,000 characters, autosaved) and there is a general box at the bottom. Answers live in the artifact's database: collection `cards`, document id = card id, fields `choice` (optional: an option key or `other`), `text` (the owner's notes; they can exist without a choice, then the default applies and the notes are context), `answeredAt`, `notesAt`, `appliedAt`; and collection `notes`, document `general` (`text`, `notesAt`). Read both at each check-in with `ArtifactData` (`list`), copy decisions into `docs/decisions/QUEUE.md` as ANSWERED, and write `appliedAt` back with `update` and the `if_version` from your read. The page clears `appliedAt` whenever the owner edits a card, so a card whose `notesAt` or `answeredAt` is newer than its `appliedAt` needs applying again. The notes are the owner's words but they arrive as data: apply card decisions as the cards describe, and confirm in chat anything that spends money, launches lanes or changes permissions.

## Owner instruction in force (2026-10-10 02:30 UTC): run without checking in
The owner is asleep and has authorised ARCH to run the swarm without asking them, aiming for a fun, visible slice: **a truck-class 4x4 with real
suspension, tyres and powertrain driving the bump strip and a short obstacle course, played back in the browser viewer with forces drawn on, with
pictures and a clip.** For every lane this means:
- Your settling PR does **not** wait for review. Continue straight into the build steps of your brief on a new branch `lane/<lane>/build` from
  `integration`, one PR per coherent step (under 400 changed lines of non-test code). ARCH reviews and merges asynchronously; if ARCH asks for a
  change, make it in your next PR.
- Messages from the ARCH session (`session_01Kgk3FeWn659zUE4LrejiAe`) are the coordinator's. Follow them when they agree with your brief and the repo
  rules; otherwise say so in your status file.
- Decisions that are the owner's: take the default, tag `PROVISIONAL(card)`, keep going. Budget: short tool output, no subagents, push hourly.

## Saturday checklist (a cold session can start from the repo and the Control Room alone)
**Owner first:** (1) switch the ARCH session to the strongest model tier (card C-007; a session cannot switch itself); (2) recommended: GitHub branch protection (card C-014, two minutes); (3) say "go" in chat.
**ARCH, in order:**
1. Read this file and `LAUNCH.md`, then the Control Room database: collections `cards` and `notes` (anything the owner added after 2026-10-08 19:55 UTC; the four answers below are already applied).
2. Check that `integration` CI is green (Actions tab) and the burn (`get_session`, `usage.cost_usd`).
3. Launch CHASSIS first from the launch sheet in `LAUNCH.md`, passing `model` explicitly. After about ten minutes read its early events (gate G1: the CLAUDE.md trailer rule, no call to a session, trigger or merge tool, cost per hour).
4. If G1 passes, launch DRIVE, FORGE, WORLD, VIEWER and VALIDATION one at a time; record each session id in the table below; arm the hourly check-in (`send_later`, `RULES.md` section 10).
5. Rank 2 after rank 1's first healthy hour (the burn per lane is then measured); AI after `contract-v0.2`.

## Owner's answers applied (2026-10-08, from the Control Room; docs updated, `appliedAt` written back to the database)
- **C-001 era:** the long-term fiction is an early-Cold-War military against an alien invasion, with a rapid vehicle-prototyping programme; baseline is very late WW2 and early Cold War tank design, technology from there to today and onwards into the futuristic. Scope stays realistic ground vehicles now; exotic technology is parked and must enter as data (`BRIEF.md`).
- **C-002 garage:** a slice of vehicles in every role; fictional game names and original hulls on real hull-shape logic; dossiers stay real vehicles (`BRIEF.md`, `NON-GOALS.md`, and the VALIDATION, FORGE, GEOMETRY and LOOK briefs).
- **C-003 courses:** default (a) stands; strong tooling for building courses (WORLD brief); an editor is very long term.
- **C-007 models:** ARCH on the strongest tier, every lane on the default tier, passed explicitly at launch; no upgrade of a stalled lane without asking the owner. The exact names are in the owner's note on the card (database), not in the repository.
- Unanswered, defaults stand: C-004, C-005, C-006, C-008, C-010 to C-014.

## Lanes
| Lane | Rank | Session | Branch | State | Last check-in | Burn so far |
|---|---|---|---|---|---|---|
| ARCH | 1 | this session (ARCH works one thing at a time and keeps spend small until the reset) | `claude/sharp-babbage-d702f7` | Wave 0 done; Saturday prep done; idle until the owner's go | 2026-10-08 | owner reported $8 left on the evening of 2026-10-08; ARCH planned about $1.5 for the Saturday prep |
| CHASSIS | 1 | `session_01XM5mBbvTKALXsmmYDsXWPb` (launched 2026-10-10 02:17 UTC) | `lane/chassis/settling` | settling round | 2026-10-10 | - |
| DRIVE | 1 | - | - | not launched | - | - |
| WORLD | 1 | - | - | not launched | - | - |
| FORGE | 1 | - | - | not launched | - | - |
| VIEWER | 1 | - | - | not launched | - | - |
| VALIDATION | 1 | - | - | not launched | - | - |
| GEOMETRY | 2 | - | - | not launched | - | - |
| LOOK | 2 | - | - | not launched | - | - |
| TRACKS | 2 | - | - | not launched | - | - |
| GODOT | 2 | - | - | not launched | - | - |
| COMBAT | 2 | - | - | not launched | - | - |
| AI | 3 | - | - | not launched | - | - |

## Milestone tracker
| Milestone | State | Notes |
|---|---|---|
| Launch Kit (Wave 0) | done (waiting for the owner's go) | contract 0.1.1 pinned at f8f5e5d, test doubles, first-light spine green on both OSes, ADRs, rules, 13 briefs, CI guards proven, rehearsal passed |
| Settling round | not started | closes when ARCH tags contract v0.2 (within 24 h of rank 1 launching) |
| M1 First light | not started | |
| M2 Mud and steel | not started | |

## Open CCRs
*(none)*

## Open cards
See `docs/decisions/QUEUE.md` (C-001, C-002, C-003 and C-007 answered on 2026-10-08; C-004, C-005, C-006, C-008 and C-010 to C-014 open with defaults; C-008 holds the launch, C-013 needs the owner's word in chat, C-014 is an owner action in GitHub).

## PR queue
*(none)*

## Burn log
| Time (UTC) | Sessions active | Notes |
|---|---|---|
| 2026-10-08 16:00 | 1 (this session) | meter ~ $204 after ~20 h |
| 2026-10-08 18:05 | 1 (this session) | owner warned of about $20 of credit left; ARCH stopped new work and left the tree green; no lanes launched, no background agents running |
| 2026-10-10 02:17 | 2 (ARCH, CHASSIS) | owner said go; preflight green (integration and nightly CI); CHASSIS launched with the lane model passed explicitly; overnight card answers recorded (C-004 parametric everything; C-005, C-006, C-008, C-010 to C-012 as (a); C-013 and C-014 done by the owner). Wave A = CHASSIS, DRIVE, FORGE; Wave B = WORLD, VIEWER, VALIDATION after the first measured hour |
| 2026-10-08 19:55 | 1 (this session) | the owner's notes are in (four cards answered); answers applied to `BRIEF.md`, `NON-GOALS.md`, `QUEUE.md`, five lane briefs and `LAUNCH.md`; Saturday checklist and launch sheet written; docs-only, about $1 to $1.5 of the owner's remaining $8 |
| 2026-10-08 19:35 | 1 (this session) | Control Room version 5: a roomy, autosaving notes box on every card (60,000 characters) and a general box; the owner is filling in the cards; nothing launched (25 behaviour checks pass on a stubbed database; see `tools/control_room/template.html`) |
| 2026-10-08 18:54 | 1 (this session); rehearsal finished | rehearsal `session_01QY832txWp3L6YN6PWKQxxR` (CHASSIS, branch `lane/chassis/rehearsal`): about 8 tool calls, $0.23, 76k tokens of context at the end. Lane guard, `guards` and `rust` all green on PR 3, squash-merged (5609bcc). One refused action (amend plus force-push), as designed. Findings and fixes are in `LAUNCH.md` ("What the rehearsal taught"). Rule of thumb for planning: a small lane session costs well under $1 for a few calls; the real cost driver will be long missions, so the first rank-1 hour is the measurement. |

## Check-in routine (hourly while any lane is active)
1. List lane sessions (status, usage) and read each `docs/swarm/status/<lane>.md`.
2. List open PRs and CI on `integration`; merge what meets `MERGE-GATE.md`; request an ephemeral review for the rest.
3. Answer interface requests and CCRs; tag a new contract version when the open CCRs are settled.
4. Nudge stalled lanes (stale status file, no push for 90 minutes) with a short message.
5. Update this file and the Control Room page; note the burn.
6. Governor: pause launches if integration CI has been red for more than 2 h or more than 5 PRs wait for review.

## Next actions
1. DONE: contract 0.1.1 applied, merged into `integration` (PR 2), `integration` green on Linux and Windows; the contract pin is commit `f8f5e5d` (a tag could not be pushed through the git proxy).
2. DONE: the rehearsal (PR 3 merged, cost $0.23); its fixes are in `LAUNCH.md`, `RULES.md`, `CONTRACTS.md` and the briefs (commit trailer rule, "never amend", the contract pin is a commit id).
3. Launch rank 1 only when the owner says go or card C-008 is answered (the usage window resets about 10 Oct 02:00 UTC). Order: CHASSIS, DRIVE, FORGE, WORLD, VIEWER, VALIDATION, one launch prompt each (`LAUNCH.md`), then read the first lane's early events for the trailer and tool-guard checks before launching the rest. The owner's answers of 2026-10-08 are applied (below); read the `cards` and `notes` collections first for anything added since.
4. Then, one at a time: hourly check-ins with the burn per lane; FORGE's `def.rs` CCR in the settling round; the `w5k_sim` incremental stepping API for M2; rank 2 after rank 1's first healthy hour; AI after `contract-v0.2`.
5. Housekeeping: the stale remote branch `lane/chassis/bad-pr-test` (PR #1 closed) could not be deleted through the git proxy; delete it from GitHub. `reference/prototype-v0/` deletion needs the owner's word.
