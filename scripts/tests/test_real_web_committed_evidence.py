from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import real_web_baseline
import real_web_corpus


class CommittedRealWebEvidenceTests(unittest.TestCase):
    def test_first_baseline_reproduces_from_committed_raw_execution(self) -> None:
        corpus = real_web_corpus.load_corpus(
            ROOT / "real-web" / "corpus.json",
            root=ROOT,
        )
        execution = json.loads(
            (ROOT / "real-web" / "evidence" / "r6-first-execution.json").read_text(
                encoding="utf-8"
            )
        )
        expected = json.loads(
            (ROOT / "real-web" / "evidence" / "r6-first-baseline.json").read_text(
                encoding="utf-8"
            )
        )
        rebuilt = real_web_baseline.build_baseline(
            corpus,
            execution,
            rarog_commit=expected["rarog_commit"],
            platform=expected["platform"],
        )
        self.assertEqual(rebuilt, expected)
        self.assertEqual(
            real_web_baseline.render_markdown(rebuilt),
            (ROOT / "real-web" / "evidence" / "r6-first-baseline.md").read_text(
                encoding="utf-8"
            ),
        )
        self.assertEqual(
            expected["rarog_commit"],
            "2301370b6ccce06a8839a94a75dfeee12e3a3499",
        )
        self.assertEqual(
            expected["execution_sha256"],
            "sha256:daa3bf52900393e3755b903f0ba2fef9a310b3c3294c84869908ee27a1acf2b8",
        )
        self.assertEqual(expected["scenario_ids"], ["rfc9110-html"])
        self.assertEqual(
            expected["results"][0]["outcome"],
            {
                "category": "completed-observation",
                "detail": "completed",
                "diagnostic": None,
            },
        )


if __name__ == "__main__":
    unittest.main()
