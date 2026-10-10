# Status: VALIDATION

**Last updated:** 2026-10-10 UTC | **Branch:** lane/validation/settling | **Contract pinned:** contract-v0.1 | **Phase:** settling

## Done
- Spike S9 (`docs/lanes/validation/spike-s9.md`), design note with CCR text (`design-note.md`), theory note stub (`docs/theory/validation.md`).

## In progress
- Settling PR open. Next branch `lane/validation/build`.

## Blocked
- Nothing blocking. Primary sources are unreachable from lane sessions (proxy denies army.mil, DTIC, archive.org, Wikipedia, globalsecurity.org).

## Next
1. Dossier loader plus M998 dossier (about 16 quantities, all `Secondary`/UNVERIFIED) with the three named dossier tests.
2. Scenario measurement on the stand-in model; verdicts with the double-power negative control.
3. M113A3 and M4A3 dossiers (about 14 and 12 quantities reachable: under the 25 target).

## Cards needed / PROVISIONAL decisions in force
- **S9-card (ARCH to number):** may the owner drop manuals in `content/dossier/sources/` or allow `*.army.mil`, `apps.dtic.mil`, `archive.org`? Default: carry on with UNVERIFIED secondary figures, never promoted to verified. Work tagged PROVISIONAL(S9-card).

## Evidence
- Reachability table in `spike-s9.md`. No code yet.

## Owner instructions received
- 2026-10-10 (STATE): run without checking in; continue into build steps. Dossier-to-game-vehicle mapping lives in `content/dossier/` (C-001).
