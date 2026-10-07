# Modding, Scripting and Technology

## Codebase
- Written in **C++**, built with **CMake** (Ninja generator recommended). Hosted at
  github.com/Warzone2100/warzone2100 (clone with `--recursive` submodules; ZIP downloads lack submodules and
  revision info).
- Key dependencies: **SDL**, **PhysicsFS** (virtual filesystem for game data/mods), **libpng**, **OpenAL-Soft**,
  **libcurl**, **SQLite**, **libzip**. JavaScript via **quickjs-ng** (migrated in 4.6).
- Platforms: Windows (MSVC), macOS, Linux (Ubuntu, Fedora, Alpine, Arch, openSUSE, Gentoo, Debian, Raspberry Pi OS),
  FreeBSD, WebAssembly (Emscripten, since 4.5). Flatpak builds exist.
- Translations via **Gettext** and **Crowdin** (the `po` directory).
- Dev builds come from GitHub Actions.

## Rendering
- Backends: OpenGL, OpenGL ES 3.0/2.0, Vulkan 1.0+; DirectX via libANGLE and Metal via MoltenVK (4.0).
- Features by version: per-pixel point lights (4.5), high-quality terrain with bump and specular mapping (4.4),
  Vulkan synchronisation and swapchain work (4.7).

## Data-driven design
Almost everything is moddable: **3D models** (PIE format; a Blender PIE addon is supported since 4.6),
**AIs/bots**, **audio**, **campaigns**, **stats**, rules and win/lose conditions.

## JavaScript API
- Original goal: allow new AI players (bots).
- A bot is an `.ai` file referencing a `.js` script; older wzscript-based AIs reference `.vlo`/`.slo`.
- Useful reference scripts: `rules.js` (game rules, win/lose), `scavfact.js` (scavenger faction).
- Used for AI, maps, campaign scripting and the script debugger. Functions added recently include
  `emitSound()` (4.7) and the `isFlying` droid field (4.6).
- Official docs live on the project's Atlassian wiki and forums (not reachable this session).

## Mod ecosystem
- Balance mods for the campaign (Classic, Pumpkin v1.10).
- Full campaigns: Remastered, Fractured Kingdom, Reclamation.
- Gameplay mods: Contingency (fork with its own weapon tree), Battleplan (1v1 leagues).
- Scavenger and AI mods.

## Design takeaway
Keep game data (units, research, structures) in plain data files and expose a scripting API early. It makes
balance iteration fast, enables community content and keeps the engine free of hardcoded content.
