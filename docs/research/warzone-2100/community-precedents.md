# Community Precedents: What Others Have Already Tried

The Warzone 2100 community has spent two decades reworking exactly the systems we want to expand. This document records what they
changed and why, as evidence for what works and what players complain about.

> **Confidence warning.** The official wiki, forum threads and most mod pages were **blocked by the sandbox proxy**; everything here
> comes from **search-result excerpts** (marked *[excerpt]*), not full pages, and much of it is from the 2.x/3.x era. Treat it as leads to
> verify, not as settled fact. The engine facts elsewhere in this dossier come from the source code and are higher confidence.

## 1. Contingency (expansion-pack mod, 2012-2019; development later stopped)

Goal *[excerpt]*: fix tech-progression problems and weak matchups in the standard metagame; the scope grew into an expansion pack.

| Area | What it changed |
|---|---|
| **Bodies** | Two broad **families**: Group A aimed at *survivability* (upgraded through the *armour* research line) and Group B aimed at *carrying heavier equipment* (upgraded through the *engine* line). The two families converge on the same armour values by tier 3. The roster grew (light and medium variants of the top lines; new named bodies). Each vehicle line gained an **ultra-heavy multi-turret body**, locked until factory upgrades. The tier-4 Dragon line needs *both* lines plus kinetic and thermal armour research. |
| **Engines** | **Engine size shrinks at each tier**, so higher-tier chassis *depend on engine upgrades* to keep their speed. Group A weights run 250 (light) to 2,000 (ultra-heavy); Group B about two-thirds of that. |
| **Propulsion** | A propulsion **weight multiplier per type** (hover highest at 12, down to 1 for VTOL). Hover became the heaviest but stayed the fastest ground option; VTOLs and cyborgs became slightly slower. |
| **Weapons** | Weapon lines made more self-contained and **merged**: mortar progress counts toward howitzers, AA pairs with machine guns, rockets with missiles; some new weapons need research in **two lines at once** (for example cannons with machine guns); weak matchups were addressed (machine guns less exposed to tanks); flamers got a workable anti-air route. |
| **Research** | A compounding upgrade curve: **nine upgrades multiply a stat by 4** (a 30-damage weapon reaches 120); power generation, research speed, factory output and construction speed excluded. Reactive armour split out as its own line; vehicle and cyborg upgrades shared across both unit types. |

Why it matters to us: Contingency explored **engines as a tech axis** (engine line as the "mobility spine" of one body family), **two
parallel body families with different upgrade lines**, **cross-line prerequisites for new weapons**, and **multi-turret top
bodies**, which are all directions on the user's list.

## 2. Enhanced Balance (EB, mod for 3.x)

*[excerpt]* The README describes a rebalance with a large set of extra components: every weapon family gets its own **range upgrade
track**; "NASDA" variants of some parts trade a small stat edge for a higher price; extra power and quicker production against
pricier research. The forum author's foundation was **a turret/body weight system plus a working engine/terrain modifier**, with the
diagnosis that in stock 3.x "about nine in ten weapon, body and propulsion pairings hit top speed", which made engine upgrades and
terrain irrelevant. The rework gave **each weapon, body and propulsion its own speed profile**, so that every engine upgrade adds
speed. The author shipped a **spreadsheet** of changes from stock values. (Our measurement of the current game: 44-62% of ground designs
sit at the cap, a milder version of the same problem.)

## 3. Battleplan (multiplayer mod / spin-off; status "on hold"; latest devlog Alpha 15, May 2025)

*[excerpt]* A multiplayer mod focused on 1v1 games with leagues, built on Warzone 2100, with notable design simplifications:

- **Body classes cut to two at identical cost**, one with an HP bonus and one with an armour bonus (the "balanced" class was scrapped);
  **body cost tracks size** (120 / 180 / 270); the spread between sizes was narrowed so that "the gain in speed and durability with
  smaller turrets" matters more.
- A **weight factor** so that "heavy weighted hover tanks are not faster than light wheeled ones"; wheels and tracks pulled back
  toward a slower baseline; wheels and tracks made visually easier to tell apart.
- The AI (NullBot) was changed to **use random bodies and propulsions** rather than favouring tracks and armoured bodies.
- Oil resources made to supply power for a fixed time (25 minutes).

Lesson: a competitive community, given the freedom, **reduced** body variety to two clear classes and added a weight rule to stop one
propulsion from dominating. That is evidence that unconstrained variety tends to collapse onto a few dominant choices.

## 4. Official balance work in the current game *(source: repository and changelog)*

- **Classic Balance** (4.5) and **"Pumpkin v1.10"** (4.7) are bundled campaign balance mods; the 4.1 release was a major balance update
  *[excerpt]*.
- The repository documents a **balance proposal process** (`doc/MPBalanceChangeProcess.md`): a pull request with rationale and
  test notes, then a Discord discussion and vote by a tester role (more than 75% consensus), then merge. Rule: no single play style may
  be advantaged at the expense of others (for example high-oil competitive versus low-oil skirmish).
- The changelog shows engine output being used as a **live balance lever** ("make Leopard and Panther have higher engine power output
  thus move faster with more weapons").

## 5. Recurring community themes *[excerpts, mostly older versions]*

| Theme | What was said |
|---|---|
| **Hover dominance** | Early rebalance threads warned that unless hover had real weaknesses players would use it for everything; "the Nexus AI is fixated on hovers". Hover's weaknesses: rocky terrain (50% on pink rock) and artillery (110%). |
| **Wheels and half-tracks** | Wheels lacked a distinct role; half-tracks said to outpace tracks "unrealistically" and to deserve no separate research topic. Reference speed caps match the data (0.98 / 1.17 / 1.37 / 2.34 tiles per second). |
| **Engine and terrain irrelevance** | The speed cap makes most combinations identical (see [unit-design/04-engine-and-speed.md](unit-design/04-engine-and-speed.md)). |
| **Research tree navigation** | A newcomer found the tree daunting and suggested showing the whole path to a target tech (as Earth 2150 does); a returning player wrote a tool to show the sequence for any tech. The current game has now added a research-tree screen with critical-path and focus filters (4.x, this snapshot). |
| **Research pacing and mandatory upgrades** | A suggestion for **era-based progression** (rushing to stronger weapons costly, demolishing old command centres retires obsolete options) drew the objection that it resembles Age of Empires. EB lengthened early-to-mid research and shortened late research; one balance tweak was explicitly aimed at making upgrades "less of a must-have". |
| **Dead-end upgrades** | A poster noted that Bunker Buster alone among early weapons never got a successor and fades with each tier. |
| **Variety claims versus reality** | The original game's review judged the practical variety narrower than the "2,000 designs" claim: "five tank bodies each paired with several tracks and weapons yield five largely similar tanks". |
| **Dual-turret bodies** | Testers note that on a multi-turret body only the **primary** mount sets the unit's attributes while the secondary fires independently; mount position matters on VTOLs (the forward weapon must be primary because rear mounts do not fire during a flyby). Consistent with the engine's slot-0-only code paths. |

## 6. What the pattern suggests (opinion)

1. **Players and modders repeatedly "found" the same three problems**: speed saturation (engine/terrain irrelevance), a dominant propulsion
   (hover), and a research tree that is either a treadmill or hard to navigate.
2. **Every serious rework introduced structure on top of the raw component lists**: families (Contingency), two classes (Battleplan),
   per-propulsion weight multipliers, cross-line prerequisites. Unstructured freedom was not enough.
3. **Engines as a tech axis** was tried (Contingency) and is consistent with the original data, where engine research gates bodies.
4. **Fewer, clearer choices** (Battleplan) is a viable direction; so is **more, structured choices** (Contingency). They are opposite answers to
   the same problem, which makes this a genuine design decision for us, not a settled matter.

## Sources

| Source | Used for |
|---|---|
| [Contingency Mod (SourceForge)](https://sourceforge.net/projects/warzone2100cont/files/Contingency%20Mod) | goals, changelog (excerpt) |
| [Contingency preview thread](https://forums.wz2100.net/viewtopic.php?p=95106) | body families, engine tiers, propulsion multipliers (excerpt) |
| [Contingency beta thread](https://forums.wz2100.net/viewtopic.php?p=104645) | status and weapons (excerpt) |
| [Contingency (project wiki)](https://warzone.atlassian.net/wiki/spaces/wzpedia/pages/7635929) | overview (excerpt) |
| [Enhanced Balance repository](https://github.com/jbreija/Warzone2100EB) | README (excerpt) |
| [Enhanced Balance forum thread](https://forums.wz2100.net/viewtopic.php?p=133654) | speed saturation diagnosis (excerpt) |
| [How is unit speed calculated?](https://forums.wz2100.net/viewtopic.php?p=101305) | community derivation of the speed formula (excerpt) |
| [Battleplan](https://cupnplategames.itch.io/wz-battleplan), [Alpha 15](https://cupnplategames.itch.io/wz-battleplan/devlog/955326/alpha-15), [Alpha 10](https://cupnplategames.itch.io/wz-battleplan/devlog/446936/alpha-10) | body classes, weight factor, AI (excerpt) |
| [Rebalance reconsidered](https://forums.wz2100.net/viewtopic.php?p=30329), [half-track thread](https://forums.wz2100.net/viewtopic.php?p=36040), [hover thread](https://forums.wz2100.net/viewtopic.php?p=64606) | propulsion debates (excerpt) |
| [Compressible research point](https://forums.wz2100.net/viewtopic.php?p=103676), [Regarding Research](https://forums.wz2100.net/viewtopic.php?p=104055) | research pacing suggestions (excerpt) |
| [GameSpot review of Warzone 2100](https://www.gamespot.com/reviews/warzone-2100-review/1900-2531842/) | variety versus the "2,000 designs" claim (excerpt) |
| [GamingOnLinux: the big balance update (4.1)](https://www.gamingonlinux.com/2021/07/the-big-balance-update-is-out-now-for-warzone-2100) | official balance release (excerpt) |
| `doc/MPBalanceChangeProcess.md`, `ChangeLog` in the Warzone 2100 repository | process; engine-power balance changes (read directly) |
