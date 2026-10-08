# The merge gate

ARCH merges a lane PR into `integration` only when **every** line below is true. The ephemeral reviewer checks the same list and reports in the PR. A PR description starts with this checklist, filled in.

```
- [ ] CI is green (fmt, clippy with the bans, affected-crate tests, lane guard, constants lint, deps lint, media lint, line budget)
- [ ] The first-light scenario still passes (or this PR deliberately changes its golden: `Golden-Change: <why>` is in the description)
- [ ] EVIDENCE: an analytic-oracle test (a closed-form answer the code did not produce) OR a validation entry against the dossier
      OR the code is tagged UNVALIDATED(<source of the figure it should match>) and the PR says why
- [ ] A theory note exists or is updated: docs/theory/<lane>.md (the principle, the trade-off, a graphics analogy where one fits)
- [ ] Impact-matrix and golden diffs are explained in the description (what moved, by how much, why)
- [ ] An image or a clip shows the change (small PNG in docs/lanes/<lane>/media/; clips are CI artifacts)
- [ ] The status file is updated (done, blocked, next, cards needed)
- [ ] No tripwire tripped (under 400 lines, no new dependency, no bare constant, no contract change without a CCR, no tests deleted, inside the brief)
- [ ] Every PROVISIONAL(C-nnn) tag points at an open card
```

## Who opened it
A pull request whose head is one of the `[unrestricted]` heads in `ownership.toml` (`arch/*`, `integration`, ARCH's own session branch) must be one **ARCH itself opened**. Branch names are self-declared, so if a lane session opened a PR from such a head to get around the lane guard, do not merge it: close it, and note it in the lane's status file and `STATE.md`. Likewise any lane PR that touches a `[protected]` path is closed, not merged, whatever the CI result says.

## The reviewer
A fresh, read-only session that sees the diff and this checklist, **not the author's reasoning**, so it is not anchored. It reports: what it checked, what it could not check, and any finding. A red-circle finding blocks the merge; a nit can ride the next PR.

## Contract and ADR PRs
ARCH reviews `CCR:` PRs line by line for expressiveness (can an HMMWV, a Sherman and a Leopard 2 still be written?), for who breaks, and for the migration. ADRs need no oracle test but must name the alternatives considered.
