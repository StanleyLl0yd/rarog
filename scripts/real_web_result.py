#!/usr/bin/env python3
"""Normalize bounded R6 real-Web execution attempts into deterministic evidence."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import re
import sys
from pathlib import Path
from typing import Any, Sequence

import real_web_corpus

SCHEMA_VERSION = 1
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
_SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
_MAX_DIAGNOSTIC_BYTES = 4096
_MAX_TEXT_BYTES = 4096

_DETAILS_BY_CATEGORY = {
    "completed-observation": {"completed"},
    "unsupported-capability": {"unsupported-capability"},
    "engine-failure": {
        "navigation-failure",
        "render-failure",
        "script-failure",
        "resource-limit-exceeded",
        "timeout-limit-exceeded",
    },
    "external-unavailable": {
        "dns-failure",
        "tls-failure",
        "http-unavailable",
        "redirect-policy-violation",
        "resource-limit-exceeded",
        "timeout-limit-exceeded",
    },
    "external-content-drift": {"content-drift"},
}
_DEPENDENCY_STATES = {
    "available",
    "not-attempted",
    "dns-failure",
    "tls-failure",
    "http-unavailable",
    "redirect-policy-violation",
    "content-drift",
    "resource-limit-exceeded",
    "timeout-limit-exceeded",
}
_DETAIL_TO_DEPENDENCY_STATE = {
    "dns-failure": "dns-failure",
    "tls-failure": "tls-failure",
    "http-unavailable": "http-unavailable",
    "redirect-policy-violation": "redirect-policy-violation",
    "content-drift": "content-drift",
    "resource-limit-exceeded": "resource-limit-exceeded",
    "timeout-limit-exceeded": "timeout-limit-exceeded",
}


class ResultError(ValueError):
    """Raised when real-Web execution evidence is malformed or ambiguous."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ResultError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise ResultError(f"non-finite JSON number is not allowed: {value}")


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ResultError(f"{label} must be an object")
    return value


def _require_exact_keys(value: dict[str, Any], keys: set[str], label: str) -> None:
    actual = set(value)
    if actual != keys:
        raise ResultError(
            f"{label} has invalid keys; "
            f"missing={sorted(keys - actual)}, extra={sorted(actual - keys)}"
        )


def _require_text(
    value: Any,
    label: str,
    *,
    maximum_bytes: int = _MAX_TEXT_BYTES,
) -> str:
    if not isinstance(value, str) or not value:
        raise ResultError(f"{label} must be a non-empty string")
    if len(value.encode("utf-8")) > maximum_bytes:
        raise ResultError(f"{label} exceeds {maximum_bytes} UTF-8 bytes")
    if "\x00" in value:
        raise ResultError(f"{label} must not contain NUL")
    return value


def _require_commit(value: Any, label: str) -> str:
    text = _require_text(value, label, maximum_bytes=40)
    if not _COMMIT_RE.fullmatch(text):
        raise ResultError(f"{label} must be an exact lowercase 40-hex commit")
    return text


def _require_sha256(value: Any, label: str) -> str:
    text = _require_text(value, label, maximum_bytes=71)
    if not _SHA256_RE.fullmatch(text):
        raise ResultError(f"{label} must be sha256:<64 lowercase hex>")
    return text


def _require_nullable_sha256(value: Any, label: str) -> str | None:
    if value is None:
        return None
    return _require_sha256(value, label)


def _require_http_status(value: Any, label: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or not 100 <= value <= 599:
        raise ResultError(f"{label} must be null or an integer HTTP status 100..599")
    return value


def _canonical_digest(document: Any) -> str:
    encoded = json.dumps(
        document,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def _require_public_address(value: Any, label: str) -> str:
    text = _require_text(value, label, maximum_bytes=64)
    try:
        address = ipaddress.ip_address(text)
    except ValueError as error:
        raise ResultError(f"{label} must be an IP address") from error
    if (
        address.is_private
        or address.is_loopback
        or address.is_link_local
        or address.is_multicast
        or address.is_reserved
        or address.is_unspecified
    ):
        raise ResultError(f"{label} must be a public IP address")
    canonical = str(address)
    if text != canonical:
        raise ResultError(f"{label} must be canonical: {canonical}")
    return canonical


def _validate_platform(raw: Any) -> dict[str, str]:
    value = _require_object(raw, "platform")
    _require_exact_keys(value, {"os", "arch", "environment"}, "platform")
    return {
        "os": _require_text(value["os"], "platform os", maximum_bytes=128),
        "arch": _require_text(value["arch"], "platform arch", maximum_bytes=128),
        "environment": _require_text(
            value["environment"], "platform environment", maximum_bytes=256
        ),
    }


def _validate_outcome(raw: Any, corpus_categories: list[str]) -> dict[str, Any]:
    value = _require_object(raw, "outcome")
    _require_exact_keys(value, {"category", "detail", "diagnostic"}, "outcome")
    category = _require_text(value["category"], "outcome category", maximum_bytes=64)
    detail = _require_text(value["detail"], "outcome detail", maximum_bytes=64)

    if category not in corpus_categories:
        raise ResultError(f"outcome category {category!r} is not declared by the corpus")
    allowed = _DETAILS_BY_CATEGORY.get(category)
    if allowed is None or detail not in allowed:
        raise ResultError(
            f"outcome detail {detail!r} is invalid for category {category!r}"
        )

    diagnostic = value["diagnostic"]
    if category == "completed-observation":
        if diagnostic is not None:
            raise ResultError("completed observation must have null diagnostic")
    else:
        diagnostic = _require_text(
            diagnostic,
            "outcome diagnostic",
            maximum_bytes=_MAX_DIAGNOSTIC_BYTES,
        )

    return {
        "category": category,
        "detail": detail,
        "diagnostic": diagnostic,
    }


def _validate_addresses(raw: Any, label: str) -> list[str]:
    if not isinstance(raw, list):
        raise ResultError(f"{label} must be a list")
    values = [
        _require_public_address(item, f"{label} item {index}")
        for index, item in enumerate(raw)
    ]
    if values != sorted(values) or len(values) != len(set(values)):
        raise ResultError(f"{label} must be unique and sorted")
    return values


def _validate_dependency_record(
    raw: Any,
    *,
    declared: dict[str, Any],
    scenario_id: str,
) -> dict[str, Any]:
    value = _require_object(raw, f"{scenario_id}: dependency result")
    _require_exact_keys(
        value,
        {
            "origin",
            "role",
            "state",
            "resolved_addresses",
            "http_status",
            "content_sha256",
            "expected_content_sha256",
        },
        f"{scenario_id}: dependency result",
    )
    origin = _require_text(value["origin"], f"{scenario_id}: dependency origin")
    role = _require_text(value["role"], f"{scenario_id}: dependency role")
    if (origin, role) != (declared["origin"], declared["role"]):
        raise ResultError(
            f"{scenario_id}: dependency result identity {(origin, role)!r} "
            f"does not match declared {(declared['origin'], declared['role'])!r}"
        )

    state = _require_text(value["state"], f"{scenario_id}: dependency state")
    if state not in _DEPENDENCY_STATES:
        raise ResultError(f"{scenario_id}: unsupported dependency state {state!r}")
    addresses = _validate_addresses(
        value["resolved_addresses"],
        f"{scenario_id}: {origin} resolved_addresses",
    )
    http_status = _require_http_status(
        value["http_status"], f"{scenario_id}: {origin} http_status"
    )
    content = _require_nullable_sha256(
        value["content_sha256"], f"{scenario_id}: {origin} content_sha256"
    )
    expected = _require_nullable_sha256(
        value["expected_content_sha256"],
        f"{scenario_id}: {origin} expected_content_sha256",
    )

    if state in {"not-attempted", "dns-failure"}:
        if addresses or http_status is not None or content is not None or expected is not None:
            raise ResultError(
                f"{scenario_id}: {state} dependency must not claim addresses, "
                "HTTP status or content identity"
            )
    elif state == "tls-failure":
        if not addresses or http_status is not None or content is not None or expected is not None:
            raise ResultError(
                f"{scenario_id}: tls-failure requires public resolved addresses only"
            )
    elif state == "http-unavailable":
        if (
            not addresses
            or http_status is None
            or not 400 <= http_status <= 599
            or content is not None
            or expected is not None
        ):
            raise ResultError(
                f"{scenario_id}: http-unavailable requires addresses and HTTP 4xx/5xx only"
            )
    elif state == "redirect-policy-violation":
        if (
            http_status is None
            or not 300 <= http_status <= 399
            or content is not None
            or expected is not None
        ):
            raise ResultError(
                f"{scenario_id}: redirect-policy-violation requires HTTP 3xx and no content identity"
            )
    elif state == "timeout-limit-exceeded":
        if http_status is not None or content is not None or expected is not None:
            raise ResultError(
                f"{scenario_id}: timeout-limit-exceeded must not claim HTTP or content identity"
            )
    elif state == "resource-limit-exceeded":
        if (
            not addresses
            or http_status is None
            or content is not None
            or expected is not None
        ):
            raise ResultError(
                f"{scenario_id}: resource-limit-exceeded requires resolved addresses and HTTP status only"
            )
    elif state == "available":
        if (
            not addresses
            or http_status is None
            or not 200 <= http_status <= 299
            or content is None
            or expected is not None
        ):
            raise ResultError(
                f"{scenario_id}: available dependency requires addresses, HTTP 2xx and content sha256"
            )
    elif state == "content-drift":
        if (
            not addresses
            or http_status is None
            or not 200 <= http_status <= 299
            or content is None
            or expected is None
            or content == expected
        ):
            raise ResultError(
                f"{scenario_id}: content-drift requires addresses, HTTP 2xx and distinct expected/observed digests"
            )

    return {
        "origin": origin,
        "role": role,
        "required": declared["required"],
        "state": state,
        "resolved_addresses": addresses,
        "http_status": http_status,
        "content_sha256": content,
        "expected_content_sha256": expected,
    }


def _validate_dependencies(
    raw: Any,
    *,
    scenario: dict[str, Any],
) -> list[dict[str, Any]]:
    scenario_id = scenario["id"]
    if not isinstance(raw, list):
        raise ResultError(f"{scenario_id}: dependency results must be a list")

    declared = scenario["external_dependencies"]
    if len(raw) != len(declared):
        raise ResultError(
            f"{scenario_id}: dependency results must cover every declared dependency"
        )

    normalized = [
        _validate_dependency_record(item, declared=declared[index], scenario_id=scenario_id)
        for index, item in enumerate(raw)
    ]
    identities = [(item["origin"], item["role"]) for item in normalized]
    declared_identities = [(item["origin"], item["role"]) for item in declared]
    if identities != declared_identities:
        raise ResultError(
            f"{scenario_id}: dependency result ordering must match the corpus"
        )
    return normalized


def _validate_observation_value(kind: str, value: Any, scenario_id: str) -> Any:
    label = f"{scenario_id}: {kind} observation value"
    if kind == "document-title":
        return _require_text(value, label)
    if kind == "final-url":
        try:
            return real_web_corpus._canonical_https_url(value, label)
        except real_web_corpus.CorpusError as error:
            raise ResultError(str(error)) from error
    if kind == "render-completion":
        if not isinstance(value, bool):
            raise ResultError(f"{label} must be boolean")
        return value
    if kind == "screenshot":
        return _require_sha256(value, label)
    raise ResultError(f"{scenario_id}: unsupported observation kind {kind!r}")


def _validate_observations(
    raw: Any,
    *,
    scenario: dict[str, Any],
) -> list[dict[str, Any]]:
    scenario_id = scenario["id"]
    if not isinstance(raw, list):
        raise ResultError(f"{scenario_id}: observations must be a list")
    declared = scenario["observations"]
    if len(raw) != len(declared):
        raise ResultError(
            f"{scenario_id}: result must contain every declared observation"
        )

    normalized: list[dict[str, Any]] = []
    for index, kind in enumerate(declared):
        value = _require_object(raw[index], f"{scenario_id}: observation result {index}")
        _require_exact_keys(
            value,
            {"kind", "state", "value"},
            f"{scenario_id}: observation result {index}",
        )
        observed_kind = _require_text(
            value["kind"], f"{scenario_id}: observation result {index} kind"
        )
        if observed_kind != kind:
            raise ResultError(
                f"{scenario_id}: observation ordering must match the corpus"
            )
        state = _require_text(
            value["state"], f"{scenario_id}: {kind} observation state"
        )
        if state not in {"observed", "not-observed"}:
            raise ResultError(
                f"{scenario_id}: {kind} observation state must be observed or not-observed"
            )
        if state == "observed":
            observed_value = _validate_observation_value(
                kind, value["value"], scenario_id
            )
        else:
            if value["value"] is not None:
                raise ResultError(
                    f"{scenario_id}: not-observed {kind} must have null value"
                )
            observed_value = None
        normalized.append(
            {"kind": kind, "state": state, "value": observed_value}
        )
    return normalized


def _enforce_final_url_consistency(
    *,
    scenario: dict[str, Any],
    observations: list[dict[str, Any]],
) -> None:
    final_url = next(
        (
            item["value"]
            for item in observations
            if item["kind"] == "final-url" and item["state"] == "observed"
        ),
        None,
    )
    if final_url is None:
        return

    if scenario["input"]["mode"] == "captured-versioned":
        if final_url != scenario["source_url"]:
            raise ResultError(
                f"{scenario['id']}: captured final-url must equal the scenario source_url"
            )
        return

    observed_origin = real_web_corpus._url_origin(final_url)
    declared_origins = {
        item["origin"] for item in scenario["external_dependencies"]
    }
    if observed_origin not in declared_origins:
        raise ResultError(
            f"{scenario['id']}: final-url origin {observed_origin!r} is not a declared dependency"
        )


def _enforce_outcome_consistency(
    *,
    scenario: dict[str, Any],
    outcome: dict[str, Any],
    dependencies: list[dict[str, Any]],
    observations: list[dict[str, Any]],
) -> None:
    scenario_id = scenario["id"]
    category = outcome["category"]
    detail = outcome["detail"]

    if scenario["input"]["mode"] == "captured-versioned" and category in {
        "external-unavailable",
        "external-content-drift",
    }:
        raise ResultError(
            f"{scenario_id}: captured-versioned input cannot have external outcome"
        )

    required_dependencies = [item for item in dependencies if item["required"]]

    if category == "completed-observation":
        if any(item["state"] != "observed" for item in observations):
            raise ResultError(
                f"{scenario_id}: completed observation requires every declared observation"
            )
        if any(item["state"] != "available" for item in required_dependencies):
            raise ResultError(
                f"{scenario_id}: completed live observation requires every required dependency available"
            )
        return

    if category == "unsupported-capability":
        if any(item["state"] != "not-attempted" for item in required_dependencies):
            raise ResultError(
                f"{scenario_id}: unsupported capability must not disguise attempted required network dependencies"
            )
        return

    if category == "engine-failure":
        if any(item["state"] != "available" for item in required_dependencies):
            raise ResultError(
                f"{scenario_id}: engine failure requires required external dependencies to be available"
            )
        return

    if category == "external-content-drift":
        if not any(item["state"] == "content-drift" for item in required_dependencies):
            raise ResultError(
                f"{scenario_id}: external-content-drift requires a required dependency content-drift record"
            )
        return

    if category == "external-unavailable":
        mapped = _DETAIL_TO_DEPENDENCY_STATE.get(detail)
        if mapped is not None:
            if not any(item["state"] == mapped for item in required_dependencies):
                raise ResultError(
                    f"{scenario_id}: {detail} outcome requires a matching required dependency state"
                )
        return


def normalize_attempt(
    corpus: dict[str, Any],
    raw: Any,
    *,
    rarog_commit: str,
    platform: Any,
) -> dict[str, Any]:
    rarog_commit = _require_commit(rarog_commit, "Rarog commit")
    platform_value = _validate_platform(platform)
    attempt = _require_object(raw, "attempt")
    _require_exact_keys(
        attempt,
        {
            "schema_version",
            "scenario_id",
            "outcome",
            "dependencies",
            "observations",
        },
        "attempt",
    )
    if attempt["schema_version"] != SCHEMA_VERSION:
        raise ResultError(f"attempt schema_version must be {SCHEMA_VERSION}")

    scenario_id = _require_text(
        attempt["scenario_id"], "attempt scenario_id", maximum_bytes=128
    )
    matches = [
        scenario for scenario in corpus["scenarios"] if scenario["id"] == scenario_id
    ]
    if len(matches) != 1:
        raise ResultError(f"attempt scenario {scenario_id!r} is not in the corpus")
    scenario = matches[0]

    outcome = _validate_outcome(attempt["outcome"], corpus["outcome_categories"])
    dependencies = _validate_dependencies(
        attempt["dependencies"], scenario=scenario
    )
    observations = _validate_observations(
        attempt["observations"], scenario=scenario
    )
    _enforce_final_url_consistency(
        scenario=scenario,
        observations=observations,
    )
    _enforce_outcome_consistency(
        scenario=scenario,
        outcome=outcome,
        dependencies=dependencies,
        observations=observations,
    )

    return {
        "schema_version": SCHEMA_VERSION,
        "rarog_commit": rarog_commit,
        "corpus_sha256": _canonical_digest(corpus),
        "corpus_revision": corpus["corpus_revision"],
        "platform": platform_value,
        "scenario_id": scenario_id,
        "source_url": scenario["source_url"],
        "input": scenario["input"],
        "outcome": outcome,
        "dependencies": dependencies,
        "observations": observations,
    }


def render_markdown(result: dict[str, Any]) -> str:
    outcome = result["outcome"]
    lines = [
        "# Rarog real-Web scenario result",
        "",
        f"- Rarog commit: `{result['rarog_commit']}`",
        f"- Corpus: `{result['corpus_sha256']}` (revision {result['corpus_revision']})",
        f"- Scenario: `{result['scenario_id']}`",
        f"- Input mode: `{result['input']['mode']}`",
        f"- Platform: `{result['platform']['os']}/{result['platform']['arch']}` — "
        f"`{result['platform']['environment']}`",
        f"- Outcome: `{outcome['category']}` / `{outcome['detail']}`",
        "",
        "This is one bounded scenario observation. It is not a general-Web compatibility score.",
        "",
        "## External dependencies",
        "",
        "| Origin | Role | Required | State | HTTP | Content |",
        "| --- | --- | --- | --- | ---: | --- |",
    ]
    for item in result["dependencies"]:
        lines.append(
            "| "
            + " | ".join(
                [
                    item["origin"],
                    item["role"],
                    "yes" if item["required"] else "no",
                    item["state"],
                    str(item["http_status"]) if item["http_status"] is not None else "-",
                    item["content_sha256"] or "-",
                ]
            )
            + " |"
        )
    if not result["dependencies"]:
        lines.append("| *(offline captured input)* | - | - | - | - | - |")

    lines.extend(
        [
            "",
            "## Observations",
            "",
            "| Observation | State | Value |",
            "| --- | --- | --- |",
        ]
    )
    for item in result["observations"]:
        value = item["value"]
        if isinstance(value, bool):
            rendered = "true" if value else "false"
        elif value is None:
            rendered = "-"
        else:
            rendered = str(value).replace("\n", " ").replace("|", "\\|")
        lines.append(f"| {item['kind']} | {item['state']} | {rendered} |")

    if outcome["diagnostic"] is not None:
        lines.extend(
            [
                "",
                "## Diagnostic",
                "",
                outcome["diagnostic"].replace("\n", " "),
            ]
        )
    return "\n".join(lines) + "\n"


def _load_attempt(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise ResultError(f"{path}: cannot read attempt JSON: {error}") from error


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--attempt", required=True, type=Path)
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
            args.manifest, root=args.root.resolve()
        )
        result = normalize_attempt(
            corpus,
            _load_attempt(args.attempt),
            rarog_commit=args.rarog_commit,
            platform={
                "os": args.platform_os,
                "arch": args.platform_arch,
                "environment": args.environment,
            },
        )
        args.json_out.write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        args.markdown_out.write_text(
            render_markdown(result),
            encoding="utf-8",
        )
    except (ResultError, real_web_corpus.CorpusError) as error:
        print(f"real-web-result: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
