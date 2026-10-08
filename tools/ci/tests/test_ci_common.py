"""Tests for the helpers shared by all the checks: glob matching, git diff parsing, GitHub annotations."""

import io
import os
import unittest
from contextlib import redirect_stdout
from unittest import mock

import ci_testlib  # noqa: F401  (puts tools/ci on sys.path and makes git hermetic)
from ci_common import (
    MAX_ANNOTATIONS_PER_LEVEL,
    Change,
    CiError,
    annotate,
    check_rev,
    reset_annotation_counts,
    diff_changes,
    glob_match,
    parse_name_status,
    printable,
)
from ci_testlib import TempRepo


class GlobTests(unittest.TestCase):
    def test_double_star_crosses_directories(self):
        self.assertTrue(glob_match("crates/w5k_chassis/**", "crates/w5k_chassis/Cargo.toml"))
        self.assertTrue(glob_match("crates/w5k_chassis/**", "crates/w5k_chassis/src/a/b/c/deep.rs"))

    def test_double_star_needs_the_prefix(self):
        self.assertFalse(glob_match("crates/w5k_chassis/**", "crates/w5k_drive/src/lib.rs"))
        self.assertFalse(glob_match("crates/w5k_chassis/**", "xcrates/w5k_chassis/src/lib.rs"))
        self.assertFalse(glob_match("crates/w5k_chassis/**", "crates/w5k_chassis_extra/src/lib.rs"))

    def test_single_star_does_not_cross_a_slash(self):
        self.assertTrue(glob_match("docs/swarm/requests/chassis-*.md", "docs/swarm/requests/chassis-bump-strip.md"))
        self.assertFalse(glob_match("docs/swarm/requests/chassis-*.md", "docs/swarm/requests/chassis-a/b.md"))
        self.assertTrue(glob_match("crates/*/Cargo.toml", "crates/w5k_math/Cargo.toml"))
        self.assertFalse(glob_match("crates/*/Cargo.toml", "crates/w5k_math/sub/Cargo.toml"))

    def test_question_mark_matches_one_non_slash_character(self):
        self.assertTrue(glob_match("a/?.rs", "a/x.rs"))
        self.assertFalse(glob_match("a/?.rs", "a/xy.rs"))
        self.assertFalse(glob_match("a?b", "a/b"))

    def test_double_star_slash_matches_zero_or_more_directories(self):
        self.assertTrue(glob_match("a/**/b.rs", "a/b.rs"))
        self.assertTrue(glob_match("a/**/b.rs", "a/x/b.rs"))
        self.assertTrue(glob_match("a/**/b.rs", "a/x/y/z/b.rs"))
        self.assertFalse(glob_match("a/**/b.rs", "a/x/c.rs"))
        self.assertTrue(glob_match("**/lib.rs", "lib.rs"))
        self.assertTrue(glob_match("**/lib.rs", "crates/x/src/lib.rs"))

    def test_bare_double_star_matches_everything(self):
        self.assertTrue(glob_match("**", "any/path/at/all.txt"))
        self.assertTrue(glob_match("**", "top.txt"))

    def test_literal_characters_are_not_regex(self):
        self.assertTrue(glob_match("a+b(1).rs", "a+b(1).rs"))
        self.assertFalse(glob_match("a.rs", "aXrs"))
        self.assertFalse(glob_match("a+b.rs", "aab.rs"))

    def test_character_classes(self):
        self.assertTrue(glob_match("f[0-9].txt", "f7.txt"))
        self.assertFalse(glob_match("f[0-9].txt", "fx.txt"))
        self.assertTrue(glob_match("f[!0-9].txt", "fx.txt"))
        self.assertFalse(glob_match("f[!0-9].txt", "f7.txt"))
        self.assertFalse(glob_match("f[!0-9].txt", "f/.txt"))

    def test_unclosed_bracket_is_literal(self):
        self.assertTrue(glob_match("a[b", "a[b"))

    def test_case_sensitive_and_whole_path(self):
        self.assertFalse(glob_match("README.md", "readme.md"))
        self.assertFalse(glob_match("docs/*.md", "docs/a.md.bak"))
        self.assertTrue(glob_match("./Cargo.toml", "Cargo.toml"))

    def test_the_real_ownership_patterns(self):
        self.assertTrue(glob_match("docs/lanes/chassis/**", "docs/lanes/chassis/media/shot.png"))
        self.assertTrue(glob_match("Cargo.lock", "Cargo.lock"))
        self.assertFalse(glob_match("Cargo.lock", "crates/x/Cargo.lock"))
        self.assertTrue(glob_match(".github/**", ".github/workflows/pr.yml"))


class ParseNameStatusTests(unittest.TestCase):
    def test_plain_entries(self):
        raw = b"A\0new.txt\0M\0crates/a/src/lib.rs\0D\0gone.txt\0"
        self.assertEqual(
            parse_name_status(raw),
            [Change("A", "new.txt"), Change("M", "crates/a/src/lib.rs"), Change("D", "gone.txt")],
        )

    def test_rename_has_both_paths(self):
        changes = parse_name_status(b"R087\0old/name.rs\0new/name.rs\0M\0x\0")
        self.assertEqual(changes[0], Change("R", "new/name.rs", "old/name.rs", 87))
        self.assertEqual(changes[0].paths, ("old/name.rs", "new/name.rs"))
        self.assertEqual(changes[1].paths, ("x",))

    def test_names_with_spaces_quotes_unicode_and_newlines(self):
        raw = 'A\0with space.txt\0A\0qu"ote.txt\0A\0café.txt\0A\0line\nbreak.txt\0'.encode()
        self.assertEqual(
            [c.path for c in parse_name_status(raw)],
            ["with space.txt", 'qu"ote.txt', "café.txt", "line\nbreak.txt"],
        )

    def test_empty_output_is_no_changes(self):
        self.assertEqual(parse_name_status(b""), [])

    def test_truncated_output_is_a_setup_error(self):
        with self.assertRaises(CiError):
            parse_name_status(b"R100\0only-one-path\0")


class DiffChangesTests(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)

    def test_three_dot_diff_ignores_what_the_base_gained_afterwards(self):
        self.repo.write("a.txt", "a")
        self.repo.commit("base")
        self.repo.branch("topic")
        self.repo.write("topic.txt", "t")
        self.repo.commit("topic work")
        self.repo.checkout("main")
        self.repo.write("later.txt", "later")  # the base moved on after the branch was cut
        self.repo.commit("base moved on")
        changes = diff_changes(self.repo.path, "main", "topic")
        self.assertEqual([(c.status, c.path) for c in changes], [("A", "topic.txt")])

    def test_detects_a_rename_and_reports_both_paths(self):
        self.repo.write("old/place.txt", "some content that is long enough to be recognised as the same file\n" * 5)
        self.repo.commit("base")
        self.repo.branch("topic")
        self.repo.move("old/place.txt", "new/place.txt")
        self.repo.commit("move")
        (change,) = diff_changes(self.repo.path, "main", "topic")
        self.assertEqual((change.status, change.old_path, change.path), ("R", "old/place.txt", "new/place.txt"))

    def test_a_revision_that_looks_like_an_option_is_refused(self):
        with self.assertRaises(CiError):
            check_rev("--output=/tmp/x")
        with self.assertRaises(CiError):
            diff_changes(self.repo.path, "--stat", "HEAD")

    def test_unknown_revision_is_a_setup_error(self):
        self.repo.write("a.txt", "a")
        self.repo.commit("base")
        with self.assertRaises(CiError):
            diff_changes(self.repo.path, "no-such-branch", "main")


class OutputTests(unittest.TestCase):
    def test_annotations_only_on_github(self):
        buf = io.StringIO()
        with mock.patch.dict(os.environ, {"GITHUB_ACTIONS": ""}), redirect_stdout(buf):
            annotate("error", "nope", file="a.rs")
        self.assertEqual(buf.getvalue(), "")

    def test_annotation_format_and_escaping(self):
        buf = io.StringIO()
        with mock.patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}), redirect_stdout(buf):
            annotate("error", "100% bad\nsecond line", file="dir/a,b:c.rs", line=7, title="T")
            annotate("warning", "plain")
        first, second = buf.getvalue().splitlines()
        self.assertEqual(first, "::error file=dir/a%2Cb%3Ac.rs,line=7,title=T::100%25 bad%0Asecond line")
        self.assertEqual(second, "::warning::plain")

    def test_only_the_first_ten_annotations_of_a_level_are_sent(self):
        buf = io.StringIO()
        reset_annotation_counts()
        with mock.patch.dict(os.environ, {"GITHUB_ACTIONS": "true"}), redirect_stdout(buf):
            for i in range(25):
                annotate("error", f"problem {i}")
            annotate("warning", "a warning still gets through")
        lines = buf.getvalue().splitlines()
        errors = [l for l in lines if l.startswith("::error")]
        self.assertEqual(len(errors), MAX_ANNOTATIONS_PER_LEVEL)
        self.assertEqual(sum(l.startswith("::notice::more error annotations") for l in lines), 1)
        self.assertIn("::warning::a warning still gets through", lines)
        reset_annotation_counts()

    def test_printable_strips_control_characters(self):
        self.assertEqual(printable("a\nb\r::c\x00"), "a?b?::c?")


if __name__ == "__main__":
    unittest.main()
