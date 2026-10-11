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

    def test_start_bat_has_no_speed_limit_and_start_kid_bat_keeps_it(self):
        out, _ = self.build()
        with zipfile.ZipFile(out) as z:
            full = z.read(f"{pkg.FOLDER}/START.bat").decode()
            kid = z.read(f"{pkg.FOLDER}/START-KID.bat").decode()
        self.assertIn("--no-speed-limit", full)
        self.assertNotIn("--no-speed-limit", kid)
        self.assertNotIn("--no-assist", full + kid)
        self.assertTrue(kid.endswith("\r\n"))

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

    def test_the_workshop_page_and_its_start_file_are_packaged_when_given(self):
        (self.root / "workshop.html").write_text("<title>Workshop</title>")
        out = self.root / "shop.zip"
        names = pkg.build(
            self.root / "w5k.exe", self.root / "content", self.root / "index.html", out, "windows", None, self.root / "workshop.html"
        )
        with zipfile.ZipFile(out) as z:
            inside = set(z.namelist())
            bat = z.read(f"{pkg.FOLDER}/START-WORKSHOP.bat").decode()
        self.assertIn(f"{pkg.FOLDER}/viewer/workshop.html", inside)
        self.assertEqual(len(names), len(inside))
        self.assertIn('cd /d "%~dp0"', bat)
        self.assertIn("w5k.exe viewer workshop --open", bat)  # the program opens the browser itself, as START.bat does
        self.assertNotIn("ping", bat)  # no delayed-start trick
        self.assertIn(pkg.WORKSHOP_URL, bat)  # the address the browser is sent to, and the one printed for a person
        self.assertTrue(bat.endswith("\r\n"))
        self.assertNotIn("\n\n", bat.replace("\r\n", "\n"))
        lin = self.root / "shop-linux.zip"
        pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", lin, "linux", None, self.root / "workshop.html")
        with zipfile.ZipFile(lin) as z:
            sh = z.read(f"{pkg.FOLDER}/start-workshop.sh").decode()
            self.assertTrue(z.getinfo(f"{pkg.FOLDER}/start-workshop.sh").external_attr >> 16 & 0o100)
        self.assertIn("./w5k viewer workshop --open", sh)

    def test_without_a_workshop_page_there_is_no_workshop_start_file_and_a_missing_one_is_refused(self):
        out, _ = self.build()
        with zipfile.ZipFile(out) as z:
            names = set(z.namelist())
        self.assertNotIn(f"{pkg.FOLDER}/START-WORKSHOP.bat", names)
        self.assertNotIn(f"{pkg.FOLDER}/viewer/workshop.html", names)
        with self.assertRaises(SystemExit) as cm:
            pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", self.root / "x.zip", "windows", None, self.root / "nope.html")
        self.assertIn("the Workshop page", str(cm.exception))

    def test_the_readme_tells_a_grown_up_how_to_open_the_workshop_and_what_the_carrier_is(self):
        self.assertIn("START-WORKSHOP.bat", pkg.README)
        self.assertIn(pkg.WORKSHOP_URL, pkg.README)
        self.assertIn("carrier", pkg.README)

    def test_skins_from_several_folders_are_all_packaged_and_a_later_folder_wins(self):
        committed, packed = self.root / "committed", self.root / "packed"
        committed.mkdir()
        packed.mkdir()
        (committed / "scout_4x4.skin").write_bytes(b"old-scout")
        (committed / "mule.skin").write_bytes(b"mule")
        (packed / "scout_4x4.skin").write_bytes(b"new-scout")
        (packed / "carrier_tracked.skin").write_bytes(b"carrier")
        out = self.root / "multi.zip"
        pkg.build(self.root / "w5k.exe", self.root / "content", self.root / "index.html", out, "windows", [committed, packed])
        with zipfile.ZipFile(out) as z:
            names = set(z.namelist())
            scout = z.read(f"{pkg.FOLDER}/viewer/skins/scout_4x4.skin")
        for skin in ("scout_4x4", "mule", "carrier_tracked"):
            self.assertIn(f"{pkg.FOLDER}/viewer/skins/{skin}.skin", names, skin)
        self.assertEqual(scout, b"new-scout")

    def test_a_missing_input_is_refused_with_its_name(self):
        (self.root / "index.html").unlink()
        with self.assertRaises(SystemExit) as cm:
            self.build()
        self.assertIn("the page", str(cm.exception))


if __name__ == "__main__":
    unittest.main()
