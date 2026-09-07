#!/usr/bin/env python3
import json
import sys
from pathlib import Path

MEDIUM_SECURITY_SEVERITY = 4.0


def security_severity(rule: dict) -> float | None:
    value = rule.get("properties", {}).get("security-severity")
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: verify_codeql_sarif.py <sarif-directory>")

    root = Path(sys.argv[1])
    files = sorted(root.rglob("*.sarif"))
    if not files:
        raise SystemExit(f"no SARIF files found under {root}")

    blocked: list[str] = []
    for path in files:
        document = json.loads(path.read_text(encoding="utf-8"))
        for run in document.get("runs", []):
            rules = run.get("tool", {}).get("driver", {}).get("rules", [])
            severities = {
                rule.get("id"): security_severity(rule)
                for rule in rules
                if rule.get("id")
            }

            for result in run.get("results", []):
                rule_id = result.get("ruleId", "<unknown>")
                severity = severities.get(rule_id)
                level = result.get("level", "warning")
                if severity is not None and severity >= MEDIUM_SECURITY_SEVERITY:
                    blocked.append(f"{rule_id}: security-severity {severity:g}")
                elif severity is None and level == "error":
                    blocked.append(f"{rule_id}: SARIF level error")

    if blocked:
        raise SystemExit(
            "CodeQL medium-or-higher gate failed:\n" + "\n".join(sorted(set(blocked)))
        )

    print("CodeQL medium-or-higher gate: OK")


if __name__ == "__main__":
    main()
