"""Build the time-trial page from the output of `w5k trial`.

    python3 -I build.py DIR out.html [--template template.html]

DIR holds `scene.json` (the course and every vehicle's mesh) and `replay.json` (one recorded run per vehicle). The page is one
self-contained HTML file: the data is embedded, the big binary arrays (vehicle meshes, replay frames) are deflated and
re-encoded as base64 (the page unpacks them with the browser's DecompressionStream), and three.js is loaded from cdnjs.
"""
import base64
import json
import pathlib
import sys
import zlib


def pack(b64: str) -> str:
    return base64.b64encode(zlib.compress(base64.b64decode(b64), 9)).decode("ascii")


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    template = pathlib.Path(__file__).with_name("template.html")
    if "--template" in argv:
        template = pathlib.Path(argv[argv.index("--template") + 1])
        args.remove(str(template))
    if len(args) != 2:
        sys.exit(__doc__)
    src, out = pathlib.Path(args[0]), pathlib.Path(args[1])
    scene = json.loads((src / "scene.json").read_text())
    replay = json.loads((src / "replay.json").read_text())
    for v in scene["vehicles"]:
        v["mesh"]["z64"] = pack(v["mesh"].pop("b64"))
    for r in replay["runs"]:
        r["frames_z64"] = pack(r.pop("frames_b64"))
        r.pop("checks", None)
    data = {"hz": replay["hz"], "course": scene["course"], "vehicles": scene["vehicles"], "runs": replay["runs"]}
    payload = json.dumps(data, separators=(",", ":")).replace("<", "\\u003c")
    html = template.read_text()
    assert html.count("__TRIAL_PAYLOAD__") == 1, "template needs exactly one __TRIAL_PAYLOAD__ placeholder"
    html = html.replace("__TRIAL_PAYLOAD__", payload)
    out.write_text(html)
    print(f"{len(data['vehicles'])} vehicles, {len(data['runs'])} runs -> {out} ({out.stat().st_size / 1e6:.2f} MB)")


if __name__ == "__main__":
    main(sys.argv[1:])
