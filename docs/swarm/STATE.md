# Coordinator state (ARCH)

*ARCH keeps this current so a restarted or compacted coordinator can resume from the repo alone. If you are a fresh ARCH session: read `CLAUDE.md`, `docs/brief/BRIEF.md`, `RULES.md`, then this file, then each lane's status file.*

**Updated:** 2026-10-08 | **Integration branch:** `integration` at the merge commit f8f5e5d (contract 0.1.1; CI green on Linux and Windows) | **Contract tag:** `contract-v0.1` (set at f8f5e5d) | **Phase:** Wave 0, the Launch Kit: contract tagged; **launch on hold** (card C-008) until the rehearsal, the owner summary and the owner's go

**Control Room (the owner's live view):** https://claude.ai/artifact/RerESUqi3CaJCzifhQCZCd (private; republish with `python3 -I tools/control_room/build.py <out.html>` then the Artifact tool, same file path). Decision-card answers live in the artifact's database collection `cards` (document id = card id, fields `choice`, `text`, `answeredAt`); read them at each check-in with `ArtifactData` (`list` on `cards`), copy them into `docs/decisions/QUEUE.md` as ANSWERED, and write `appliedAt` back to the document.

## Lanes
| Lane | Rank | Session | Branch | State | Last check-in | Burn so far |
|---|---|---|---|---|---|---|
| ARCH | 1 | this session (the owner set a credit floor; ARCH works one thing at a time) | `claude/sharp-babbage-d702f7` | contract 0.1.1 applied; PR and tag next | 2026-10-08 | about $20 left at stop |
| CHASSIS | 1 | - | - | not launched | - | - |
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
| Launch Kit (Wave 0) | in progress | contract v0.1.0, test doubles, first-light spine green on stand-ins, ADRs, rules, briefs, tooling |
| Settling round | not started | closes when ARCH tags contract v0.2 (within 24 h of rank 1 launching) |
| M1 First light | not started | |
| M2 Mud and steel | not started | |

## Open CCRs
*(none)*

## Open cards
See `docs/decisions/QUEUE.md` (C-001 to C-012 open, all with defaults).

## PR queue
*(none)*

## Burn log
| Time (UTC) | Sessions active | Notes |
|---|---|---|
| 2026-10-08 16:00 | 1 (this session) | meter ~ $204 after ~20 h |
| 2026-10-08 18:05 | 1 (this session) | owner warned of about $20 of credit left; ARCH stopped new work and left the tree green; no lanes launched, no background agents running |

## Check-in routine (hourly while any lane is active)
1. List lane sessions (status, usage) and read each `docs/swarm/status/<lane>.md`.
2. List open PRs and CI on `integration`; merge what meets `MERGE-GATE.md`; request an ephemeral review for the rest.
3. Answer interface requests and CCRs; tag a new contract version when the open CCRs are settled.
4. Nudge stalled lanes (stale status file, no push for 90 minutes) with a short message.
5. Update this file and the Control Room page; note the burn.
6. Governor: pause launches if integration CI has been red for more than 2 h or more than 5 PRs wait for review.

## Next actions
1. DONE: contract 0.1.1 applied, merged into `integration` (PR 2), `integration` green on Linux and Windows, tag `contract-v0.1` pushed.
2. Run the rehearsal (one small lane session, see `LAUNCH.md`, and have it list which `mcp__` tools it has and report the cost); republish the Control Room; send the owner the C0 summary. The lane-brief edits for contract 0.1.1 are done.
3. Launch rank 1 only when the owner says go or card C-008 is answered. Open owner cards: C-001 to C-014 (defaults apply); C-013 (settings rules) needs the owner's word in chat; C-014 (GitHub branch protection) is a two-minute owner action.
4. Housekeeping: the stale remote branch `lane/chassis/bad-pr-test` (PR #1 closed) could not be deleted through the git proxy; delete it from GitHub. `reference/prototype-v0/` deletion needs the owner's word.
