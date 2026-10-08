"""Tests for media_lint.py. Each test makes a branch with some files of given sizes and runs the lint on it."""

import unittest

import ci_testlib  # noqa: F401
import media_lint
from ci_testlib import TempRepo, run_main, run_script

KB = 1024


def blob(kilobytes=0, extra=0):
    return b"x" * (kilobytes * KB + extra)


class MediaLintCase(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.repo.write("README.md", "hello\n")
        self.repo.commit("base")

    def pr(self, files, env=None, extra=(), branch=None):
        """Branch from main, apply `files` ({path: bytes, or None to delete}), run the lint."""
        self.repo.checkout("main")
        self.repo.branch(branch or f"topic-{len(self.repo.git('branch').splitlines())}")
        for path, content in files.items():
            if content is None:
                self.repo.remove(path)
            else:
                self.repo.write(path, content)
        self.repo.commit("work")
        argv = ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path), *extra]
        return run_main(media_lint.main, argv, env)

    def ok(self, files, **kw):
        code, out, err = self.pr(files, **kw)
        self.assertEqual(code, 0, out + err)
        return out

    def fails(self, files, *needles, **kw):
        code, out, err = self.pr(files, **kw)
        self.assertEqual(code, 1, out + err)
        for needle in needles:
            self.assertIn(needle, out)
        return out


class ImageLimits(MediaLintCase):
    def test_small_images_pass(self):
        self.ok({"docs/lanes/chassis/media/a.png": blob(10), "b.jpg": blob(10), "c.jpeg": blob(10), "d.webp": blob(10), "e.gif": blob(10)})

    def test_png_jpg_webp_limit_is_400_kb(self):
        for ext in ("png", "jpg", "jpeg", "webp"):
            with self.subTest(ext=ext):
                self.ok({f"at-limit.{ext}": blob(400)})
                out = self.fails({f"over.{ext}": blob(400, 1)}, f"over.{ext} is 400 KB, over the 400 KB limit for .{ext} images")
                self.assertIn("Clips are CI artifacts, not git objects", out)

    def test_gif_limit_is_800_kb(self):
        self.ok({"fine.gif": blob(500)})
        self.ok({"at-limit.gif": blob(800)})
        self.fails({"over.gif": blob(800, 1)}, "over the 800 KB limit for .gif images")

    def test_extension_case_does_not_matter(self):
        self.fails({"BIG.PNG": blob(401)}, "BIG.PNG")
        self.fails({"BIG.Gif": blob(801)}, "BIG.Gif")


class ForbiddenExtensions(MediaLintCase):
    def test_every_forbidden_extension_fails_even_when_tiny(self):
        for ext in "mp4 mov avi mkv zip 7z tar gz exe dll so dylib pdf".split():
            with self.subTest(ext=ext):
                out = self.fails({f"a/b/thing.{ext}": b"tiny"}, f"a/b/thing.{ext} has the forbidden extension .{ext}")
                self.assertIn("Clips are CI artifacts, not git objects", out)

    def test_case_and_compound_extensions(self):
        self.fails({"CLIP.MP4": b"x"}, "CLIP.MP4")
        self.fails({"backup.tar.gz": b"x"}, "backup.tar.gz")

    def test_lookalikes_are_fine(self):
        self.ok({"notes-mp4.txt": b"x", "zip": b"x", "pdf.txt": b"x", "docs/gz/readme.md": b"x", "tar.png": b"x"})


class OtherFilesAndTotal(MediaLintCase):
    def test_other_files_are_limited_to_1_mb(self):
        self.ok({"data/at-limit.csv": blob(1024)})
        self.fails({"data/over.csv": blob(1024, 1)}, "data/over.csv is 1024 KB, over the 1024 KB limit for ordinary files",
                   "large_files.allow")
        self.fails({"src/huge.rs": blob(2048)}, "src/huge.rs")

    def test_total_added_bytes_limit_is_8_mb(self):
        self.ok({f"data/part{i}.bin": blob(1000) for i in range(8)})  # 8000 KB
        out = self.fails({f"data/part{i}.bin": blob(1000) for i in range(9)}, "over the 8192 KB total limit")
        self.assertIn("9000 KB", out)

    def test_modified_files_count_towards_the_total(self):
        self.repo.checkout("main")
        for i in range(9):
            self.repo.write(f"data/part{i}.bin", blob(1000, i))
        self.repo.commit("legacy data, committed before the lint existed")
        self.fails({f"data/part{i}.bin": blob(1000, i + 100) for i in range(9)}, "total limit")


class AllowList(MediaLintCase):
    def test_a_listed_path_is_exempt_from_size_and_type_rules(self):
        files = {
            "reference/manual.pdf": blob(50),
            "data/big.bin": blob(3000),
            "tools/ci/large_files.allow": b"# approved by ARCH\n\nreference/manual.pdf\ndata/big.bin   # the terrain heightmap\n",
        }
        out = self.ok(files)
        self.assertIn("2 exempt via tools/ci/large_files.allow", out)

    def test_exempt_files_do_not_count_towards_the_total(self):
        files = {"data/big.bin": blob(20000), "tools/ci/large_files.allow": b"data/big.bin\n", "small.txt": b"x"}
        self.ok(files)

    def test_unlisted_neighbours_are_still_checked(self):
        self.fails({"data/big.bin": blob(3000), "data/other.bin": blob(3000), "tools/ci/large_files.allow": b"data/big.bin\n"},
                   "data/other.bin")

    def test_comments_and_a_commented_out_path_do_not_exempt(self):
        self.fails({"data/big.bin": blob(3000), "tools/ci/large_files.allow": b"# data/big.bin\n   # data/big.bin\n"}, "data/big.bin")

    def test_the_allow_list_is_read_from_the_head_revision(self):
        # Present on main already: a later branch that merely adds the big file is exempt.
        self.repo.checkout("main")
        self.repo.write("tools/ci/large_files.allow", "data/big.bin\n")
        self.repo.commit("ARCH approves one file")
        self.ok({"data/big.bin": blob(3000)})

    def test_allow_option_reads_a_file_from_disk(self):
        import tempfile, pathlib

        with tempfile.TemporaryDirectory() as tmp:
            allow = pathlib.Path(tmp) / "allow.txt"
            allow.write_text("data/big.bin\n")
            self.ok({"data/big.bin": blob(3000)}, extra=["--allow", str(allow)])

    def test_shipped_allow_file_exists_and_is_empty_apart_from_comments(self):
        text = (ci_testlib.CI_DIR / "large_files.allow").read_text()
        self.assertEqual(media_lint.parse_allow_list(text), set())
        self.assertTrue(text.startswith("#"))


class WhatCounts(MediaLintCase):
    def test_deleting_a_big_file_is_fine(self):
        self.repo.checkout("main")
        self.repo.write("legacy/big.bin", blob(3000))
        self.repo.commit("legacy")
        self.ok({"legacy/big.bin": None})

    def test_modifying_a_big_file_is_checked(self):
        self.repo.checkout("main")
        self.repo.write("legacy/big.bin", blob(3000))
        self.repo.commit("legacy")
        self.fails({"legacy/big.bin": blob(3000, 5)}, "legacy/big.bin")

    def test_a_pure_rename_adds_nothing(self):
        self.repo.checkout("main")
        self.repo.write("legacy/big.bin", b"a distinctive line of content\n" * 100000)
        self.repo.commit("legacy")
        self.repo.checkout("main")
        self.repo.branch("mover")
        self.repo.move("legacy/big.bin", "archive/big.bin")
        self.repo.commit("move")
        code, out, _ = run_main(media_lint.main, ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path)])
        self.assertEqual(code, 0, out)

    def test_nothing_changed(self):
        self.repo.branch("empty")
        code, out, _ = run_main(media_lint.main, ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path)])
        self.assertEqual(code, 0)
        self.assertIn("0 file(s)", out)

    def test_file_names_with_spaces_unicode_and_newlines(self):
        self.ok({"docs/café menu.png": blob(10), "docs/odd\nname.png": blob(10)})
        self.fails({"docs/big café.png": blob(500), "docs/big\nline.png": blob(500)}, "big café.png")


class Output(MediaLintCase):
    def test_github_annotations(self):
        _, out, _ = self.pr({"clip.mp4": b"x", "big.png": blob(500)}, env={"GITHUB_ACTIONS": "true"})
        errors = [line for line in out.splitlines() if line.startswith("::error ")]
        self.assertEqual(len(errors), 2)
        self.assertTrue(any(e.startswith("::error file=clip.mp4,title=Media lint::clip.mp4 has the forbidden extension") for e in errors))
        self.assertTrue(any(e.startswith("::error file=big.png,title=Media lint::big.png is 500 KB") for e in errors))

    def test_bad_revision_is_a_setup_error(self):
        code, _, err = run_main(media_lint.main, ["--base", "nope", "--head", "HEAD", "--repo", str(self.repo.path)])
        self.assertEqual(code, 2)
        self.assertIn("setup problem", err)

    def test_runs_under_python_dash_I(self):
        self.repo.branch("x")
        self.repo.write("clip.mov", b"x")
        self.repo.commit("w")
        proc = run_script("media_lint.py", "--base", "main", "--head", "HEAD", "--repo", str(self.repo.path))
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn("clip.mov has the forbidden extension .mov", proc.stdout)
        self.assertEqual(run_script("media_lint.py", "--help").returncode, 0)


if __name__ == "__main__":
    unittest.main()
