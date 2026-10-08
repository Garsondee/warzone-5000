"""docs/swarm/GUARDRAILS.md stays in step with the tooling: every check script and every hook is described there, and the
section that the lane guard's output points to exists. Runs against the real document when it is available (set
W5K_GUARDRAILS=/path/to/GUARDRAILS.md when testing from a staging tree that does not contain docs/)."""

import os
import re
import unittest
from pathlib import Path

import ci_testlib

HELPERS = {"ci_common.py", "rust_lex.py"}  # shared modules, not checks


def doc_path():
    for candidate in (os.environ.get("W5K_GUARDRAILS"), ci_testlib.ROOT / "docs/swarm/GUARDRAILS.md"):
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    return None


@unittest.skipUnless(doc_path(), "docs/swarm/GUARDRAILS.md not available (set W5K_GUARDRAILS)")
class GuardrailsDocument(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = doc_path().read_text()

    def test_every_check_script_is_described(self):
        for script in sorted(p.name for p in ci_testlib.CI_DIR.glob("*.py") if p.name not in HELPERS):
            self.assertIn(script, self.doc, f"{script} is not mentioned in GUARDRAILS.md")

    def test_every_hook_is_described(self):
        for hook in sorted(p.name for p in (ci_testlib.ROOT / ".claude/hooks").glob("*.sh")):
            self.assertIn(hook, self.doc, f"{hook} is not mentioned in GUARDRAILS.md")

    def test_every_workflow_is_described(self):
        for workflow in sorted(p.name for p in (ci_testlib.ROOT / ".github/workflows").glob("*.yml")):
            self.assertIn(workflow, self.doc, f"{workflow} is not mentioned in GUARDRAILS.md")

    def test_the_sections_other_files_point_to_exist(self):
        self.assertRegex(self.doc, r"(?m)^## Lane guard$")  # lane_guard.py's output says: see section 'Lane guard'
        lane_guard = (ci_testlib.CI_DIR / "lane_guard.py").read_text()
        self.assertIn("GUARDRAILS.md, section 'Lane guard'", lane_guard)

    def test_the_key_rules_are_stated(self):
        for needle in ("[protected]", "CCR:", "Golden-Change:", "const-ok", "large_files.allow", "budgets.toml", "libm", "interface request",
                       "decision card", "workspace = true", "claude-code-remote"):
            self.assertIn(needle, self.doc)

    def test_the_numbers_in_the_document_match_the_code(self):
        import constants_lint, media_lint

        self.assertIn("400 KB", self.doc)
        self.assertEqual(media_lint.IMAGE_LIMITS["png"], 400 * 1024)
        self.assertIn("GIF 800 KB", self.doc)
        self.assertEqual(media_lint.IMAGE_LIMITS["gif"], 800 * 1024)
        self.assertIn("8 MB", self.doc)
        self.assertEqual(media_lint.TOTAL_LIMIT, 8 * 1024 * 1024)
        self.assertIn("anything else 1 MB", self.doc)
        self.assertEqual(media_lint.OTHER_LIMIT, 1024 * 1024)
        for ext in media_lint.FORBIDDEN_EXTENSIONS:
            self.assertIn(ext, self.doc)
        for value in ("0.0", "0.5", "1.0", "2.0", "3.0", "4.0", "0.25"):
            self.assertIn(value, self.doc)
        self.assertEqual(sorted(str(v) for v in constants_lint.ALLOWED_VALUES), sorted(["0.0", "0.5", "1.0", "2.0", "3.0", "4.0", "0.25"]))

    def test_budgets_in_the_document_match_the_budgets_file(self):
        budgets = Path(os.environ.get("W5K_BUDGETS", ci_testlib.ROOT / "docs/swarm/budgets.toml"))
        if not budgets.is_file():
            self.skipTest("budgets.toml not available (set W5K_BUDGETS)")
        import line_budget

        default, overrides = line_budget.read_budgets(budgets)
        self.assertIn(f"{default:,} lines by default", self.doc)
        for crate, limit in overrides.items():
            self.assertRegex(self.doc, rf"`{crate}` {limit:,}")


if __name__ == "__main__":
    unittest.main()
