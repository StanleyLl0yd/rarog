from __future__ import annotations

import sys
import time
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import real_web_corpus
import real_web_execute


class RealWebExecuteTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.corpus = real_web_corpus.load_corpus(
            ROOT / "real-web" / "corpus.json",
            root=ROOT,
        )
        cls.scenario = cls.corpus["scenarios"][0]

    def test_private_dns_resolution_is_rejected(self) -> None:
        answer = [
            (
                2,
                1,
                6,
                "",
                ("127.0.0.1", 443),
            )
        ]
        with patch.object(real_web_execute.socket, "getaddrinfo", return_value=answer):
            with self.assertRaisesRegex(
                real_web_execute.ExecutionError,
                "forbidden address",
            ):
                real_web_execute._public_addresses("example.test", 443)

    def test_same_origin_redirect_is_followed_and_content_addressed(self) -> None:
        responses = [
            (
                302,
                ["1.1.1.1"],
                {"location": "/rfc/rfc9110.html?redirected=1"},
                b"",
            ),
            (
                200,
                ["1.1.1.1"],
                {},
                b"<!doctype html><title>RFC 9110</title>",
            ),
        ]
        with patch.object(real_web_execute, "_request_once", side_effect=responses):
            body, final_url, dependency, failure = real_web_execute._fetch_live(
                self.scenario,
                deadline=time.monotonic() + 5,
            )

        self.assertIsNone(failure)
        self.assertEqual(body, responses[1][3])
        self.assertEqual(
            final_url,
            "https://www.rfc-editor.org/rfc/rfc9110.html?redirected=1",
        )
        self.assertEqual(dependency["state"], "available")
        self.assertEqual(dependency["resolved_addresses"], ["1.1.1.1"])
        self.assertEqual(dependency["http_status"], 200)
        self.assertRegex(
            dependency["content_sha256"],
            r"^sha256:[0-9a-f]{64}$",
        )

    def test_redirect_to_undeclared_origin_is_external_policy_failure(self) -> None:
        with patch.object(
            real_web_execute,
            "_request_once",
            return_value=(
                302,
                ["1.1.1.1"],
                {"location": "https://example.com/outside"},
                b"",
            ),
        ):
            body, final_url, dependency, failure = real_web_execute._fetch_live(
                self.scenario,
                deadline=time.monotonic() + 5,
            )

        self.assertIsNone(body)
        self.assertIsNone(final_url)
        self.assertEqual(dependency["state"], "redirect-policy-violation")
        self.assertEqual(
            failure[0:2],
            ("external-unavailable", "redirect-policy-violation"),
        )

    def test_network_failure_preserves_resolved_addresses(self) -> None:
        failure = real_web_execute.NetworkFailure(
            "tls-failure",
            "certificate failure",
            addresses=["1.1.1.1"],
        )
        with patch.object(real_web_execute, "_request_once", side_effect=failure):
            body, final_url, dependency, outcome = real_web_execute._fetch_live(
                self.scenario,
                deadline=time.monotonic() + 5,
            )

        self.assertIsNone(body)
        self.assertIsNone(final_url)
        self.assertEqual(dependency["state"], "tls-failure")
        self.assertEqual(dependency["resolved_addresses"], ["1.1.1.1"])
        self.assertEqual(
            outcome,
            ("external-unavailable", "tls-failure", "certificate failure"),
        )

    def test_completed_attempt_keeps_network_and_engine_observations_distinct(self) -> None:
        dependency = real_web_execute._dependency(
            self.scenario["external_dependencies"][0],
            state="available",
            addresses=["1.1.1.1"],
            http_status=200,
            content_sha256="sha256:" + "a" * 64,
        )
        with (
            patch.object(
                real_web_execute,
                "_fetch_live",
                return_value=(
                    b"<!doctype html><title>RFC 9110</title>",
                    self.scenario["source_url"],
                    dependency,
                    None,
                ),
            ),
            patch.object(
                real_web_execute,
                "_run_renderer",
                return_value=(
                    {
                        "document-title": "RFC 9110",
                        "render-completion": True,
                        "screenshot": "sha256:" + "b" * 64,
                    },
                    None,
                ),
            ),
        ):
            attempt = real_web_execute.execute_live_scenario(
                self.scenario,
                renderer=Path("/fixture/renderer"),
            )

        self.assertEqual(
            attempt["outcome"],
            {
                "category": "completed-observation",
                "detail": "completed",
                "diagnostic": None,
            },
        )
        self.assertTrue(all(item["state"] == "observed" for item in attempt["observations"]))
        self.assertEqual(attempt["dependencies"][0]["state"], "available")

    def test_engine_failure_requires_available_network_evidence(self) -> None:
        dependency = real_web_execute._dependency(
            self.scenario["external_dependencies"][0],
            state="available",
            addresses=["1.1.1.1"],
            http_status=200,
            content_sha256="sha256:" + "a" * 64,
        )
        with (
            patch.object(
                real_web_execute,
                "_fetch_live",
                return_value=(
                    b"<!doctype html><title>RFC 9110</title>",
                    self.scenario["source_url"],
                    dependency,
                    None,
                ),
            ),
            patch.object(
                real_web_execute,
                "_run_renderer",
                return_value=(
                    None,
                    ("engine-failure", "render-failure", "fixture failure"),
                ),
            ),
        ):
            attempt = real_web_execute.execute_live_scenario(
                self.scenario,
                renderer=Path("/fixture/renderer"),
            )

        self.assertEqual(attempt["outcome"]["category"], "engine-failure")
        self.assertEqual(attempt["outcome"]["detail"], "render-failure")
        self.assertEqual(attempt["dependencies"][0]["state"], "available")
        final_url = next(
            item for item in attempt["observations"] if item["kind"] == "final-url"
        )
        self.assertEqual(final_url["state"], "observed")

    def test_corpus_execution_does_not_filter_scenarios(self) -> None:
        fixture = {
            "schema_version": 1,
            "scenario_id": self.scenario["id"],
            "outcome": {
                "category": "external-unavailable",
                "detail": "dns-failure",
                "diagnostic": "fixture",
            },
            "dependencies": [
                real_web_execute._dependency(
                    self.scenario["external_dependencies"][0],
                    state="dns-failure",
                )
            ],
            "observations": real_web_execute._blank_observations(self.scenario),
        }
        with patch.object(
            real_web_execute,
            "execute_live_scenario",
            return_value=fixture,
        ) as execute:
            result = real_web_execute.execute_corpus(
                self.corpus,
                renderer=Path("/fixture/renderer"),
            )

        self.assertEqual(
            result["scenario_ids"],
            [scenario["id"] for scenario in self.corpus["scenarios"]],
        )
        self.assertEqual(len(result["attempts"]), len(self.corpus["scenarios"]))
        self.assertEqual(execute.call_count, len(self.corpus["scenarios"]))


if __name__ == "__main__":
    unittest.main()
