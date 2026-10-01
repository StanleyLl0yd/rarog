#!/usr/bin/env python3
"""Bind a full real-Web execution to the exact R6 corpus and result contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, Sequence

import real_web_corpus
import real_web_result

SCHEMA_VERSION = 1
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


class BaselineError(ValueError):
    """Raised when a corpus execution is incomplete or internally inconsistent."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise BaselineError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise BaselineError(f"non-finite JSON number is not allowed: {value}")


def _load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise BaselineError(f"{path}: cannot read JSON: {error}") from error


def _digest(document: Any) -> str:
    canonical = json.dumps(
        document,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(canonical).hexdigest()


def _require_commit(value: str) -> str:
    if not _COMMIT_RE.fullmatch(value):
        raise BaselineError("Rarog commit must be an exact lowercase 40-hex commit")
    return value


def build_baseline(
    corpus: dict[str, Any],
    execution: Any,
    *,
    rarog_commit: str,
    platform: dict[str, str],
) -> dict[str, Any]:
    rarog_commit = _require_commit(rarog_commit)
    if not isinstance(execution, dict):
        raise BaselineError("execution root must be an object")
    expected_keys = {
        "schema_version",
        "corpus_revision",
        "scenario_ids",
        "attempts",
    }
    if set(execution) != expected_keys:
        raise BaselineError(
            "execution has invalid keys; "
            f"missing={sorted(expected_keys - set(execution))}, "
            f"extra={sorted(set(execution) - expected_keys)}"
        )
    if execution["schema_version"] != SCHEMA_VERSION:
        raise BaselineError(f"execution schema_version must be {SCHEMA_VERSION}")
    if execution["corpus_revision"] != corpus["corpus_revision"]:
        raise BaselineError("execution corpus_revision does not match corpus")

    expected_ids = [scenario["id"] for scenario in corpus["scenarios"]]
    scenario_ids = execution["scenario_ids"]
    if not isinstance(scenario_ids, list) or scenario_ids != expected_ids:
        raise BaselineError(
            f"execution scenario_ids must exactly match corpus order {expected_ids}"
        )
    if len(set(scenario_ids)) != len(scenario_ids):
        raise BaselineError("execution scenario_ids contain duplicates")

    attempts = execution["attempts"]
    if not isinstance(attempts, list) or len(attempts) != len(expected_ids):
        raise BaselineError("execution attempts must cover every corpus scenario exactly once")

    observed_ids: list[str] = []
    results: list[dict[str, Any]] = []
    for index, attempt in enumerate(attempts):
        if not isinstance(attempt, dict):
            raise BaselineError(f"execution attempt {index} must be an object")
        scenario_id = attempt.get("scenario_id")
        if scenario_id != expected_ids[index]:
            raise BaselineError(
                f"execution attempt {index} must be scenario {expected_ids[index]!r}"
            )
        observed_ids.append(scenario_id)
        try:
            result = real_web_result.normalize_attempt(
                corpus,
                attempt,
                rarog_commit=rarog_commit,
                platform=platform,
            )
        except real_web_result.ResultError as error:
            raise BaselineError(str(error)) from error
        results.append(result)

    if observed_ids != expected_ids or len(set(observed_ids)) != len(observed_ids):
        raise BaselineError("execution attempts do not match the exact corpus denominator")

    corpus_sha256 = real_web_result._canonical_digest(corpus)
    if any(result["corpus_sha256"] != corpus_sha256 for result in results):
        raise BaselineError("normalized result corpus digest mismatch")

    return {
        "schema_version": SCHEMA_VERSION,
        "rarog_commit": rarog_commit,
        "corpus_sha256": corpus_sha256,
        "corpus_revision": corpus["corpus_revision"],
        "execution_sha256": _digest(execution),
        "platform": results[0]["platform"] if results else platform,
        "scenario_ids": expected_ids,
        "results": results,
    }


def _md(value: Any) -> str:
    text = str(value).replace("\n", " ")
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("|", "\\|")
        .replace(chr(96), "\\" + chr(96))
    )


def render_markdown(baseline: dict[str, Any]) -> str:
    platform = baseline["platform"]
    lines = [
        "# Rarog real-Web corpus baseline",
        "",
        f"- Rarog commit: {baseline['rarog_commit']}",
        f"- Corpus: {baseline['corpus_sha256']} "
        f"(revision {baseline['corpus_revision']})",
        f"- Raw execution: {baseline['execution_sha256']}",
        f"- Platform: {_md(platform['os'])}/{_md(platform['arch'])} — "
        f"{_md(platform['environment'])}",
        f"- Measured scenarios: {len(baseline['results'])}",
        "",
        "This is a bounded corpus baseline. It is not a general-Web compatibility score.",
        "",
        "| Scenario | Input | Outcome | Detail |",
        "| --- | --- | --- | --- |",
    ]
    for result in baseline["results"]:
        lines.append(
            "| "
            + " | ".join(
                [
                    _md(result["scenario_id"]),
                    _md(result["input"]["mode"]),
                    _md(result["outcome"]["category"]),
                    _md(result["outcome"]["detail"]),
                ]
            )
            + " |"
        )

    lines.extend(["", "## Scenario evidence", ""])
    for result in baseline["results"]:
        lines.extend(
            [
                f"### {_md(result['scenario_id'])}",
                "",
                real_web_result.render_markdown(result).strip(),
                "",
            ]
        )
    return "\n".join(lines).rstrip() + "\n"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--execution", required=True, type=Path)
    parser.add_argument("--rarog-commit", required=True)
    parser.add_argument("--platform-os", required=True)
    parser.add_argument("--platform-arch", required=True)
    parser.add_argument("--environment", required=True)
    parser.add_argument("--json-out", required=True, type=Path)
    parser.add_argument("--markdown-out", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        corpus = real_web_corpus.load_corpus(
            args.manifest,
            root=args.root.resolve(),
        )
        baseline = build_baseline(
            corpus,
            _load_json(args.execution),
            rarog_commit=args.rarog_commit,
            platform={
                "os": args.platform_os,
                "arch": args.platform_arch,
                "environment": args.environment,
            },
        )
        args.json_out.write_text(
            json.dumps(baseline, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        args.markdown_out.write_text(
            render_markdown(baseline),
            encoding="utf-8",
        )
    except (BaselineError, real_web_corpus.CorpusError) as error:
        print(f"real-web-baseline: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
