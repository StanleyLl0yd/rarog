from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import wpt_dashboard
import wpt_evidence
import wpt_selection

MODULE_PATH = SCRIPTS / "wpt_compare.py"
WPT_COMMIT = "a" * 40
OTHER_WPT_COMMIT = "b" * 40
RAROG_BASE = "c" * 40
RAROG_CANDIDATE = "d" * 40


def load_module():
    spec = importlib.util.spec_from_file_location("wpt_compare", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load wpt_compare module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def raw_selection(
    *,
    wpt_commit: str = WPT_COMMIT,
    paths: tuple[str, ...] = ("a.html", "b.html"),
    blob_override: dict[str, str] | None = None,
):
    blob_override = blob_override or {}
    tests = []
    for index, path in enumerate(paths):
        tests.append(
            {
                "area": "fixture",
                "path": path,
                "kind": "testharness",
                "blob": blob_override.get(path, f"{index + 1:x}" * 40),
                "references": [],
            }
        )
    return {
        "schema_version": 1,
        "source": {
            "repository": "web-platform-tests/wpt",
            "commit": wpt_commit,
        },
        "tests": tests,
    }


def raw_report(
    states: dict[str, tuple[str, str | list[str] | None]],
):
    results = []
    for test_id in sorted(states):
        status, expected = states[test_id]
        item = {
            "test": test_id,
            "status": status,
            "subtests": [],
        }
        if expected is not None:
            item["expected"] = expected
        results.append(item)
    return {"run_info": {"product": "rarog"}, "results": results}


def make_bundle(
    *,
    selection=None,
    states=None,
    rarog_commit: str = RAROG_BASE,
    platform: str = "linux-x86_64",
):
    selection = selection or raw_selection()
    normalized_selection = wpt_selection.validate_manifest(selection)
    states = states or {
        "/a.html": ("PASS", "PASS"),
        "/b.html": ("ERROR", "OK"),
    }
    report = raw_report(states)
    wpt_commit = normalized_selection["source"]["commit"]
    evidence = wpt_evidence.build_evidence(
        normalized_selection,
        report,
        rarog_commit=rarog_commit,
        wpt_commit=wpt_commit,
    )
    dashboard = wpt_dashboard.normalize_reports(
        [(evidence["report_sha256"], report)],
        rarog_commit=rarog_commit,
        wpt_commit=wpt_commit,
        platform=platform,
        synthetic=False,
    )
    return normalized_selection, report, evidence, dashboard


class WptCompareTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.compare = load_module()

    def validated(self, raw, label="bundle"):
        selection, report, evidence, dashboard = raw
        return self.compare.validate_bundle(
            selection=selection,
            report=report,
            evidence=evidence,
            dashboard=dashboard,
            label=label,
        )

    def test_committed_first_evidence_is_self_comparable(self) -> None:
        bundle = self.compare.load_bundle(
            selection_path=ROOT / "wpt" / "r6-selection.json",
            report_path=ROOT / "wpt" / "evidence" / "r6-first-wptreport.json",
            evidence_path=ROOT / "wpt" / "evidence" / "r6-first-evidence.json",
            dashboard_path=ROOT / "wpt" / "evidence" / "r6-first-dashboard.json",
            label="committed",
        )
        result = self.compare.compare_bundles(bundle, bundle)
        self.assertEqual(
            result["classification"],
            "same-upstream-same-denominator",
        )
        self.assertTrue(result["direct_behavior_comparable"])
        self.assertEqual(result["summary"]["changed_results"], 0)
        self.assertEqual(result["summary"]["regressions"], 0)
        self.assertEqual(result["summary"]["improvements"], 0)

    def test_expected_to_unexpected_is_regression(self) -> None:
        baseline = self.validated(
            make_bundle(
                states={
                    "/a.html": ("PASS", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                }
            ),
            "baseline",
        )
        candidate = self.validated(
            make_bundle(
                states={
                    "/a.html": ("FAIL", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
            ),
            "candidate",
        )
        result = self.compare.compare_bundles(baseline, candidate)
        changed = {item["test"]: item for item in result["results"]}
        self.assertTrue(result["direct_behavior_comparable"])
        self.assertEqual(changed["/a.html"]["interpretation"], "regression")
        self.assertEqual(result["summary"]["regressions"], 1)

    def test_unexpected_to_expected_is_improvement(self) -> None:
        baseline = self.validated(
            make_bundle(
                states={
                    "/a.html": ("FAIL", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                }
            )
        )
        candidate = self.validated(
            make_bundle(
                states={
                    "/a.html": ("PASS", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        changed = {item["test"]: item for item in result["results"]}
        self.assertEqual(changed["/a.html"]["interpretation"], "improvement")
        self.assertEqual(result["summary"]["improvements"], 1)

    def test_unexpected_to_unexpected_is_not_ranked(self) -> None:
        baseline = self.validated(
            make_bundle(
                states={
                    "/a.html": ("ERROR", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                }
            )
        )
        candidate = self.validated(
            make_bundle(
                states={
                    "/a.html": ("FAIL", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        changed = {item["test"]: item for item in result["results"]}
        self.assertEqual(
            changed["/a.html"]["interpretation"],
            "changed-unexpected-status",
        )
        self.assertEqual(result["summary"]["improvements"], 0)
        self.assertEqual(result["summary"]["regressions"], 0)

    def test_platform_change_blocks_behavior_interpretation(self) -> None:
        baseline = self.validated(make_bundle(platform="linux"))
        candidate = self.validated(
            make_bundle(
                states={
                    "/a.html": ("FAIL", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
                platform="windows",
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        changed = {item["test"]: item for item in result["results"]}
        self.assertFalse(result["direct_behavior_comparable"])
        self.assertIn("platform-changed", result["non_comparable_reasons"])
        self.assertEqual(
            changed["/a.html"]["interpretation"],
            "observed-change-not-directly-comparable",
        )

    def test_expectation_change_is_not_behavior_verdict(self) -> None:
        baseline = self.validated(make_bundle())
        candidate = self.validated(
            make_bundle(
                states={
                    "/a.html": ("FAIL", ["FAIL", "PASS"]),
                    "/b.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        self.assertFalse(result["direct_behavior_comparable"])
        self.assertEqual(
            result["expectation_changes"],
            [{"test": "/a.html", "before": ["PASS"], "after": ["FAIL", "PASS"]}],
        )
        changed = {item["test"]: item for item in result["results"]}
        self.assertEqual(
            changed["/a.html"]["interpretation"],
            "observed-change-not-directly-comparable",
        )

    def test_same_upstream_changed_denominator_is_explicit(self) -> None:
        baseline = self.validated(make_bundle())
        selection = raw_selection(paths=("a.html", "b.html", "c.html"))
        candidate = self.validated(
            make_bundle(
                selection=selection,
                states={
                    "/a.html": ("PASS", "PASS"),
                    "/b.html": ("ERROR", "OK"),
                    "/c.html": ("PASS", "PASS"),
                },
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        self.assertEqual(
            result["classification"],
            "same-upstream-changed-denominator",
        )
        self.assertEqual(result["scope"]["added_tests"], ["/c.html"])
        self.assertFalse(result["direct_behavior_comparable"])

    def test_changed_upstream_same_logical_selection_is_explicit(self) -> None:
        baseline = self.validated(make_bundle())
        changed = raw_selection(
            wpt_commit=OTHER_WPT_COMMIT,
            blob_override={"a.html": "e" * 40},
        )
        candidate = self.validated(
            make_bundle(
                selection=changed,
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        self.assertEqual(
            result["classification"],
            "changed-upstream-same-logical-selection",
        )
        self.assertEqual(
            [item["test"] for item in result["scope"]["source_changes"]],
            ["/a.html"],
        )
        self.assertFalse(result["direct_behavior_comparable"])

    def test_changed_upstream_and_denominator_are_separate(self) -> None:
        baseline = self.validated(make_bundle())
        candidate = self.validated(
            make_bundle(
                selection=raw_selection(
                    wpt_commit=OTHER_WPT_COMMIT,
                    paths=("a.html", "c.html"),
                ),
                states={
                    "/a.html": ("PASS", "PASS"),
                    "/c.html": ("ERROR", "OK"),
                },
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        self.assertEqual(
            result["classification"],
            "changed-upstream-changed-denominator",
        )
        self.assertEqual(result["scope"]["added_tests"], ["/c.html"])
        self.assertEqual(result["scope"]["removed_tests"], ["/b.html"])

    def test_same_upstream_source_blob_drift_blocks_direct_comparison(self) -> None:
        baseline = self.validated(make_bundle())
        changed = raw_selection(blob_override={"a.html": "f" * 40})
        candidate = self.validated(
            make_bundle(
                selection=changed,
                rarog_commit=RAROG_CANDIDATE,
            )
        )
        result = self.compare.compare_bundles(baseline, candidate)
        self.assertEqual(
            result["classification"],
            "same-upstream-same-denominator",
        )
        self.assertFalse(result["direct_behavior_comparable"])
        self.assertIn(
            "selected-source-content-changed",
            result["non_comparable_reasons"],
        )

    def test_invalid_evidence_or_dashboard_fails_closed(self) -> None:
        selection, report, evidence, dashboard = make_bundle()
        broken_evidence = copy.deepcopy(evidence)
        broken_evidence["observed_tests"] = 99
        with self.assertRaisesRegex(
            self.compare.ComparisonError,
            "evidence JSON does not reproduce exactly",
        ):
            self.compare.validate_bundle(
                selection=selection,
                report=report,
                evidence=broken_evidence,
                dashboard=dashboard,
                label="broken",
            )

        broken_dashboard = copy.deepcopy(dashboard)
        broken_dashboard["summary"]["measured_tests"] = 99
        with self.assertRaisesRegex(
            self.compare.ComparisonError,
            "dashboard JSON does not reproduce exactly",
        ):
            self.compare.validate_bundle(
                selection=selection,
                report=report,
                evidence=evidence,
                dashboard=broken_dashboard,
                label="broken",
            )

    def test_cli_outputs_are_repeatable(self) -> None:
        baseline = make_bundle()
        candidate = make_bundle(rarog_commit=RAROG_CANDIDATE)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def write_bundle(prefix, bundle):
                paths = {}
                for suffix, document in zip(
                    ("selection", "report", "evidence", "dashboard"),
                    bundle,
                ):
                    path = root / f"{prefix}-{suffix}.json"
                    path.write_text(
                        json.dumps(document, sort_keys=True),
                        encoding="utf-8",
                    )
                    paths[suffix] = path
                return paths

            before = write_bundle("before", baseline)
            after = write_bundle("after", candidate)
            first_json = root / "first.json"
            first_md = root / "first.md"
            second_json = root / "second.json"
            second_md = root / "second.md"

            common = [
                "--baseline-selection", str(before["selection"]),
                "--baseline-report", str(before["report"]),
                "--baseline-evidence", str(before["evidence"]),
                "--baseline-dashboard", str(before["dashboard"]),
                "--candidate-selection", str(after["selection"]),
                "--candidate-report", str(after["report"]),
                "--candidate-evidence", str(after["evidence"]),
                "--candidate-dashboard", str(after["dashboard"]),
            ]
            self.assertEqual(
                self.compare.main(
                    common
                    + ["--json-out", str(first_json), "--markdown-out", str(first_md)]
                ),
                0,
            )
            self.assertEqual(
                self.compare.main(
                    common
                    + ["--json-out", str(second_json), "--markdown-out", str(second_md)]
                ),
                0,
            )
            self.assertEqual(first_json.read_bytes(), second_json.read_bytes())
            self.assertEqual(first_md.read_bytes(), second_md.read_bytes())


if __name__ == "__main__":
    unittest.main()
