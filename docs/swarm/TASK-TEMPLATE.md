# Writing a lane brief or a task

A **lane brief** (`docs/swarm/lanes/<lane>.md`) is the initial prompt of a lane: the session starts with one line, "Follow `docs/swarm/lanes/<lane>.md`", and everything it needs is
in the repo. A **follow-up task** (ARCH to a running lane, or a new session) uses the same shape, shorter. Briefs are written for a capable engineer who has never seen this repo and cannot ask
you questions: concrete paths, named tests, numbers, and what *not* to do.

## Lane brief template
```
# Lane <NAME>: <the mission in one line>

## Mission
Two or three sentences: what exists when this lane has succeeded, in terms the owner would recognise (a picture, a number, a behaviour).

## You own   (the CI lane guard enforces it)
Crates, directories, files. Plus always: docs/swarm/status/<lane>.md, docs/swarm/requests/<lane>-*.md, docs/theory/<lane>.md, docs/lanes/<lane>/**, spikes/<lane>/**.

## You read, never edit
The contract version you pin; the docs that matter; the other lanes' *public* surfaces only through the contract.

## Stand-ins you start on
Which `w5k_contract::testing` items stand in for your neighbours, and what you will need from them later.

## Settling round (first hours; then stop for review)
1. The risk spike(s) and the question each answers, with the finding note's path.
2. The one-page design note: the questions it must answer.
3. The CCRs you expect to need.

## Build order
Numbered steps; each ends with named tests that must pass.

## Acceptance tests (named; physics sentences)
`name_of_test`: what it asserts, to what tolerance, against what oracle or dossier figure.

## Milestone deliverables
What must pass in CI, and what the owner will see (images, clips, plots).

## Theory to explain in docs/theory/<lane>.md
The principles, in plain language, with a graphics analogy where one fits.

## Non-goals
What looks attractive and is not yours.

## Needs from others / gives to others
Interface requests you expect to file; what other lanes will ask of you.

## Tripwires specific to this lane
Beyond the general ones in CLAUDE.md.

## Done
The exact condition under which you stop and write the handoff note.
```

## Follow-up task card (shorter)
```
Goal: <one sentence>      Why now: <what it unblocks>      Lane/paths: <...>
Tests that prove it: <named>      Evidence for the owner: <image, plot, number>
Out of scope: <...>       Report: update docs/swarm/status/<lane>.md and open a PR (merge gate).
```

## Rules for good briefs
- Name the tests. A brief without acceptance tests is a wish.
- Give the oracle (the closed-form answer or the dossier figure), not just "make it realistic".
- Say what is out of scope; the failure mode of a capable agent is helpful scope growth.
- Point at stand-ins so the lane can start before its neighbours exist.
- Keep it under two pages. If it needs more, the lane is two lanes.
