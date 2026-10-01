from __future__ import annotations

import copy
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

import compat_profile


def load_profile() -> dict:
    return json.loads(
        (ROOT / "compatibility" / "profile.json").read_text(encoding="utf-8")
    )


def copy_wpt_bundle(destination: Path) -> None:
    (destination / "wpt" / "evidence").mkdir(parents=True)
    for relative in (
        Path("wpt/r6-selection.json"),
        Path("wpt/evidence/r6-first-wptreport.json"),
        Path("wpt/evidence/r6-first-evidence.json"),
        Path("wpt/evidence/r6-first-dashboard.json"),
        Path("wpt/evidence/r6-first-dashboard.md"),
    ):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / relative, target)


class CompatibilityProfileTests(unittest.TestCase):
    def test_committed_profile_reproduces_from_raw_evidence(self) -> None:
        expected = load_profile()
        rebuilt = compat_profile.build_initial_profile(ROOT)
        self.assertEqual(rebuilt, expected)
        self.assertEqual(
            compat_profile.render_markdown(rebuilt),
            (ROOT / "compatibility" / "profile.md").read_text(encoding="utf-8"),
        )
        self.assertEqual(
            [entry["id"] for entry in rebuilt["evidence"]],
            ["real-web-first-baseline", "wpt-first-selected-baseline"],
        )
        measured = {
            entry["id"]: entry["measured_rarog_commit"]
            for entry in rebuilt["evidence"]
        }
        self.assertEqual(
            measured["real-web-first-baseline"],
            "2301370b6ccce06a8839a94a75dfeee12e3a3499",
        )
        self.assertEqual(
            measured["wpt-first-selected-baseline"],
            "dcd349dd37b0340ec67a2fb8d36b13980e2fd918",
        )
        self.assertNotEqual(
            measured["real-web-first-baseline"],
            measured["wpt-first-selected-baseline"],
        )
        self.assertEqual(
            rebuilt["evidence"][0]["normalized_evidence"]["sha256"],
            "sha256:821907e3e5903f6184abd34001559e141a8e0f17f2dd04ac9bcc94245f585053",
        )
        self.assertEqual(
            rebuilt["evidence"][1]["normalized_evidence"]["sha256"],
            "sha256:f02f45dd2b0e0affa7c0bf0fab97f0b1f0dc52c769201af71f4f300a66df3ae0",
        )

    def test_wpt_source_commit_mismatch_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            copy_wpt_bundle(root)
            selection_path = root / "wpt" / "r6-selection.json"
            selection = json.loads(selection_path.read_text(encoding="utf-8"))
            selection["source"]["commit"] = "b" * 40
            selection_path.write_text(
                json.dumps(selection, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                compat_profile.ProfileError,
                "selection source commit does not match committed evidence",
            ):
                compat_profile.reproduce_wpt_entry(root)

    def test_wpt_normalized_content_mismatch_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            copy_wpt_bundle(root)
            dashboard_path = root / "wpt" / "evidence" / "r6-first-dashboard.json"
            dashboard = json.loads(dashboard_path.read_text(encoding="utf-8"))
            dashboard["summary"]["measured_tests"] = 99
            dashboard_path.write_text(
                json.dumps(dashboard, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                compat_profile.ProfileError,
                "dashboard JSON does not reproduce exactly",
            ):
                compat_profile.reproduce_wpt_entry(root)

    def test_duplicate_evidence_identity_is_rejected(self) -> None:
        profile = load_profile()
        duplicate = copy.deepcopy(profile["evidence"][0])
        duplicate["id"] = "real-web-second-baseline"
        profile["evidence"].insert(1, duplicate)
        with self.assertRaisesRegex(
            compat_profile.ProfileError,
            "duplicate evidence identity",
        ):
            compat_profile.validate_profile(profile)

    def test_unknown_kind_and_unknown_keys_are_rejected(self) -> None:
        profile = load_profile()
        profile["evidence"][0]["kind"] = "mystery"
        with self.assertRaisesRegex(
            compat_profile.ProfileError,
            "unsupported evidence kind",
        ):
            compat_profile.validate_profile(profile)

        profile = load_profile()
        profile["unexpected"] = True
        with self.assertRaisesRegex(
            compat_profile.ProfileError,
            "invalid keys",
        ):
            compat_profile.validate_profile(profile)

    def test_evidence_ordering_drift_is_rejected(self) -> None:
        profile = load_profile()
        profile["evidence"].reverse()
        with self.assertRaisesRegex(
            compat_profile.ProfileError,
            "strictly sorted by id",
        ):
            compat_profile.validate_profile(profile)

    def test_invalid_digest_is_rejected(self) -> None:
        profile = load_profile()
        profile["evidence"][0]["normalized_evidence"]["sha256"] = "sha256:bad"
        with self.assertRaisesRegex(
            compat_profile.ProfileError,
            "sha256:<64 lowercase hex>",
        ):
            compat_profile.validate_profile(profile)

    def test_cli_outputs_are_repeatable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first_json = root / "first.json"
            first_md = root / "first.md"
            second_json = root / "second.json"
            second_md = root / "second.md"
            common = ["--root", str(ROOT)]
            self.assertEqual(
                compat_profile.main(
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
                compat_profile.main(
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
            self.assertEqual(
                first_json.read_bytes(),
                (ROOT / "compatibility" / "profile.json").read_bytes(),
            )
            self.assertEqual(
                first_md.read_bytes(),
                (ROOT / "compatibility" / "profile.md").read_bytes(),
            )


if __name__ == "__main__":
    unittest.main()
