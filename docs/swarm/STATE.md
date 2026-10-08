# Coordinator state (ARCH)

*ARCH keeps this current so a restarted or compacted coordinator can resume from the repo alone. If you are a fresh ARCH session: read `CLAUDE.md`, `docs/brief/BRIEF.md`, `RULES.md`, then this file, then each lane's status file.*

**Updated:** 2026-10-08 (Launch Kit in progress) | **Integration branch:** `integration` (not yet created) | **Contract tag:** none yet (v0.1.0 draft) | **Phase:** Wave 0, the Launch Kit

## Lanes
| Lane | Rank | Session | Branch | State | Last check-in | Burn so far |
|---|---|---|---|---|---|---|
| ARCH | 1 | this session | `claude/sharp-babbage-d702f7` | writing the Launch Kit | - | see session meter |
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

## Check-in routine (hourly while any lane is active)
1. List lane sessions (status, usage) and read each `docs/swarm/status/<lane>.md`.
2. List open PRs and CI on `integration`; merge what meets `MERGE-GATE.md`; request an ephemeral review for the rest.
3. Answer interface requests and CCRs; tag a new contract version when the open CCRs are settled.
4. Nudge stalled lanes (stale status file, no push for 90 minutes) with a short message.
5. Update this file and the Control Room page; note the burn.
6. Governor: pause launches if integration CI has been red for more than 2 h or more than 5 PRs wait for review.

## Next actions
- Finish the Launch Kit (see the task list in the coordinator session); get permission to move the old prototype tree out of the root; copy the staged workspace in; create `integration`; rehearsal; rank 1.
