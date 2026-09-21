#!/usr/bin/env python3
"""Normalize WPT wptreport JSON into deterministic Rarog compatibility dashboards."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Iterable, Sequence

SCHEMA_VERSION = 1
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
KNOWN_SYNTHETIC_REPORT_DIGESTS = {
    "sha256:cf9e06050ce2445663ffb248eed90e7103df1e88bd24980ff171020ef2179c91",
}


class DashboardError(ValueError):
    """Raised when WPT report input cannot be normalized safely."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DashboardError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise DashboardError(f"non-finite JSON number is not allowed: {value}")


def _require_commit(value: str, label: str) -> str:
    if not _COMMIT_RE.fullmatch(value):
        raise DashboardError(f"{label} must be an exact lowercase 40-hex commit")
    return value


def _require_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise DashboardError(f"{label} must be a non-empty string")
    return value


def _normalize_expected(value: Any, label: str) -> list[str] | None:
    if value is None:
        return None
    if isinstance(value, str):
        return [_require_text(value, label)]
    if isinstance(value, list):
        if not value:
            raise DashboardError(f"{label} must not be an empty list")
        normalized = [_require_text(item, label) for item in value]
        if len(set(normalized)) != len(normalized):
            raise DashboardError(f"{label} contains duplicate statuses")
        return sorted(normalized)
    raise DashboardError(f"{label} must be a string, list of strings, or absent")


def _normalize_subtests(raw: Any, test_id: str) -> list[dict[str, Any]]:
    if raw is None:
        return []
    if not isinstance(raw, list):
        raise DashboardError(f"{test_id}: subtests must be a list")
    seen: set[str] = set()
    normalized: list[dict[str, Any]] = []
    for index, item in enumerate(raw):
        if not isinstance(item, dict):
            raise DashboardError(f"{test_id}: subtest {index} must be an object")
        name = _require_text(item.get("name"), f"{test_id}: subtest {index} name")
        if name in seen:
            raise DashboardError(f"{test_id}: duplicate subtest name {name!r}")
        seen.add(name)
        status = _require_text(item.get("status"), f"{test_id}/{name}: status")
        expected = _normalize_expected(item.get("expected"), f"{test_id}/{name}: expected")
        normalized.append(
            {
                "name": name,
                "status": status,
                "expected": expected,
                "unexpected": expected is not None and status not in expected,
            }
        )
    normalized.sort(key=lambda subtest: subtest["name"])
    return normalized


def normalize_reports(
    reports: Iterable[tuple[str, Any]],
    *,
    rarog_commit: str,
    wpt_commit: str,
    platform: str,
    synthetic: bool,
) -> dict[str, Any]:
    rarog_commit = _require_commit(rarog_commit, "Rarog commit")
    wpt_commit = _require_commit(wpt_commit, "WPT commit")
    platform = _require_text(platform, "platform")

    tests: list[dict[str, Any]] = []
    seen_tests: set[str] = set()
    report_names: list[str] = []

    for report_name, document in reports:
        report_names.append(report_name)
        if not isinstance(document, dict):
            raise DashboardError(f"{report_name}: report root must be an object")
        raw_results = document.get("results")
        if not isinstance(raw_results, list):
            raise DashboardError(f"{report_name}: results must be a list")
        for index, item in enumerate(raw_results):
            if not isinstance(item, dict):
                raise DashboardError(f"{report_name}: result {index} must be an object")
            test_id = _require_text(item.get("test"), f"{report_name}: result {index} test")
            if test_id in seen_tests:
                raise DashboardError(f"duplicate test id across reports: {test_id}")
            seen_tests.add(test_id)
            status = _require_text(item.get("status"), f"{test_id}: status")
            expected = _normalize_expected(item.get("expected"), f"{test_id}: expected")
            tests.append(
                {
                    "test": test_id,
                    "status": status,
                    "expected": expected,
                    "unexpected": expected is not None and status not in expected,
                    "subtests": _normalize_subtests(item.get("subtests"), test_id),
                }
            )

    if not report_names:
        raise DashboardError("at least one WPT report is required")
    if not synthetic and any(name in KNOWN_SYNTHETIC_REPORT_DIGESTS for name in report_names):
        raise DashboardError("known synthetic fixture requires --synthetic")
    if not tests:
        raise DashboardError("WPT reports contain no test results")

    tests.sort(key=lambda result: result["test"])
    report_names.sort()

    test_statuses = Counter(item["status"] for item in tests)
    subtests = [subtest for item in tests for subtest in item["subtests"]]
    subtest_statuses = Counter(item["status"] for item in subtests)

    summary = {
        "measured_tests": len(tests),
        "measured_subtests": len(subtests),
        "tests_with_expectations": sum(item["expected"] is not None for item in tests),
        "subtests_with_expectations": sum(item["expected"] is not None for item in subtests),
        "unexpected_tests": sum(item["unexpected"] for item in tests),
        "unexpected_subtests": sum(item["unexpected"] for item in subtests),
        "test_statuses": dict(sorted(test_statuses.items())),
        "subtest_statuses": dict(sorted(subtest_statuses.items())),
    }

    return {
        "schema_version": SCHEMA_VERSION,
        "synthetic": synthetic,
        "rarog_commit": rarog_commit,
        "wpt_commit": wpt_commit,
        "platform": platform,
        "source_reports": report_names,
        "summary": summary,
        "results": tests,
    }


def _md(value: Any) -> str:
    if value is None:
        return "unknown"
    if isinstance(value, list):
        value = ", ".join(value)
    return str(value).replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def render_markdown(dashboard: dict[str, Any]) -> str:
    summary = dashboard["summary"]
    lines = ["# Rarog WPT compatibility dashboard", ""]
    if dashboard["synthetic"]:
        lines.extend(
            [
                "> **Synthetic fixture — not compatibility evidence.**",
                "> These records test dashboard tooling only and must not be reported as Rarog compatibility results.",
                "",
            ]
        )
    lines.extend(
        [
            f"- Rarog commit: `{dashboard['rarog_commit']}`",
            f"- Upstream WPT commit: `{dashboard['wpt_commit']}`",
            f"- Platform: `{_md(dashboard['platform'])}`",
            f"- Source reports: {len(dashboard['source_reports'])}",
            "",
            "## Measured scope",
            "",
            "| Level | Measured | With expectation metadata | Unexpected |",
            "| --- | ---: | ---: | ---: |",
            f"| Tests | {summary['measured_tests']} | {summary['tests_with_expectations']} | {summary['unexpected_tests']} |",
            f"| Subtests | {summary['measured_subtests']} | {summary['subtests_with_expectations']} | {summary['unexpected_subtests']} |",
            "",
            "Only records present in the supplied WPT reports are measured. This dashboard does not infer results for unmeasured tests or directories and does not claim general Web compatibility.",
            "",
            "## Test statuses",
            "",
            "| Status | Count |",
            "| --- | ---: |",
        ]
    )
    for status, count in summary["test_statuses"].items():
        lines.append(f"| {_md(status)} | {count} |")
    if not summary["test_statuses"]:
        lines.append("| *(none)* | 0 |")

    lines.extend(["", "## Subtest statuses", "", "| Status | Count |", "| --- | ---: |"])
    for status, count in summary["subtest_statuses"].items():
        lines.append(f"| {_md(status)} | {count} |")
    if not summary["subtest_statuses"]:
        lines.append("| *(none)* | 0 |")

    lines.extend(
        [
            "",
            "## Test outcomes",
            "",
            "| Test | Observed | Expected | Unexpected | Subtests |",
            "| --- | --- | --- | --- | ---: |",
        ]
    )
    for result in dashboard["results"]:
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{_md(result['test'])}`",
                    _md(result["status"]),
                    _md(result["expected"]),
                    "yes" if result["unexpected"] else "no",
                    str(len(result["subtests"])),
                ]
            )
            + " |"
        )
    if not dashboard["results"]:
        lines.append("| *(none)* | - | - | - | 0 |")
    return "\n".join(lines) + "\n"


def load_reports(paths: Sequence[Path]) -> list[tuple[str, Any]]:
    reports: list[tuple[str, Any]] = []
    for path in paths:
        try:
            with path.open("r", encoding="utf-8") as handle:
                document = json.load(
                    handle,
                    object_pairs_hook=_reject_duplicate_keys,
                    parse_constant=_reject_nonfinite,
                )
        except (OSError, json.JSONDecodeError) as error:
            raise DashboardError(f"{path}: cannot read JSON report: {error}") from error
        canonical = json.dumps(document, sort_keys=True, separators=(",", ":")).encode("utf-8")
        digest = hashlib.sha256(canonical).hexdigest()
        reports.append((f"sha256:{digest}", document))
    return reports


def write_json(path: Path, dashboard: dict[str, Any]) -> None:
    path.write_text(json.dumps(dashboard, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", action="append", required=True, type=Path)
    parser.add_argument("--rarog-commit", required=True)
    parser.add_argument("--wpt-commit", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--json-out", required=True, type=Path)
    parser.add_argument("--markdown-out", required=True, type=Path)
    parser.add_argument("--synthetic", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        dashboard = normalize_reports(
            load_reports(args.report),
            rarog_commit=args.rarog_commit,
            wpt_commit=args.wpt_commit,
            platform=args.platform,
            synthetic=args.synthetic,
        )
        write_json(args.json_out, dashboard)
        args.markdown_out.write_text(render_markdown(dashboard), encoding="utf-8")
    except DashboardError as error:
        print(f"wpt-dashboard: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
