# Spike S9: can a lane session reach the sources? (VALIDATION, 2026-10-10)

**Verdict: partly. Kill criterion triggered for *verified* figures; the dossier can start, with every web figure marked UNVERIFIED.**

## What I tried (from a cloud lane session)
| Source | Shell (curl) | WebFetch | WebSearch |
|---|---|---|---|
| Wikipedia, globalsecurity.org, fas.org, army.mil, amgeneral.com, archive.org, DTIC, Gutenberg | 403 from the proxy | cannot resolve the host | titles and URLs, plus a model-written summary of the page |
| raw.githubusercontent.com / api.github.com (Project Chrono parameter files) | **200, full text** | works | n/a |
| pypi.org, npm, crates.io | 200 | n/a | n/a |

So a lane session can **read open vehicle models on GitHub in full**, and can see **search-engine summaries** of the published spec pages,
but cannot open the spec pages, the technical manuals or the test reports themselves.

## Consequences
1. A figure that I only saw in a search summary is recorded with `src: "UNVERIFIED(search summary): <page title, URL>"` and a band that covers the disagreement between summaries. It is **not** a verified `Spec`; the dashboard shows these as a separate slice of the provenance meter ("secondary").
2. Chrono parameter files were read directly (e.g. `data/vehicle/hmmwv/chassis/HMMWV_Chassis.json`: chassis mass 2086.52 kg, wheelbase 3.378 m, wheel radius 0.268 m; tyre unloaded radius 0.4699 m, width 0.3175 m, rolling-resistance coefficient 0.015). They are **plausibility cross-checks recorded in `notes`**, never dossier values and never imported.
3. Counts of quantities reachable this way today: M998 about 16 (all secondary), M113A3 about 14, M4A3 about 12. M113A3 and M4A3 sit at or under the 15 line, and none is primary.
4. Public sources disagree openly (M998 curb 5,200 vs 7,700 lb; M113A3 side slope 30% vs 40%; M4A3 range 100 vs 130 mi; top speed 55 vs 70 mph for the M998 depending on governor and loading). The band covers the disagreement; the harness will therefore tend to score such quantities amber/green by width, so the provenance meter must show them as weak evidence.

## Ask (card text for ARCH to lift)
**Question:** Will the owner drop public-domain manuals into `content/dossier/sources/` (TM 9-2320-387-10 or -280-10 for the HMMWV, TM 9-2350-261-10 for the M113A3, the M4A3 technical manual, FM 17-series test data), or approve a network allow-list entry for `*.army.mil`, `apps.dtic.mil`, `archive.org`?
**Default:** continue with UNVERIFIED secondary figures; no dossier is promoted to "verified" and no M2 held-out claim is made from them.
**Cost of being wrong:** re-checking about 60 figures against the primary documents (a day).
