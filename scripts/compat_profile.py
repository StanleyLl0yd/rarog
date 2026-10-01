#!/usr/bin/env python3
"""Build and validate the canonical R6 compatibility-profile payload."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, Sequence

import real_web_baseline
import real_web_corpus
import wpt_dashboard
import wpt_evidence
import wpt_selection

SCHEMA_VERSION = 1
PROFILE_REVISION = 1
_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
_SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
_ALLOWED_KINDS = {"real-web", "wpt"}


class ProfileError(ValueError):
    """Raised when a compatibility profile or referenced evidence is invalid."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ProfileError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise ProfileError(f"non-finite JSON number is not allowed: {value}")


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise ProfileError(f"{path}: cannot read JSON: {error}") from error


def _canonical_bytes(document: Any) -> bytes:
    return json.dumps(
        document,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def canonical_digest(document: Any) -> str:
    return "sha256:" + hashlib.sha256(_canonical_bytes(document)).hexdigest()


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProfileError(f"{label} must be an object")
    return value


def _require_exact_keys(value: dict[str, Any], keys: set[str], label: str) -> None:
    actual = set(value)
    if actual != keys:
        raise ProfileError(
            f"{label} has invalid keys; "
            f"missing={sorted(keys - actual)}, extra={sorted(actual - keys)}"
        )


def _require_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ProfileError(f"{label} must be a non-empty string")
    return value


def _require_commit(value: Any, label: str) -> str:
    text = _require_text(value, label)
    if not _COMMIT_RE.fullmatch(text):
        raise ProfileError(f"{label} must be an exact lowercase 40-hex commit")
    return text


def _require_sha256(value: Any, label: str) -> str:
    text = _require_text(value, label)
    if not _SHA256_RE.fullmatch(text):
        raise ProfileError(f"{label} must be sha256:<64 lowercase hex>")
    return text


def _require_nonnegative_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ProfileError(f"{label} must be a non-negative integer")
    return value


def _require_repo_path(value: Any, label: str, prefix: str) -> str:
    path = _require_text(value, label)
    if (
        "\\" in path
        or path.startswith("/")
        or path.endswith("/")
        or ".." in path.split("/")
        or not path.startswith(prefix)
    ):
        raise ProfileError(f"{label} must be a normalized path under {prefix}")
    return path


def _validate_artifact(raw: Any, label: str, prefix: str) -> dict[str, str]:
    value = _require_object(raw, label)
    _require_exact_keys(value, {"path", "sha256"}, label)
    return {
        "path": _require_repo_path(value["path"], f"{label} path", prefix),
        "sha256": _require_sha256(value["sha256"], f"{label} sha256"),
    }


def _validate_wpt_entry(entry: dict[str, Any], label: str) -> dict[str, Any]:
    source = _require_object(entry["source"], f"{label} source")
    _require_exact_keys(source, {"repository", "commit"}, f"{label} source")
    repository = _require_text(source["repository"], f"{label} source repository")
    if repository != "web-platform-tests/wpt":
        raise ProfileError(f"{label} source repository must be web-platform-tests/wpt")
    source_commit = _require_commit(source["commit"], f"{label} WPT commit")

    platform = _require_object(entry["platform"], f"{label} platform")
    _require_exact_keys(platform, {"environment"}, f"{label} platform")
    environment = _require_text(platform["environment"], f"{label} platform environment")

    scope = _require_object(entry["scope"], f"{label} scope")
    _require_exact_keys(scope, {"selection_sha256", "test_ids"}, f"{label} scope")
    selection_digest = _require_sha256(
        scope["selection_sha256"], f"{label} selection sha256"
    )
    test_ids = scope["test_ids"]
    if (
        not isinstance(test_ids, list)
        or not test_ids
        or any(not isinstance(item, str) or not item.startswith("/") for item in test_ids)
        or test_ids != sorted(test_ids)
        or len(test_ids) != len(set(test_ids))
    ):
        raise ProfileError(f"{label} test_ids must be a non-empty sorted unique list")

    observed = _require_object(entry["observed"], f"{label} observed")
    expected_observed = {
        "measured_subtests",
        "measured_tests",
        "subtest_statuses",
        "subtests_with_expectations",
        "test_statuses",
        "tests_with_expectations",
        "unexpected_subtests",
        "unexpected_tests",
    }
    _require_exact_keys(observed, expected_observed, f"{label} observed")
    normalized_observed: dict[str, Any] = {}
    for key in (
        "measured_subtests",
        "measured_tests",
        "subtests_with_expectations",
        "tests_with_expectations",
        "unexpected_subtests",
        "unexpected_tests",
    ):
        normalized_observed[key] = _require_nonnegative_int(
            observed[key], f"{label} observed {key}"
        )
    for key in ("subtest_statuses", "test_statuses"):
        statuses = _require_object(observed[key], f"{label} observed {key}")
        if list(statuses) != sorted(statuses):
            raise ProfileError(f"{label} observed {key} keys must be sorted")
        normalized_observed[key] = {
            _require_text(status, f"{label} observed status"):
            _require_nonnegative_int(count, f"{label} observed {status} count")
            for status, count in statuses.items()
        }
    if sum(normalized_observed["test_statuses"].values()) != normalized_observed["measured_tests"]:
        raise ProfileError(f"{label} test status counts do not match measured_tests")
    if sum(normalized_observed["subtest_statuses"].values()) != normalized_observed["measured_subtests"]:
        raise ProfileError(f"{label} subtest status counts do not match measured_subtests")
    if normalized_observed["measured_tests"] != len(test_ids):
        raise ProfileError(f"{label} measured_tests does not match test_ids")

    return {
        "source": {"repository": repository, "commit": source_commit},
        "platform": {"environment": environment},
        "scope": {
            "selection_sha256": selection_digest,
            "test_ids": list(test_ids),
        },
        "observed": normalized_observed,
    }


def _validate_real_web_entry(entry: dict[str, Any], label: str) -> dict[str, Any]:
    source = _require_object(entry["source"], f"{label} source")
    _require_exact_keys(
        source, {"corpus_revision", "corpus_sha256"}, f"{label} source"
    )
    revision = source["corpus_revision"]
    if isinstance(revision, bool) or not isinstance(revision, int) or revision < 1:
        raise ProfileError(f"{label} corpus_revision must be a positive integer")
    corpus_digest = _require_sha256(
        source["corpus_sha256"], f"{label} corpus sha256"
    )

    platform = _require_object(entry["platform"], f"{label} platform")
    _require_exact_keys(platform, {"os", "arch", "environment"}, f"{label} platform")
    normalized_platform = {
        "os": _require_text(platform["os"], f"{label} platform os"),
        "arch": _require_text(platform["arch"], f"{label} platform arch"),
        "environment": _require_text(
            platform["environment"], f"{label} platform environment"
        ),
    }

    scope = _require_object(entry["scope"], f"{label} scope")
    _require_exact_keys(scope, {"scenario_ids"}, f"{label} scope")
    scenario_ids = scope["scenario_ids"]
    if (
        not isinstance(scenario_ids, list)
        or not scenario_ids
        or any(not isinstance(item, str) or not item for item in scenario_ids)
        or scenario_ids != sorted(scenario_ids)
        or len(scenario_ids) != len(set(scenario_ids))
    ):
        raise ProfileError(f"{label} scenario_ids must be a non-empty sorted unique list")

    observed = _require_object(entry["observed"], f"{label} observed")
    _require_exact_keys(
        observed, {"scenario_outcomes", "dependency_states"}, f"{label} observed"
    )
    scenario_outcomes = observed["scenario_outcomes"]
    if not isinstance(scenario_outcomes, list):
        raise ProfileError(f"{label} scenario_outcomes must be a list")
    normalized_outcomes: list[dict[str, str]] = []
    for index, raw in enumerate(scenario_outcomes):
        item = _require_object(raw, f"{label} scenario outcome {index}")
        _require_exact_keys(
            item, {"scenario_id", "category", "detail"},
            f"{label} scenario outcome {index}",
        )
        normalized_outcomes.append(
            {
                "scenario_id": _require_text(
                    item["scenario_id"], f"{label} scenario outcome id"
                ),
                "category": _require_text(
                    item["category"], f"{label} scenario outcome category"
                ),
                "detail": _require_text(
                    item["detail"], f"{label} scenario outcome detail"
                ),
            }
        )
    if [item["scenario_id"] for item in normalized_outcomes] != scenario_ids:
        raise ProfileError(f"{label} scenario outcomes must exactly cover scenario_ids")

    dependency_states = observed["dependency_states"]
    if not isinstance(dependency_states, list):
        raise ProfileError(f"{label} dependency_states must be a list")
    normalized_dependencies: list[dict[str, str]] = []
    for index, raw in enumerate(dependency_states):
        item = _require_object(raw, f"{label} dependency state {index}")
        _require_exact_keys(
            item, {"scenario_id", "origin", "role", "state"},
            f"{label} dependency state {index}",
        )
        normalized_dependencies.append(
            {
                "scenario_id": _require_text(
                    item["scenario_id"], f"{label} dependency scenario"
                ),
                "origin": _require_text(item["origin"], f"{label} dependency origin"),
                "role": _require_text(item["role"], f"{label} dependency role"),
                "state": _require_text(item["state"], f"{label} dependency state"),
            }
        )
    dependency_keys = [
        (item["scenario_id"], item["origin"], item["role"])
        for item in normalized_dependencies
    ]
    if dependency_keys != sorted(dependency_keys) or len(dependency_keys) != len(set(dependency_keys)):
        raise ProfileError(f"{label} dependency_states must be sorted and unique")
    if any(item["scenario_id"] not in scenario_ids for item in normalized_dependencies):
        raise ProfileError(f"{label} dependency state references unknown scenario")

    return {
        "source": {
            "corpus_revision": revision,
            "corpus_sha256": corpus_digest,
        },
        "platform": normalized_platform,
        "scope": {"scenario_ids": list(scenario_ids)},
        "observed": {
            "scenario_outcomes": normalized_outcomes,
            "dependency_states": normalized_dependencies,
        },
    }


def validate_profile(raw: Any) -> dict[str, Any]:
    profile = _require_object(raw, "profile")
    _require_exact_keys(
        profile, {"schema_version", "profile_revision", "evidence"}, "profile"
    )
    if profile["schema_version"] != SCHEMA_VERSION:
        raise ProfileError(f"profile schema_version must be {SCHEMA_VERSION}")
    revision = profile["profile_revision"]
    if isinstance(revision, bool) or not isinstance(revision, int) or revision < 1:
        raise ProfileError("profile_revision must be a positive integer")

    raw_entries = profile["evidence"]
    if not isinstance(raw_entries, list) or not raw_entries:
        raise ProfileError("profile evidence must be a non-empty list")

    entries: list[dict[str, Any]] = []
    seen_ids: set[str] = set()
    seen_identity: set[tuple[str, str, str, str]] = set()
    previous_id: str | None = None
    entry_keys = {
        "id",
        "kind",
        "measured_rarog_commit",
        "source",
        "raw_evidence",
        "normalized_evidence",
        "platform",
        "scope",
        "observed",
    }

    for index, raw_entry in enumerate(raw_entries):
        entry = _require_object(raw_entry, f"evidence {index}")
        _require_exact_keys(entry, entry_keys, f"evidence {index}")
        entry_id = _require_text(entry["id"], f"evidence {index} id")
        if not _ID_RE.fullmatch(entry_id):
            raise ProfileError(f"evidence {index} id must be lowercase kebab-case")
        if entry_id in seen_ids:
            raise ProfileError(f"duplicate evidence id {entry_id!r}")
        if previous_id is not None and entry_id <= previous_id:
            raise ProfileError("evidence entries must be strictly sorted by id")
        seen_ids.add(entry_id)
        previous_id = entry_id

        kind = _require_text(entry["kind"], f"{entry_id} kind")
        if kind not in _ALLOWED_KINDS:
            raise ProfileError(f"{entry_id} has unsupported evidence kind {kind!r}")
        measured_commit = _require_commit(
            entry["measured_rarog_commit"], f"{entry_id} measured Rarog commit"
        )
        prefix = "wpt/evidence/" if kind == "wpt" else "real-web/evidence/"
        raw_artifact = _validate_artifact(
            entry["raw_evidence"], f"{entry_id} raw evidence", prefix
        )
        normalized_artifact = _validate_artifact(
            entry["normalized_evidence"], f"{entry_id} normalized evidence", prefix
        )

        specific = (
            _validate_wpt_entry(entry, entry_id)
            if kind == "wpt"
            else _validate_real_web_entry(entry, entry_id)
        )
        identity = (
            kind,
            measured_commit,
            raw_artifact["sha256"],
            normalized_artifact["sha256"],
        )
        if identity in seen_identity:
            raise ProfileError(f"duplicate evidence identity for {entry_id}")
        seen_identity.add(identity)

        entries.append(
            {
                "id": entry_id,
                "kind": kind,
                "measured_rarog_commit": measured_commit,
                "source": specific["source"],
                "raw_evidence": raw_artifact,
                "normalized_evidence": normalized_artifact,
                "platform": specific["platform"],
                "scope": specific["scope"],
                "observed": specific["observed"],
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "profile_revision": revision,
        "evidence": entries,
    }


def _read_text(path: Path, label: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise ProfileError(f"{label}: cannot read text: {error}") from error


def reproduce_wpt_entry(root: Path) -> dict[str, Any]:
    selection_path = root / "wpt" / "r6-selection.json"
    report_path = root / "wpt" / "evidence" / "r6-first-wptreport.json"
    evidence_path = root / "wpt" / "evidence" / "r6-first-evidence.json"
    dashboard_path = root / "wpt" / "evidence" / "r6-first-dashboard.json"
    dashboard_md_path = root / "wpt" / "evidence" / "r6-first-dashboard.md"

    selection = wpt_selection.load_manifest(selection_path)
    report = load_json(report_path)
    committed_evidence = load_json(evidence_path)
    committed_dashboard = load_json(dashboard_path)

    measured_commit = _require_commit(
        committed_evidence.get("rarog_commit"), "WPT measured Rarog commit"
    )
    wpt_commit = _require_commit(
        committed_evidence.get("wpt_commit"), "WPT upstream commit"
    )
    if selection["source"]["commit"] != wpt_commit:
        raise ProfileError("WPT selection source commit does not match committed evidence")

    rebuilt_evidence = wpt_evidence.build_evidence(
        selection,
        report,
        rarog_commit=measured_commit,
        wpt_commit=wpt_commit,
    )
    if rebuilt_evidence != committed_evidence:
        raise ProfileError("committed WPT evidence JSON does not reproduce exactly")

    platform = _require_text(
        committed_dashboard.get("platform"), "WPT dashboard platform"
    )
    rebuilt_dashboard = wpt_dashboard.normalize_reports(
        [(rebuilt_evidence["report_sha256"], report)],
        rarog_commit=measured_commit,
        wpt_commit=wpt_commit,
        platform=platform,
        synthetic=False,
    )
    if rebuilt_dashboard != committed_dashboard:
        raise ProfileError("committed WPT dashboard JSON does not reproduce exactly")
    if wpt_dashboard.render_markdown(rebuilt_dashboard) != _read_text(
        dashboard_md_path, "WPT dashboard Markdown"
    ):
        raise ProfileError("committed WPT dashboard Markdown does not reproduce exactly")
    if rebuilt_dashboard["synthetic"]:
        raise ProfileError("synthetic WPT evidence cannot enter a compatibility profile")

    return {
        "id": "wpt-first-selected-baseline",
        "kind": "wpt",
        "measured_rarog_commit": measured_commit,
        "source": {
            "repository": selection["source"]["repository"],
            "commit": wpt_commit,
        },
        "raw_evidence": {
            "path": "wpt/evidence/r6-first-wptreport.json",
            "sha256": rebuilt_evidence["report_sha256"],
        },
        "normalized_evidence": {
            "path": "wpt/evidence/r6-first-dashboard.json",
            "sha256": canonical_digest(rebuilt_dashboard),
        },
        "platform": {"environment": platform},
        "scope": {
            "selection_sha256": rebuilt_evidence["selection_sha256"],
            "test_ids": rebuilt_evidence["test_ids"],
        },
        "observed": rebuilt_dashboard["summary"],
    }


def reproduce_real_web_entry(root: Path) -> dict[str, Any]:
    corpus_path = root / "real-web" / "corpus.json"
    execution_path = root / "real-web" / "evidence" / "r6-first-execution.json"
    baseline_path = root / "real-web" / "evidence" / "r6-first-baseline.json"
    baseline_md_path = root / "real-web" / "evidence" / "r6-first-baseline.md"

    corpus = real_web_corpus.load_corpus(corpus_path, root=root)
    execution = load_json(execution_path)
    committed_baseline = load_json(baseline_path)

    measured_commit = _require_commit(
        committed_baseline.get("rarog_commit"), "real-Web measured Rarog commit"
    )
    platform = _require_object(
        committed_baseline.get("platform"), "real-Web baseline platform"
    )
    rebuilt_baseline = real_web_baseline.build_baseline(
        corpus,
        execution,
        rarog_commit=measured_commit,
        platform=platform,
    )
    if rebuilt_baseline != committed_baseline:
        raise ProfileError("committed real-Web baseline JSON does not reproduce exactly")
    if real_web_baseline.render_markdown(rebuilt_baseline) != _read_text(
        baseline_md_path, "real-Web baseline Markdown"
    ):
        raise ProfileError("committed real-Web baseline Markdown does not reproduce exactly")

    scenario_outcomes = [
        {
            "scenario_id": result["scenario_id"],
            "category": result["outcome"]["category"],
            "detail": result["outcome"]["detail"],
        }
        for result in rebuilt_baseline["results"]
    ]
    dependency_states = sorted(
        [
            {
                "scenario_id": result["scenario_id"],
                "origin": dependency["origin"],
                "role": dependency["role"],
                "state": dependency["state"],
            }
            for result in rebuilt_baseline["results"]
            for dependency in result["dependencies"]
        ],
        key=lambda item: (item["scenario_id"], item["origin"], item["role"]),
    )

    return {
        "id": "real-web-first-baseline",
        "kind": "real-web",
        "measured_rarog_commit": measured_commit,
        "source": {
            "corpus_revision": rebuilt_baseline["corpus_revision"],
            "corpus_sha256": rebuilt_baseline["corpus_sha256"],
        },
        "raw_evidence": {
            "path": "real-web/evidence/r6-first-execution.json",
            "sha256": rebuilt_baseline["execution_sha256"],
        },
        "normalized_evidence": {
            "path": "real-web/evidence/r6-first-baseline.json",
            "sha256": canonical_digest(rebuilt_baseline),
        },
        "platform": rebuilt_baseline["platform"],
        "scope": {"scenario_ids": rebuilt_baseline["scenario_ids"]},
        "observed": {
            "scenario_outcomes": scenario_outcomes,
            "dependency_states": dependency_states,
        },
    }


def build_initial_profile(root: Path) -> dict[str, Any]:
    entries = [
        reproduce_real_web_entry(root),
        reproduce_wpt_entry(root),
    ]
    entries.sort(key=lambda item: item["id"])
    return validate_profile(
        {
            "schema_version": SCHEMA_VERSION,
            "profile_revision": PROFILE_REVISION,
            "evidence": entries,
        }
    )


def _md(value: Any) -> str:
    return str(value).replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def render_markdown(profile: dict[str, Any]) -> str:
    profile = validate_profile(profile)
    lines = [
        "# Rarog compatibility profile",
        "",
        f"- Schema version: {profile['schema_version']}",
        f"- Profile revision: {profile['profile_revision']}",
        f"- Evidence sets: {len(profile['evidence'])}",
        "",
        "> Evidence entries are historical measurements bound to their own exact Rarog commits.",
        "> The profile revision does not imply that all evidence was measured on one current engine commit.",
        "",
        "No aggregate compatibility score or cross-kind status is computed.",
        "",
        "## Evidence index",
        "",
        "| ID | Kind | Measured Rarog commit | Platform | Raw digest | Normalized digest |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for entry in profile["evidence"]:
        platform = entry["platform"]["environment"]
        lines.append(
            "| "
            + " | ".join(
                [
                    _md(entry["id"]),
                    _md(entry["kind"]),
                    f"`{entry['measured_rarog_commit']}`",
                    _md(platform),
                    f"`{entry['raw_evidence']['sha256']}`",
                    f"`{entry['normalized_evidence']['sha256']}`",
                ]
            )
            + " |"
        )

    for entry in profile["evidence"]:
        lines.extend(["", f"## {_md(entry['id'])}", ""])
        if entry["kind"] == "wpt":
            lines.extend(
                [
                    f"- Upstream WPT: `{entry['source']['commit']}`",
                    f"- Selection: `{entry['scope']['selection_sha256']}`",
                    f"- Measured tests: {entry['observed']['measured_tests']}",
                    f"- Unexpected tests: {entry['observed']['unexpected_tests']}",
                    "",
                    "### WPT observed statuses",
                    "",
                    "| Status | Count |",
                    "| --- | ---: |",
                ]
            )
            for status, count in entry["observed"]["test_statuses"].items():
                lines.append(f"| {_md(status)} | {count} |")
        else:
            lines.extend(
                [
                    f"- Corpus revision: {entry['source']['corpus_revision']}",
                    f"- Corpus digest: `{entry['source']['corpus_sha256']}`",
                    f"- Scenarios: {len(entry['scope']['scenario_ids'])}",
                    "",
                    "### Real-Web scenario outcomes",
                    "",
                    "| Scenario | Category | Detail |",
                    "| --- | --- | --- |",
                ]
            )
            for item in entry["observed"]["scenario_outcomes"]:
                lines.append(
                    f"| {_md(item['scenario_id'])} | {_md(item['category'])} | {_md(item['detail'])} |"
                )
            lines.extend(
                [
                    "",
                    "### Real-Web dependency states",
                    "",
                    "| Scenario | Origin | Role | State |",
                    "| --- | --- | --- | --- |",
                ]
            )
            for item in entry["observed"]["dependency_states"]:
                lines.append(
                    "| "
                    + " | ".join(
                        [
                            _md(item["scenario_id"]),
                            _md(item["origin"]),
                            _md(item["role"]),
                            _md(item["state"]),
                        ]
                    )
                    + " |"
                )
    return "\n".join(lines).rstrip() + "\n"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--json-out", required=True, type=Path)
    parser.add_argument("--markdown-out", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        profile = build_initial_profile(args.root.resolve())
        args.json_out.write_text(
            json.dumps(profile, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        args.markdown_out.write_text(render_markdown(profile), encoding="utf-8")
    except (
        ProfileError,
        real_web_baseline.BaselineError,
        real_web_corpus.CorpusError,
        wpt_dashboard.DashboardError,
        wpt_evidence.EvidenceError,
        wpt_selection.SelectionError,
    ) as error:
        print(f"compat-profile: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
