#!/usr/bin/env python3
"""Execute the bounded R6 real-Web corpus with explicit network safety checks."""

from __future__ import annotations

import argparse
import hashlib
import http.client
import ipaddress
import json
import socket
import ssl
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any, Sequence
from urllib.parse import urljoin, urlsplit

import real_web_corpus

SCHEMA_VERSION = 1
_MAX_REDIRECTS = 8
_READ_CHUNK = 65536
_MAX_DIAGNOSTIC_BYTES = 4096


class ExecutionError(RuntimeError):
    """Raised for local executor/configuration errors rather than measured outcomes."""


class NetworkFailure(RuntimeError):
    def __init__(
        self,
        state: str,
        diagnostic: str,
        *,
        addresses: list[str],
        http_status: int | None = None,
    ) -> None:
        super().__init__(diagnostic)
        self.state = state
        self.diagnostic = diagnostic
        self.addresses = addresses
        self.http_status = http_status


class _PinnedHTTPSConnection(http.client.HTTPSConnection):
    def __init__(
        self,
        host: str,
        *,
        port: int,
        address: str,
        timeout: float,
    ) -> None:
        super().__init__(
            host,
            port=port,
            timeout=timeout,
            context=ssl.create_default_context(),
        )
        self._address = address

    def connect(self) -> None:
        raw = socket.create_connection(
            (self._address, self.port),
            timeout=self.timeout,
        )
        try:
            self.sock = self._context.wrap_socket(
                raw,
                server_hostname=self.host,
            )
        except BaseException:
            raw.close()
            raise


def _public_addresses(
    host: str,
    port: int,
    *,
    deadline: float | None = None,
) -> list[str]:
    try:
        resolved = socket.getaddrinfo(
            host,
            port,
            family=socket.AF_UNSPEC,
            type=socket.SOCK_STREAM,
        )
    except socket.gaierror as error:
        raise LookupError(str(error)) from error
    if deadline is not None and time.monotonic() >= deadline:
        raise TimeoutError("DNS resolution exceeded scenario timeout")

    addresses: set[str] = set()
    for item in resolved:
        raw = item[4][0]
        address = ipaddress.ip_address(raw)
        if (
            address.is_private
            or address.is_loopback
            or address.is_link_local
            or address.is_multicast
            or address.is_reserved
            or address.is_unspecified
        ):
            raise ExecutionError(
                f"DNS for {host!r} resolved to forbidden address {address}"
            )
        addresses.add(str(address))

    if not addresses:
        raise LookupError(f"DNS for {host!r} returned no addresses")
    return sorted(addresses)


def _origin(url: str) -> str:
    parsed = urlsplit(url)
    port = parsed.port
    suffix = f":{port}" if port is not None else ""
    return f"{parsed.scheme}://{parsed.hostname}{suffix}"


def _remaining(deadline: float) -> float:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError("scenario timeout exhausted")
    return remaining


def _diagnostic(value: object) -> str:
    text = str(value).replace("\x00", "")
    encoded = text.encode("utf-8", "replace")
    if len(encoded) <= _MAX_DIAGNOSTIC_BYTES:
        return encoded.decode("utf-8", "replace").strip() or "unspecified failure"
    truncated = encoded[:_MAX_DIAGNOSTIC_BYTES].decode("utf-8", "ignore").strip()
    return f"{truncated}\n[diagnostic truncated]"


def _blank_observations(scenario: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {"kind": kind, "state": "not-observed", "value": None}
        for kind in scenario["observations"]
    ]


def _dependency(
    declared: dict[str, Any],
    *,
    state: str,
    addresses: list[str] | None = None,
    http_status: int | None = None,
    content_sha256: str | None = None,
    expected_content_sha256: str | None = None,
) -> dict[str, Any]:
    return {
        "origin": declared["origin"],
        "role": declared["role"],
        "state": state,
        "resolved_addresses": addresses or [],
        "http_status": http_status,
        "content_sha256": content_sha256,
        "expected_content_sha256": expected_content_sha256,
    }


def _attempt(
    scenario: dict[str, Any],
    *,
    category: str,
    detail: str,
    diagnostic: str | None,
    dependencies: list[dict[str, Any]],
    observations: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "scenario_id": scenario["id"],
        "outcome": {
            "category": category,
            "detail": detail,
            "diagnostic": diagnostic,
        },
        "dependencies": dependencies,
        "observations": observations,
    }


def _read_bounded(
    response: http.client.HTTPResponse,
    *,
    connection: _PinnedHTTPSConnection,
    deadline: float,
    maximum: int,
) -> bytes:
    content_length = response.getheader("Content-Length")
    if content_length is not None:
        try:
            declared = int(content_length)
        except ValueError as error:
            raise ExecutionError("invalid Content-Length response header") from error
        if declared < 0:
            raise ExecutionError("negative Content-Length response header")
        if declared > maximum:
            raise OverflowError(
                f"response declares {declared} bytes; limit is {maximum}"
            )

    output = bytearray()
    while True:
        remaining = maximum + 1 - len(output)
        if remaining <= 0:
            raise OverflowError(f"response exceeded {maximum} bytes")
        if connection.sock is None:
            raise ExecutionError("HTTPS connection lost its socket during response read")
        connection.sock.settimeout(min(_remaining(deadline), 5.0))
        chunk = response.read(min(_READ_CHUNK, remaining))
        if not chunk:
            break
        output.extend(chunk)
        if len(output) > maximum:
            raise OverflowError(f"response exceeded {maximum} bytes")
    return bytes(output)


def _request_once(
    url: str,
    *,
    deadline: float,
    maximum_bytes: int,
) -> tuple[
    int,
    list[str],
    dict[str, str],
    bytes,
]:
    parsed = urlsplit(url)
    if parsed.scheme != "https" or not parsed.hostname:
        raise ExecutionError(f"executor received non-HTTPS URL {url!r}")
    port = parsed.port or 443
    try:
        addresses = _public_addresses(
            parsed.hostname,
            port,
            deadline=deadline,
        )
    except TimeoutError as error:
        raise NetworkFailure(
            "timeout-limit-exceeded",
            _diagnostic(error),
            addresses=[],
        ) from error
    target = parsed.path or "/"
    if parsed.query:
        target += f"?{parsed.query}"

    last_error: BaseException | None = None
    for address in addresses:
        connection = _PinnedHTTPSConnection(
            parsed.hostname,
            port=port,
            address=address,
            timeout=min(_remaining(deadline), 5.0),
        )
        try:
            connection.request(
                "GET",
                target,
                headers={
                    "Accept": "text/html,application/xhtml+xml;q=0.9,*/*;q=0.1",
                    "Accept-Encoding": "identity",
                    "Connection": "close",
                    "User-Agent": "Rarog-R6-Compatibility-Qualification/1",
                },
            )
            response = connection.getresponse()
            _remaining(deadline)
            headers = {key.lower(): value for key, value in response.getheaders()}
            try:
                body = _read_bounded(
                    response,
                    connection=connection,
                    deadline=deadline,
                    maximum=maximum_bytes,
                )
            except OverflowError as error:
                raise NetworkFailure(
                    "resource-limit-exceeded",
                    _diagnostic(error),
                    addresses=addresses,
                    http_status=response.status,
                ) from error
            return response.status, addresses, headers, body
        except NetworkFailure:
            raise
        except TimeoutError as error:
            last_error = error
        except (OSError, ssl.SSLError, http.client.HTTPException) as error:
            last_error = error
        finally:
            connection.close()

    if isinstance(last_error, TimeoutError):
        raise NetworkFailure(
            "timeout-limit-exceeded",
            _diagnostic(last_error),
            addresses=addresses,
        ) from last_error
    raise NetworkFailure(
        "tls-failure",
        _diagnostic(last_error or "connection failed"),
        addresses=addresses,
    ) from last_error


def _fetch_live(
    scenario: dict[str, Any],
    *,
    deadline: float,
) -> tuple[
    bytes | None,
    str | None,
    dict[str, Any],
    tuple[str, str, str] | None,
]:
    declared = scenario["external_dependencies"][0]
    declared_origins = {item["origin"] for item in scenario["external_dependencies"]}
    current = scenario["source_url"]
    observed_addresses: set[str] = set()
    total_bytes = 0

    for redirect_index in range(_MAX_REDIRECTS + 1):
        try:
            canonical = real_web_corpus._canonical_https_url(
                current,
                f"{scenario['id']}: live URL",
            )
        except real_web_corpus.CorpusError as error:
            return (
                None,
                None,
                _dependency(
                    declared,
                    state="redirect-policy-violation",
                    addresses=sorted(observed_addresses),
                    http_status=302,
                ),
                (
                    "external-unavailable",
                    "redirect-policy-violation",
                    _diagnostic(error),
                ),
            )

        if _origin(canonical) not in declared_origins:
            return (
                None,
                None,
                _dependency(
                    declared,
                    state="redirect-policy-violation",
                    addresses=sorted(observed_addresses),
                    http_status=302,
                ),
                (
                    "external-unavailable",
                    "redirect-policy-violation",
                    f"redirect target origin {_origin(canonical)!r} is not declared",
                ),
            )

        try:
            status, addresses, headers, body = _request_once(
                canonical,
                deadline=deadline,
                maximum_bytes=scenario["limits"]["max_response_bytes"],
            )
        except LookupError as error:
            return (
                None,
                None,
                _dependency(declared, state="dns-failure"),
                ("external-unavailable", "dns-failure", _diagnostic(error)),
            )
        except NetworkFailure as error:
            addresses = sorted(set(observed_addresses) | set(error.addresses))
            return (
                None,
                None,
                _dependency(
                    declared,
                    state=error.state,
                    addresses=addresses,
                    http_status=error.http_status,
                ),
                (
                    "external-unavailable",
                    error.state,
                    error.diagnostic,
                ),
            )
        except ExecutionError:
            raise

        observed_addresses.update(addresses)
        total_bytes += len(body)
        if total_bytes > scenario["limits"]["max_total_bytes"]:
            return (
                None,
                None,
                _dependency(
                    declared,
                    state="resource-limit-exceeded",
                    addresses=sorted(observed_addresses),
                    http_status=status,
                ),
                (
                    "external-unavailable",
                    "resource-limit-exceeded",
                    "total response byte budget exceeded",
                ),
            )

        if status in {301, 302, 303, 307, 308}:
            location = headers.get("location")
            if location is None:
                return (
                    None,
                    None,
                    _dependency(
                        declared,
                        state="redirect-policy-violation",
                        addresses=sorted(observed_addresses),
                        http_status=status,
                    ),
                    (
                        "external-unavailable",
                        "redirect-policy-violation",
                        "redirect response omitted Location",
                    ),
                )
            if redirect_index == _MAX_REDIRECTS:
                return (
                    None,
                    None,
                    _dependency(
                        declared,
                        state="redirect-policy-violation",
                        addresses=sorted(observed_addresses),
                        http_status=status,
                    ),
                    (
                        "external-unavailable",
                        "redirect-policy-violation",
                        f"redirect count exceeded {_MAX_REDIRECTS}",
                    ),
                )
            current = urljoin(canonical, location)
            continue

        if status < 200 or status > 299:
            return (
                None,
                None,
                _dependency(
                    declared,
                    state="http-unavailable",
                    addresses=sorted(observed_addresses),
                    http_status=status,
                ),
                (
                    "external-unavailable",
                    "http-unavailable",
                    f"HTTP status {status}",
                ),
            )

        digest = "sha256:" + hashlib.sha256(body).hexdigest()
        return (
            body,
            canonical,
            _dependency(
                declared,
                state="available",
                addresses=sorted(observed_addresses),
                http_status=status,
                content_sha256=digest,
            ),
            None,
        )

    raise AssertionError("redirect loop escaped bounded iteration")


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(_READ_CHUNK):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def _run_renderer(
    source: bytes,
    *,
    renderer: Path,
    scenario: dict[str, Any],
    deadline: float,
) -> tuple[dict[str, Any] | None, tuple[str, str, str] | None]:
    try:
        source_text = source.decode("utf-8")
    except UnicodeDecodeError as error:
        return None, (
            "engine-failure",
            "navigation-failure",
            f"primary document is not UTF-8: {_diagnostic(error)}",
        )

    with tempfile.TemporaryDirectory(prefix="rarog-real-web-") as directory:
        root = Path(directory)
        input_path = root / "input.html"
        screenshot_path = root / "screenshot.ppm"
        title_path = root / "title.txt"
        input_path.write_text(source_text, encoding="utf-8")

        command = [
            str(renderer),
            "--input",
            str(input_path),
            "--screenshot",
            str(screenshot_path),
            "--title",
            str(title_path),
            "--width",
            str(scenario["viewport"]["width"]),
            "--height",
            str(scenario["viewport"]["height"]),
        ]
        try:
            completed = subprocess.run(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=_remaining(deadline),
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            return None, (
                "engine-failure",
                "timeout-limit-exceeded",
                _diagnostic(error),
            )

        if completed.returncode != 0:
            return None, (
                "engine-failure",
                "render-failure",
                _diagnostic(completed.stderr or f"renderer exit {completed.returncode}"),
            )
        if not screenshot_path.is_file() or not title_path.is_file():
            return None, (
                "engine-failure",
                "render-failure",
                "renderer omitted required observation artifact",
            )

        title = title_path.read_text(encoding="utf-8")
        if not title:
            return None, (
                "engine-failure",
                "navigation-failure",
                "Rarog DOM exposed no non-empty document title",
            )
        if len(title.encode("utf-8")) > 4096:
            return None, (
                "engine-failure",
                "resource-limit-exceeded",
                "document title exceeds result evidence bound",
            )

        return {
            "document-title": title,
            "render-completion": True,
            "screenshot": _sha256_file(screenshot_path),
        }, None


def execute_live_scenario(
    scenario: dict[str, Any],
    *,
    renderer: Path,
) -> dict[str, Any]:
    if scenario["input"]["mode"] != "live-external":
        raise ExecutionError(
            f"{scenario['id']}: R6.7 live executor only accepts live-external input"
        )
    if len(scenario["external_dependencies"]) != 1:
        raise ExecutionError(
            f"{scenario['id']}: R6.7 baseline requires exactly one declared dependency"
        )
    if scenario["external_dependencies"][0]["role"] != "primary-document":
        raise ExecutionError(
            f"{scenario['id']}: R6.7 baseline dependency must be primary-document"
        )
    if scenario["viewport"]["device_scale"] != 1:
        raise ExecutionError(
            f"{scenario['id']}: R6.7 renderer requires device_scale=1"
        )

    deadline = time.monotonic() + scenario["limits"]["timeout_ms"] / 1000.0
    observations = _blank_observations(scenario)
    body, final_url, dependency, external_failure = _fetch_live(
        scenario,
        deadline=deadline,
    )
    dependencies = [dependency]

    if external_failure is not None:
        category, detail, diagnostic = external_failure
        return _attempt(
            scenario,
            category=category,
            detail=detail,
            diagnostic=diagnostic,
            dependencies=dependencies,
            observations=observations,
        )

    assert body is not None
    assert final_url is not None
    render, engine_failure = _run_renderer(
        body,
        renderer=renderer,
        scenario=scenario,
        deadline=deadline,
    )

    values: dict[str, Any] = {"final-url": final_url}
    if render is not None:
        values.update(render)

    observations = [
        {
            "kind": item["kind"],
            "state": "observed" if item["kind"] in values else "not-observed",
            "value": values.get(item["kind"]),
        }
        for item in observations
    ]

    if engine_failure is not None:
        category, detail, diagnostic = engine_failure
        return _attempt(
            scenario,
            category=category,
            detail=detail,
            diagnostic=diagnostic,
            dependencies=dependencies,
            observations=observations,
        )

    return _attempt(
        scenario,
        category="completed-observation",
        detail="completed",
        diagnostic=None,
        dependencies=dependencies,
        observations=observations,
    )


def execute_corpus(
    corpus: dict[str, Any],
    *,
    renderer: Path,
) -> dict[str, Any]:
    attempts: list[dict[str, Any]] = []
    for scenario in corpus["scenarios"]:
        attempts.append(execute_live_scenario(scenario, renderer=renderer))
    return {
        "schema_version": SCHEMA_VERSION,
        "corpus_revision": corpus["corpus_revision"],
        "scenario_ids": [scenario["id"] for scenario in corpus["scenarios"]],
        "attempts": attempts,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--renderer", required=True, type=Path)
    parser.add_argument("--json-out", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        corpus = real_web_corpus.load_corpus(
            args.manifest,
            root=args.root.resolve(),
        )
        renderer = args.renderer.resolve()
        if not renderer.is_file():
            raise ExecutionError(f"renderer is missing: {renderer}")
        execution = execute_corpus(corpus, renderer=renderer)
        args.json_out.write_text(
            json.dumps(execution, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except (ExecutionError, real_web_corpus.CorpusError) as error:
        print(f"real-web-execute: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
