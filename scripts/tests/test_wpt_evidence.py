from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "wpt_evidence.py"
RAROG_COMMIT = "d" * 40
WPT_COMMIT = "a" * 40


def load_module():
    spec = importlib.util.spec_from_file_location("wpt_evidence", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load wpt_evidence module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def selection():
    return {
        "schema_version": 1,
        "source": {
            "repository": "web-platform-tests/wpt",
            "commit": WPT_COMMIT,
        },
        "tests": [
            {"path": "a.html"},
            {"path": "b.html"},
        ],
    }


def report(ids=("/a.html", "/b.html")):
    return {
        "run_info": {"product": "rarog"},
        "results": [
            {"test": test_id, "status": "ERROR", "subtests": []}
            for test_id in ids
        ],
    }


class WptEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.evidence = load_module()

    def test_exact_denominator_is_content_addressed(self) -> None:
        result = self.evidence.build_evidence(
            selection(),
            report(),
            rarog_commit=RAROG_COMMIT,
            wpt_commit=WPT_COMMIT,
        )
        self.assertEqual(result["selected_tests"], 2)
        self.assertEqual(result["observed_tests"], 2)
        self.assertEqual(result["test_ids"], ["/a.html", "/b.html"])
        self.assertRegex(result["selection_sha256"], r"^sha256:[0-9a-f]{64}$")
        self.assertRegex(result["report_sha256"], r"^sha256:[0-9a-f]{64}$")

    def test_missing_or_extra_tests_are_rejected(self) -> None:
        for ids in (("/a.html",), ("/a.html", "/b.html", "/c.html")):
            with self.subTest(ids=ids):
                with self.assertRaisesRegex(
                    self.evidence.EvidenceError,
                    "denominator mismatch",
                ):
                    self.evidence.build_evidence(
                        selection(),
                        report(ids),
                        rarog_commit=RAROG_COMMIT,
                        wpt_commit=WPT_COMMIT,
                    )

    def test_duplicate_report_ids_are_rejected(self) -> None:
        with self.assertRaisesRegex(
            self.evidence.EvidenceError,
            "duplicate test IDs",
        ):
            self.evidence.build_evidence(
                selection(),
                report(("/a.html", "/a.html")),
                rarog_commit=RAROG_COMMIT,
                wpt_commit=WPT_COMMIT,
            )

    def test_selection_commit_must_match_execution_commit(self) -> None:
        with self.assertRaisesRegex(
            self.evidence.EvidenceError,
            "selection WPT commit",
        ):
            self.evidence.build_evidence(
                selection(),
                report(),
                rarog_commit=RAROG_COMMIT,
                wpt_commit="b" * 40,
            )

    def test_cli_output_is_repeatable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selection_path = root / "selection.json"
            report_path = root / "report.json"
            first = root / "first.json"
            second = root / "second.json"
            selection_path.write_text(json.dumps(selection()), encoding="utf-8")
            report_path.write_text(json.dumps(report()), encoding="utf-8")
            args = [
                "--selection",
                str(selection_path),
                "--report",
                str(report_path),
                "--rarog-commit",
                RAROG_COMMIT,
                "--wpt-commit",
                WPT_COMMIT,
            ]
            self.assertEqual(
                self.evidence.main(args + ["--json-out", str(first)]),
                0,
            )
            self.assertEqual(
                self.evidence.main(args + ["--json-out", str(second)]),
                0,
            )
            self.assertEqual(first.read_bytes(), second.read_bytes())


if __name__ == "__main__":
    unittest.main()
