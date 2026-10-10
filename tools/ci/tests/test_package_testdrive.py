"""Tests for tools/ci/package_testdrive.py: the zip the owner downloads must unpack to a folder that starts by itself."""

import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

import ci_testlib

sys.path.append(str(ci_testlib.CI_DIR))
import package_testdrive as pkg  # noqa: E402


class PackageTestDrive(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        (self.root / "w5k.exe").write_bytes(b"MZ-fake-exe")
        (self.root / "content" / "vehicles" / "game").mkdir(parents=True)
        (self.root / "content" / "vehicles" / "game" / "mule_4x4.ron").write_text("(id: \"mule_4x4\")")
        (self.root / "content" / "world" / "courses").mkdir(parents=True)
        (self.root / "content" / "world" / "courses" / "slice.ron").write_text("(seed: 1)")
        (self.root / "index.html").write_text("<html>drive</html>")

    def tearDown(self):
        self._tmp.cleanup()

    def build(self, name="out.zip", target_os="windows"):
        out = self.root / name
        names = pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", out, target_os)
        return out, names

    def test_the_windows_zip_holds_the_program_the_content_the_page_a_start_file_and_a_readme(self):
        out, names = self.build()
        with zipfile.ZipFile(out) as z:
            inside = set(z.namelist())
        for must in ("w5k.exe", "START.bat", "README.txt", "viewer/index.html", pkg.VEHICLE, pkg.COURSE):
            self.assertIn(f"{pkg.FOLDER}/{must}", inside, must)
        self.assertEqual(len(names), len(inside))

    def test_start_bat_runs_the_program_from_its_own_folder_with_the_page_and_opens_the_browser(self):
        out, _ = self.build()
        with zipfile.ZipFile(out) as z:
            bat = z.read(f"{pkg.FOLDER}/START.bat").decode()
        self.assertIn('cd /d "%~dp0"', bat)
        self.assertIn("w5k.exe drive", bat)
        self.assertIn("--web viewer", bat)
        self.assertIn("--open", bat)
        self.assertIn("content\\vehicles\\game\\mule_4x4.ron", bat)
        self.assertNotIn("\n\n", bat.replace("\r\n", "\n"))  # CRLF line ends: Windows batch files need them
        self.assertTrue(bat.endswith("\r\n"))

    def test_the_linux_package_gets_a_shell_start_file_and_an_executable_program(self):
        out, _ = self.build("linux.zip", "linux")
        with zipfile.ZipFile(out) as z:
            names = set(z.namelist())
            mode = z.getinfo(f"{pkg.FOLDER}/w5k").external_attr >> 16
        self.assertIn(f"{pkg.FOLDER}/start.sh", names)
        self.assertNotIn(f"{pkg.FOLDER}/START.bat", names)
        self.assertTrue(mode & 0o100)

    def test_the_same_inputs_give_a_byte_identical_zip(self):
        a, _ = self.build("a.zip")
        b, _ = self.build("b.zip")
        self.assertEqual(a.read_bytes(), b.read_bytes())

    def test_the_truck_skins_the_page_loads_go_into_viewer_skins_and_a_missing_folder_is_not_an_error(self):
        skins = self.root / "skins"
        skins.mkdir()
        (skins / "scout_4x4.skin").write_bytes(b"skin-a")
        (skins / "notes.txt").write_text("ignored")
        out = self.root / "with.zip"
        pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", out, "windows", skins)
        with zipfile.ZipFile(out) as z:
            names = set(z.namelist())
        self.assertIn(f"{pkg.FOLDER}/viewer/skins/scout_4x4.skin", names)
        self.assertNotIn(f"{pkg.FOLDER}/viewer/skins/notes.txt", names)
        out2 = self.root / "without.zip"
        pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", out2, "windows", self.root / "no-such-folder")
        self.assertTrue(out2.is_file())

    def test_a_missing_input_is_refused_with_its_name(self):
        (self.root / "index.html").unlink()
        with self.assertRaises(SystemExit) as cm:
            self.build()
        self.assertIn("the page", str(cm.exception))


if __name__ == "__main__":
    unittest.main()
