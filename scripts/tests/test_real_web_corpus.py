from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "real_web_corpus.py"
CORPUS = ROOT / "real-web" / "corpus.json"


def load_module():
    spec = importlib.util.spec_from_file_location("real_web_corpus", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load real_web_corpus module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def live_scenario():
    return {
        "id": "example-live",
        "source_url": "https://example.test/page",
        "input": {"mode": "live-external"},
        "viewport": {"width": 800, "height": 600, "device_scale": 1},
        "actions": [
            {"kind": "load-input"},
            {"kind": "wait-for-idle", "timeout_ms": 1000},
        ],
        "observations": [
            "document-title",
            "final-url",
            "render-completion",
            "screenshot",
        ],
        "external_dependencies": [
            {
                "origin": "https://example.test",
                "role": "primary-document",
                "required": True,
            }
        ],
        "limits": {
            "timeout_ms": 5000,
            "max_response_bytes": 1024,
            "max_total_bytes": 4096,
            "max_subresources": 4,
        },
    }


def corpus_with(*scenarios):
    return {
        "schema_version": 1,
        "corpus_revision": 1,
        "policy": {
            "credentials": "forbidden",
            "authenticated_content": "forbidden",
            "payments": "forbidden",
            "destructive_actions": "forbidden",
            "persistent_identity": "forbidden",
            "undeclared_network_origins": "forbidden",
        },
        "outcome_categories": [
            "completed-observation",
            "engine-failure",
            "external-content-drift",
            "external-unavailable",
            "unsupported-capability",
        ],
        "scenarios": list(scenarios),
    }


class RealWebCorpusTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.corpus = load_module()

    def test_committed_corpus_is_contract_only_and_valid(self) -> None:
        result = self.corpus.load_corpus(CORPUS, root=ROOT)
        self.assertEqual(result["corpus_revision"], 1)
        self.assertEqual(len(result["scenarios"]), 1)
        scenario = result["scenarios"][0]
        self.assertEqual(scenario["id"], "rfc9110-html")
        self.assertEqual(scenario["input"], {"mode": "live-external"})
        self.assertNotIn("result", scenario)
        self.assertNotIn("outcome", scenario)

    def test_live_input_cannot_smuggle_capture_identity(self) -> None:
        scenario = live_scenario()
        scenario["input"]["sha256"] = "sha256:" + "0" * 64
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "live input.*invalid keys"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_live_input_requires_exact_primary_origin(self) -> None:
        scenario = live_scenario()
        scenario["external_dependencies"] = []
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "requires declared dependencies"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

        scenario = live_scenario()
        scenario["external_dependencies"][0]["origin"] = "https://cdn.example.test"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "source origin must be declared"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_urls_and_origins_are_strict_https_without_identity(self) -> None:
        for source in (
            "http://example.test/page",
            "https://user@example.test/page",
            "https://example.test/page#fragment",
            "https://example.test:443/page",
            "https://example.test:bad/page",
        ):
            scenario = live_scenario()
            scenario["source_url"] = source
            with self.subTest(source=source):
                with self.assertRaises(self.corpus.CorpusError):
                    self.corpus.validate_corpus(
                        corpus_with(scenario), root=ROOT
                    )

        scenario = live_scenario()
        scenario["external_dependencies"][0]["origin"] = "https://example.test/"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "must be canonical"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_private_and_wildcard_network_targets_are_rejected(self) -> None:
        for source in (
            "https://localhost/page",
            "https://127.0.0.1/page",
            "https://10.0.0.1/page",
            "https://*.example.test/page",
        ):
            scenario = live_scenario()
            scenario["source_url"] = source
            with self.subTest(source=source):
                with self.assertRaisesRegex(
                    self.corpus.CorpusError, "public"
                ):
                    self.corpus.validate_corpus(
                        corpus_with(scenario), root=ROOT
                    )

        scenario = live_scenario()
        scenario["external_dependencies"][0]["origin"] = "https://127.0.0.1"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "public"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_captured_input_is_content_addressed_and_offline(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            capture = root / "real-web" / "captures" / "page.html"
            capture.parent.mkdir(parents=True)
            capture.write_text("<!doctype html><title>fixture</title>\n", encoding="utf-8")
            digest = "sha256:" + hashlib.sha256(capture.read_bytes()).hexdigest()

            scenario = live_scenario()
            scenario["id"] = "captured-page"
            scenario["input"] = {
                "mode": "captured-versioned",
                "path": "real-web/captures/page.html",
                "sha256": digest,
                "media_type": "text/html",
            }
            scenario["external_dependencies"] = []
            invalid_media = copy.deepcopy(scenario)
            invalid_media["input"]["media_type"] = "application/octet-stream"
            with self.assertRaisesRegex(
                self.corpus.CorpusError,
                "unsupported captured media type",
            ):
                self.corpus.validate_corpus(
                    corpus_with(invalid_media), root=root
                )

            result = self.corpus.validate_corpus(
                corpus_with(scenario), root=root
            )
            self.assertEqual(
                result["scenarios"][0]["input"]["sha256"], digest
            )

            scenario["external_dependencies"] = [
                {
                    "origin": "https://example.test",
                    "role": "primary-document",
                    "required": True,
                }
            ]
            with self.assertRaisesRegex(
                self.corpus.CorpusError,
                "must not require external dependencies",
            ):
                self.corpus.validate_corpus(
                    corpus_with(scenario), root=root
                )

    def test_capture_digest_traversal_and_symlink_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            captures = root / "real-web" / "captures"
            captures.mkdir(parents=True)
            capture = captures / "page.html"
            capture.write_text("fixture", encoding="utf-8")
            digest = "sha256:" + hashlib.sha256(capture.read_bytes()).hexdigest()

            scenario = live_scenario()
            scenario["id"] = "captured-page"
            scenario["input"] = {
                "mode": "captured-versioned",
                "path": "real-web/captures/page.html",
                "sha256": "sha256:" + "0" * 64,
                "media_type": "text/html",
            }
            scenario["external_dependencies"] = []
            with self.assertRaisesRegex(
                self.corpus.CorpusError, "digest mismatch"
            ):
                self.corpus.validate_corpus(
                    corpus_with(scenario), root=root
                )

            scenario["input"]["sha256"] = digest
            scenario["input"]["path"] = "real-web/captures/../secret.html"
            with self.assertRaisesRegex(
                self.corpus.CorpusError, "normalized path"
            ):
                self.corpus.validate_corpus(
                    corpus_with(scenario), root=root
                )

            if hasattr(os, "symlink"):
                link = captures / "link.html"
                try:
                    link.symlink_to(capture)
                except OSError:
                    return
                scenario["input"]["path"] = "real-web/captures/link.html"
                with self.assertRaisesRegex(
                    self.corpus.CorpusError, "must not be a symlink"
                ):
                    self.corpus.validate_corpus(
                        corpus_with(scenario), root=root
                    )

    def test_actions_are_closed_ordered_and_bounded(self) -> None:
        scenario = live_scenario()
        scenario["actions"].reverse()
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "first action must be load-input"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

        scenario = live_scenario()
        scenario["actions"][1]["timeout_ms"] = 5001
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "between 1 and 5000"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

        scenario = live_scenario()
        scenario["actions"].append({"kind": "click"})
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "unsupported action kind"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_observations_and_dependencies_are_unique_and_sorted(self) -> None:
        scenario = live_scenario()
        scenario["observations"] = ["screenshot", "document-title"]
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "observations must be unique and sorted"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

        scenario = live_scenario()
        scenario["external_dependencies"].append(
            {
                "origin": "https://assets.example.test",
                "role": "subresource",
                "required": False,
            }
        )
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "dependencies must be unique and sorted"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_limits_are_bounded_and_consistent(self) -> None:
        scenario = live_scenario()
        scenario["limits"]["timeout_ms"] = 30001
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "between 1 and 30000"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

        scenario = live_scenario()
        scenario["limits"]["max_response_bytes"] = 5000
        scenario["limits"]["max_total_bytes"] = 4000
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "must not exceed"
        ):
            self.corpus.validate_corpus(
                corpus_with(scenario), root=ROOT
            )

    def test_root_policy_and_outcome_taxonomy_are_exact(self) -> None:
        document = corpus_with(live_scenario())
        document["policy"]["credentials"] = "allowed"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "policy must exactly prohibit"
        ):
            self.corpus.validate_corpus(document, root=ROOT)

        document = corpus_with(live_scenario())
        document["outcome_categories"].append("pass")
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "outcome_categories must exactly equal"
        ):
            self.corpus.validate_corpus(document, root=ROOT)

    def test_scenario_ids_are_unique_sorted_slugs(self) -> None:
        first = live_scenario()
        first["id"] = "z-last"
        second = live_scenario()
        second["id"] = "a-first"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "IDs must be unique and strictly sorted"
        ):
            self.corpus.validate_corpus(
                corpus_with(first, second), root=ROOT
            )

        first = live_scenario()
        first["id"] = "Bad_ID"
        with self.assertRaisesRegex(
            self.corpus.CorpusError, "lowercase hyphenated slug"
        ):
            self.corpus.validate_corpus(
                corpus_with(first), root=ROOT
            )

    def test_strict_json_rejects_duplicate_keys_and_nonfinite_numbers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text(
                '{"schema_version":1,"schema_version":1}',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                self.corpus.CorpusError, "duplicate key"
            ):
                self.corpus.load_corpus(duplicate, root=root)

            nonfinite = root / "nonfinite.json"
            nonfinite.write_text(
                '{"schema_version":NaN}',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                self.corpus.CorpusError, "non-finite JSON"
            ):
                self.corpus.load_corpus(nonfinite, root=root)

    def test_cli_normalization_is_repeatable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            first = Path(directory) / "first.json"
            second = Path(directory) / "second.json"
            common = [
                "--manifest",
                str(CORPUS),
                "--root",
                str(ROOT),
            ]
            self.assertEqual(
                self.corpus.main(common + ["--json-out", str(first)]),
                0,
            )
            self.assertEqual(
                self.corpus.main(common + ["--json-out", str(second)]),
                0,
            )
            self.assertEqual(first.read_bytes(), second.read_bytes())


if __name__ == "__main__":
    unittest.main()
