from __future__ import annotations

import json
import posixpath
import re
from pathlib import Path
from typing import Iterable
from urllib.parse import unquote, urlsplit

_VIEWPORT_RE = re.compile(r"^(?P<width>[1-9][0-9]*)x(?P<height>[1-9][0-9]*)$")
_MAX_DIAGNOSTIC_BYTES = 4096


class AdapterError(ValueError):
    """Raised when the bounded R6 WPT adapter cannot map an input safely."""


def load_allowed_paths(selection_path: Path) -> frozenset[str]:
    try:
        document = json.loads(selection_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise AdapterError(f"cannot read selection manifest: {error}") from error

    tests = document.get("tests")
    if not isinstance(tests, list) or not tests:
        raise AdapterError("selection manifest must contain a non-empty tests list")

    paths: set[str] = set()
    for index, item in enumerate(tests):
        if not isinstance(item, dict):
            raise AdapterError(f"selection test {index} must be an object")
        path = _normalized_relative_path(item.get("path"), f"selection test {index}")
        if path in paths:
            raise AdapterError(f"duplicate selected path {path!r}")
        paths.add(path)

        references = item.get("references")
        if not isinstance(references, list):
            raise AdapterError(f"{path}: references must be a list")
        for reference_index, reference in enumerate(references):
            if not isinstance(reference, dict):
                raise AdapterError(
                    f"{path}: reference {reference_index} must be an object"
                )
            reference_path = _normalized_relative_path(
                reference.get("path"), f"{path}: reference {reference_index}"
            )
            paths.add(reference_path)

    return frozenset(paths)


def selected_url_to_path(url: str, allowed_paths: Iterable[str]) -> str:
    split = urlsplit(url)
    if split.scheme or split.netloc:
        raise AdapterError(f"test URL must be path-only, got {url!r}")
    decoded = unquote(split.path)
    if not decoded.startswith("/"):
        raise AdapterError(f"test URL must start with '/', got {url!r}")
    path = _normalized_relative_path(decoded.lstrip("/"), "test URL")
    allowed = set(allowed_paths)
    if path not in allowed:
        raise AdapterError(f"test URL is outside the pinned R6 selection: {path}")
    return path


def parse_viewport(value: object) -> tuple[int, int]:
    if value is None:
        return (800, 600)
    if isinstance(value, str):
        match = _VIEWPORT_RE.fullmatch(value)
        if match is None:
            raise AdapterError(f"invalid viewport {value!r}")
        return (int(match.group("width")), int(match.group("height")))
    if isinstance(value, (tuple, list)) and len(value) == 2:
        width, height = value
        if (
            isinstance(width, int)
            and not isinstance(width, bool)
            and isinstance(height, int)
            and not isinstance(height, bool)
            and width > 0
            and height > 0
        ):
            return (width, height)
    raise AdapterError(f"invalid viewport {value!r}")


def build_render_command(
    binary: Path,
    source: Path,
    output: Path,
    viewport: tuple[int, int],
) -> list[str]:
    width, height = viewport
    return [
        str(binary),
        "--input",
        str(source),
        "--output",
        str(output),
        "--width",
        str(width),
        "--height",
        str(height),
    ]


def classify_render_result(
    *,
    returncode: int | None,
    timed_out: bool,
    output_exists: bool,
    diagnostic: str = "",
) -> tuple[str, str] | None:
    diagnostic = bounded_diagnostic(diagnostic)
    if timed_out:
        return ("EXTERNAL-TIMEOUT", _with_detail("Rarog renderer timed out", diagnostic))
    if returncode is None:
        return (
            "INTERNAL-ERROR",
            _with_detail("Rarog renderer produced no process status", diagnostic),
        )
    if returncode != 0:
        return (
            "CRASH",
            _with_detail(f"Rarog renderer exited with status {returncode}", diagnostic),
        )
    if not output_exists:
        return (
            "INTERNAL-ERROR",
            _with_detail("Rarog renderer produced no screenshot file", diagnostic),
        )
    return None


def unsupported_testharness_message(test_url: str) -> str:
    return (
        "R6 selected execution does not yet integrate WPT testharness with "
        f"Rarog DOM/script; explicit unsupported result for {test_url}"
    )


def bounded_diagnostic(value: str) -> str:
    encoded = value.encode("utf-8", "replace")
    if len(encoded) <= _MAX_DIAGNOSTIC_BYTES:
        return encoded.decode("utf-8", "replace").strip()
    truncated = encoded[:_MAX_DIAGNOSTIC_BYTES].decode("utf-8", "ignore").strip()
    return f"{truncated}\n[diagnostic truncated]"


def _normalized_relative_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise AdapterError(f"{label} path must be a non-empty string")
    if "\\" in value or value.startswith("/") or value.endswith("/"):
        raise AdapterError(f"{label} path must be normalized repository-relative POSIX")
    normalized = posixpath.normpath(value)
    if normalized != value or value in {".", ".."} or value.startswith("../"):
        raise AdapterError(f"{label} path must be normalized repository-relative POSIX")
    return value


def _with_detail(message: str, detail: str) -> str:
    return f"{message}: {detail}" if detail else message
