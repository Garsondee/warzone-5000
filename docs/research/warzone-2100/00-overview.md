# Warzone 2100: Overview

## One-paragraph summary
Warzone 2100 is a 3D real-time strategy (RTS) game, originally released in 1999 by Pumpkin Studios (published by
Eidos Interactive) for Windows and PlayStation. It is set in a post-apocalyptic Earth after a nuclear collapse. Its
defining traits are a **large research tree (400+ technologies)**, a **modular unit design system** (you build your own
units from a body, propulsion and weapon), and a strong emphasis on **artillery, sensors and counter-battery** play.
Since December 2004 it has been free, open-source software (GPL-2.0-or-later), maintained by a community project.

## Quick facts

| Item | Value |
|---|---|
| Genre | Real-time strategy, 3D |
| Original developer / publisher | Pumpkin Studios / Eidos Interactive |
| Original release | 1999 (Windows, PlayStation) |
| Open-sourced | Source 6 Dec 2004; remaining data 10 Jun 2008 (dates vary slightly by source) |
| License | GNU GPL v2 or later |
| Engine language | C++ (scripting/AI/campaign logic in JavaScript) |
| Latest stable (as found) | 4.7.0, released 2026-04-06 |
| Platforms | Windows, macOS, Linux, plus FreeBSD and WebAssembly dev builds |
| Graphics backends | OpenGL, OpenGL ES, Vulkan (DirectX/Metal via translation layers) |
| Players | Up to 10 online (the original retail game supported 8) |
| Modes | Single-player campaign, online/LAN multiplayer, offline skirmish vs. AI |
| Repo | github.com/Warzone2100/warzone2100 |

## Headline features
1. **Persistent-base campaign** with timed missions and "away missions" delivered by transporter.
2. **Research tree with 400+ items.** In the campaign, new tech is unlocked by picking up *artifacts*; in skirmish
   you progress up the tree directly.
3. **Unit design system.** Body + propulsion + turret. Over 2,000 possible designs are quoted by sources.
4. **Artillery and information warfare.** Sensors spot, artillery fires, counter-battery sensors find enemy artillery.
5. **Air power.** VTOLs with limited ammo that need rearming pads; anti-air counters.
6. **Commanders**, **experience ranks**, **factions** (multiplayer, since 4.0), **transporters**.
7. **Scriptable.** JavaScript API for AI bots, maps, campaign rules and mods.
8. **Modern community features.** Spectators, lobby browser, auto-rated duels, campaign balance mods.

## Why it is a good reference for this project
- It is a complete, shipped RTS whose **source code is open**, so every mechanic can be inspected, not just guessed at.
- Its design choices (research as progression, custom units, fixed-size economy) are distinctive and well documented.
- It has a long life as a community project, so lessons exist about what players keep and what they change.

> Licensing note for later: Warzone 2100's code is GPL. Reading it for ideas is fine; copying its code or assets
> into this project would impose GPL terms on us. Decide this deliberately before reusing anything.

See [sources.md](sources.md) for where each fact came from.
