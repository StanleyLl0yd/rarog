#!/usr/bin/env python3
"""Independently verify the canonical R6 profile against committed source evidence.

This module deliberately never imports the compatibility-profile generator.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import posixpath
import re
import sys
from pathlib import Path
from typing import Any, Sequence

import real_web_baseline
import real_web_corpus
import wpt_dashboard
import wpt_evidence
import wpt_selection

_COMMIT = re.compile(r"^[0-9a-f]{40}$")
_SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
_ID = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_SCHEMA = 1
_ARTIFACT_KEYS = {"path", "sha256"}
_ENTRY_KEYS = {
    "id", "kind", "measured_rarog_commit", "source", "raw_evidence",
    "normalized_evidence", "platform", "scope", "observed",
}


class VerificationError(ValueError):
    """The supplied profile or any of its committed sources is not reproducible."""


def _duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    output: dict[str, Any] = {}
    for key, value in pairs:
        if key in output:
            raise VerificationError(f"duplicate JSON key {key!r}")
        output[key] = value
    return output


def _nonfinite(value: str) -> None:
    raise VerificationError(f"non-finite JSON number {value!r}")


def _read_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_duplicate_pairs,
                parse_constant=_nonfinite,
            )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise VerificationError(f"{path}: cannot load strict JSON: {error}") from error


def _read_bytes(path: Path) -> bytes:
    try:
        return path.read_bytes()
    except OSError as error:
        raise VerificationError(f"{path}: cannot read source: {error}") from error


def _canonical(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def _digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(_canonical(value)).hexdigest()


def _pretty(value: Any) -> bytes:
    return (
        json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False)
        + "\n"
    ).encode("utf-8")


def _object(value: Any, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise VerificationError(f"{label}: invalid object/unknown or missing keys")
    return value


def _text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise VerificationError(f"{label}: non-empty string required")
    return value


def _commit(value: Any, label: str) -> str:
    value = _text(value, label)
    if not _COMMIT.fullmatch(value):
        raise VerificationError(f"{label}: invalid commit")
    return value


def _sha(value: Any, label: str) -> str:
    value = _text(value, label)
    if not _SHA256.fullmatch(value):
        raise VerificationError(f"{label}: invalid sha256")
    return value


def _positive(value: Any, label: str) -> int:
    if type(value) is not int or value < 1:
        raise VerificationError(f"{label}: positive integer required")
    return value


def _resolved_source(root: Path, name: str, prefix: str) -> Path:
    name = _text(name, "artifact path")
    if (
        not name.startswith(prefix) or name.startswith("/")
        or "\\" in name or name.endswith("/")
        or posixpath.normpath(name) != name
    ):
        raise VerificationError(f"artifact path not normalized under {prefix}: {name}")
    relative = Path(*name.split("/"))
    cursor = root
    for part in relative.parts:
        cursor = cursor / part
        if cursor.is_symlink():
            raise VerificationError(f"artifact path traverses a symlink: {name}")
    try:
        path = cursor.resolve(strict=True)
        path.relative_to(root)
    except (OSError, ValueError) as error:
        raise VerificationError(f"artifact outside checkout or missing: {name}") from error
    if not path.is_file():
        raise VerificationError(f"artifact is not a regular file: {name}")
    return path


def _artifact(root: Path, raw: Any, prefix: str, label: str) -> tuple[dict[str, str], Any]:
    record = _object(raw, label, _ARTIFACT_KEYS)
    path = _text(record["path"], f"{label} path")
    expected = _sha(record["sha256"], f"{label} digest")
    document = _read_json(_resolved_source(root, path, prefix))
    observed = _digest(document)
    if expected != observed:
        raise VerificationError(f"{label}: artifact digest mismatch")
    return {"path": path, "sha256": expected}, document


def _same(actual: Any, expected: Any, label: str) -> None:
    if _canonical(actual) != _canonical(expected):
        raise VerificationError(f"{label}: independent reproduction mismatch")


def _wpt(root: Path, entry: dict[str, Any]) -> dict[str, Any]:
    source = _object(entry["source"], "WPT source", {"repository", "commit"})
    if source["repository"] != "web-platform-tests/wpt":
        raise VerificationError("WPT source repository mismatch")
    upstream = _commit(source["commit"], "WPT upstream")

    selection = wpt_selection.load_manifest(
        _resolved_source(root, "wpt/r6-selection.json", "wpt/")
    )
    if selection["source"] != source:
        raise VerificationError("WPT selection source mismatch")
    measured = _commit(entry["measured_rarog_commit"], "WPT measured commit")
    raw_ref, raw = _artifact(
        root, entry["raw_evidence"], "wpt/evidence/", "WPT raw"
    )
    if raw_ref["path"] != "wpt/evidence/r6-first-wptreport.json":
        raise VerificationError("WPT revision-1 raw artifact path mismatch")
    envelope = wpt_evidence.build_evidence(
        selection, raw, rarog_commit=measured, wpt_commit=upstream,
    )
    stored_envelope = _read_json(
        _resolved_source(root, "wpt/evidence/r6-first-evidence.json", "wpt/evidence/")
    )
    _same(stored_envelope, envelope, "WPT evidence envelope")
    if raw_ref["sha256"] != envelope["report_sha256"]:
        raise VerificationError("WPT report digest mismatch")

    normalized_ref, normalized = _artifact(
        root, entry["normalized_evidence"], "wpt/evidence/", "WPT dashboard"
    )
    if normalized_ref["path"] != "wpt/evidence/r6-first-dashboard.json":
        raise VerificationError("WPT revision-1 normalized artifact path mismatch")
    platform = _object(entry["platform"], "WPT platform", {"environment"})
    environment = _text(platform["environment"], "WPT platform environment")
    rebuilt = wpt_dashboard.normalize_reports(
        [(envelope["report_sha256"], raw)],
        rarog_commit=measured, wpt_commit=upstream,
        platform=environment, synthetic=False,
    )
    _same(normalized, rebuilt, "WPT dashboard")
    markdown_path = _resolved_source(
        root, "wpt/evidence/r6-first-dashboard.md", "wpt/evidence/"
    )
    if _read_bytes(markdown_path) != wpt_dashboard.render_markdown(rebuilt).encode("utf-8"):
        raise VerificationError("WPT dashboard Markdown mismatch")
    if rebuilt["synthetic"]:
        raise VerificationError("synthetic WPT report cannot be profiled")
    return {
        "id": "wpt-first-selected-baseline",
        "kind": "wpt",
        "measured_rarog_commit": measured,
        "source": source,
        "raw_evidence": raw_ref,
        "normalized_evidence": normalized_ref,
        "platform": platform,
        "scope": {
            "selection_sha256": envelope["selection_sha256"],
            "test_ids": envelope["test_ids"],
        },
        "observed": rebuilt["summary"],
    }


def _real_web(root: Path, entry: dict[str, Any]) -> dict[str, Any]:
    measured = _commit(entry["measured_rarog_commit"], "real-Web measured commit")
    platform = _object(
        entry["platform"], "real-Web platform", {"os", "arch", "environment"}
    )
    for field, value in platform.items():
        _text(value, f"real-Web platform {field}")
    corpus = real_web_corpus.load_corpus(
        _resolved_source(root, "real-web/corpus.json", "real-web/"), root=root
    )
    raw_ref, raw = _artifact(
        root, entry["raw_evidence"], "real-web/evidence/", "real-Web raw"
    )
    if raw_ref["path"] != "real-web/evidence/r6-first-execution.json":
        raise VerificationError("real-Web revision-1 raw artifact path mismatch")
    baseline = real_web_baseline.build_baseline(
        corpus, raw, rarog_commit=measured, platform=platform
    )
    normalized_ref, normalized = _artifact(
        root, entry["normalized_evidence"], "real-web/evidence/", "real-Web baseline"
    )
    if normalized_ref["path"] != "real-web/evidence/r6-first-baseline.json":
        raise VerificationError("real-Web revision-1 normalized artifact path mismatch")
    _same(normalized, baseline, "real-Web baseline")
    if raw_ref["sha256"] != baseline["execution_sha256"]:
        raise VerificationError("real-Web execution digest mismatch")
    markdown_path = _resolved_source(
        root, "real-web/evidence/r6-first-baseline.md", "real-web/evidence/"
    )
    if _read_bytes(markdown_path) != real_web_baseline.render_markdown(baseline).encode("utf-8"):
        raise VerificationError("real-Web baseline Markdown mismatch")

    outcomes = [
        {
            "scenario_id": result["scenario_id"],
            "category": result["outcome"]["category"],
            "detail": result["outcome"]["detail"],
        }
        for result in baseline["results"]
    ]
    dependencies = sorted(
        (
            {
                "scenario_id": result["scenario_id"],
                "origin": dep["origin"],
                "role": dep["role"],
                "state": dep["state"],
            }
            for result in baseline["results"]
            for dep in result["dependencies"]
        ),
        key=lambda item: (item["scenario_id"], item["origin"], item["role"]),
    )
    return {
        "id": "real-web-first-baseline",
        "kind": "real-web",
        "measured_rarog_commit": measured,
        "source": {
            "corpus_revision": baseline["corpus_revision"],
            "corpus_sha256": baseline["corpus_sha256"],
        },
        "raw_evidence": raw_ref,
        "normalized_evidence": normalized_ref,
        "platform": platform,
        "scope": {"scenario_ids": baseline["scenario_ids"]},
        "observed": {
            "scenario_outcomes": outcomes,
            "dependency_states": dependencies,
        },
    }


def _md(value: Any) -> str:
    return (
        str(value).replace("\n", " ").replace("&", "&amp;")
        .replace("<", "&lt;").replace(">", "&gt;")
        .replace("\\", "\\\\").replace("|", "\\|").replace("`", "\\`")
    )


def _render_markdown(profile: dict[str, Any]) -> str:
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
    for item in profile["evidence"]:
        lines.append(
            "| " + " | ".join([
                _md(item["id"]), _md(item["kind"]),
                f"`{item['measured_rarog_commit']}`",
                _md(item["platform"]["environment"]),
                f"`{item['raw_evidence']['sha256']}`",
                f"`{item['normalized_evidence']['sha256']}`",
            ]) + " |"
        )
    for item in profile["evidence"]:
        lines.extend(["", f"## {_md(item['id'])}", ""])
        if item["kind"] == "wpt":
            lines.extend([
                f"- Upstream WPT: `{item['source']['commit']}`",
                f"- Selection: `{item['scope']['selection_sha256']}`",
                f"- Measured tests: {item['observed']['measured_tests']}",
                f"- Unexpected tests: {item['observed']['unexpected_tests']}",
                "",
                "### WPT observed statuses",
                "",
                "| Status | Count |",
                "| --- | ---: |",
            ])
            for status, count in item["observed"]["test_statuses"].items():
                lines.append(f"| {_md(status)} | {count} |")
        else:
            lines.extend([
                f"- Corpus revision: {item['source']['corpus_revision']}",
                f"- Corpus digest: `{item['source']['corpus_sha256']}`",
                f"- Scenarios: {len(item['scope']['scenario_ids'])}",
                "",
                "### Real-Web scenario outcomes",
                "",
                "| Scenario | Category | Detail |",
                "| --- | --- | --- |",
            ])
            for outcome in item["observed"]["scenario_outcomes"]:
                lines.append(
                    f"| {_md(outcome['scenario_id'])} | {_md(outcome['category'])} | {_md(outcome['detail'])} |"
                )
            lines.extend([
                "",
                "### Real-Web dependency states",
                "",
                "| Scenario | Origin | Role | State |",
                "| --- | --- | --- | --- |",
            ])
            for dep in item["observed"]["dependency_states"]:
                lines.append(
                    "| " + " | ".join([
                        _md(dep["scenario_id"]), _md(dep["origin"]),
                        _md(dep["role"]), _md(dep["state"]),
                    ]) + " |"
                )
    return "\n".join(lines).rstrip() + "\n"


def verify(root: Path) -> dict[str, Any]:
    root = root.resolve(strict=True)
    source = _resolved_source(root, "compatibility/profile.json", "compatibility/")
    profile = _read_json(source)
    profile = _object(
        profile, "profile", {"schema_version", "profile_revision", "evidence"}
    )
    if type(profile["schema_version"]) is not int or profile["schema_version"] != _SCHEMA:
        raise VerificationError("invalid profile schema version")
    _positive(profile["profile_revision"], "profile revision")
    entries = profile["evidence"]
    if not isinstance(entries, list) or not entries:
        raise VerificationError("profile evidence must be non-empty")

    seen_ids: set[str] = set()
    seen_identity: set[tuple[str, str, str, str]] = set()
    verified: list[dict[str, Any]] = []
    for item in entries:
        item = _object(item, "profile evidence", _ENTRY_KEYS)
        name = _text(item["id"], "evidence id")
        if not _ID.fullmatch(name) or name in seen_ids:
            raise VerificationError("duplicate or malformed evidence id")
        seen_ids.add(name)
        kind = _text(item["kind"], "evidence kind")
        if kind == "wpt" and name == "wpt-first-selected-baseline":
            reproduced = _wpt(root, item)
        elif kind == "real-web" and name == "real-web-first-baseline":
            reproduced = _real_web(root, item)
        else:
            raise VerificationError("unsupported evidence kind or source identity")
        _same(item, reproduced, f"{name} independently reconstructed entry")
        identity = (
            kind, item["measured_rarog_commit"],
            item["raw_evidence"]["sha256"],
            item["normalized_evidence"]["sha256"],
        )
        if identity in seen_identity:
            raise VerificationError("duplicate evidence identity")
        seen_identity.add(identity)
        verified.append(reproduced)
    if [item["id"] for item in verified] != sorted(seen_ids):
        raise VerificationError("evidence ordering drift")
    # Revision 1 has exactly the two historical independently reproducible inputs.
    if set(seen_ids) != {"wpt-first-selected-baseline", "real-web-first-baseline"}:
        raise VerificationError("profile revision 1 requires both historical evidence sets")
    if profile["profile_revision"] != 1:
        raise VerificationError("unsupported profile revision")

    expected = {
        "schema_version": _SCHEMA,
        "profile_revision": 1,
        "evidence": verified,
    }
    _same(profile, expected, "canonical profile")
    if _read_bytes(source) != _pretty(expected):
        raise VerificationError("committed profile JSON bytes are not canonical")
    markdown = _render_markdown(expected).encode("utf-8")
    actual_markdown = _read_bytes(
        _resolved_source(root, "compatibility/profile.md", "compatibility/")
    )
    if actual_markdown != markdown:
        raise VerificationError("committed profile Markdown does not reproduce independently")

    payload_digest = _digest(expected)
    envelope = {
        "schema_version": 1,
        "profile_revision": 1,
        "profile_sha256": payload_digest,
        "canonical_payload_bytes": len(_canonical(expected)),
        "evidence_count": len(verified),
        "evidence_ids": [item["id"] for item in verified],
        "measured_rarog_commits": sorted(
            {item["measured_rarog_commit"] for item in verified}
        ),
    }
    published = _resolved_source(
        root, "compatibility/verification.json", "compatibility/"
    )
    if _read_bytes(published) != _pretty(envelope):
        raise VerificationError("published verification envelope mismatch")
    return envelope


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args(argv)
    try:
        envelope = verify(args.root)
        output = _pretty(envelope)
        if args.json_out is None:
            sys.stdout.buffer.write(output)
        else:
            args.json_out.write_bytes(output)
    except (
        VerificationError,
        wpt_selection.SelectionError,
        wpt_evidence.EvidenceError,
        wpt_dashboard.DashboardError,
        real_web_baseline.BaselineError,
        real_web_corpus.CorpusError,
    ) as error:
        print(f"profile-verification: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
