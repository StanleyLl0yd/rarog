#!/usr/bin/env python3
"""Bind an executed WPT report to the exact R6 file-level selection."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, Sequence

SCHEMA_VERSION = 1
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


class EvidenceError(ValueError):
    """Raised when WPT execution evidence does not match the pinned denominator."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise EvidenceError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise EvidenceError(f"non-finite JSON number is not allowed: {value}")


def _load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"{path}: cannot read JSON: {error}") from error


def _require_commit(value: str, label: str) -> str:
    if not _COMMIT_RE.fullmatch(value):
        raise EvidenceError(f"{label} must be an exact lowercase 40-hex commit")
    return value


def _canonical_bytes(document: Any) -> bytes:
    return json.dumps(
        document,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def _digest(document: Any) -> str:
    return "sha256:" + hashlib.sha256(_canonical_bytes(document)).hexdigest()


def build_evidence(
    selection: Any,
    report: Any,
    *,
    rarog_commit: str,
    wpt_commit: str,
) -> dict[str, Any]:
    rarog_commit = _require_commit(rarog_commit, "Rarog commit")
    wpt_commit = _require_commit(wpt_commit, "WPT commit")

    if not isinstance(selection, dict):
        raise EvidenceError("selection root must be an object")
    source = selection.get("source")
    tests = selection.get("tests")
    if not isinstance(source, dict) or not isinstance(tests, list) or not tests:
        raise EvidenceError("selection must contain source and non-empty tests")
    if source.get("commit") != wpt_commit:
        raise EvidenceError(
            f"selection WPT commit {source.get('commit')!r} does not match {wpt_commit}"
        )

    selected_ids: list[str] = []
    for index, item in enumerate(tests):
        if not isinstance(item, dict):
            raise EvidenceError(f"selection test {index} must be an object")
        path = item.get("path")
        if not isinstance(path, str) or not path or path.startswith("/"):
            raise EvidenceError(f"selection test {index} has invalid path")
        selected_ids.append("/" + path)
    if selected_ids != sorted(selected_ids) or len(set(selected_ids)) != len(selected_ids):
        raise EvidenceError("selection test IDs must be unique and sorted")

    if not isinstance(report, dict):
        raise EvidenceError("report root must be an object")
    results = report.get("results")
    if not isinstance(results, list) or not results:
        raise EvidenceError("report must contain non-empty results")

    observed_ids: list[str] = []
    for index, result in enumerate(results):
        if not isinstance(result, dict):
            raise EvidenceError(f"report result {index} must be an object")
        test_id = result.get("test")
        if not isinstance(test_id, str) or not test_id:
            raise EvidenceError(f"report result {index} has invalid test id")
        observed_ids.append(test_id)

    if len(set(observed_ids)) != len(observed_ids):
        raise EvidenceError("report contains duplicate test IDs")

    selected_set = set(selected_ids)
    observed_set = set(observed_ids)
    missing = sorted(selected_set - observed_set)
    extra = sorted(observed_set - selected_set)
    if missing or extra:
        raise EvidenceError(
            f"report denominator mismatch; missing={missing}, extra={extra}"
        )

    observed_ids.sort()
    return {
        "schema_version": SCHEMA_VERSION,
        "rarog_commit": rarog_commit,
        "wpt_commit": wpt_commit,
        "selection_sha256": _digest(selection),
        "report_sha256": _digest(report),
        "selected_tests": len(selected_ids),
        "observed_tests": len(observed_ids),
        "test_ids": observed_ids,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--rarog-commit", required=True)
    parser.add_argument("--wpt-commit", required=True)
    parser.add_argument("--json-out", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        evidence = build_evidence(
            _load_json(args.selection),
            _load_json(args.report),
            rarog_commit=args.rarog_commit,
            wpt_commit=args.wpt_commit,
        )
        args.json_out.write_text(
            json.dumps(evidence, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except EvidenceError as error:
        print(f"wpt-evidence: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
