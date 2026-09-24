#!/usr/bin/env python3
"""Validate the bounded R6 real-Web corpus contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import posixpath
import re
import sys
from pathlib import Path
from typing import Any, Sequence
from urllib.parse import urlsplit

SCHEMA_VERSION = 1
_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
_ALLOWED_INPUT_MODES = {"live-external", "captured-versioned"}
_ALLOWED_ACTIONS = {"load-input", "wait-for-idle"}
_ALLOWED_OBSERVATIONS = {
    "document-title",
    "final-url",
    "render-completion",
    "screenshot",
}
_REQUIRED_OUTCOMES = [
    "completed-observation",
    "engine-failure",
    "external-content-drift",
    "external-unavailable",
    "unsupported-capability",
]
_REQUIRED_POLICY = {
    "credentials": "forbidden",
    "authenticated_content": "forbidden",
    "payments": "forbidden",
    "destructive_actions": "forbidden",
    "persistent_identity": "forbidden",
    "undeclared_network_origins": "forbidden",
}
_MAX_VIEWPORT = 4096
_MAX_DEVICE_SCALE = 4
_MAX_ACTIONS = 16
_MAX_ACTION_TIMEOUT_MS = 5000
_MAX_SCENARIO_TIMEOUT_MS = 30000
_MAX_RESPONSE_BYTES = 16 * 1024 * 1024
_MAX_TOTAL_BYTES = 64 * 1024 * 1024
_MAX_SUBRESOURCES = 128


class CorpusError(ValueError):
    """Raised when the real-Web corpus contract is invalid."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CorpusError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise CorpusError(f"non-finite JSON number is not allowed: {value}")


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise CorpusError(f"{label} must be an object")
    return value


def _require_exact_keys(value: dict[str, Any], keys: set[str], label: str) -> None:
    actual = set(value)
    if actual != keys:
        raise CorpusError(
            f"{label} has invalid keys; "
            f"missing={sorted(keys - actual)}, extra={sorted(actual - keys)}"
        )


def _require_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise CorpusError(f"{label} must be a non-empty string")
    return value


def _require_int(
    value: Any,
    label: str,
    *,
    minimum: int,
    maximum: int,
) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise CorpusError(f"{label} must be an integer")
    if not minimum <= value <= maximum:
        raise CorpusError(
            f"{label} must be between {minimum} and {maximum}"
        )
    return value


def _require_number(
    value: Any,
    label: str,
    *,
    minimum: float,
    maximum: float,
) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise CorpusError(f"{label} must be a number")
    numeric = float(value)
    if not math.isfinite(numeric) or not minimum <= numeric <= maximum:
        raise CorpusError(
            f"{label} must be finite and between {minimum} and {maximum}"
        )
    return numeric


def _canonical_https_url(value: Any, label: str) -> str:
    text = _require_text(value, label)
    parsed = urlsplit(text)
    if parsed.scheme != "https":
        raise CorpusError(f"{label} must use https")
    if not parsed.hostname:
        raise CorpusError(f"{label} must include a hostname")
    if parsed.username is not None or parsed.password is not None:
        raise CorpusError(f"{label} must not contain userinfo")
    if parsed.fragment:
        raise CorpusError(f"{label} must not contain a fragment")
    if parsed.hostname != parsed.hostname.lower():
        raise CorpusError(f"{label} hostname must be lowercase")
    if parsed.port == 443:
        raise CorpusError(f"{label} must omit the default https port")
    if parsed.path == "":
        raise CorpusError(f"{label} must include an explicit path")
    return text


def _origin(value: Any, label: str) -> str:
    text = _require_text(value, label)
    parsed = urlsplit(text)
    if parsed.scheme != "https" or not parsed.hostname:
        raise CorpusError(f"{label} must be an https origin")
    if (
        parsed.username is not None
        or parsed.password is not None
        or parsed.path not in ("", "/")
        or parsed.query
        or parsed.fragment
    ):
        raise CorpusError(
            f"{label} must contain only scheme, host and optional non-default port"
        )
    if "*" in text:
        raise CorpusError(f"{label} must not contain wildcards")
    if parsed.port == 443:
        raise CorpusError(f"{label} must omit the default https port")
    host = parsed.hostname.lower()
    if parsed.hostname != host:
        raise CorpusError(f"{label} hostname must be lowercase")
    port = f":{parsed.port}" if parsed.port is not None else ""
    canonical = f"https://{host}{port}"
    if text.rstrip("/") != canonical:
        raise CorpusError(f"{label} must be canonical: {canonical}")
    return canonical


def _url_origin(url: str) -> str:
    parsed = urlsplit(url)
    port = f":{parsed.port}" if parsed.port is not None else ""
    return f"{parsed.scheme}://{parsed.hostname}{port}"


def _repo_path(value: Any, label: str) -> str:
    path = _require_text(value, label)
    if "\\" in path or path.startswith("/") or path.endswith("/"):
        raise CorpusError(
            f"{label} must be a normalized repository-relative POSIX path"
        )
    normalized = posixpath.normpath(path)
    if (
        normalized != path
        or path in {".", ".."}
        or path.startswith("../")
        or not path.startswith("real-web/captures/")
    ):
        raise CorpusError(
            f"{label} must be a normalized path under real-web/captures/"
        )
    return path


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(65536):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def _validate_input(
    raw: Any,
    *,
    scenario_id: str,
    root: Path,
) -> dict[str, Any]:
    value = _require_object(raw, f"{scenario_id}: input")
    mode = _require_text(value.get("mode"), f"{scenario_id}: input mode")
    if mode not in _ALLOWED_INPUT_MODES:
        raise CorpusError(f"{scenario_id}: unsupported input mode {mode!r}")

    if mode == "live-external":
        _require_exact_keys(value, {"mode"}, f"{scenario_id}: live input")
        return {"mode": mode}

    _require_exact_keys(
        value,
        {"mode", "path", "sha256", "media_type"},
        f"{scenario_id}: captured input",
    )
    path = _repo_path(value["path"], f"{scenario_id}: capture path")
    digest = _require_text(value["sha256"], f"{scenario_id}: capture sha256")
    if not _SHA256_RE.fullmatch(digest):
        raise CorpusError(f"{scenario_id}: capture sha256 must be sha256:<64hex>")
    media_type = _require_text(
        value["media_type"], f"{scenario_id}: capture media_type"
    )
    full = root / Path(*path.split("/"))
    if not full.is_file():
        raise CorpusError(f"{scenario_id}: captured input is missing: {path}")
    observed = _sha256(full)
    if observed != digest:
        raise CorpusError(
            f"{scenario_id}: captured input digest mismatch: "
            f"expected {digest}, got {observed}"
        )
    return {
        "mode": mode,
        "path": path,
        "sha256": digest,
        "media_type": media_type,
    }


def _validate_actions(raw: Any, scenario_id: str) -> list[dict[str, Any]]:
    if not isinstance(raw, list) or not raw:
        raise CorpusError(f"{scenario_id}: actions must be a non-empty list")
    if len(raw) > _MAX_ACTIONS:
        raise CorpusError(f"{scenario_id}: too many actions")

    normalized: list[dict[str, Any]] = []
    for index, item in enumerate(raw):
        action = _require_object(item, f"{scenario_id}: action {index}")
        kind = _require_text(
            action.get("kind"), f"{scenario_id}: action {index} kind"
        )
        if kind not in _ALLOWED_ACTIONS:
            raise CorpusError(f"{scenario_id}: unsupported action kind {kind!r}")
        if kind == "load-input":
            _require_exact_keys(
                action, {"kind"}, f"{scenario_id}: action {index}"
            )
            normalized.append({"kind": kind})
        else:
            _require_exact_keys(
                action,
                {"kind", "timeout_ms"},
                f"{scenario_id}: action {index}",
            )
            timeout = _require_int(
                action["timeout_ms"],
                f"{scenario_id}: action {index} timeout_ms",
                minimum=1,
                maximum=_MAX_ACTION_TIMEOUT_MS,
            )
            normalized.append({"kind": kind, "timeout_ms": timeout})

    if normalized[0]["kind"] != "load-input":
        raise CorpusError(f"{scenario_id}: first action must be load-input")
    if sum(item["kind"] == "load-input" for item in normalized) != 1:
        raise CorpusError(f"{scenario_id}: load-input must appear exactly once")
    return normalized


def _validate_observations(raw: Any, scenario_id: str) -> list[str]:
    if not isinstance(raw, list) or not raw:
        raise CorpusError(f"{scenario_id}: observations must be a non-empty list")
    values = [
        _require_text(item, f"{scenario_id}: observation")
        for item in raw
    ]
    if values != sorted(values) or len(values) != len(set(values)):
        raise CorpusError(
            f"{scenario_id}: observations must be unique and sorted"
        )
    unsupported = sorted(set(values) - _ALLOWED_OBSERVATIONS)
    if unsupported:
        raise CorpusError(
            f"{scenario_id}: unsupported observations {unsupported}"
        )
    return values


def _validate_dependencies(
    raw: Any,
    *,
    scenario_id: str,
    source_url: str,
) -> list[dict[str, Any]]:
    if not isinstance(raw, list) or not raw:
        raise CorpusError(
            f"{scenario_id}: external_dependencies must be a non-empty list"
        )
    normalized: list[dict[str, Any]] = []
    for index, item in enumerate(raw):
        dep = _require_object(
            item, f"{scenario_id}: external dependency {index}"
        )
        _require_exact_keys(
            dep,
            {"origin", "role", "required"},
            f"{scenario_id}: external dependency {index}",
        )
        origin = _origin(
            dep["origin"], f"{scenario_id}: external dependency {index} origin"
        )
        role = _require_text(
            dep["role"], f"{scenario_id}: external dependency {index} role"
        )
        required = dep["required"]
        if not isinstance(required, bool):
            raise CorpusError(
                f"{scenario_id}: external dependency {index} required "
                "must be boolean"
            )
        normalized.append(
            {"origin": origin, "role": role, "required": required}
        )

    identities = [(item["origin"], item["role"]) for item in normalized]
    if identities != sorted(identities) or len(identities) != len(set(identities)):
        raise CorpusError(
            f"{scenario_id}: external dependencies must be unique and sorted"
        )

    primary_origin = _url_origin(source_url)
    primary = [
        item
        for item in normalized
        if item["origin"] == primary_origin
        and item["role"] == "primary-document"
        and item["required"]
    ]
    if len(primary) != 1:
        raise CorpusError(
            f"{scenario_id}: source origin must be declared exactly once as "
            "required primary-document"
        )
    return normalized


def _validate_limits(raw: Any, scenario_id: str) -> dict[str, int]:
    value = _require_object(raw, f"{scenario_id}: limits")
    _require_exact_keys(
        value,
        {
            "timeout_ms",
            "max_response_bytes",
            "max_total_bytes",
            "max_subresources",
        },
        f"{scenario_id}: limits",
    )
    timeout = _require_int(
        value["timeout_ms"],
        f"{scenario_id}: timeout_ms",
        minimum=1,
        maximum=_MAX_SCENARIO_TIMEOUT_MS,
    )
    response = _require_int(
        value["max_response_bytes"],
        f"{scenario_id}: max_response_bytes",
        minimum=1,
        maximum=_MAX_RESPONSE_BYTES,
    )
    total = _require_int(
        value["max_total_bytes"],
        f"{scenario_id}: max_total_bytes",
        minimum=1,
        maximum=_MAX_TOTAL_BYTES,
    )
    subresources = _require_int(
        value["max_subresources"],
        f"{scenario_id}: max_subresources",
        minimum=0,
        maximum=_MAX_SUBRESOURCES,
    )
    if response > total:
        raise CorpusError(
            f"{scenario_id}: max_response_bytes must not exceed max_total_bytes"
        )
    return {
        "timeout_ms": timeout,
        "max_response_bytes": response,
        "max_total_bytes": total,
        "max_subresources": subresources,
    }


def validate_corpus(document: Any, *, root: Path) -> dict[str, Any]:
    corpus = _require_object(document, "corpus")
    _require_exact_keys(
        corpus,
        {
            "schema_version",
            "corpus_revision",
            "policy",
            "outcome_categories",
            "scenarios",
        },
        "corpus",
    )
    if corpus["schema_version"] != SCHEMA_VERSION:
        raise CorpusError(f"schema_version must be {SCHEMA_VERSION}")
    revision = _require_int(
        corpus["corpus_revision"],
        "corpus_revision",
        minimum=1,
        maximum=2**31 - 1,
    )

    policy = _require_object(corpus["policy"], "policy")
    if policy != _REQUIRED_POLICY:
        raise CorpusError(
            "policy must exactly prohibit credentials, authenticated content, "
            "payments, destructive actions, persistent identity and "
            "undeclared network origins"
        )

    outcomes = corpus["outcome_categories"]
    if outcomes != _REQUIRED_OUTCOMES:
        raise CorpusError(
            f"outcome_categories must exactly equal {_REQUIRED_OUTCOMES}"
        )

    raw_scenarios = corpus["scenarios"]
    if not isinstance(raw_scenarios, list) or not raw_scenarios:
        raise CorpusError("scenarios must be a non-empty list")

    normalized: list[dict[str, Any]] = []
    previous_id: str | None = None
    for index, raw in enumerate(raw_scenarios):
        item = _require_object(raw, f"scenario {index}")
        _require_exact_keys(
            item,
            {
                "id",
                "source_url",
                "input",
                "viewport",
                "actions",
                "observations",
                "external_dependencies",
                "limits",
            },
            f"scenario {index}",
        )
        scenario_id = _require_text(item["id"], f"scenario {index} id")
        if not _ID_RE.fullmatch(scenario_id):
            raise CorpusError(
                f"scenario {index} id must be a lowercase hyphenated slug"
            )
        if previous_id is not None and scenario_id <= previous_id:
            raise CorpusError("scenario IDs must be unique and strictly sorted")
        previous_id = scenario_id

        source_url = _canonical_https_url(
            item["source_url"], f"{scenario_id}: source_url"
        )
        input_value = _validate_input(
            item["input"], scenario_id=scenario_id, root=root
        )
        viewport = _require_object(item["viewport"], f"{scenario_id}: viewport")
        _require_exact_keys(
            viewport,
            {"width", "height", "device_scale"},
            f"{scenario_id}: viewport",
        )
        viewport_value = {
            "width": _require_int(
                viewport["width"],
                f"{scenario_id}: viewport width",
                minimum=1,
                maximum=_MAX_VIEWPORT,
            ),
            "height": _require_int(
                viewport["height"],
                f"{scenario_id}: viewport height",
                minimum=1,
                maximum=_MAX_VIEWPORT,
            ),
            "device_scale": _require_number(
                viewport["device_scale"],
                f"{scenario_id}: device_scale",
                minimum=0.25,
                maximum=_MAX_DEVICE_SCALE,
            ),
        }

        normalized.append(
            {
                "id": scenario_id,
                "source_url": source_url,
                "input": input_value,
                "viewport": viewport_value,
                "actions": _validate_actions(item["actions"], scenario_id),
                "observations": _validate_observations(
                    item["observations"], scenario_id
                ),
                "external_dependencies": _validate_dependencies(
                    item["external_dependencies"],
                    scenario_id=scenario_id,
                    source_url=source_url,
                ),
                "limits": _validate_limits(item["limits"], scenario_id),
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "corpus_revision": revision,
        "policy": dict(_REQUIRED_POLICY),
        "outcome_categories": list(_REQUIRED_OUTCOMES),
        "scenarios": normalized,
    }


def load_corpus(path: Path, *, root: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            document = json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise CorpusError(f"{path}: cannot read corpus JSON: {error}") from error
    return validate_corpus(document, root=root)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--json-out", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        corpus = load_corpus(args.manifest, root=args.root.resolve())
        if args.json_out is not None:
            args.json_out.write_text(
                json.dumps(corpus, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
    except CorpusError as error:
        print(f"real-web-corpus: {error}", file=sys.stderr)
        return 2
    print(
        f"validated real-Web corpus revision {corpus['corpus_revision']} "
        f"with {len(corpus['scenarios'])} scenario(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
