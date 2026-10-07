# Gaps and Open Questions

Honest list of what this first pass did **not** establish.

## Access limits
The sandbox network proxy blocked: en.wikipedia.org, wz2100.net, guide.wz2100.net, store.steampowered.com,
gamingonlinux.com, benbridle.com, and the Atlassian wiki (warzone.atlassian.net). Only search-result summaries and
GitHub (README, Quick Start Guide, ChangeLog) were readable. Facts derived from blocked pages come from search-result
snippets and are lower confidence.

## Missing content
- Full **research tree** structure and counts per branch.
- **Exact stats** for bodies, propulsions and weapons (HP, damage, range, cost). The game's data files in the
  repository (stats JSON under `data/`) would answer this directly.
- **Faction differences** in multiplayer (4.0 factions) and campaign enemy tech.
- Full **Alpha and Beta mission lists** and mission counts.
- **Map editor** tooling and map file format.
- **Advanced base** and base-level settings details.
- Vanilla **laser** weapons, **Nexus Link** weapon (whether it exists as a player weapon), structure defence lists.
- Cobra and BoneCrusher AI details.
- 4.1 and 4.2 release notes; the 4.6/4.7 changelog beyond the headline items; the rest of ChangeLog (358k chars unread).
- Reception, sales, reviews of the 1999 release.

## Conflicting data
- Pumpkin closure date: 13 vs. 15 March 2000.
- Open-source date: 4, 6 or 7 Dec 2004 (founding vs. release vs. report).
- Player cap: 8 (retail) vs. 10 (current).
- Campaign size: "20 timed missions" vs. "50 missions".
- Nexus's creator: Dr. Reed bankrupted by the military vs. Dr. Alan Reed, CEO of Reed Corporation.

## Recommended next research steps
1. Clone the repo (GitHub is reachable via the git proxy) and read `data/mp/stats/*.json` to extract exact unit and
   research data into a spreadsheet.
2. Read the repository `doc/` folder and `data/base` campaign scripts for mission structure.
3. Try fetching the wiki through alternative mirrors or ask the user to export pages.
4. Pull the remaining ChangeLog in 100k chunks for 4.1 to 4.4.
