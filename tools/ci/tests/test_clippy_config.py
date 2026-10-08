"""Tests for clippy.toml and rustfmt.toml: the determinism bans really bite, and the things they must not touch stay legal.

The clippy tests build a throw-away crate (no dependencies, so they work offline), copy the repository's clippy.toml next
to it and run `cargo clippy` on it. They are skipped when cargo or clippy is not installed."""

import json
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

import ci_testlib

CLIPPY_TOML = ci_testlib.ROOT / "clippy.toml"
RUSTFMT_TOML = ci_testlib.ROOT / "rustfmt.toml"

MATH = "sin cos tan asin acos atan atan2 sin_cos sinh cosh tanh asinh acosh atanh exp exp2 exp_m1 ln ln_1p log log2 log10 powf powi hypot cbrt mul_add".split()
BRIEF_LIST = "sin cos tan asin acos atan atan2 sinh cosh tanh exp exp2 exp_m1 ln ln_1p log log2 log10 powf powi hypot cbrt mul_add sin_cos".split()
UNARY = set("sin cos tan asin acos atan sinh cosh tanh asinh acosh atanh exp exp2 exp_m1 ln ln_1p log2 log10 cbrt".split())
BINARY = {"atan2", "log", "powf", "hypot"}


def method_call(name):
    if name in UNARY:
        return f"acc += x.{name}();"
    if name in BINARY:
        return f"acc += x.{name}(y);"
    return {"powi": "acc += x.powi(2);", "mul_add": "acc += x.mul_add(y, y);", "sin_cos": "{ let (s, c) = x.sin_cos(); acc += s + c; }"}[name]


@unittest.skipUnless(CLIPPY_TOML.is_file(), "clippy.toml not found")
class ClippyTomlContents(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.config = tomllib.loads(CLIPPY_TOML.read_text())

    def paths(self, key):
        return [entry["path"] for entry in self.config[key]]

    def test_every_entry_has_a_path_and_a_reason_and_no_duplicates(self):
        for key in ("disallowed-methods", "disallowed-types"):
            entries = self.config[key]
            self.assertEqual(len(self.paths(key)), len(set(self.paths(key))))
            for entry in entries:
                self.assertTrue(entry["reason"].strip(), entry)

    def test_every_maths_function_of_the_brief_is_banned_for_f64_and_f32(self):
        banned = self.paths("disallowed-methods")
        for ty in ("f64", "f32"):
            for name in BRIEF_LIST:
                self.assertIn(f"{ty}::{name}", banned)

    def test_the_wall_clock_is_banned(self):
        banned = self.paths("disallowed-methods")
        self.assertIn("std::time::Instant::now", banned)
        self.assertIn("std::time::SystemTime::now", banned)

    def test_hash_collections_are_banned(self):
        self.assertEqual(sorted(self.paths("disallowed-types")), ["std::collections::HashMap", "std::collections::HashSet"])

    def test_reasons_point_at_the_fix(self):
        for entry in self.config["disallowed-methods"]:
            if entry["path"].startswith(("f64::", "f32::")):
                # The reason must name the fix: the w5k_math::scalar wrappers, libm itself, or plain multiplication.
                self.assertRegex(entry["reason"], r"libm|w5k_math::scalar|x \* x", entry)
            else:
                self.assertIn("no wall clock in simulation", entry["reason"])
        for entry in self.config["disallowed-types"]:
            self.assertIn("iteration order", entry["reason"])
            self.assertTrue("BTree" in entry["reason"] and "Vec" in entry["reason"])

    def test_non_banned_maths_is_not_listed(self):
        banned = " ".join(self.paths("disallowed-methods"))
        for fine in ("sqrt", "abs", "floor", "ceil", "round", "trunc", "min", "max", "clamp", "copysign", "signum", "recip"):
            self.assertNotIn(f"::{fine}", banned)


@unittest.skipUnless(RUSTFMT_TOML.is_file(), "rustfmt.toml not found")
class RustfmtToml(unittest.TestCase):
    def test_settings(self):
        config = tomllib.loads(RUSTFMT_TOML.read_text())
        self.assertEqual(config, {"edition": "2021", "max_width": 120, "use_small_heuristics": "Max"})


def clippy_available():
    if not (shutil.which("cargo") and CLIPPY_TOML.is_file()):
        return False
    return subprocess.run(["cargo", "clippy", "--version"], capture_output=True).returncode == 0


@unittest.skipUnless(clippy_available(), "cargo clippy is not installed")
class ClippyBites(unittest.TestCase):
    """Run the real clippy on a probe crate with one call per banned method."""

    @classmethod
    def setUpClass(cls):
        cls._tmp = tempfile.TemporaryDirectory(prefix="w5k-clippy-")
        cls.work = Path(cls._tmp.name)
        config = tomllib.loads(CLIPPY_TOML.read_text())
        methods = [e["path"] for e in config["disallowed-methods"]]

        lines, cls.expect, cls.clean = [], {}, set()

        def emit(text, lint=None, clean=False):
            lines.append(text)
            if lint:
                cls.expect[len(lines)] = lint
            if clean:
                cls.clean.add(len(lines))

        emit("use std::collections::BTreeMap;")
        emit("use std::collections::HashMap;", "clippy::disallowed_types")
        emit("mod libm { pub fn sin(x: f64) -> f64 { x } pub fn pow(x: f64, _y: f64) -> f64 { x } }  // a stand-in: no network needed")
        for ty in ("f64", "f32"):
            emit(f"pub fn banned_{ty}(x: {ty}, y: {ty}) -> {ty} {{")
            emit("    let mut acc = 0.0;")
            for name in MATH:
                assert f"{ty}::{name}" in methods, f"{ty}::{name} missing from clippy.toml"
                emit("    " + method_call(name), "clippy::disallowed_methods")
            emit("    acc")
            emit("}")
        emit("pub fn clocks() {")
        emit("    let _a = std::time::Instant::now();", "clippy::disallowed_methods")
        emit("    let _b = std::time::SystemTime::now();", "clippy::disallowed_methods")
        emit("}")
        emit("pub fn types() -> usize {")
        emit("    let a: HashMap<u32, u32> = HashMap::new();", "clippy::disallowed_types")
        emit("    let b = std::collections::HashSet::<u8>::new();", "clippy::disallowed_types")
        emit("    a.len() + b.len()")
        emit("}")
        emit("pub fn allowed(x: f64, y: f64) -> f64 {")
        emit("    let m: BTreeMap<u32, u32> = BTreeMap::new();", clean=True)
        emit("    let mut acc = libm::sin(x) + libm::pow(x, y) + x.sqrt() + x.abs() + x.floor() + x.ceil() + x.round();", clean=True)
        emit("    acc += x.min(y) + x.max(y) + x.clamp(0.0, 1.0) + x.copysign(y) + x.signum() + x.trunc() + x.fract();", clean=True)
        emit("    acc + m.len() as f64", clean=True)
        emit("}")
        cls.lines = lines

        (cls.work / "crates/probe/src").mkdir(parents=True)
        shutil.copy(CLIPPY_TOML, cls.work / "clippy.toml")  # at the workspace root, like the real one
        (cls.work / "Cargo.toml").write_text('[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n')
        (cls.work / "crates/probe/Cargo.toml").write_text('[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n')
        (cls.work / "crates/probe/src/lib.rs").write_text("\n".join(lines) + "\n")
        cls.found, cls.other = cls.run_clippy(cls.work)

    @classmethod
    def run_clippy(cls, work):
        proc = subprocess.run(
            ["cargo", "clippy", "--offline", "--workspace", "--all-targets", "--message-format=json", "--", "-D", "warnings"],
            cwd=work, capture_output=True, text=True,
        )
        found, other = {}, []
        for raw in proc.stdout.splitlines():
            try:
                msg = json.loads(raw)
            except ValueError:
                continue
            if msg.get("reason") != "compiler-message":
                continue
            message = msg["message"]
            code = (message.get("code") or {}).get("code")
            spans = [s for s in message.get("spans", []) if s.get("is_primary") and s["file_name"].endswith("lib.rs")]
            if code and spans:
                found.setdefault(spans[0]["line_start"], set()).add(code)
            elif message["level"] in ("warning", "error") and "aborting due to" not in message["message"] and "could not compile" not in message["message"]:
                other.append(message["message"])
        cls.returncode = proc.returncode
        return found, other

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def test_every_banned_call_is_flagged_on_its_own_line(self):
        for number, lint in sorted(self.expect.items()):
            self.assertIn(lint, self.found.get(number, set()), f"line {number} not flagged: {self.lines[number - 1].strip()}")

    def test_the_allowed_calls_are_clean(self):
        for number in sorted(self.clean):
            self.assertNotIn(number, self.found, f"line {number} wrongly flagged: {self.lines[number - 1].strip()}")

    def test_nothing_unexpected_is_flagged_and_the_config_has_no_warnings(self):
        unexpected = {n: c for n, c in self.found.items() if n not in self.expect}
        self.assertEqual(unexpected, {})
        self.assertEqual(self.other, [], "clippy reported something about the configuration itself")

    def test_clippy_fails_the_build_like_ci_does(self):
        self.assertNotEqual(self.returncode, 0)

    def test_a_module_level_allow_is_the_documented_exception_for_types_only(self):
        work = self.work / "exception"
        (work / "crates/tool/src").mkdir(parents=True)
        shutil.copy(CLIPPY_TOML, work / "clippy.toml")
        (work / "Cargo.toml").write_text('[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n')
        (work / "crates/tool/Cargo.toml").write_text('[package]\nname = "tool"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n')
        (work / "crates/tool/src/lib.rs").write_text(
            "#![allow(clippy::disallowed_types)] // report tool: iteration order never reaches the simulation\n"
            "use std::collections::HashMap;\n"
            "pub fn count(words: &[&str]) -> usize { let mut m: HashMap<&str, u32> = HashMap::new(); for w in words { *m.entry(w).or_default() += 1; } m.len() }\n"
            "pub fn still_banned(x: f64) -> f64 { x.sin() }\n"
        )
        found, _ = self.run_clippy(work)
        self.assertEqual({n: sorted(c) for n, c in found.items()}, {4: ["clippy::disallowed_methods"]})


if __name__ == "__main__":
    unittest.main()
