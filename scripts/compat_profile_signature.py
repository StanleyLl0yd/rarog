#!/usr/bin/env python3
"""Offline Ed25519 signatures for independently verified R6 compatibility profiles."""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Sequence

import compat_profile_verify

_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
_SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
_SIGNATURE_KEYS = {
    "schema_version", "algorithm", "key_id", "payload_sha256", "signature_base64"
}
_TRUST_KEYS = {"schema_version", "keys"}
_KEY_KEYS = {"algorithm", "key_id", "public_key_spki_der_base64", "status"}


class SignatureError(ValueError):
    """Invalid signature, invalid trust policy or unsupported signing operation."""


def _object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if type(value) is not dict or set(value) != keys:
        raise SignatureError(f"{label}: unexpected or missing keys")
    return value


def _fingerprint(der: bytes) -> str:
    return "sha256:" + hashlib.sha256(der).hexdigest()


def _b64(value: Any, label: str) -> bytes:
    if type(value) is not str:
        raise SignatureError(f"{label}: base64 string required")
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise SignatureError(f"{label}: invalid base64") from error
    if base64.b64encode(raw).decode("ascii") != value:
        raise SignatureError(f"{label}: noncanonical base64")
    return raw


def _pretty(document: Any) -> bytes:
    return (
        json.dumps(document, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False)
        + "\n"
    ).encode("utf-8")


def _load(path: Path, keys: set[str], label: str) -> dict[str, Any]:
    try:
        document = compat_profile_verify._read_json(path)
        raw = path.read_bytes()
    except (OSError, UnicodeError, ValueError) as error:
        raise SignatureError(f"{label}: cannot read strict JSON") from error
    document = _object(document, keys, label)
    if raw != _pretty(document):
        raise SignatureError(f"{label}: noncanonical JSON bytes")
    return document


def _trust(path: Path) -> dict[str, tuple[bytes, str]]:
    root = _load(path, _TRUST_KEYS, "trust root")
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise SignatureError("unsupported trust schema")
    keys = root["keys"]
    if type(keys) is not list or not keys:
        raise SignatureError("trust root must contain at least one pinned key")
    indexed: dict[str, tuple[bytes, str]] = {}
    order: list[str] = []
    for key in keys:
        key = _object(key, _KEY_KEYS, "trusted key")
        if key["algorithm"] != "Ed25519" or key["status"] not in ("active", "revoked"):
            raise SignatureError("unknown algorithm or key status")
        der = _b64(key["public_key_spki_der_base64"], "trusted public key")
        if len(der) != 44 or not der.startswith(_SPKI_PREFIX):
            raise SignatureError("trusted key must be canonical RFC 8410 Ed25519 SPKI")
        fingerprint = _fingerprint(der)
        if type(key["key_id"]) is not str or key["key_id"] != fingerprint:
            raise SignatureError("trusted key ID does not match its SPKI digest")
        if fingerprint in indexed:
            raise SignatureError("duplicate trusted key")
        indexed[fingerprint] = (der, key["status"])
        order.append(fingerprint)
    if order != sorted(order):
        raise SignatureError("trust entries must be sorted by key ID")
    return indexed


def _payload(root: Path) -> tuple[bytes, str]:
    envelope = compat_profile_verify.verify(root)
    profile_path = root / "compatibility/profile.json"
    try:
        profile = compat_profile_verify._read_json(profile_path)
    except (OSError, UnicodeError, ValueError) as error:
        raise SignatureError("verified profile became unreadable") from error
    payload = json.dumps(
        profile, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    digest = _fingerprint(payload)
    if (
        digest != envelope["profile_sha256"]
        or len(payload) != envelope["canonical_payload_bytes"]
    ):
        raise SignatureError("verified profile changed during signing/verification")
    return payload, digest


def _openssl(argv: list[str], *, executable: str) -> None:
    try:
        result = subprocess.run(
            [executable, *argv], capture_output=True, check=False, timeout=30
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SignatureError("OpenSSL unavailable or timed out") from error
    if result.returncode:
        raise SignatureError("OpenSSL signing or verification operation failed")


def _check_signature(
    payload: bytes, der: bytes, signature: bytes, *, executable: str
) -> None:
    if len(signature) != 64:
        raise SignatureError("Ed25519 signature must contain exactly 64 bytes")
    with tempfile.TemporaryDirectory(prefix="rarog-signature-") as directory:
        root = Path(directory)
        data, public, sig = root / "payload", root / "public.der", root / "signature"
        data.write_bytes(payload)
        public.write_bytes(der)
        sig.write_bytes(signature)
        _openssl([
            "pkeyutl", "-verify", "-rawin", "-pubin", "-keyform", "DER",
            "-inkey", str(public), "-in", str(data), "-sigfile", str(sig),
        ], executable=executable)


def verify_signature(
    root: Path, trust_file: Path, signature_file: Path, *, executable: str = "openssl"
) -> dict[str, Any]:
    root = root.resolve(strict=True)
    payload, digest = _payload(root)
    keys = _trust(trust_file)
    record = _load(signature_file, _SIGNATURE_KEYS, "signature")
    if type(record["schema_version"]) is not int or record["schema_version"] != 1:
        raise SignatureError("unsupported signature schema")
    if record["algorithm"] != "Ed25519":
        raise SignatureError("unsupported signature algorithm")
    key_id = record["key_id"]
    if type(key_id) is not str or not _SHA256.fullmatch(key_id):
        raise SignatureError("malformed key ID")
    if key_id not in keys or keys[key_id][1] != "active":
        raise SignatureError("signer not trusted or key revoked")
    if type(record["payload_sha256"]) is not str or record["payload_sha256"] != digest:
        raise SignatureError("signed payload digest mismatch")
    signature = _b64(record["signature_base64"], "signature")
    _check_signature(payload, keys[key_id][0], signature, executable=executable)
    return record


def sign_profile(
    root: Path, private_key: Path, trust_file: Path, output: Path,
    *, executable: str = "openssl",
) -> dict[str, Any]:
    root = root.resolve(strict=True)
    key = private_key.resolve(strict=True)
    try:
        key.relative_to(root)
    except ValueError:
        pass
    else:
        raise SignatureError("private signing key must remain outside the checkout")
    if not key.is_file():
        raise SignatureError("private signing key is not a regular file")
    payload, digest = _payload(root)
    trusted = _trust(trust_file)
    with tempfile.TemporaryDirectory(prefix="rarog-signature-") as directory:
        directory = Path(directory)
        data, public, sig = (
            directory / "payload", directory / "public.der", directory / "signature"
        )
        data.write_bytes(payload)
        _openssl([
            "pkey", "-in", str(key), "-pubout", "-outform", "DER", "-out", str(public),
        ], executable=executable)
        der = public.read_bytes()
        key_id = _fingerprint(der)
        if key_id not in trusted or trusted[key_id] != (der, "active"):
            raise SignatureError("signing key does not match an active pinned trust root")
        _openssl([
            "pkeyutl", "-sign", "-rawin", "-inkey", str(key),
            "-in", str(data), "-out", str(sig),
        ], executable=executable)
        signature = sig.read_bytes()
    _check_signature(payload, der, signature, executable=executable)
    record = {
        "schema_version": 1,
        "algorithm": "Ed25519",
        "key_id": key_id,
        "payload_sha256": digest,
        "signature_base64": base64.b64encode(signature).decode("ascii"),
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.is_symlink() or output.exists():
        raise SignatureError("signature output already exists or is a symlink")
    try:
        with output.open("xb") as stream:
            stream.write(_pretty(record))
    except FileExistsError as error:
        raise SignatureError("signature output already exists") from error
    verify_signature(root, trust_file, output, executable=executable)
    return record


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("sign", "verify"))
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--trust", type=Path, required=True)
    parser.add_argument("--signature", type=Path, required=True)
    parser.add_argument("--private-key", type=Path)
    parser.add_argument("--openssl", default="openssl")
    args = parser.parse_args(argv)
    try:
        if args.operation == "sign":
            if args.private_key is None:
                parser.error("sign requires --private-key")
            sign_profile(args.root, args.private_key, args.trust, args.signature,
                         executable=args.openssl)
        else:
            if args.private_key is not None:
                parser.error("verify does not accept a private key")
            verify_signature(args.root, args.trust, args.signature,
                             executable=args.openssl)
    except (SignatureError, compat_profile_verify.VerificationError, OSError) as error:
        print(f"profile signature rejected: {error}", file=sys.stderr)
        return 1
    print("canonical compatibility profile signature verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
