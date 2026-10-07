# Gaps and Open Questions

Honest list of what the research has **not** established. Updated after the second pass (unit design and research deep dives).

## Access limits
- **Blocked by the sandbox network proxy:** en.wikipedia.org, wz2100.net, guide.wz2100.net, forums.wz2100.net, betaguide.wz2100.net,
  store.steampowered.com, gamingonlinux.com, benbridle.com, and the Atlassian wiki (warzone.atlassian.net). Anything from those comes from
  **search-result excerpts**, which are lower confidence.
- **Reachable:** GitHub (the Warzone 2100 repository was cloned read-only, outside this project, for the second pass).

## Answered in the second pass
- Exact **stats** for bodies, propulsion, weapons, sensors, repair, construction and brains (tables in [`data/`](data/)).
- The full **research tree** (390 multiplayer / 413 campaign topics), its shape, costs and coupling.
- The **speed/engine model**, derived-stat formulas, validity rules, damage pipeline and upgrade arithmetic (from the engine source).
- Faction differences: the 4.0 factions are **cosmetic** model swaps.
- Vanilla **lasers** (the energy sub-class, 11 entries) and the **Nexus Link** weapon (a researchable, designable takeover turret).
- **Bundled AIs** (Cobra, BoneCrusher, NullBot, Nexus, SemperFi).
- The **scripting surface** for research, components and design.

## Still missing
- Full **Alpha and Beta campaign mission lists** and a reliable mission count.
- **Map editor** tooling and the map file format.
- **Advanced-base** and base-level details beyond the tech-level timestamp tables.
- **Structures and defences in depth** (only 124 defensive structures were counted, not analysed; power economy only touched).
- **Campaign weapon and structure stats** (only bodies and propulsion were compared with multiplayer).
- **Production time and economy interplay** (factory output versus power income) as a pacing model.
- **Balance history**: how stats changed over releases (would need deeper git history than the shallow clone).
- Reception, sales, reviews of the 1999 release beyond one review excerpt.
- 4.1-4.4 release notes; ChangeLog beyond what was read.
- Details of the **Autohoster/rating** systems and current player statistics.
- **Community material** has not been read in full (excerpts only).

## Conflicting data (unchanged)
- Pumpkin closure date: 13 vs. 15 March 2000.
- Open-source date: 4, 6 or 7 Dec 2004.
- Player cap: 8 (retail) vs. 10 (current).
- Campaign size: "20 timed missions" vs. "50 missions".
- Nexus's creator: Dr. Reed bankrupted by the military vs. Dr. Alan Reed, CEO of Reed Corporation.

## Known limitations of the analysis (new)
- The speed and saturation numbers use **flat terrain** (100% factor; 250% for air) or road/off-road contexts; real maps vary.
- The "efficiency frontier" analysis uses three or four axes and a single reference weapon; it is an indicator, not a balance verdict.
- **Nominal DPS ignores splash, burn, penetration and projectile flight**, and the damage tables show a direct hit only.
- Agent-read engine behaviour beyond the key formulas was **not independently re-verified** line by line.

## Recommended next steps
1. Analyse structures, defences and the **power/production economy** (needed to model research and production pacing together).
2. Read deeper balance history (fetch more git history for the stats files) to see which numbers were retuned and why.
3. Study constraint-based design systems outside Warzone 2100 (see [expansion-analysis.md](expansion-analysis.md) section 8).
4. Ask the user to export wiki and forum pages the sandbox cannot reach, if community detail matters.
