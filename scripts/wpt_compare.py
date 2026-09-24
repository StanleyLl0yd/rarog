#!/usr/bin/env python3
"""Compare two validated R6 WPT evidence sets without hiding scope drift."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Sequence

import wpt_dashboard
import wpt_evidence
import wpt_selection

SCHEMA_VERSION = 1


class ComparisonError(ValueError):
    """Raised when evidence cannot be validated or compared safely."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ComparisonError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise ComparisonError(f"non-finite JSON number is not allowed: {value}")


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise ComparisonError(f"{path}: cannot read JSON: {error}") from error


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ComparisonError(f"{label} must be an object")
    return value


def validate_bundle(
    *,
    selection: Any,
    report: Any,
    evidence: Any,
    dashboard: Any,
    label: str,
) -> dict[str, Any]:
    try:
        normalized_selection = wpt_selection.validate_manifest(selection)
    except wpt_selection.SelectionError as error:
        raise ComparisonError(f"{label}: invalid selection: {error}") from error

    evidence = _require_object(evidence, f"{label} evidence")
    dashboard = _require_object(dashboard, f"{label} dashboard")

    rarog_commit = evidence.get("rarog_commit")
    wpt_commit = evidence.get("wpt_commit")
    if not isinstance(rarog_commit, str) or not isinstance(wpt_commit, str):
        raise ComparisonError(f"{label}: evidence commit identities are missing")

    try:
        regenerated_evidence = wpt_evidence.build_evidence(
            normalized_selection,
            report,
            rarog_commit=rarog_commit,
            wpt_commit=wpt_commit,
        )
    except wpt_evidence.EvidenceError as error:
        raise ComparisonError(f"{label}: invalid evidence binding: {error}") from error

    if regenerated_evidence != evidence:
        raise ComparisonError(f"{label}: evidence JSON does not reproduce exactly")

    if dashboard.get("synthetic") is not False:
        raise ComparisonError(f"{label}: only non-synthetic dashboards are comparable")
    platform = dashboard.get("platform")
    if not isinstance(platform, str) or not platform:
        raise ComparisonError(f"{label}: dashboard platform must be a non-empty string")

    try:
        regenerated_dashboard = wpt_dashboard.normalize_reports(
            [(evidence["report_sha256"], report)],
            rarog_commit=rarog_commit,
            wpt_commit=wpt_commit,
            platform=platform,
            synthetic=False,
        )
    except wpt_dashboard.DashboardError as error:
        raise ComparisonError(f"{label}: invalid dashboard source: {error}") from error

    if regenerated_dashboard != dashboard:
        raise ComparisonError(f"{label}: dashboard JSON does not reproduce exactly")

    return {
        "selection": normalized_selection,
        "report": report,
        "evidence": evidence,
        "dashboard": dashboard,
        "identity": {
            "rarog_commit": rarog_commit,
            "wpt_commit": wpt_commit,
            "selection_sha256": evidence["selection_sha256"],
            "report_sha256": evidence["report_sha256"],
            "platform": platform,
        },
    }


def _test_id(item: dict[str, Any]) -> str:
    return "/" + item["path"]


def _logical_definition(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "kind": item["kind"],
        "references": [
            {"relation": ref["relation"], "path": ref["path"]}
            for ref in item["references"]
        ],
    }


def _source_definition(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "blob": item["blob"],
        "references": [
            {
                "relation": ref["relation"],
                "path": ref["path"],
                "blob": ref["blob"],
            }
            for ref in item["references"]
        ],
    }


def _selection_maps(
    selection: dict[str, Any],
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    logical: dict[str, dict[str, Any]] = {}
    source: dict[str, dict[str, Any]] = {}
    for item in selection["tests"]:
        test_id = _test_id(item)
        logical[test_id] = _logical_definition(item)
        source[test_id] = _source_definition(item)
    return logical, source


def _result_map(dashboard: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {item["test"]: item for item in dashboard["results"]}


def _result_state(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "status": item["status"],
        "expected": item["expected"],
        "unexpected": item["unexpected"],
        "subtests": item["subtests"],
    }


def _classification(
    *,
    same_upstream: bool,
    same_logical_selection: bool,
) -> str:
    if same_upstream and same_logical_selection:
        return "same-upstream-same-denominator"
    if same_upstream:
        return "same-upstream-changed-denominator"
    if same_logical_selection:
        return "changed-upstream-same-logical-selection"
    return "changed-upstream-changed-denominator"


def compare_bundles(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
) -> dict[str, Any]:
    baseline_logical, baseline_source = _selection_maps(baseline["selection"])
    candidate_logical, candidate_source = _selection_maps(candidate["selection"])
    baseline_results = _result_map(baseline["dashboard"])
    candidate_results = _result_map(candidate["dashboard"])

    baseline_ids = set(baseline_logical)
    candidate_ids = set(candidate_logical)
    common_ids = sorted(baseline_ids & candidate_ids)
    added_tests = sorted(candidate_ids - baseline_ids)
    removed_tests = sorted(baseline_ids - candidate_ids)

    changed_test_definitions = []
    for test_id in common_ids:
        if baseline_logical[test_id] != candidate_logical[test_id]:
            changed_test_definitions.append(
                {
                    "test": test_id,
                    "before": baseline_logical[test_id],
                    "after": candidate_logical[test_id],
                }
            )

    same_logical_selection = (
        not added_tests
        and not removed_tests
        and not changed_test_definitions
    )
    same_upstream = (
        baseline["identity"]["wpt_commit"]
        == candidate["identity"]["wpt_commit"]
    )
    classification = _classification(
        same_upstream=same_upstream,
        same_logical_selection=same_logical_selection,
    )

    source_changes = []
    source_change_ids: set[str] = set()
    for test_id in common_ids:
        before = baseline_source[test_id]
        after = candidate_source[test_id]
        if before != after:
            source_change_ids.add(test_id)
            source_changes.append(
                {"test": test_id, "before": before, "after": after}
            )

    expectation_changes = []
    expectation_change_ids: set[str] = set()
    for test_id in common_ids:
        before_expected = baseline_results[test_id]["expected"]
        after_expected = candidate_results[test_id]["expected"]
        if before_expected != after_expected:
            expectation_change_ids.add(test_id)
            expectation_changes.append(
                {
                    "test": test_id,
                    "before": before_expected,
                    "after": after_expected,
                }
            )

    same_platform = (
        baseline["identity"]["platform"]
        == candidate["identity"]["platform"]
    )
    same_selection_digest = (
        baseline["identity"]["selection_sha256"]
        == candidate["identity"]["selection_sha256"]
    )

    global_reasons: list[str] = []
    if classification != "same-upstream-same-denominator":
        global_reasons.append(classification)
    if not same_selection_digest:
        global_reasons.append("selection-digest-changed")
    if not same_platform:
        global_reasons.append("platform-changed")
    if source_changes:
        global_reasons.append("selected-source-content-changed")
    if expectation_changes:
        global_reasons.append("expectation-basis-changed")

    base_direct_comparable = (
        classification == "same-upstream-same-denominator"
        and same_selection_digest
        and same_platform
    )

    result_changes = []
    summary = {
        "common_tests": len(common_ids),
        "added_tests": len(added_tests),
        "removed_tests": len(removed_tests),
        "changed_results": 0,
        "regressions": 0,
        "improvements": 0,
        "changed_unexpected_statuses": 0,
        "changed_expected_statuses": 0,
        "observed_changes_not_directly_comparable": 0,
        "unchanged_results": 0,
    }

    for test_id in common_ids:
        before = _result_state(baseline_results[test_id])
        after = _result_state(candidate_results[test_id])
        changed = before != after
        test_comparable = (
            base_direct_comparable
            and test_id not in source_change_ids
            and test_id not in expectation_change_ids
        )

        if not changed:
            interpretation = "unchanged"
            summary["unchanged_results"] += 1
        elif not test_comparable:
            interpretation = "observed-change-not-directly-comparable"
            summary["observed_changes_not_directly_comparable"] += 1
        elif not before["unexpected"] and after["unexpected"]:
            interpretation = "regression"
            summary["regressions"] += 1
        elif before["unexpected"] and not after["unexpected"]:
            interpretation = "improvement"
            summary["improvements"] += 1
        elif before["unexpected"] and after["unexpected"]:
            interpretation = "changed-unexpected-status"
            summary["changed_unexpected_statuses"] += 1
        else:
            interpretation = "changed-expected-status"
            summary["changed_expected_statuses"] += 1

        if changed:
            summary["changed_results"] += 1

        result_changes.append(
            {
                "test": test_id,
                "comparable": test_comparable,
                "interpretation": interpretation,
                "before": before,
                "after": after,
            }
        )

    direct_behavior_comparable = (
        base_direct_comparable
        and not source_changes
        and not expectation_changes
    )

    return {
        "schema_version": SCHEMA_VERSION,
        "classification": classification,
        "direct_behavior_comparable": direct_behavior_comparable,
        "non_comparable_reasons": sorted(set(global_reasons)),
        "baseline": baseline["identity"],
        "candidate": candidate["identity"],
        "scope": {
            "logical_selection_equal": same_logical_selection,
            "selection_digest_equal": same_selection_digest,
            "platform_equal": same_platform,
            "added_tests": added_tests,
            "removed_tests": removed_tests,
            "changed_test_definitions": changed_test_definitions,
            "source_changes": source_changes,
        },
        "expectation_changes": expectation_changes,
        "results": result_changes,
        "summary": summary,
    }


def _md(value: Any) -> str:
    if value is None:
        return "unknown"
    if isinstance(value, list):
        value = ", ".join(str(item) for item in value)
    return str(value).replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def render_markdown(comparison: dict[str, Any]) -> str:
    lines = [
        "# Rarog WPT evidence comparison",
        "",
        f"- Classification: `{comparison['classification']}`",
        "- Direct behavior comparison: "
        + ("yes" if comparison["direct_behavior_comparable"] else "no"),
    ]
    if comparison["non_comparable_reasons"]:
        lines.append(
            "- Comparison cautions: "
            + ", ".join(f"`{item}`" for item in comparison["non_comparable_reasons"])
        )

    lines.extend(
        [
            "",
            "## Evidence identities",
            "",
            "| Identity | Baseline | Candidate |",
            "| --- | --- | --- |",
        ]
    )
    labels = [
        ("Rarog commit", "rarog_commit"),
        ("WPT commit", "wpt_commit"),
        ("Selection digest", "selection_sha256"),
        ("Report digest", "report_sha256"),
        ("Platform", "platform"),
    ]
    for label, key in labels:
        lines.append(
            f"| {label} | `{_md(comparison['baseline'][key])}` | "
            f"`{_md(comparison['candidate'][key])}` |"
        )

    scope = comparison["scope"]
    lines.extend(
        [
            "",
            "## Scope changes",
            "",
            f"- Added tests: {len(scope['added_tests'])}",
            f"- Removed tests: {len(scope['removed_tests'])}",
            f"- Changed logical test definitions: {len(scope['changed_test_definitions'])}",
            f"- Selected source-content changes: {len(scope['source_changes'])}",
        ]
    )
    for label, items in (
        ("Added", scope["added_tests"]),
        ("Removed", scope["removed_tests"]),
    ):
        if items:
            lines.extend(["", f"### {label} tests", ""])
            lines.extend(f"- `{_md(item)}`" for item in items)

    if scope["changed_test_definitions"]:
        lines.extend(["", "### Changed logical definitions", ""])
        for item in scope["changed_test_definitions"]:
            lines.append(f"- `{_md(item['test'])}`")

    if scope["source_changes"]:
        lines.extend(["", "### Selected source changes", ""])
        for item in scope["source_changes"]:
            lines.append(f"- `{_md(item['test'])}`")

    if comparison["expectation_changes"]:
        lines.extend(["", "## Expectation changes", ""])
        for item in comparison["expectation_changes"]:
            lines.append(
                f"- `{_md(item['test'])}`: "
                f"{_md(item['before'])} -> {_md(item['after'])}"
            )

    lines.extend(
        [
            "",
            "## Per-test observations",
            "",
            "| Test | Before | After | Before unexpected | After unexpected | Interpretation |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for item in comparison["results"]:
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{_md(item['test'])}`",
                    _md(item["before"]["status"]),
                    _md(item["after"]["status"]),
                    "yes" if item["before"]["unexpected"] else "no",
                    "yes" if item["after"]["unexpected"] else "no",
                    _md(item["interpretation"]),
                ]
            )
            + " |"
        )

    summary = comparison["summary"]
    lines.extend(
        [
            "",
            "## Summary",
            "",
            f"- Common tests: {summary['common_tests']}",
            f"- Changed results: {summary['changed_results']}",
            f"- Regressions: {summary['regressions']}",
            f"- Improvements: {summary['improvements']}",
            f"- Changed unexpected statuses: {summary['changed_unexpected_statuses']}",
            f"- Changed expected statuses: {summary['changed_expected_statuses']}",
            "- Observed changes not directly comparable: "
            f"{summary['observed_changes_not_directly_comparable']}",
            "",
            "No compatibility percentage or aggregate score is inferred from this comparison.",
        ]
    )
    return "\n".join(lines) + "\n"


def load_bundle(
    *,
    selection_path: Path,
    report_path: Path,
    evidence_path: Path,
    dashboard_path: Path,
    label: str,
) -> dict[str, Any]:
    return validate_bundle(
        selection=load_json(selection_path),
        report=load_json(report_path),
        evidence=load_json(evidence_path),
        dashboard=load_json(dashboard_path),
        label=label,
    )


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    for prefix in ("baseline", "candidate"):
        parser.add_argument(f"--{prefix}-selection", required=True, type=Path)
        parser.add_argument(f"--{prefix}-report", required=True, type=Path)
        parser.add_argument(f"--{prefix}-evidence", required=True, type=Path)
        parser.add_argument(f"--{prefix}-dashboard", required=True, type=Path)
    parser.add_argument("--json-out", required=True, type=Path)
    parser.add_argument("--markdown-out", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        baseline = load_bundle(
            selection_path=args.baseline_selection,
            report_path=args.baseline_report,
            evidence_path=args.baseline_evidence,
            dashboard_path=args.baseline_dashboard,
            label="baseline",
        )
        candidate = load_bundle(
            selection_path=args.candidate_selection,
            report_path=args.candidate_report,
            evidence_path=args.candidate_evidence,
            dashboard_path=args.candidate_dashboard,
            label="candidate",
        )
        comparison = compare_bundles(baseline, candidate)
        args.json_out.write_text(
            json.dumps(comparison, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        args.markdown_out.write_text(
            render_markdown(comparison),
            encoding="utf-8",
        )
    except ComparisonError as error:
        print(f"wpt-compare: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
