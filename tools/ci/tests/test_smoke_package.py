"""Tests for tools/ci/smoke_package.py: the helpers that decide whether the unzipped package answers (the real run needs the program)."""

import http.server
import json
import subprocess
import sys
import tempfile
import threading
import unittest
import zipfile
from pathlib import Path

import ci_testlib

sys.path.append(str(ci_testlib.CI_DIR))
import smoke_package as smoke  # noqa: E402


class _Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = json.dumps([{"id": "scout_4x4"}]).encode()
        self.send_response(200 if self.path == "/api/vehicles" else 404)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class SmokePackage(unittest.TestCase):
    def setUp(self):
        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        self.base = f"http://127.0.0.1:{self.server.server_address[1]}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()

    def test_a_free_port_can_be_bound_straight_away(self):
        import socket

        with socket.socket() as s:
            s.bind(("127.0.0.1", smoke.free_port()))

    def test_fetch_returns_the_status_and_the_body_and_posts_json(self):
        self.assertEqual(smoke.fetch(f"{self.base}/api/vehicles")[0], 200)
        self.assertEqual(smoke.fetch(f"{self.base}/nope")[0], 404)
        status, body = smoke.fetch(f"{self.base}/echo", {"a": 1})
        self.assertEqual((status, json.loads(body)), (200, {"a": 1}))

    def test_wait_for_returns_when_the_url_answers_and_names_the_program_that_exited_early(self):
        alive = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
        try:
            smoke.wait_for(f"{self.base}/api/vehicles", alive, 5)
        finally:
            alive.kill()
            alive.wait()
        dead = subprocess.Popen([sys.executable, "-c", "raise SystemExit(3)"])
        dead.wait()
        with self.assertRaises(SystemExit) as cm:
            smoke.wait_for(f"{self.base}/nope", dead, 5)
        self.assertIn("exited with 3", str(cm.exception))

    def test_wait_for_gives_up_with_the_url_when_nothing_answers(self):
        alive = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
        try:
            with self.assertRaises(SystemExit) as cm:
                smoke.wait_for(f"{self.base}/nope", alive, 1)
        finally:
            alive.kill()
            alive.wait()
        self.assertIn("/nope did not answer", str(cm.exception))

    def test_a_package_without_the_program_fails_with_a_clear_check(self):
        with tempfile.TemporaryDirectory() as tmp:
            z = Path(tmp) / "empty.zip"
            with zipfile.ZipFile(z, "w") as zf:
                zf.writestr(f"{smoke.FOLDER}/README.txt", "x")
            with self.assertRaises(SystemExit) as cm:
                smoke.run(z, "linux")
        self.assertIn("the package holds w5k", str(cm.exception))

    def test_check_passes_silently_and_fails_with_the_sentence(self):
        smoke.check(True, "fine")
        with self.assertRaises(SystemExit) as cm:
            smoke.check(False, "the sentence")
        self.assertIn("the sentence", str(cm.exception))


if __name__ == "__main__":
    unittest.main()
