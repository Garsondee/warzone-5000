#!/usr/bin/env python3
"""Build the test-drive package: one zip the owner can unpack anywhere and double-click.

The zip holds the `w5k` program, the content folder (vehicles, course, tuning), the browser page the program serves, a
START script and a plain-language README. The program needs nothing else at run time: no Node, no Rust, no internet.

    python3 -I tools/ci/package_testdrive.py --exe target/release/w5k.exe --content content \
        --page tools/viewer/dist/index.html --skins tools/viewer/dist/skins --out out/Warzone5000-TestDrive-windows.zip --os windows

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

To stop: close the black window.

Everything runs on this computer; nothing is sent anywhere. This is an early test version.
"""


def build(exe: Path, content: Path, page: Path, out: Path, target_os: str, skins: Path | None = None) -> list[str]:
    """Write the zip and return the names of the files in it (relative to the package folder)."""
    for what, path in (("the program", exe), ("the content folder", content), ("the page", page)):
        if not path.exists():
            raise SystemExit(f"package_testdrive: {what} not found: {path}")
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
        # The truck bodies the page loads (tools/viewer/dist/skins/*.skin). Optional: without them the page draws plain boxes.
        if skins is not None and skins.is_dir():
            for p in sorted(skins.glob("*.skin")):
                add(z, p, f"viewer/skins/{p.name}")
        if windows:
            add(z, None, "START.bat", START_BAT)
            add(z, None, "START-KID.bat", START_KID_BAT)
        else:
            add(z, None, "start.sh", START_SH, executable=True)
        add(z, None, "README.txt", README)
    return names


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--exe", required=True, type=Path)
    ap.add_argument("--content", required=True, type=Path)
    ap.add_argument("--page", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--os", choices=["windows", "linux"], default="windows")
    ap.add_argument("--skins", type=Path, default=None, help="folder of *.skin files the page loads (optional)")
    args = ap.parse_args(argv)
    names = build(args.exe, args.content, args.page, args.out, args.os, args.skins)
    size_mb = args.out.stat().st_size / 1_000_000  # const-ok: bytes to megabytes for the log line
    print(f"package_testdrive: wrote {args.out} ({len(names)} files, {size_mb:.1f} MB)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
