#!/usr/bin/env python3
"""Smoke test of the test-drive package: unzip it, start the drive server and the Workshop the way the START files do, and check that they answer.

    python3 -I tools/ci/smoke_package.py out/Warzone5000-TestDrive-windows.zip --os windows

It proves what the owner would otherwise have to find out by double-clicking: the program starts from an unzipped folder (content, pages and skins
found beside it), the drive server lists all four vehicles and serves the page and the tracked carrier's skin, and the Workshop serves its page, builds
a design, scores it (which makes the program start itself again) and says so. Standard library only; exit 0 means every check passed.
"""

from __future__ import annotations

import argparse
import json
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

FOLDER = "Warzone5000-TestDrive"
VEHICLE = "content/vehicles/game/mule_4x4.ron"
COURSE = "content/world/courses/slice.ron"
VEHICLES = ("scout_4x4", "mule_4x4", "hauler_4x4", "carrier_tracked")
START_TIMEOUT_S = 90  # const-ok: a cold Windows runner compiles the garage and generates the course before it answers
SCORE_TIMEOUT_S = 240  # const-ok: the Workshop's scoreboard runs the whole proving battery twice (the base and the design)


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def fetch(url: str, body: dict | None = None, timeout: float = 30.0) -> tuple[int, bytes]:
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, method="POST" if body is not None else "GET")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def wait_for(url: str, process: subprocess.Popen, timeout_s: float) -> None:
    """Poll `url` until it answers 200; fail with the program's own output if it exits first or the time runs out."""
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if process.poll() is not None:
            raise SystemExit(f"smoke_package: the program exited with {process.returncode} before {url} answered")
        try:
            if fetch(url, timeout=5.0)[0] == 200:
                return
        except (urllib.error.URLError, ConnectionError, TimeoutError, OSError):
            pass
        time.sleep(0.5)
    raise SystemExit(f"smoke_package: {url} did not answer within {timeout_s:.0f} s")


def check(ok: bool, what: str) -> None:
    print(("PASS  " if ok else "FAIL  ") + what, flush=True)
    if not ok:
        raise SystemExit(f"smoke_package: failed: {what}")


def run(zip_path: Path, target_os: str) -> None:
    exe_name = "w5k.exe" if target_os == "windows" else "w5k"
    with tempfile.TemporaryDirectory() as tmp:
        with zipfile.ZipFile(zip_path) as z:
            z.extractall(tmp)
        root = Path(tmp) / FOLDER
        exe = root / exe_name
        check(exe.is_file(), f"the package holds {exe_name}")
        if target_os != "windows":
            exe.chmod(0o755)  # extractall does not keep the executable bit
        for must in ("viewer/index.html", "viewer/workshop.html", "viewer/skins/carrier_tracked.skin", VEHICLE, COURSE, "README.txt"):
            check((root / must).is_file(), f"the package holds {must}")

        procs: list[subprocess.Popen] = []
        try:
            # the drive server, as START.bat runs it (without --open, on a free port)
            port = free_port()
            drive = subprocess.Popen(
                [str(exe), "drive", "--vehicle", VEHICLE, "--course", COURSE, "--web", "viewer", "--port", str(port), "--no-speed-limit"],
                cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            procs.append(drive)
            base = f"http://127.0.0.1:{port}"
            wait_for(f"{base}/api/vehicles", drive, START_TIMEOUT_S)
            _, body = fetch(f"{base}/api/vehicles")
            listed = json.loads(body)
            ids = [v["id"] for v in listed]
            check(all(v in ids for v in VEHICLES), f"the drive server lists {', '.join(VEHICLES)} (got {ids})")
            carrier = next(v for v in listed if v["id"] == "carrier_tracked")
            check(len(carrier.get("tracks", [])) == 2, "the carrier lists its two belts")
            status, page = fetch(f"{base}/")
            check(status == 200 and b"<html" in page.lower(), "the drive server serves the page")
            status, skin = fetch(f"{base}/skins/carrier_tracked.skin")
            check(status == 200 and len(skin) > 100_000, f"the drive server serves the carrier's skin ({len(skin)} bytes)")  # const-ok: a real skin is hundreds of KB
            drive.terminate()

            # the Workshop, as START-WORKSHOP.bat runs it
            port = free_port()
            shop = subprocess.Popen([str(exe), "viewer", "workshop", "--port", str(port)], cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            procs.append(shop)
            base = f"http://127.0.0.1:{port}"
            wait_for(f"{base}/api/bases", shop, START_TIMEOUT_S)
            _, body = fetch(f"{base}/api/bases")
            bases = [b["id"] for b in json.loads(body)]
            check({"scout_4x4", "mule_4x4", "hauler_4x4"} <= set(bases), f"the Workshop lists its bases ({bases})")
            status, page = fetch(f"{base}/")
            check(status == 200 and b"Workshop" in page, "the Workshop serves its page")
            status, body = fetch(f"{base}/api/design", {"base": "scout_4x4", "levers": {"engine_peak_power": 1.3}}, timeout=60.0)
            design = json.loads(body)
            check(status == 200 and design.get("skin"), "the Workshop builds a design with its body")
            status, body = fetch(f"{base}/api/score", {"id": design["id"]}, timeout=SCORE_TIMEOUT_S)
            rows = json.loads(body).get("rows", [])
            check(status == 200 and len(rows) >= 5, f"the Workshop scores it (the program starts itself again): {len(rows)} rows")  # const-ok: six benchmarks are scored
        finally:
            for p in procs:
                if p.poll() is None:
                    p.kill()
                p.wait()
    print("smoke_package: OK")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("zip", type=Path)
    ap.add_argument("--os", choices=["windows", "linux"], default="windows")
    args = ap.parse_args(argv)
    if not args.zip.is_file():
        raise SystemExit(f"smoke_package: {args.zip} not found")
    run(args.zip, args.os)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
