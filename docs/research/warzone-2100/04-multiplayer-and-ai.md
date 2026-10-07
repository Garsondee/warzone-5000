# Multiplayer and AI

## Modes
- **Online and LAN multiplayer**, free-for-all or team play. Up to **10 players** in current builds (the 1999 retail
  version supported 8).
- **Skirmish**: offline against AI bots.
- **Cross-platform** multiplayer between Windows, macOS and Linux.
- In multiplayer/skirmish you progress up the tech tree directly (no artifacts).
- **Factions** for multiplayer/skirmish arrived in 4.0.
- **Rated games:** an auto-hosting and rating system ("Autorating") hosts 1v1 duels from a small map pool
  (as of a 2020 post: 9 maps in low/medium/high oil sets, random base level and scavenger on/off). Keep the same
  player name/profile to accumulate stats. The pool may have changed since.
- **Leagues** are provided by the Battleplan mod (pick a league and join; no automatic matchmaking).
- **Spectators:** hosts can enable up to 10 spectator slots (since 4.2.0).
- **Lobby:** 4.7 introduced a new lobby supporting join-by-GameId, IPv4 and IPv6 hosting, HTTPS for all lobby traffic,
  a unified lobby/join screen and an expanded Host Lobby Options panel. 4.6 added blind lobby/game support
  (initial) and a new lobby game browser; 4.4 added Quick Chat.
- **Networking backend:** GameNetworkingSockets (GNS) backend added in 4.6 beta.

## Maps and base settings
Games can be started with no base, a base, or an advanced base, with scavengers on/off and different oil levels
(low/medium/high). Map editor and map file details were **not** found (see gaps doc).

## AI bots
- The JavaScript API was created primarily to enable new AI bots.
- **NullBot** (by NoQ): adaptive AI, good on low-oil maps, can often beat two conventional AIs; updated
  regularly (versioned JS files such as v1.38).
- **Semperfi JS**: bundled example bot, with its own `.ai` registration file.
- **Nexus** (older, wzscript-based): uses `.vlo`/`.slo` files. Not the same as the story antagonist's role
  (though the name is shared) and distinct from "The NEXUS Project" scripting library.
- **Scavenger scripts:** `scavfact.js` is the simple starting script; community mods such as the "Ultimate
  Scavenger AI Mod" replace it.
- Cobra and BoneCrusher are commonly mentioned bots *(general knowledge)* but were **not** confirmed by sources.
- A bot is registered by writing an `.ai` file pointing to a `.js` script (path relative to the `.ai` file).

## Design takeaway
An AI that is just a script against a public API makes the game vastly more extensible: community bots keep
the single-player/skirmish experience fresh long after the developers move on.
