"""Tests for affected.py: a fake `cargo metadata` for the logic, a real one on a tiny workspace for the plumbing."""

import json
import shutil
import tempfile
import unittest
from pathlib import Path

import ci_testlib  # noqa: F401  (must come first: it puts tools/ci on sys.path)
import affected
from ci_testlib import TempRepo, run_main, run_script


def package(name, *deps, dev=(), root="/repo"):
    """One entry of `cargo metadata --no-deps` output."""
    entries = [{"name": d, "source": None, "kind": None, "path": f"{root}/crates/{d}"} for d in deps]
    entries += [{"name": d, "source": None, "kind": "dev", "path": f"{root}/crates/{d}"} for d in dev]
    entries.append({"name": "serde", "source": "registry+https://github.com/rust-lang/crates.io-index", "kind": None})
    return {"name": name, "manifest_path": f"{root}/crates/{name}/Cargo.toml", "dependencies": entries}


#   w5k_math <- w5k_contract <- w5k_chassis <- w5k_vehicle <- w5k_sim <- w5k_tools
#                           \\<- w5k_drive  <--/                  w5k_validate (dev-depends on w5k_forge) -> w5k_sim
METADATA = {
    "workspace_root": "/repo",
    "packages": [
        package("w5k_math"),
        package("w5k_contract", "w5k_math"),
        package("w5k_chassis", "w5k_math", "w5k_contract"),
        package("w5k_drive", "w5k_math", "w5k_contract"),
        package("w5k_forge", "w5k_math", "w5k_contract"),
        package("w5k_vehicle", "w5k_math", "w5k_contract", "w5k_chassis", "w5k_drive"),
        package("w5k_sim", "w5k_math", "w5k_contract", "w5k_vehicle"),
        package("w5k_validate", "w5k_sim", dev=("w5k_forge",)),
        package("w5k_tools", "w5k_sim", "w5k_validate", "w5k_forge"),
    ],
}


def args_for(*paths):
    return affected.affected_args(list(paths), METADATA)


class CrateMapping(unittest.TestCase):
    def test_a_leaf_crate_affects_only_itself(self):
        self.assertEqual(args_for("crates/w5k_tools/src/cmd/arch.rs"), "-p w5k_tools")

    def test_reverse_dependencies_are_included_transitively(self):
        self.assertEqual(args_for("crates/w5k_chassis/src/lib.rs"),
                         "-p w5k_chassis -p w5k_sim -p w5k_tools -p w5k_validate -p w5k_vehicle")

    def test_the_root_of_the_graph_affects_everything(self):
        every = " ".join(f"-p {p['name']}" for p in sorted(METADATA["packages"], key=lambda p: p["name"]))
        self.assertEqual(args_for("crates/w5k_math/src/vec3.rs"), every)

    def test_dev_dependencies_count(self):
        # w5k_validate only dev-depends on w5k_forge, but its tests use it.
        self.assertIn("-p w5k_validate", args_for("crates/w5k_forge/src/lib.rs"))

    def test_several_changes_are_merged_and_sorted(self):
        self.assertEqual(args_for("crates/w5k_tools/Cargo.toml", "crates/w5k_validate/src/lib.rs", "crates/w5k_tools/src/main.rs"),
                         "-p w5k_tools -p w5k_validate")

    def test_files_directly_under_a_crate_folder_count(self):
        self.assertEqual(args_for("crates/w5k_tools/Cargo.toml"), "-p w5k_tools")
        self.assertEqual(args_for("crates/w5k_tools/tests/first.rs"), "-p w5k_tools")

    def test_output_is_one_line_in_the_documented_format(self):
        out = args_for("crates/w5k_vehicle/src/lib.rs")
        self.assertEqual(out, "-p w5k_sim -p w5k_tools -p w5k_validate -p w5k_vehicle")
        self.assertNotIn("\n", out)


class WholeWorkspace(unittest.TestCase):
    def test_root_level_files_and_ci_changes(self):
        for path in ("Cargo.toml", "Cargo.lock", "clippy.toml", "rustfmt.toml", "rust-toolchain.toml", "deny.toml",
                     ".github/workflows/pr.yml", ".github/CODEOWNERS", "tools/ci/lane_guard.py", "tools/ci/tests/test_x.py",
                     ".cargo/config.toml"):
            with self.subTest(path=path):
                self.assertEqual(args_for(path), "--workspace")

    def test_one_trigger_among_many_files_wins(self):
        self.assertEqual(args_for("crates/w5k_tools/src/main.rs", "docs/x.md", "Cargo.lock"), "--workspace")

    def test_content_changes_test_everything_because_the_tests_read_the_content(self):
        self.assertEqual(args_for("content/world/course.ron"), "--workspace")
        self.assertEqual(args_for("content/fixtures/hmmwv.golden.ron"), "--workspace")

    def test_an_unknown_crate_folder_means_test_everything(self):
        self.assertEqual(args_for("crates/w5k_new/src/lib.rs"), "--workspace")

    def test_a_deleted_crate_folder_means_test_everything(self):
        self.assertEqual(args_for("crates/w5k_removed/Cargo.toml", "crates/w5k_tools/src/main.rs"), "--workspace")


class NothingToTest(unittest.TestCase):
    def test_paths_that_cannot_affect_cargo_tests(self):
        for path in ("docs/swarm/status/chassis.md", "README.md", "CLAUDE.md", ".gitignore", "assets/shaders/camo.gdshader",
                     "game/project.godot", "tools/viewer/index.html", "spikes/chassis/s1/main.rs", "reference/x.rs",
                     "crates/README.md", "docs/lanes/chassis/media/a.png"):
            with self.subTest(path=path):
                self.assertEqual(args_for(path), "")

    def test_no_changes_at_all(self):
        self.assertEqual(args_for(), "")


class MetadataShapes(unittest.TestCase):
    def test_packages_without_a_manifest_path_use_the_crates_name_convention(self):
        meta = {"packages": [{"name": "a", "dependencies": []}, {"name": "b", "dependencies": [{"name": "a"}]}]}
        self.assertEqual(affected.affected_args(["crates/a/src/lib.rs"], meta), "-p a -p b")

    def test_a_package_outside_crates_is_mapped_by_its_real_directory(self):
        meta = {"workspace_root": "/repo", "packages": [
            {"name": "x", "manifest_path": "/repo/tools/x/Cargo.toml", "dependencies": []}]}
        self.assertEqual(affected.package_dirs(meta), {"tools/x": "x"})

    def test_only_workspace_members_form_edges(self):
        meta = {"packages": [{"name": "a", "dependencies": [{"name": "serde"}, {"name": "libm"}]}]}
        self.assertEqual(affected.affected_args(["crates/a/src/lib.rs"], meta), "-p a")

    def test_unsafe_looking_names_fall_back_to_the_whole_workspace(self):
        meta = {"packages": [{"name": "a; rm -rf", "manifest_path": "/r/crates/a/Cargo.toml", "dependencies": []}],
                "workspace_root": "/r"}
        self.assertEqual(affected.affected_args(["crates/a/src/lib.rs"], meta), "--workspace")

    def test_dependency_cycles_do_not_hang(self):
        meta = {"packages": [{"name": "a", "dependencies": [{"name": "b"}]}, {"name": "b", "dependencies": [{"name": "a"}]}]}
        self.assertEqual(affected.affected_args(["crates/a/src/lib.rs"], meta), "-p a -p b")


class CommandLine(unittest.TestCase):
    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.meta = Path(self._tmp.name) / "meta.json"
        self.meta.write_text(json.dumps(METADATA))
        self.repo.write("crates/w5k_chassis/src/lib.rs", "// a\n" * 30)
        self.repo.write("docs/readme.md", "x\n")
        self.repo.commit("base")

    def run_cli(self, files, *extra):
        self.repo.checkout("main")
        self.repo.branch(f"t{len(self.repo.git('branch').splitlines())}")
        for path, content in files.items():
            if content is None:
                self.repo.remove(path)
            else:
                self.repo.write(path, content)
        self.repo.commit("work")
        return run_main(affected.main, ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path), *extra])

    def test_prints_the_selection_on_stdout_and_the_reason_on_stderr(self):
        code, out, err = self.run_cli({"crates/w5k_chassis/src/lib.rs": "// b\n"}, "--metadata-file", str(self.meta))
        self.assertEqual(code, 0)
        self.assertEqual(out, "-p w5k_chassis -p w5k_sim -p w5k_tools -p w5k_validate -p w5k_vehicle\n")
        self.assertIn("1 changed path(s)", err)

    def test_docs_only_prints_an_empty_line(self):
        code, out, err = self.run_cli({"docs/readme.md": "y\n"}, "--metadata-file", str(self.meta))
        self.assertEqual((code, out), (0, "\n"))
        self.assertIn("no crates to test", err)

    def test_a_rename_between_crates_affects_both(self):
        self.repo.checkout("main")
        self.repo.branch("mv")
        self.repo.move("crates/w5k_chassis/src/lib.rs", "crates/w5k_drive/src/lib.rs")
        self.repo.commit("move")
        code, out, _ = run_main(affected.main, ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path),
                                                "--metadata-file", str(self.meta)])
        self.assertEqual(code, 0)
        self.assertIn("-p w5k_chassis", out)
        self.assertIn("-p w5k_drive", out)

    def test_root_file_prints_workspace(self):
        _, out, _ = self.run_cli({"clippy.toml": "# x\n"}, "--metadata-file", str(self.meta))
        self.assertEqual(out, "--workspace\n")

    def test_unreadable_metadata_falls_back_to_workspace_with_a_warning(self):
        self.meta.write_text("not json")
        code, out, err = self.run_cli({"crates/w5k_chassis/src/lib.rs": "// b\n"}, "--metadata-file", str(self.meta))
        self.assertEqual((code, out), (0, "--workspace\n"))
        self.assertIn("cannot tell what is affected", err)

    def test_missing_metadata_file_falls_back_to_workspace(self):
        _, out, _ = self.run_cli({"docs/readme.md": "y\n"}, "--metadata-file", "/nonexistent/meta.json")
        self.assertEqual(out, "--workspace\n")

    def test_git_failure_falls_back_to_workspace(self):
        code, out, err = run_main(affected.main, ["--base", "nope", "--head", "HEAD", "--repo", str(self.repo.path),
                                                  "--metadata-file", str(self.meta)])
        self.assertEqual((code, out), (0, "--workspace\n"))
        self.assertIn("cannot tell", err)

    def test_runs_under_python_dash_I(self):
        self.repo.checkout("main")
        self.repo.branch("x")
        self.repo.write("crates/w5k_tools/src/main.rs", "fn main() {}\n")
        self.repo.commit("w")
        proc = run_script("affected.py", "--base", "main", "--head", "HEAD", "--repo", str(self.repo.path),
                          "--metadata-file", str(self.meta))
        self.assertEqual((proc.returncode, proc.stdout), (0, "-p w5k_tools\n"), proc.stderr)
        self.assertEqual(run_script("affected.py", "--help").returncode, 0)


@unittest.skipUnless(shutil.which("cargo"), "cargo is not installed")
class AgainstRealCargoMetadata(unittest.TestCase):
    """The same logic fed by the real `cargo metadata --no-deps` of a three-crate workspace (offline, no registry)."""

    def setUp(self):
        self.repo = TempRepo()
        self.addCleanup(self.repo.cleanup)
        self.repo.write("Cargo.toml", '[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n')
        for name, deps in (("w5k_a", ""), ("w5k_b", '[dependencies]\nw5k_a = { path = "../w5k_a" }\n'),
                           ("w5k_c", '[dev-dependencies]\nw5k_b = { path = "../w5k_b" }\n')):
            self.repo.write(f"crates/{name}/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n\n{deps}')
            self.repo.write(f"crates/{name}/src/lib.rs", "// lib\n")
        self.repo.commit("base")

    def selection(self, *files):
        self.repo.checkout("main")
        self.repo.branch(f"t{len(self.repo.git('branch').splitlines())}")
        for path in files:
            self.repo.write(path, "// changed\n" + path)
        self.repo.commit("work")
        code, out, err = run_main(affected.main, ["--base", "main", "--head", "HEAD", "--repo", str(self.repo.path)])
        self.assertEqual(code, 0, err)
        return out.strip(), err

    def test_reverse_dependencies_from_real_metadata(self):
        self.assertEqual(self.selection("crates/w5k_a/src/lib.rs")[0], "-p w5k_a -p w5k_b -p w5k_c")  # c dev-depends on b
        self.assertEqual(self.selection("crates/w5k_b/src/lib.rs")[0], "-p w5k_b -p w5k_c")
        self.assertEqual(self.selection("crates/w5k_c/src/lib.rs")[0], "-p w5k_c")

    def test_outside_a_cargo_workspace_falls_back(self):
        (self.repo.path / "Cargo.toml").unlink()
        self.repo.commit("break the workspace")
        out, err = self.selection("crates/w5k_a/src/lib.rs")
        self.assertEqual(out, "--workspace")
        self.assertIn("cannot tell", err)


if __name__ == "__main__":
    unittest.main()
