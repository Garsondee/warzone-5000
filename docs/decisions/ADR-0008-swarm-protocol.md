# ADR-0008: The swarm protocol

**Status:** Accepted (owner approved the plan, 2026-10-08). Full rules: `docs/swarm/RULES.md`.

## Context
The owner's fear: if the first prompts are not right, parallel work "spins outwards" and the foundation suffers. They asked for domain agents
building separate parts at the same time (course, visuals, physics, ...).

## Decision
1. **Thirteen lanes** (ARCH plus twelve), each a separate cloud session with its own branch, launched by ARCH in three ranks. One writer per path (`docs/swarm/ownership.toml`); the CI lane guard rejects a PR that touches another lane's files.
2. **Guardrails live in the repo.** `CLAUDE.md`, the brief, the lane briefs and the contracts are what every session reads; the initial prompt for a lane is one line pointing at its brief.
3. **Contracts and stand-ins before code.** Lanes pin a tagged `w5k_contract` version and build against stand-ins for their neighbours. A settling round (each lane's design note, spike and CCRs) hardens the contract into v0.2 within 24 hours; contracts freeze at M1.
4. **An integration spine** (the first-light scenario) runs on every merge, on stand-ins first and on real parts as they land.
5. **Branches and merging:** lanes work on `lane/<lane>/<topic>` and open PRs into `integration`; only ARCH merges, after CI and an independent (ephemeral) reviewer; `main` moves only at an owner-approved milestone.
6. **Merge gate:** an analytic-oracle test or a validation entry (or an `UNVALIDATED` tag with a source), a theory note, golden and impact-matrix diffs explained, and an image or clip.
7. **Decision cards** instead of silent choices; work continues on the default, tagged `PROVISIONAL(card)`. A message from the owner to a lane is logged as a card.
8. **Governor:** up to 13 sessions at once; bounded missions; hourly check-ins that report each lane's status and usage; launches pause if integration CI is red for more than two hours or more than five PRs wait for review; the owner can pause any lane or all.
9. **CI tiers** to conserve minutes: PR = Linux fmt, clippy, affected-crate tests and the guards; `integration` = full Linux + Windows with goldens; nightly = fuzz and the validation dashboard.

## Consequences
Coordinator state lives in `docs/swarm/STATE.md` so a restarted or compacted coordinator can resume. Cost is dominated by the number of simultaneous lanes, which is why launches are staged and the burn is reported.
