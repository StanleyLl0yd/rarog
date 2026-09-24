from __future__ import annotations

import copy
import hashlib
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

import real_web_corpus

MODULE_PATH = SCRIPTS / "real_web_result.py"
CORPUS = ROOT / "real-web" / "corpus.json"
RAROG_COMMIT = "d" * 40
CONTENT = "sha256:" + "1" * 64
EXPECTED = "sha256:" + "2" * 64
SCREENSHOT = "sha256:" + "3" * 64
PUBLIC_IP = "8.8.8.8"


def load_module():
    spec = importlib.util.spec_from_file_location("real_web_result", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load real_web_result module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def platform():
    return {
        "os": "linux",
        "arch": "x86_64",
        "environment": "synthetic-test",
    }


def not_observed():
    return [
        {"kind": "document-title", "state": "not-observed", "value": None},
        {"kind": "final-url", "state": "not-observed", "value": None},
        {"kind": "render-completion", "state": "not-observed", "value": None},
        {"kind": "screenshot", "state": "not-observed", "value": None},
    ]


def observed():
    return [
        {"kind": "document-title", "state": "observed", "value": "RFC 9110"},
        {
            "kind": "final-url",
            "state": "observed",
            "value": "https://www.rfc-editor.org/rfc/rfc9110.html",
        },
        {"kind": "render-completion", "state": "observed", "value": True},
        {"kind": "screenshot", "state": "observed", "value": SCREENSHOT},
    ]


def dependency(
    state: str,
    *,
    addresses=None,
    http_status=None,
    content=None,
    expected=None,
):
    return {
        "origin": "https://www.rfc-editor.org",
        "role": "primary-document",
        "state": state,
        "resolved_addresses": [] if addresses is None else addresses,
        "http_status": http_status,
        "content_sha256": content,
        "expected_content_sha256": expected,
    }


def attempt(
    *,
    category: str,
    detail: str,
    dep,
    observations=None,
    diagnostic="synthetic diagnostic",
):
    return {
        "schema_version": 1,
        "scenario_id": "rfc9110-html",
        "outcome": {
            "category": category,
            "detail": detail,
            "diagnostic": None
            if category == "completed-observation"
            else diagnostic,
        },
        "dependencies": [dep],
        "observations": not_observed() if observations is None else observations,
    }


class RealWebResultTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.result = load_module()
        cls.corpus = real_web_corpus.load_corpus(CORPUS, root=ROOT)

    def normalize(self, value):
        return self.result.normalize_attempt(
            self.corpus,
            value,
            rarog_commit=RAROG_COMMIT,
            platform=platform(),
        )

    def test_dns_failure_stays_external_and_content_addressed(self) -> None:
        normalized = self.normalize(
            attempt(
                category="external-unavailable",
                detail="dns-failure",
                dep=dependency("dns-failure"),
            )
        )
        self.assertEqual(normalized["outcome"]["category"], "external-unavailable")
        self.assertEqual(normalized["dependencies"][0]["state"], "dns-failure")
        self.assertRegex(normalized["corpus_sha256"], r"^sha256:[0-9a-f]{64}$")
        self.assertEqual(normalized["rarog_commit"], RAROG_COMMIT)

    def test_engine_failure_cannot_hide_required_external_failure(self) -> None:
        value = attempt(
            category="engine-failure",
            detail="render-failure",
            dep=dependency("dns-failure"),
        )
        with self.assertRaisesRegex(
            self.result.ResultError,
            "engine failure requires required external dependencies",
        ):
            self.normalize(value)

    def test_completed_observation_requires_available_dependency_and_all_observations(
        self,
    ) -> None:
        value = attempt(
            category="completed-observation",
            detail="completed",
            dep=dependency(
                "available",
                addresses=[PUBLIC_IP],
                http_status=200,
                content=CONTENT,
            ),
            observations=observed(),
        )
        normalized = self.normalize(value)
        self.assertTrue(
            all(item["state"] == "observed" for item in normalized["observations"])
        )

        missing = copy.deepcopy(value)
        missing["observations"][0] = {
            "kind": "document-title",
            "state": "not-observed",
            "value": None,
        }
        with self.assertRaisesRegex(
            self.result.ResultError, "requires every declared observation"
        ):
            self.normalize(missing)

    def test_external_content_drift_requires_distinct_digests(self) -> None:
        value = attempt(
            category="external-content-drift",
            detail="content-drift",
            dep=dependency(
                "content-drift",
                addresses=[PUBLIC_IP],
                http_status=200,
                content=CONTENT,
                expected=EXPECTED,
            ),
        )
        normalized = self.normalize(value)
        self.assertEqual(
            normalized["dependencies"][0]["expected_content_sha256"], EXPECTED
        )

        same = copy.deepcopy(value)
        same["dependencies"][0]["expected_content_sha256"] = CONTENT
        with self.assertRaisesRegex(
            self.result.ResultError, "distinct expected/observed"
        ):
            self.normalize(same)

    def test_unsupported_capability_must_not_disguise_attempted_network(self) -> None:
        allowed = attempt(
            category="unsupported-capability",
            detail="unsupported-capability",
            dep=dependency("not-attempted"),
        )
        self.assertEqual(
            self.normalize(allowed)["outcome"]["category"],
            "unsupported-capability",
        )

        attempted = copy.deepcopy(allowed)
        attempted["dependencies"][0] = dependency(
            "available",
            addresses=[PUBLIC_IP],
            http_status=200,
            content=CONTENT,
        )
        with self.assertRaisesRegex(
            self.result.ResultError, "must not disguise attempted"
        ):
            self.normalize(attempted)

    def test_dependency_results_are_complete_ordered_and_public(self) -> None:
        value = attempt(
            category="external-unavailable",
            detail="dns-failure",
            dep=dependency("dns-failure"),
        )
        missing = copy.deepcopy(value)
        missing["dependencies"] = []
        with self.assertRaisesRegex(
            self.result.ResultError, "must cover every declared dependency"
        ):
            self.normalize(missing)

        private = copy.deepcopy(value)
        private["dependencies"][0] = dependency(
            "tls-failure",
            addresses=["127.0.0.1"],
        )
        private["outcome"]["detail"] = "tls-failure"
        with self.assertRaisesRegex(self.result.ResultError, "public IP"):
            self.normalize(private)

    def test_observation_records_are_complete_ordered_and_typed(self) -> None:
        value = attempt(
            category="completed-observation",
            detail="completed",
            dep=dependency(
                "available",
                addresses=[PUBLIC_IP],
                http_status=200,
                content=CONTENT,
            ),
            observations=observed(),
        )
        reordered = copy.deepcopy(value)
        reordered["observations"][0], reordered["observations"][1] = (
            reordered["observations"][1],
            reordered["observations"][0],
        )
        with self.assertRaisesRegex(
            self.result.ResultError, "observation ordering"
        ):
            self.normalize(reordered)

        bad_type = copy.deepcopy(value)
        bad_type["observations"][2]["value"] = "yes"
        with self.assertRaisesRegex(
            self.result.ResultError, "must be boolean"
        ):
            self.normalize(bad_type)

    def test_live_final_url_cannot_escape_declared_dependency_origins(self) -> None:
        value = attempt(
            category="completed-observation",
            detail="completed",
            dep=dependency(
                "available",
                addresses=[PUBLIC_IP],
                http_status=200,
                content=CONTENT,
            ),
            observations=observed(),
        )
        value["observations"][1]["value"] = "https://example.com/redirected"
        with self.assertRaisesRegex(
            self.result.ResultError, "not a declared dependency"
        ):
            self.normalize(value)

    def test_live_timeout_classification_is_external_without_fake_dependency_state(
        self,
    ) -> None:
        value = attempt(
            category="external-unavailable",
            detail="timeout-limit-exceeded",
            dep=dependency("timeout-limit-exceeded"),
        )
        normalized = self.normalize(value)
        self.assertEqual(
            normalized["outcome"]["detail"], "timeout-limit-exceeded"
        )

    def test_captured_input_cannot_emit_external_outcome(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            capture = root / "real-web" / "captures" / "page.html"
            capture.parent.mkdir(parents=True)
            capture.write_text("<!doctype html><title>captured</title>\n", encoding="utf-8")
            digest = "sha256:" + hashlib.sha256(capture.read_bytes()).hexdigest()
            scenario = {
                "id": "captured-page",
                "source_url": "https://example.com/captured",
                "input": {
                    "mode": "captured-versioned",
                    "path": "real-web/captures/page.html",
                    "sha256": digest,
                    "media_type": "text/html",
                },
                "viewport": {"width": 800, "height": 600, "device_scale": 1},
                "actions": [{"kind": "load-input"}],
                "observations": ["render-completion"],
                "external_dependencies": [],
                "limits": {
                    "timeout_ms": 5000,
                    "max_response_bytes": 1024,
                    "max_total_bytes": 4096,
                    "max_subresources": 0,
                },
            }
            document = {
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
                "scenarios": [scenario],
            }
            corpus = real_web_corpus.validate_corpus(document, root=root)
            raw = {
                "schema_version": 1,
                "scenario_id": "captured-page",
                "outcome": {
                    "category": "external-unavailable",
                    "detail": "dns-failure",
                    "diagnostic": "must not happen",
                },
                "dependencies": [],
                "observations": [
                    {
                        "kind": "render-completion",
                        "state": "not-observed",
                        "value": None,
                    }
                ],
            }
            with self.assertRaisesRegex(
                self.result.ResultError, "cannot have external outcome"
            ):
                self.result.normalize_attempt(
                    corpus,
                    raw,
                    rarog_commit=RAROG_COMMIT,
                    platform=platform(),
                )

    def test_cli_outputs_are_repeatable(self) -> None:
        value = attempt(
            category="external-unavailable",
            detail="dns-failure",
            dep=dependency("dns-failure"),
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            attempt_path = root / "attempt.json"
            first_json = root / "first.json"
            second_json = root / "second.json"
            first_md = root / "first.md"
            second_md = root / "second.md"
            attempt_path.write_text(json.dumps(value), encoding="utf-8")
            common = [
                "--manifest",
                str(CORPUS),
                "--root",
                str(ROOT),
                "--attempt",
                str(attempt_path),
                "--rarog-commit",
                RAROG_COMMIT,
                "--platform-os",
                "linux",
                "--platform-arch",
                "x86_64",
                "--environment",
                "synthetic-test",
            ]
            self.assertEqual(
                self.result.main(
                    common
                    + [
                        "--json-out",
                        str(first_json),
                        "--markdown-out",
                        str(first_md),
                    ]
                ),
                0,
            )
            self.assertEqual(
                self.result.main(
                    common
                    + [
                        "--json-out",
                        str(second_json),
                        "--markdown-out",
                        str(second_md),
                    ]
                ),
                0,
            )
            self.assertEqual(first_json.read_bytes(), second_json.read_bytes())
            self.assertEqual(first_md.read_bytes(), second_md.read_bytes())


if __name__ == "__main__":
    unittest.main()
