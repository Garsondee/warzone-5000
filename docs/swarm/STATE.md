# Coordinator state (ARCH)

*ARCH keeps this current so a restarted or compacted coordinator can resume from the repo alone. If you are a fresh ARCH session: read `CLAUDE.md`, `docs/brief/BRIEF.md`, `RULES.md`, then this file, then each lane's status file.*

**Updated:** 2026-10-08 | **Integration branch:** `integration` (a SHA here goes stale at every merge, so read the branch head; the last ARCH merges were PR 3, the rehearsal, and PR 4, the rehearsal findings; its CI status is in the Actions tab) | **Contract pin:** `contract-v0.1` = commit `f8f5e5d` (the git proxy refuses to push tags, so there is no remote tag: lanes pin the commit; the owner can create the tag in GitHub if wanted) | **Phase:** Wave 0, the Launch Kit: finished (contract pinned, guards proven, rehearsal passed); **launch on hold** (card C-008) until the owner says go or the usage window resets

**Control Room (the owner's live view):** https://claude.ai/artifact/RerESUqi3CaJCzifhQCZCd (private; republish with `python3 -I tools/control_room/build.py <out.html>` then the Artifact tool, same file path). Decision-card answers live in the artifact's database collection `cards` (document id = card id, fields `choice`, `text`, `answeredAt`); read them at each check-in with `ArtifactData` (`list` on `cards`), copy them into `docs/decisions/QUEUE.md` as ANSWERED, and write `appliedAt` back to the document.

## Lanes
| Lane | Rank | Session | Branch | State | Last check-in | Burn so far |
|---|---|---|---|---|---|---|
| ARCH | 1 | this session (the owner set a credit floor of about $5; ARCH works one thing at a time) | `claude/sharp-babbage-d702f7` | Wave 0 done; idle until the owner's go | 2026-10-08 | owner reported $14 left before the rehearsal wrap-up |
| CHASSIS | 1 | rehearsal only: `session_01QY832txWp3L6YN6PWKQxxR` (finished) | `lane/chassis/rehearsal` (PR 3 merged) | real lane not launched | 2026-10-08 | rehearsal $0.23 |
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
See `docs/decisions/QUEUE.md` (C-001 to C-014 open, all with defaults; C-008 holds the launch, C-013 needs the owner's word in chat, C-014 is an owner action in GitHub).

## PR queue
*(none)*

## Burn log
| Time (UTC) | Sessions active | Notes |
|---|---|---|
| 2026-10-08 16:00 | 1 (this session) | meter ~ $204 after ~20 h |
| 2026-10-08 18:05 | 1 (this session) | owner warned of about $20 of credit left; ARCH stopped new work and left the tree green; no lanes launched, no background agents running |
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
3. Launch rank 1 only when the owner says go or card C-008 is answered (the usage window resets about 10 Oct 02:00 UTC). Order: CHASSIS, DRIVE, FORGE, WORLD, VIEWER, VALIDATION, one launch prompt each (`LAUNCH.md`), then read the first lane's early events for the trailer and tool-guard checks before launching the rest. Open owner cards: C-001 to C-014 (defaults apply).
4. Then, one at a time: hourly check-ins with the burn per lane; FORGE's `def.rs` CCR in the settling round; the `w5k_sim` incremental stepping API for M2; rank 2 after rank 1's first healthy hour; AI after `contract-v0.2`.
5. Housekeeping: the stale remote branch `lane/chassis/bad-pr-test` (PR #1 closed) could not be deleted through the git proxy; delete it from GitHub. `reference/prototype-v0/` deletion needs the owner's word.
