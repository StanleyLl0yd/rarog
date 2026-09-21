#!/usr/bin/env python3
"""Validate the exact R6 WPT selection against a pinned upstream checkout."""

from __future__ import annotations

import argparse
import json
import posixpath
import re
import subprocess
import sys
from html.parser import HTMLParser
from pathlib import Path
from typing import Any, Sequence

SCHEMA_VERSION = 1
UPSTREAM_REPOSITORY = "web-platform-tests/wpt"
_HEX40_RE = re.compile(r"^[0-9a-f]{40}$")
_ALLOWED_KINDS = {"testharness", "reftest"}
_ALLOWED_RELATIONS = {"match", "mismatch"}


class SelectionError(ValueError):
    """Raised when a WPT selection or checkout cannot be verified."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise SelectionError(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> None:
    raise SelectionError(f"non-finite JSON number is not allowed: {value}")


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SelectionError(f"{label} must be an object")
    return value


def _require_exact_keys(value: dict[str, Any], keys: set[str], label: str) -> None:
    actual = set(value)
    if actual != keys:
        missing = sorted(keys - actual)
        extra = sorted(actual - keys)
        raise SelectionError(
            f"{label} has invalid keys; missing={missing}, extra={extra}"
        )


def _require_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise SelectionError(f"{label} must be a non-empty string")
    return value


def _require_hex40(value: Any, label: str) -> str:
    text = _require_text(value, label)
    if not _HEX40_RE.fullmatch(text):
        raise SelectionError(
            f"{label} must be an exact lowercase 40-hex git object id"
        )
    return text


def _require_repo_path(value: Any, label: str) -> str:
    path = _require_text(value, label)
    if "\\" in path or path.startswith("/") or path.endswith("/"):
        raise SelectionError(
            f"{label} must be a normalized relative POSIX file path"
        )
    normalized = posixpath.normpath(path)
    if normalized != path or path in {".", ".."} or path.startswith("../"):
        raise SelectionError(
            f"{label} must be a normalized relative POSIX file path"
        )
    return path


class _ReferenceParser(HTMLParser):
    def __init__(self, test_path: str) -> None:
        super().__init__(convert_charrefs=True)
        self.test_path = test_path
        self.references: list[dict[str, str]] = []
        self.has_testharness = False

    def handle_starttag(
        self, tag: str, attrs: list[tuple[str, str | None]]
    ) -> None:
        values = {key.lower(): value for key, value in attrs}
        if tag.lower() == "script":
            source = values.get("src")
            if source == "/resources/testharness.js":
                self.has_testharness = True
        if tag.lower() != "link":
            return

        rel_value = values.get("rel") or ""
        relations = set(rel_value.lower().split()) & _ALLOWED_RELATIONS
        href = values.get("href")
        if not relations:
            return
        if len(relations) != 1 or not href:
            raise SelectionError(
                f"{self.test_path}: ambiguous reftest reference metadata"
            )
        relation = next(iter(relations))
        if "://" in href or href.startswith("//"):
            raise SelectionError(
                f"{self.test_path}: external reftest reference is not allowed"
            )
        if href.startswith("/"):
            normalized = posixpath.normpath(href.lstrip("/"))
        else:
            normalized = posixpath.normpath(
                posixpath.join(posixpath.dirname(self.test_path), href)
            )
        normalized = _require_repo_path(
            normalized, f"{self.test_path}: reference"
        )
        self.references.append({"relation": relation, "path": normalized})


def _parsed_test_metadata(
    path: str, content: str
) -> tuple[bool, list[dict[str, str]]]:
    parser = _ReferenceParser(path)
    parser.feed(content)
    parser.close()
    references = sorted(
        parser.references, key=lambda item: (item["path"], item["relation"])
    )
    if len(
        {(item["path"], item["relation"]) for item in references}
    ) != len(references):
        raise SelectionError(f"{path}: duplicate reftest reference metadata")
    return parser.has_testharness, references


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            document = json.load(
                handle,
                object_pairs_hook=_reject_duplicate_keys,
                parse_constant=_reject_nonfinite,
            )
    except (OSError, json.JSONDecodeError) as error:
        raise SelectionError(
            f"{path}: cannot read selection JSON: {error}"
        ) from error
    return validate_manifest(document)


def validate_manifest(document: Any) -> dict[str, Any]:
    root = _require_object(document, "selection")
    _require_exact_keys(
        root, {"schema_version", "source", "tests"}, "selection"
    )
    if root["schema_version"] != SCHEMA_VERSION:
        raise SelectionError(
            f"selection schema_version must be {SCHEMA_VERSION}"
        )

    source = _require_object(root["source"], "source")
    _require_exact_keys(source, {"repository", "commit"}, "source")
    repository = _require_text(
        source["repository"], "source repository"
    )
    if repository != UPSTREAM_REPOSITORY:
        raise SelectionError(
            f"source repository must be {UPSTREAM_REPOSITORY}"
        )
    commit = _require_hex40(source["commit"], "source commit")

    raw_tests = root["tests"]
    if not isinstance(raw_tests, list) or not raw_tests:
        raise SelectionError("tests must be a non-empty list")

    tests: list[dict[str, Any]] = []
    seen_paths: set[str] = set()
    previous_path: str | None = None

    for index, raw in enumerate(raw_tests):
        item = _require_object(raw, f"test {index}")
        _require_exact_keys(
            item,
            {"area", "path", "kind", "blob", "references"},
            f"test {index}",
        )
        area = _require_text(item["area"], f"test {index} area")
        path = _require_repo_path(item["path"], f"test {index} path")
        kind = _require_text(item["kind"], f"{path}: kind")
        if kind not in _ALLOWED_KINDS:
            raise SelectionError(
                f"{path}: unsupported kind {kind!r}"
            )
        blob = _require_hex40(item["blob"], f"{path}: blob")

        if path in seen_paths:
            raise SelectionError(f"duplicate test path: {path}")
        seen_paths.add(path)
        if previous_path is not None and path <= previous_path:
            raise SelectionError("test paths must be strictly sorted")
        previous_path = path

        raw_refs = item["references"]
        if not isinstance(raw_refs, list):
            raise SelectionError(
                f"{path}: references must be a list"
            )

        references: list[dict[str, str]] = []
        seen_refs: set[tuple[str, str]] = set()
        previous_ref: tuple[str, str] | None = None

        for ref_index, raw_ref in enumerate(raw_refs):
            ref = _require_object(
                raw_ref, f"{path}: reference {ref_index}"
            )
            _require_exact_keys(
                ref,
                {"relation", "path", "blob"},
                f"{path}: reference {ref_index}",
            )
            relation = _require_text(
                ref["relation"],
                f"{path}: reference {ref_index} relation",
            )
            if relation not in _ALLOWED_RELATIONS:
                raise SelectionError(
                    f"{path}: unsupported relation {relation!r}"
                )
            ref_path = _require_repo_path(
                ref["path"], f"{path}: reference {ref_index} path"
            )
            ref_blob = _require_hex40(
                ref["blob"], f"{path}: reference {ref_index} blob"
            )
            identity = (ref_path, relation)
            if identity in seen_refs:
                raise SelectionError(
                    f"{path}: duplicate reference {identity!r}"
                )
            seen_refs.add(identity)
            if previous_ref is not None and identity <= previous_ref:
                raise SelectionError(
                    f"{path}: references must be strictly sorted"
                )
            previous_ref = identity
            references.append(
                {
                    "relation": relation,
                    "path": ref_path,
                    "blob": ref_blob,
                }
            )

        if kind == "testharness" and references:
            raise SelectionError(
                f"{path}: testharness entry must not declare references"
            )
        if kind == "reftest" and not references:
            raise SelectionError(
                f"{path}: reftest entry must declare a reference"
            )

        tests.append(
            {
                "area": area,
                "path": path,
                "kind": kind,
                "blob": blob,
                "references": references,
            }
        )

    return {
        "schema_version": SCHEMA_VERSION,
        "source": {
            "repository": repository,
            "commit": commit,
        },
        "tests": tests,
    }


def _git(checkout: Path, *args: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(checkout), *args],
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as error:
        output = getattr(error, "output", "")
        raise SelectionError(
            f"{checkout}: git {' '.join(args)} failed: "
            f"{str(output).strip() or error}"
        ) from error


def _read_text(path: Path, label: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise SelectionError(
            f"{label}: cannot read UTF-8 test file: {error}"
        ) from error


def verify_checkout(
    manifest: dict[str, Any], checkout: Path
) -> dict[str, int]:
    checkout = checkout.resolve()
    if not checkout.is_dir():
        raise SelectionError(
            f"{checkout}: WPT checkout must be a directory"
        )

    head = _git(checkout, "rev-parse", "HEAD")
    if head != manifest["source"]["commit"]:
        raise SelectionError(
            "WPT checkout commit mismatch: "
            f"expected {manifest['source']['commit']}, got {head}"
        )

    reference_count = 0
    for item in manifest["tests"]:
        path = item["path"]
        full = checkout / Path(*path.split("/"))
        if not full.is_file():
            raise SelectionError(
                f"{path}: selected test file is missing"
            )

        committed_blob = _git(
            checkout, "rev-parse", f"HEAD:{path}"
        )
        if committed_blob != item["blob"]:
            raise SelectionError(
                f"{path}: pinned commit blob mismatch: "
                f"expected {item['blob']}, got {committed_blob}"
            )
        observed_blob = _git(
            checkout, "hash-object", "--", path
        )
        if observed_blob != item["blob"]:
            raise SelectionError(
                f"{path}: working-tree blob mismatch: "
                f"expected {item['blob']}, got {observed_blob}"
            )

        content = _read_text(full, path)
        has_testharness, parsed_refs = _parsed_test_metadata(
            path, content
        )
        declared_refs = [
            {
                "relation": ref["relation"],
                "path": ref["path"],
            }
            for ref in item["references"]
        ]

        if item["kind"] == "testharness":
            if not has_testharness:
                raise SelectionError(
                    f"{path}: declared testharness "
                    "but testharness.js is absent"
                )
            if parsed_refs:
                raise SelectionError(
                    f"{path}: testharness entry unexpectedly "
                    "carries reftest references"
                )
        elif parsed_refs != declared_refs:
            raise SelectionError(
                f"{path}: declared references do not match "
                f"file metadata: declared={declared_refs}, "
                f"observed={parsed_refs}"
            )

        for reference in item["references"]:
            ref_path = reference["path"]
            ref_full = checkout / Path(*ref_path.split("/"))
            if not ref_full.is_file():
                raise SelectionError(
                    f"{path}: reference file is missing: {ref_path}"
                )
            committed_ref_blob = _git(
                checkout, "rev-parse", f"HEAD:{ref_path}"
            )
            if committed_ref_blob != reference["blob"]:
                raise SelectionError(
                    f"{path}: pinned reference blob mismatch "
                    f"for {ref_path}: expected {reference['blob']}, "
                    f"got {committed_ref_blob}"
                )
            observed_ref_blob = _git(
                checkout, "hash-object", "--", ref_path
            )
            if observed_ref_blob != reference["blob"]:
                raise SelectionError(
                    f"{path}: working-tree reference blob mismatch "
                    f"for {ref_path}: expected {reference['blob']}, "
                    f"got {observed_ref_blob}"
                )
            reference_count += 1

    return {
        "tests": len(manifest["tests"]),
        "references": reference_count,
    }


def parse_args(
    argv: Sequence[str] | None = None,
) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest", required=True, type=Path
    )
    parser.add_argument(
        "--wpt-checkout", required=True, type=Path
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        manifest = load_manifest(args.manifest)
        summary = verify_checkout(
            manifest, args.wpt_checkout
        )
    except SelectionError as error:
        print(f"wpt-selection: {error}", file=sys.stderr)
        return 2

    print(
        f"verified {summary['tests']} selected WPT tests and "
        f"{summary['references']} reference files at "
        f"{manifest['source']['commit']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
