from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "wpt_dashboard.py"
FIXTURE = ROOT / "wpt" / "fixtures" / "synthetic-wptreport.json"
RAROG_COMMIT = "b" * 40
WPT_COMMIT = "c" * 40


def load_module():
    spec = importlib.util.spec_from_file_location("wpt_dashboard", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load wpt_dashboard module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class WptDashboardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dashboard = load_module()

    def test_synthetic_fixture_summary_is_bounded_and_explicit(self) -> None:
        reports = self.dashboard.load_reports([FIXTURE])
        normalized = self.dashboard.normalize_reports(
            reports,
            rarog_commit=RAROG_COMMIT,
            wpt_commit=WPT_COMMIT,
            platform="synthetic-linux",
            synthetic=True,
        )

        self.assertEqual(
            normalized["source_reports"],
            ["sha256:cf9e06050ce2445663ffb248eed90e7103df1e88bd24980ff171020ef2179c91"],
        )
        self.assertEqual(
            normalized["summary"],
            {
                "measured_tests": 3,
                "measured_subtests": 2,
                "tests_with_expectations": 2,
                "subtests_with_expectations": 2,
                "unexpected_tests": 0,
                "unexpected_subtests": 1,
                "test_statuses": {"ERROR": 1, "OK": 1, "TIMEOUT": 1},
                "subtest_statuses": {"FAIL": 1, "PASS": 1},
            },
        )
        markdown = self.dashboard.render_markdown(normalized)
        self.assertIn("Synthetic fixture — not compatibility evidence.", markdown)
        self.assertIn("does not infer results for unmeasured tests", markdown)

    def test_missing_expectation_is_not_counted_as_unexpected(self) -> None:
        normalized = self.dashboard.normalize_reports(
            [
                (
                    "sha256:fixture",
                    {"results": [{"test": "/unmeasured-expectation.html", "status": "FAIL"}]},
                )
            ],
            rarog_commit=RAROG_COMMIT,
            wpt_commit=WPT_COMMIT,
            platform="linux",
            synthetic=False,
        )
        self.assertEqual(normalized["summary"]["tests_with_expectations"], 0)
        self.assertEqual(normalized["summary"]["unexpected_tests"], 0)
        self.assertIsNone(normalized["results"][0]["expected"])

    def test_duplicate_test_ids_across_shards_are_rejected(self) -> None:
        report = {"results": [{"test": "/duplicate.html", "status": "PASS"}]}
        with self.assertRaisesRegex(self.dashboard.DashboardError, "duplicate test id"):
            self.dashboard.normalize_reports(
                [("sha256:a", report), ("sha256:b", report)],
                rarog_commit=RAROG_COMMIT,
                wpt_commit=WPT_COMMIT,
                platform="linux",
                synthetic=False,
            )

    def test_duplicate_subtests_are_rejected(self) -> None:
        report = {
            "results": [
                {
                    "test": "/duplicate-subtest.html",
                    "status": "OK",
                    "subtests": [
                        {"name": "same", "status": "PASS"},
                        {"name": "same", "status": "FAIL"},
                    ],
                }
            ]
        }
        with self.assertRaisesRegex(self.dashboard.DashboardError, "duplicate subtest name"):
            self.dashboard.normalize_reports(
                [("sha256:a", report)],
                rarog_commit=RAROG_COMMIT,
                wpt_commit=WPT_COMMIT,
                platform="linux",
                synthetic=False,
            )

    def test_empty_reports_are_rejected(self) -> None:
        with self.assertRaisesRegex(self.dashboard.DashboardError, "no test results"):
            self.dashboard.normalize_reports(
                [("sha256:a", {"results": []})],
                rarog_commit=RAROG_COMMIT,
                wpt_commit=WPT_COMMIT,
                platform="linux",
                synthetic=False,
            )

    def test_commits_must_be_exact_lowercase_sha1s(self) -> None:
        with self.assertRaisesRegex(self.dashboard.DashboardError, "exact lowercase 40-hex"):
            self.dashboard.normalize_reports(
                [("sha256:a", {"results": [{"test": "/a.html", "status": "PASS"}]})],
                rarog_commit="ABC",
                wpt_commit=WPT_COMMIT,
                platform="linux",
                synthetic=False,
            )

    def test_cli_outputs_are_repeatable(self) -> None:
        with tempfile.TemporaryDirectory() as first_dir, tempfile.TemporaryDirectory() as second_dir:
            first_json = Path(first_dir) / "dashboard.json"
            first_md = Path(first_dir) / "dashboard.md"
            second_json = Path(second_dir) / "dashboard.json"
            second_md = Path(second_dir) / "dashboard.md"
            common = [
                "--report",
                str(FIXTURE),
                "--rarog-commit",
                RAROG_COMMIT,
                "--wpt-commit",
                WPT_COMMIT,
                "--platform",
                "synthetic-linux",
                "--synthetic",
            ]
            self.assertEqual(
                self.dashboard.main(
                    common
                    + ["--json-out", str(first_json), "--markdown-out", str(first_md)]
                ),
                0,
            )
            self.assertEqual(
                self.dashboard.main(
                    common
                    + ["--json-out", str(second_json), "--markdown-out", str(second_md)]
                ),
                0,
            )
            self.assertEqual(first_json.read_bytes(), second_json.read_bytes())
            self.assertEqual(first_md.read_bytes(), second_md.read_bytes())
            parsed = json.loads(first_json.read_text(encoding="utf-8"))
            self.assertTrue(parsed["synthetic"])


if __name__ == "__main__":
    unittest.main()
