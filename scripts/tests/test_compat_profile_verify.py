from __future__ import annotations

import copy
import hashlib
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import compat_profile_verify as verifier

SOURCE_FILES = (
    "compatibility/profile.json",
    "compatibility/profile.md",
    "wpt/r6-selection.json",
    "wpt/evidence/r6-first-wptreport.json",
    "wpt/evidence/r6-first-evidence.json",
    "wpt/evidence/r6-first-dashboard.json",
    "wpt/evidence/r6-first-dashboard.md",
    "real-web/corpus.json",
    "real-web/evidence/r6-first-execution.json",
    "real-web/evidence/r6-first-baseline.json",
    "real-web/evidence/r6-first-baseline.md",
)


class IndependentProfileVerificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        for name in SOURCE_FILES:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def load(self, name: str) -> dict:
        return json.loads((self.root / name).read_text(encoding="utf-8"))

    def store(self, name: str, document: dict) -> None:
        (self.root / name).write_text(
            json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

    def test_independent_verification_reproduces_first_profile(self) -> None:
        output = verifier.verify(self.root)
        profile = self.load("compatibility/profile.json")
        payload = json.dumps(
            profile, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode("utf-8")
        self.assertEqual(
            output["profile_sha256"],
            "sha256:" + hashlib.sha256(payload).hexdigest(),
        )
        self.assertEqual(output["canonical_payload_bytes"], len(payload))
        self.assertEqual(output["evidence_count"], 2)
        self.assertEqual(
            output["evidence_ids"],
            ["real-web-first-baseline", "wpt-first-selected-baseline"],
        )
        self.assertEqual(len(output["measured_rarog_commits"]), 2)

    def test_independent_cli_output_is_reproducible(self) -> None:
        first = self.root / "one.json"
        second = self.root / "two.json"
        self.assertEqual(
            verifier.main(["--root", str(self.root), "--json-out", str(first)]),
            0,
        )
        self.assertEqual(
            verifier.main(["--root", str(self.root), "--json-out", str(second)]),
            0,
        )
        self.assertEqual(first.read_bytes(), second.read_bytes())

    def test_rejects_profile_schema_bool_and_unknown_keys(self) -> None:
        for change in ("boolean-schema", "unexpected-key"):
            with self.subTest(change=change):
                profile = self.load("compatibility/profile.json")
                if change == "boolean-schema":
                    profile["schema_version"] = True
                else:
                    profile["unknown"] = 1
                self.store("compatibility/profile.json", profile)
                with self.assertRaises(verifier.VerificationError):
                    verifier.verify(self.root)
                shutil.copyfile(
                    ROOT / "compatibility/profile.json",
                    self.root / "compatibility/profile.json",
                )

    def test_rejects_profile_digest_and_commit_tampering(self) -> None:
        for field in ("commit", "raw", "normalized", "platform", "scope"):
            with self.subTest(field=field):
                profile = self.load("compatibility/profile.json")
                item = profile["evidence"][1]
                if field == "commit":
                    item["measured_rarog_commit"] = "0" * 40
                elif field == "raw":
                    item["raw_evidence"]["sha256"] = "sha256:" + "0" * 64
                elif field == "normalized":
                    item["normalized_evidence"]["sha256"] = "sha256:" + "0" * 64
                elif field == "platform":
                    item["platform"]["environment"] = "unknown-platform"
                else:
                    item["scope"]["test_ids"] = ["/missing.html"]
                self.store("compatibility/profile.json", profile)
                with self.assertRaises(verifier.VerificationError):
                    verifier.verify(self.root)
                shutil.copyfile(
                    ROOT / "compatibility/profile.json",
                    self.root / "compatibility/profile.json",
                )

    def test_rejects_duplicate_evidence_and_reordering(self) -> None:
        for change in ("duplicate", "reverse"):
            with self.subTest(change=change):
                profile = self.load("compatibility/profile.json")
                if change == "duplicate":
                    profile["evidence"].append(copy.deepcopy(profile["evidence"][0]))
                else:
                    profile["evidence"].reverse()
                self.store("compatibility/profile.json", profile)
                with self.assertRaises(verifier.VerificationError):
                    verifier.verify(self.root)
                shutil.copyfile(
                    ROOT / "compatibility/profile.json",
                    self.root / "compatibility/profile.json",
                )

    def test_rejects_raw_wpt_and_corpus_source_mutation(self) -> None:
        for name, mutate in (
            (
                "wpt/evidence/r6-first-wptreport.json",
                lambda data: data["results"][0].update(status="PASS"),
            ),
            (
                "wpt/r6-selection.json",
                lambda data: data["source"].update(commit="b" * 40),
            ),
            (
                "real-web/evidence/r6-first-execution.json",
                lambda data: data["attempts"][0]["observations"][0].update(
                    value="Tampered document title"
                ),
            ),
            (
                "real-web/corpus.json",
                lambda data: data["scenarios"][0].update(
                    source_url="https://www.example.org/"
                ),
            ),
        ):
            with self.subTest(source=name):
                document = self.load(name)
                mutate(document)
                self.store(name, document)
                with self.assertRaises(
                    (
                        verifier.VerificationError,
                        verifier.wpt_selection.SelectionError,
                        verifier.wpt_evidence.EvidenceError,
                        verifier.wpt_dashboard.DashboardError,
                        verifier.real_web_corpus.CorpusError,
                        verifier.real_web_baseline.BaselineError,
                    )
                ):
                    verifier.verify(self.root)
                shutil.copyfile(ROOT / name, self.root / name)

    def test_rejects_dashboard_bool_instead_of_integer(self) -> None:
        name = "wpt/evidence/r6-first-dashboard.json"
        document = self.load(name)
        document["summary"]["measured_subtests"] = False
        self.store(name, document)
        with self.assertRaises(verifier.VerificationError):
            verifier.verify(self.root)

    def test_rejects_noncanonical_profile_json_bytes(self) -> None:
        name = "compatibility/profile.json"
        profile = self.load(name)
        (self.root / name).write_text(json.dumps(profile), encoding="utf-8")
        with self.assertRaisesRegex(
            verifier.VerificationError, "not canonical"
        ):
            verifier.verify(self.root)

    def test_rejects_profile_markdown_tampering(self) -> None:
        name = "compatibility/profile.md"
        path = self.root / name
        path.write_bytes(path.read_bytes() + b"tampered\n")
        with self.assertRaisesRegex(
            verifier.VerificationError, "Markdown does not reproduce"
        ):
            verifier.verify(self.root)

    def test_rejects_unsafe_evidence_paths(self) -> None:
        for path in (
            "wpt/evidence/../r6-first-wptreport.json",
            "wpt/evidence//r6-first-wptreport.json",
            "/tmp/wpt/evidence/r6-first-wptreport.json",
        ):
            with self.subTest(path=path):
                profile = self.load("compatibility/profile.json")
                profile["evidence"][1]["raw_evidence"]["path"] = path
                self.store("compatibility/profile.json", profile)
                with self.assertRaises(verifier.VerificationError):
                    verifier.verify(self.root)
                shutil.copyfile(
                    ROOT / "compatibility/profile.json",
                    self.root / "compatibility/profile.json",
                )

    def test_rejects_duplicate_json_keys_and_nonfinite_numbers(self) -> None:
        path = self.root / "compatibility/profile.json"
        for bad in ('{"schema_version":1,"schema_version":1}', '{"schema_version":NaN}'):
            with self.subTest(json=bad):
                path.write_text(bad, encoding="utf-8")
                with self.assertRaises(verifier.VerificationError):
                    verifier.verify(self.root)

    def test_rejects_symlink_artifact_escape(self) -> None:
        link = self.root / "wpt/evidence/r6-first-wptreport.json"
        link.unlink()
        try:
            link.symlink_to(ROOT / "wpt/evidence/r6-first-wptreport.json")
        except (NotImplementedError, OSError):
            self.skipTest("runner does not permit symlink creation")
        with self.assertRaisesRegex(verifier.VerificationError, "symlink"):
            verifier.verify(self.root)


if __name__ == "__main__":
    unittest.main()
