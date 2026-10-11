#!/usr/bin/env python3
"""Build the test-drive package: one zip the owner can unpack anywhere and double-click.

The zip holds the `w5k` program, the content folder (vehicles, course, tuning), the browser pages the program serves (the driving page
and the Workshop), START scripts and a plain-language README. The program needs nothing else at run time: no Node, no Rust, no internet.

    python3 -I tools/ci/package_testdrive.py --exe target/release/w5k.exe --content content \
        --page tools/viewer/dist/index.html --skins tools/viewer/dist/skins --skins out/extra-skins --workshop tools/viewer/dist/workshop.html \
        --out out/Warzone5000-TestDrive-windows.zip --os windows

Only the standard library is used. The zip layout is checked by tools/ci/tests/test_package_testdrive.py.
"""

from __future__ import annotations

import argparse
import sys
import zipfile
from pathlib import Path

FOLDER = "Warzone5000-TestDrive"
VEHICLE = "content/vehicles/game/mule_4x4.ron"
COURSE = "content/world/courses/slice.ron"

def _bat(extra: str, who: str) -> str:
    drive = f"w5k.exe drive --vehicle {VEHICLE.replace('/', chr(92))} --course {COURSE.replace('/', chr(92))} --web viewer --open{extra}"
    return (
        "@echo off\r\n"
        'cd /d "%~dp0"\r\n'
        f"echo Starting the Warzone 5000 test drive ({who}). A browser window will open.\r\n"
        "echo Leave this window open while you drive. Close it to stop.\r\n"
        f"{drive}\r\n"
        "pause\r\n"
    )


# START.bat: no speed limit (the other helpers stay). START-KID.bat: the 25 km/h cap for a small driver.
START_BAT = _bat(" --no-speed-limit", "full speed")
START_KID_BAT = _bat("", "kid mode, speed limited")

# START-WORKSHOP.bat: the Workshop (sliders, scoreboard, DRIVE) in the browser. The program prints its address and serves in this window;
# a second, hidden command waits a few seconds and opens the browser at it (the program is not asked to: `w5k viewer workshop` has no
# --open yet; if the browser does not come up the address is in the window and in the README).
WORKSHOP_URL = "http://127.0.0.1:8789/"
START_WORKSHOP_BAT = (
    "@echo off\r\n"
    'cd /d "%~dp0"\r\n'
    "echo Starting the Warzone 5000 Workshop. A browser window will open in a few seconds.\r\n"
    f"echo If it does not, open {WORKSHOP_URL} in your browser.\r\n"
    "echo Leave this window open while you work. Close it to stop.\r\n"
    f'start "" /b cmd /c "ping -n 4 127.0.0.1 >nul & start {WORKSHOP_URL}"\r\n'
    "w5k.exe viewer workshop\r\n"
    "pause\r\n"
)

START_WORKSHOP_SH = (
    "#!/bin/sh\n"
    'cd "$(dirname "$0")" || exit 1\n'
    f"echo 'Starting the Warzone 5000 Workshop. Open {WORKSHOP_URL} in a browser.'\n"
    "./w5k viewer workshop\n"
)

START_SH = (
    "#!/bin/sh\n"
    'cd "$(dirname "$0")" || exit 1\n'
    "echo 'Starting the Warzone 5000 test drive. A browser window will open.'\n"
    f"./w5k drive --vehicle {VEHICLE} --course {COURSE} --web viewer --open --no-speed-limit\n"
)

README = """WARZONE 5000 - TEST DRIVE
=========================

To start:  double-click START.bat   (on Linux or Mac: run ./start.sh)
A black window opens (leave it open) and your web browser opens the driving page.
For your child, double-click START-KID.bat instead: the same game with a top speed of 25 km/h.
(w5k.exe is the engine; it does nothing when double-clicked on its own. Always start from a START file.)

To drive:
  - Arrow keys or W A S D: go, brake, turn left, turn right.
  - Or press the big buttons on the screen with the mouse (they work with a touch screen too).
  - Pick a different vehicle from the pictures at the start.
  - Tipped over or stuck? The game puts the vehicle back on the road by itself, or press the big RESET button.
  - C changes the camera between close-up and far away.

The game helps small drivers: steering is gentle at speed and the vehicle slows down by itself when you let go of the
keys. START-KID.bat also limits the top speed to 25 km/h; START.bat does not. Grown-ups can switch all the helpers
off by adding   --no-assist   to the line in a START file.

The vehicles: the three trucks steer with their front wheels. The tracked carrier steers like a tank: left and right run
its two tracks at different speeds, so it can turn on the spot.

THE WORKSHOP (for grown-ups): double-click START-WORKSHOP.bat. Your browser opens a page with sliders (wheelbase, tyres,
springs, engine power and more). The vehicle on the turntable changes shape as you move them, and the scoreboard shows how
braking, acceleration, hills, side slopes and cornering changed against the original, in a few seconds. Press DRIVE to take
your design out on the course with the keys above. If the browser does not open by itself, go to http://127.0.0.1:8789/

To stop: close the black window.

Everything runs on this computer; nothing is sent anywhere. This is an early test version.
"""


def build(
    exe: Path,
    content: Path,
    page: Path,
    out: Path,
    target_os: str,
    skins: Path | list[Path] | None = None,
    workshop: Path | None = None,
) -> list[str]:
    """Write the zip and return the names of the files in it (relative to the package folder)."""
    for what, path in (("the program", exe), ("the content folder", content), ("the page", page)):
        if not path.exists():
            raise SystemExit(f"package_testdrive: {what} not found: {path}")
    if workshop is not None and not workshop.exists():
        raise SystemExit(f"package_testdrive: the Workshop page not found: {workshop}")
    windows = target_os == "windows"
    names: list[str] = []

    def add(z: zipfile.ZipFile, source: Path | None, name: str, data: str | None = None, executable: bool = False) -> None:
        info = zipfile.ZipInfo(f"{FOLDER}/{name}", date_time=(2026, 1, 1, 0, 0, 0))  # fixed time: the zip is reproducible
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = (0o755 if executable else 0o644) << 16
        z.writestr(info, data.encode("utf-8") if data is not None else source.read_bytes())
        names.append(name)

    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w") as z:
        add(z, exe, "w5k.exe" if windows else "w5k", executable=True)
        files = sorted(p for p in content.rglob("*") if p.is_file())  # sorted: reproducible
        for p in files:
            add(z, p, "content/" + p.relative_to(content).as_posix())
        add(z, page, "viewer/index.html")
        # The vehicle bodies the page loads (*.skin). Optional: without them the page draws plain boxes. Several folders may be given (the
        # committed ones, then a folder the build just packed); a skin in a later folder replaces one of the same name in an earlier one.
        chosen: dict[str, Path] = {}
        for folder in [] if skins is None else [skins] if isinstance(skins, Path) else skins:
            if folder.is_dir():
                chosen.update({p.name: p for p in sorted(folder.glob("*.skin"))})
        for name in sorted(chosen):
            add(z, chosen[name], f"viewer/skins/{name}")
        if workshop is not None:
            add(z, workshop, "viewer/workshop.html")
        if windows:
            add(z, None, "START.bat", START_BAT)
            add(z, None, "START-KID.bat", START_KID_BAT)
            if workshop is not None:
                add(z, None, "START-WORKSHOP.bat", START_WORKSHOP_BAT)
        else:
            add(z, None, "start.sh", START_SH, executable=True)
            if workshop is not None:
                add(z, None, "start-workshop.sh", START_WORKSHOP_SH, executable=True)
        add(z, None, "README.txt", README)
    return names


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--exe", required=True, type=Path)
    ap.add_argument("--content", required=True, type=Path)
    ap.add_argument("--page", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--os", choices=["windows", "linux"], default="windows")
    ap.add_argument("--skins", type=Path, action="append", default=None, help="folder of *.skin files the page loads (optional; repeatable, the later folder wins)")
    ap.add_argument("--workshop", type=Path, default=None, help="the Workshop page (workshop.html); adds START-WORKSHOP (optional)")
    args = ap.parse_args(argv)
    names = build(args.exe, args.content, args.page, args.out, args.os, args.skins, args.workshop)
    size_mb = args.out.stat().st_size / 1_000_000  # const-ok: bytes to megabytes for the log line
    print(f"package_testdrive: wrote {args.out} ({len(names)} files, {size_mb:.1f} MB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
