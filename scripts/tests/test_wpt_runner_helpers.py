from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "wpt" / "runner" / "rarog_wpt_helpers.py"
SELECTION = ROOT / "wpt" / "r6-selection.json"


def load_module():
    spec = importlib.util.spec_from_file_location(
        "rarog_wpt_helpers", MODULE_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load rarog_wpt_helpers module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RarogWptHelperTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.helpers = load_module()
        cls.allowed = cls.helpers.load_allowed_paths(SELECTION)

    def test_selection_allowlist_contains_tests_and_references(self) -> None:
        self.assertEqual(len(self.allowed), 7)
        self.assertIn("css/selectors/dir-style-01a.html", self.allowed)
        self.assertIn("css/selectors/dir-style-01-ref.html", self.allowed)
        self.assertIn(
            "html/syntax/parsing/ambiguous-ampersand.html",
            self.allowed,
        )

    def test_selected_url_maps_only_pinned_paths(self) -> None:
        self.assertEqual(
            self.helpers.selected_url_to_path(
                "/css/selectors/dir-style-01a.html?cache=1#fragment",
                self.allowed,
            ),
            "css/selectors/dir-style-01a.html",
        )
        with self.assertRaisesRegex(
            self.helpers.AdapterError, "outside the pinned R6 selection"
        ):
            self.helpers.selected_url_to_path(
                "/css/selectors/not-selected.html", self.allowed
            )

    def test_selected_url_rejects_absolute_and_traversing_inputs(self) -> None:
        with self.assertRaisesRegex(
            self.helpers.AdapterError, "must be path-only"
        ):
            self.helpers.selected_url_to_path(
                "https://web-platform.test/css/selectors/dir-style-01a.html",
                self.allowed,
            )
        with self.assertRaisesRegex(
            self.helpers.AdapterError, "normalized repository-relative POSIX"
        ):
            self.helpers.selected_url_to_path(
                "/css/selectors/%2e%2e/dir-style-01a.html",
                self.allowed,
            )

    def test_viewport_parsing_is_bounded_to_positive_dimensions(self) -> None:
        self.assertEqual(self.helpers.parse_viewport(None), (800, 600))
        self.assertEqual(self.helpers.parse_viewport("640x480"), (640, 480))
        self.assertEqual(self.helpers.parse_viewport((320, 200)), (320, 200))
        for malformed in ("0x600", "800X600", (800, 0), True):
            with self.subTest(malformed=malformed):
                with self.assertRaises(self.helpers.AdapterError):
                    self.helpers.parse_viewport(malformed)

    def test_selected_workflow_invokes_exact_manifest_denominator(self) -> None:
        workflow = (
            ROOT / ".github" / "workflows" / "r6-wpt-selected.yml"
        ).read_text(encoding="utf-8")
        selection = __import__("json").loads(SELECTION.read_text(encoding="utf-8"))
        selected_paths = [item["path"] for item in selection["tests"]]

        self.assertIn("--test-types reftest testharness", workflow)
        self.assertNotIn("--test-type=", workflow)
        self.assertEqual(workflow.count("--include=/"), len(selected_paths))
        for path in selected_paths:
            self.assertIn(f"--include=/{path}", workflow)

    def test_render_command_is_argument_vector_not_shell_text(self) -> None:
        command = self.helpers.build_render_command(
            Path("/tmp/rarog renderer"),
            Path("/tmp/input test.html"),
            Path("/tmp/frame.ppm"),
            (800, 600),
        )
        self.assertEqual(
            command,
            [
                "/tmp/rarog renderer",
                "--input",
                "/tmp/input test.html",
                "--output",
                "/tmp/frame.ppm",
                "--width",
                "800",
                "--height",
                "600",
            ],
        )

    def test_render_result_classification_preserves_failure_kind(self) -> None:
        self.assertIsNone(
            self.helpers.classify_render_result(
                returncode=0,
                timed_out=False,
                output_exists=True,
            )
        )
        self.assertEqual(
            self.helpers.classify_render_result(
                returncode=None,
                timed_out=True,
                output_exists=False,
            )[0],
            "EXTERNAL-TIMEOUT",
        )
        self.assertEqual(
            self.helpers.classify_render_result(
                returncode=7,
                timed_out=False,
                output_exists=False,
            )[0],
            "CRASH",
        )
        self.assertEqual(
            self.helpers.classify_render_result(
                returncode=0,
                timed_out=False,
                output_exists=False,
            )[0],
            "INTERNAL-ERROR",
        )

    def test_diagnostics_are_bounded(self) -> None:
        diagnostic = self.helpers.bounded_diagnostic("x" * 10_000)
        self.assertLess(len(diagnostic.encode("utf-8")), 4_200)
        self.assertTrue(diagnostic.endswith("[diagnostic truncated]"))

    def test_testharness_unsupported_result_is_explicit(self) -> None:
        message = self.helpers.unsupported_testharness_message("/html/test.html")
        self.assertIn("does not yet integrate WPT testharness", message)
        self.assertIn("/html/test.html", message)


if __name__ == "__main__":
    unittest.main()
