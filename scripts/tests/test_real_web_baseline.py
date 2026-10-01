from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import real_web_baseline
import real_web_corpus

RAROG_COMMIT = "d" * 40


class RealWebBaselineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.corpus = real_web_corpus.load_corpus(
            ROOT / "real-web" / "corpus.json",
            root=ROOT,
        )
        cls.scenario = cls.corpus["scenarios"][0]

    def completed_attempt(self) -> dict:
        dependency = self.scenario["external_dependencies"][0]
        observations = []
        values = {
            "document-title": "RFC 9110",
            "final-url": self.scenario["source_url"],
            "render-completion": True,
            "screenshot": "sha256:" + "b" * 64,
        }
        for kind in self.scenario["observations"]:
            observations.append(
                {"kind": kind, "state": "observed", "value": values[kind]}
            )
        return {
            "schema_version": 1,
            "scenario_id": self.scenario["id"],
            "outcome": {
                "category": "completed-observation",
                "detail": "completed",
                "diagnostic": None,
            },
            "dependencies": [
                {
                    "origin": dependency["origin"],
                    "role": dependency["role"],
                    "state": "available",
                    "resolved_addresses": ["1.1.1.1"],
                    "http_status": 200,
                    "content_sha256": "sha256:" + "a" * 64,
                    "expected_content_sha256": None,
                }
            ],
            "observations": observations,
        }

    def execution(self) -> dict:
        return {
            "schema_version": 1,
            "corpus_revision": self.corpus["corpus_revision"],
            "scenario_ids": [self.scenario["id"]],
            "attempts": [self.completed_attempt()],
        }

    def platform(self) -> dict[str, str]:
        return {
            "os": "linux",
            "arch": "x86_64",
            "environment": "fixture",
        }

    def test_exact_corpus_execution_builds_content_addressed_baseline(self) -> None:
        result = real_web_baseline.build_baseline(
            self.corpus,
            self.execution(),
            rarog_commit=RAROG_COMMIT,
            platform=self.platform(),
        )
        self.assertEqual(result["scenario_ids"], [self.scenario["id"]])
        self.assertEqual(len(result["results"]), 1)
        self.assertRegex(result["execution_sha256"], r"^sha256:[0-9a-f]{64}$")
        self.assertEqual(
            result["results"][0]["outcome"]["category"],
            "completed-observation",
        )
        markdown = real_web_baseline.render_markdown(result)
        self.assertIn("not a general-Web compatibility score", markdown)

    def test_missing_scenario_is_rejected(self) -> None:
        execution = self.execution()
        execution["scenario_ids"] = []
        execution["attempts"] = []
        with self.assertRaisesRegex(
            real_web_baseline.BaselineError,
            "scenario_ids must exactly match",
        ):
            real_web_baseline.build_baseline(
                self.corpus,
                execution,
                rarog_commit=RAROG_COMMIT,
                platform=self.platform(),
            )

    def test_attempt_order_and_identity_are_fail_closed(self) -> None:
        execution = self.execution()
        execution["attempts"][0]["scenario_id"] = "other"
        with self.assertRaisesRegex(
            real_web_baseline.BaselineError,
            "must be scenario",
        ):
            real_web_baseline.build_baseline(
                self.corpus,
                execution,
                rarog_commit=RAROG_COMMIT,
                platform=self.platform(),
            )

    def test_invalid_r6_6_classification_is_rejected(self) -> None:
        execution = self.execution()
        attempt = execution["attempts"][0]
        attempt["outcome"] = {
            "category": "engine-failure",
            "detail": "render-failure",
            "diagnostic": "fixture",
        }
        attempt["dependencies"][0] = {
            "origin": self.scenario["external_dependencies"][0]["origin"],
            "role": "primary-document",
            "state": "dns-failure",
            "resolved_addresses": [],
            "http_status": None,
            "content_sha256": None,
            "expected_content_sha256": None,
        }
        with self.assertRaises(real_web_baseline.BaselineError):
            real_web_baseline.build_baseline(
                self.corpus,
                execution,
                rarog_commit=RAROG_COMMIT,
                platform=self.platform(),
            )

    def test_baseline_is_deterministic_for_identical_raw_execution(self) -> None:
        execution = self.execution()
        first = real_web_baseline.build_baseline(
            self.corpus,
            copy.deepcopy(execution),
            rarog_commit=RAROG_COMMIT,
            platform=self.platform(),
        )
        second = real_web_baseline.build_baseline(
            self.corpus,
            copy.deepcopy(execution),
            rarog_commit=RAROG_COMMIT,
            platform=self.platform(),
        )
        self.assertEqual(first, second)
        self.assertEqual(
            real_web_baseline.render_markdown(first),
            real_web_baseline.render_markdown(second),
        )


if __name__ == "__main__":
    unittest.main()
