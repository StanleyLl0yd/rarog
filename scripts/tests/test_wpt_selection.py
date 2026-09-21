from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "scripts" / "wpt_selection.py"
SELECTION = ROOT / "wpt" / "r6-selection.json"


def load_module():
    spec = importlib.util.spec_from_file_location(
        "wpt_selection", MODULE_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load wpt_selection module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args],
        text=True,
        stderr=subprocess.STDOUT,
    ).strip()


class WptSelectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.selection = load_module()

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.repo = Path(self.temp.name) / "wpt"
        self.repo.mkdir()
        subprocess.run(
            ["git", "init", "-q"], cwd=self.repo, check=True
        )
        subprocess.run(
            ["git", "config", "user.email", "test@example.invalid"],
            cwd=self.repo,
            check=True,
        )
        subprocess.run(
            ["git", "config", "user.name", "Rarog test"],
            cwd=self.repo,
            check=True,
        )

        (self.repo / "css").mkdir()
        (self.repo / "html").mkdir()
        (self.repo / "css" / "ref.html").write_text(
            "<!doctype html><p>reference</p>\n",
            encoding="utf-8",
        )
        (self.repo / "css" / "test.html").write_text(
            '<!doctype html><link rel="match" href="ref.html">\n',
            encoding="utf-8",
        )
        (self.repo / "html" / "test.html").write_text(
            '<!doctype html>'
            '<script src="/resources/testharness.js"></script>\n',
            encoding="utf-8",
        )
        subprocess.run(
            ["git", "add", "."], cwd=self.repo, check=True
        )
        subprocess.run(
            ["git", "commit", "-q", "-m", "fixture"],
            cwd=self.repo,
            check=True,
        )

        self.head = git(self.repo, "rev-parse", "HEAD")
        self.manifest = {
            "schema_version": 1,
            "source": {
                "repository": "web-platform-tests/wpt",
                "commit": self.head,
            },
            "tests": [
                {
                    "area": "css-selectors",
                    "path": "css/test.html",
                    "kind": "reftest",
                    "blob": git(
                        self.repo,
                        "hash-object",
                        "--",
                        "css/test.html",
                    ),
                    "references": [
                        {
                            "relation": "match",
                            "path": "css/ref.html",
                            "blob": git(
                                self.repo,
                                "hash-object",
                                "--",
                                "css/ref.html",
                            ),
                        }
                    ],
                },
                {
                    "area": "html-parsing",
                    "path": "html/test.html",
                    "kind": "testharness",
                    "blob": git(
                        self.repo,
                        "hash-object",
                        "--",
                        "html/test.html",
                    ),
                    "references": [],
                },
            ],
        }

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_committed_selection_is_strict_and_file_scoped(self) -> None:
        manifest = self.selection.load_manifest(SELECTION)
        self.assertEqual(
            manifest["source"]["commit"],
            "a83afd4402cffdc876508fe9a47f916d4136099f",
        )
        self.assertEqual(len(manifest["tests"]), 5)
        self.assertEqual(
            sum(len(item["references"]) for item in manifest["tests"]),
            2,
        )
        paths = [item["path"] for item in manifest["tests"]]
        self.assertEqual(paths, sorted(paths))
        self.assertTrue(all(not path.endswith("/") for path in paths))

    def test_valid_checkout_verifies_exact_commit_blobs_and_metadata(self) -> None:
        manifest = self.selection.validate_manifest(self.manifest)
        self.assertEqual(
            self.selection.verify_checkout(manifest, self.repo),
            {"tests": 2, "references": 1},
        )

    def test_wrong_checkout_commit_is_rejected(self) -> None:
        (self.repo / "unrelated.txt").write_text(
            "new commit\n", encoding="utf-8"
        )
        subprocess.run(
            ["git", "add", "."], cwd=self.repo, check=True
        )
        subprocess.run(
            ["git", "commit", "-q", "-m", "drift"],
            cwd=self.repo,
            check=True,
        )
        manifest = self.selection.validate_manifest(self.manifest)
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "checkout commit mismatch",
        ):
            self.selection.verify_checkout(manifest, self.repo)

    def test_worktree_blob_drift_is_rejected(self) -> None:
        (self.repo / "html" / "test.html").write_text(
            "<!doctype html><p>changed</p>\n",
            encoding="utf-8",
        )
        manifest = self.selection.validate_manifest(self.manifest)
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "blob mismatch",
        ):
            self.selection.verify_checkout(manifest, self.repo)

    def test_reftest_reference_metadata_must_match_manifest(self) -> None:
        malformed = copy.deepcopy(self.manifest)
        malformed["tests"][0]["references"][0]["relation"] = "mismatch"
        manifest = self.selection.validate_manifest(malformed)
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "declared references do not match",
        ):
            self.selection.verify_checkout(manifest, self.repo)

    def test_test_paths_must_be_unique_and_sorted(self) -> None:
        unsorted = copy.deepcopy(self.manifest)
        unsorted["tests"].reverse()
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "strictly sorted",
        ):
            self.selection.validate_manifest(unsorted)

        duplicate = copy.deepcopy(self.manifest)
        duplicate["tests"].insert(
            1, copy.deepcopy(duplicate["tests"][0])
        )
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "duplicate test path",
        ):
            self.selection.validate_manifest(duplicate)

    def test_kind_and_reference_contracts_fail_closed(self) -> None:
        missing_reference = copy.deepcopy(self.manifest)
        missing_reference["tests"][0]["references"] = []
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "must declare a reference",
        ):
            self.selection.validate_manifest(missing_reference)

        bad_kind = copy.deepcopy(self.manifest)
        bad_kind["tests"][1]["kind"] = "manual"
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "unsupported kind",
        ):
            self.selection.validate_manifest(bad_kind)

    def test_paths_cannot_escape_checkout(self) -> None:
        malformed = copy.deepcopy(self.manifest)
        malformed["tests"][0]["path"] = "../outside.html"
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "normalized relative POSIX",
        ):
            self.selection.validate_manifest(malformed)

    def test_strict_json_rejects_duplicate_keys_and_nonfinite_numbers(
        self,
    ) -> None:
        duplicate = Path(self.temp.name) / "duplicate.json"
        duplicate.write_text(
            '{"schema_version":1,"schema_version":1}',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "duplicate key",
        ):
            self.selection.load_manifest(duplicate)

        nonfinite = Path(self.temp.name) / "nonfinite.json"
        nonfinite.write_text(
            '{"schema_version":NaN}',
            encoding="utf-8",
        )
        with self.assertRaisesRegex(
            self.selection.SelectionError,
            "non-finite JSON",
        ):
            self.selection.load_manifest(nonfinite)


if __name__ == "__main__":
    unittest.main()
