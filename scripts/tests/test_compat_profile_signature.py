from __future__ import annotations

import base64
import copy
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import compat_profile_signature as signatures
import compat_profile_verify as verifier
from test_compat_profile_verify import SOURCE_FILES


class CompatibilitySignatureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.private = self.base / "private.pem"
        self.trust = self.base / "trust.json"
        self.signature = self.base / "signature.json"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(self.private)],
            check=True, capture_output=True,
        )
        public = self.base / "public.der"
        subprocess.run(
            ["openssl", "pkey", "-in", str(self.private), "-pubout", "-outform", "DER",
             "-out", str(public)],
            check=True, capture_output=True,
        )
        self.der = public.read_bytes()
        self.key_id = "sha256:" + hashlib.sha256(self.der).hexdigest()
        self.trust_record = {
            "schema_version": 1,
            "keys": [{
                "algorithm": "Ed25519",
                "key_id": self.key_id,
                "public_key_spki_der_base64": base64.b64encode(self.der).decode("ascii"),
                "status": "active",
            }],
        }
        self.write(self.trust, self.trust_record)

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def write(path: Path, value: dict) -> None:
        path.write_bytes(signatures._pretty(value))

    def sign(self, root: Path = ROOT) -> dict:
        return signatures.sign_profile(root, self.private, self.trust, self.signature)

    def test_roundtrip_matches_independent_profile_digest(self) -> None:
        record = self.sign()
        self.assertEqual(record["key_id"], self.key_id)
        self.assertEqual(
            record["payload_sha256"], verifier.verify(ROOT)["profile_sha256"]
        )
        self.assertEqual(
            signatures.verify_signature(ROOT, self.trust, self.signature), record
        )
        self.assertEqual(self.signature.read_bytes(), signatures._pretty(record))

    def test_cli_offline_verify(self) -> None:
        self.sign()
        self.assertEqual(signatures.main([
            "verify", "--root", str(ROOT), "--trust", str(self.trust),
            "--signature", str(self.signature),
        ]), 0)

    def test_rejects_signature_envelope_tampering(self) -> None:
        original = self.sign()
        cases = ("algorithm", "key_id", "digest", "signature", "unknown", "bool-schema")
        for case in cases:
            with self.subTest(case=case):
                altered = copy.deepcopy(original)
                if case == "algorithm":
                    altered["algorithm"] = "RSA"
                elif case == "key_id":
                    altered["key_id"] = "sha256:" + "0" * 64
                elif case == "digest":
                    altered["payload_sha256"] = "sha256:" + "0" * 64
                elif case == "signature":
                    signature = bytearray(base64.b64decode(altered["signature_base64"]))
                    signature[0] ^= 1
                    altered["signature_base64"] = base64.b64encode(signature).decode()
                elif case == "unknown":
                    altered["unexpected"] = "value"
                else:
                    altered["schema_version"] = True
                self.write(self.signature, altered)
                with self.assertRaises(signatures.SignatureError):
                    signatures.verify_signature(ROOT, self.trust, self.signature)
        self.write(self.signature, original)

    def test_rejects_noncanonical_json_and_duplicate_keys(self) -> None:
        original = self.sign()
        self.signature.write_text(json.dumps(original), encoding="utf-8")
        with self.assertRaises(signatures.SignatureError):
            signatures.verify_signature(ROOT, self.trust, self.signature)
        raw = signatures._pretty(original)
        self.signature.write_bytes(raw.replace(
            b'"schema_version": 1,', b'"schema_version": 1, "schema_version": 1,'
        ))
        with self.assertRaises(signatures.SignatureError):
            signatures.verify_signature(ROOT, self.trust, self.signature)

    def test_rejects_noncanonical_base64_and_signature_length(self) -> None:
        original = self.sign()
        for changed in ("invalid-base64", "wrong-length"):
            with self.subTest(case=changed):
                data = copy.deepcopy(original)
                data["signature_base64"] = (
                    "!!!" if changed == "invalid-base64" else base64.b64encode(b"x").decode()
                )
                self.write(self.signature, data)
                with self.assertRaises(signatures.SignatureError):
                    signatures.verify_signature(ROOT, self.trust, self.signature)

    def test_rejects_revoked_or_replaced_trust(self) -> None:
        self.sign()
        for case in ("revoked", "unknown-key", "different-spki", "unknown-status"):
            with self.subTest(case=case):
                root = copy.deepcopy(self.trust_record)
                if case == "revoked":
                    root["keys"][0]["status"] = "revoked"
                elif case == "unknown-key":
                    root["keys"][0]["key_id"] = "sha256:" + "0" * 64
                elif case == "different-spki":
                    other = bytearray(self.der)
                    other[-1] ^= 1
                    root["keys"][0]["public_key_spki_der_base64"] = base64.b64encode(other).decode()
                else:
                    root["keys"][0]["status"] = "retired"
                self.write(self.trust, root)
                with self.assertRaises(signatures.SignatureError):
                    signatures.verify_signature(ROOT, self.trust, self.signature)
        self.write(self.trust, self.trust_record)

    def test_rejects_untrusted_signing_key(self) -> None:
        other = self.base / "other-private.pem"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(other)],
            check=True, capture_output=True,
        )
        with self.assertRaises(signatures.SignatureError):
            signatures.sign_profile(ROOT, other, self.trust, self.signature)
        self.assertFalse(self.signature.exists())

    def test_rejects_private_key_inside_checkout(self) -> None:
        checkout = self.base / "checkout"
        checkout.mkdir()
        private = checkout / "private.pem"
        shutil.copyfile(self.private, private)
        with self.assertRaisesRegex(signatures.SignatureError, "outside the checkout"):
            signatures.sign_profile(checkout, private, self.trust, self.signature)

    def test_rejects_mutated_profile_before_verifying_signature(self) -> None:
        checkout = self.base / "checkout"
        for name in SOURCE_FILES:
            destination = checkout / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        self.sign(checkout)
        profile = checkout / "compatibility/profile.json"
        content = json.loads(profile.read_text(encoding="utf-8"))
        content["profile_revision"] = 2
        self.write(profile, content)
        with self.assertRaises((signatures.SignatureError, verifier.VerificationError)):
            signatures.verify_signature(checkout, self.trust, self.signature)


if __name__ == "__main__":
    unittest.main()
