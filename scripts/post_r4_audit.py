#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import subprocess
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TRACKED = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
TRACKED = [Path(path) for path in TRACKED if path]

RUST = [p for p in TRACKED if p.suffix == ".rs"]
PRODUCTION_RUST = [p for p in RUST if p.parts[:1] == ("crates",) and "tests" not in p.parts and "examples" not in p.parts and "benches" not in p.parts]
MARKDOWN = [p for p in TRACKED if p.suffix.lower() == ".md"]
MANIFESTS = [p for p in TRACKED if p.name == "Cargo.toml"]

ALLOWED_UNSAFE_CRATES = {
    "rarog-platform-windows-native",
    "rarog-script-spidermonkey",
}
PATTERNS = {
    "unwrap": re.compile(r"\.unwrap\s*\("),
    "expect": re.compile(r"\.expect\s*\("),
    "panic": re.compile(r"\bpanic!\s*\("),
    "todo": re.compile(r"\btodo!\s*\("),
    "unimplemented": re.compile(r"\bunimplemented!\s*\("),
    "unsafe_block": re.compile(r"\bunsafe\s*\{"),
    "unsafe_fn": re.compile(r"\bunsafe\s+fn\b"),
    "allow_attr": re.compile(r"#!?\s*\[allow\("),
}
STALE_DOC_PATTERNS = {
    "r4_in_progress": re.compile(r"R4[^\n]{0,80}(?:in progress|underway)", re.I),
    "single_process_current": re.compile(r"current single-process", re.I),
    "planned_r4_security": re.compile(r"planned multi-process isolation|sandbox milestones are implemented", re.I),
    "dependency_review_disabled": re.compile(r"Dependency Review remains intentionally disabled", re.I),
    "future_site_process": re.compile(r"A future site process", re.I),
    "planned_windows_sandbox": re.compile(r"sandbox/process primitives will be implemented", re.I),
}

def lines(path: Path):
    return (ROOT / path).read_text(encoding="utf-8").splitlines()

def production_lines(path: Path):
    source = lines(path)
    for index, line in enumerate(source):
        if line.strip() != "#[cfg(test)]":
            continue
        lookahead = source[index + 1 : index + 5]
        if any(re.match(r"\s*(?:pub\s+)?mod\s+tests\s*\{", candidate) for candidate in lookahead):
            return source[:index]
    return source

def findings(pattern: re.Pattern[str], paths: list[Path], *, production: bool = False):
    result = []
    for path in paths:
        source = production_lines(path) if production else lines(path)
        for number, line in enumerate(source, start=1):
            if pattern.search(line):
                result.append({"path": str(path), "line": number, "text": line.strip()[:240]})
    return result

def crate_for(path: Path) -> str | None:
    if len(path.parts) >= 2 and path.parts[0] == "crates":
        return path.parts[1]
    return None

prod_findings = {
    name: findings(pattern, PRODUCTION_RUST, production=True)
    for name, pattern in PATTERNS.items()
}
all_findings = {name: findings(pattern, RUST) for name, pattern in PATTERNS.items()}

unsafe_outside_boundary = []
for category in ("unsafe_block", "unsafe_fn"):
    for item in prod_findings[category]:
        crate = crate_for(Path(item["path"]))
        if crate not in ALLOWED_UNSAFE_CRATES:
            unsafe_outside_boundary.append(item)

manifest_policy = []
direct_dependencies = defaultdict(list)
for manifest in MANIFESTS:
    if not str(manifest).startswith("crates/"):
        continue
    data = tomllib.loads((ROOT / manifest).read_text(encoding="utf-8"))
    package = data.get("package", {})
    name = package.get("name", str(manifest))
    lints = data.get("lints", {})
    rust_lints = lints.get("rust", {})
    workspace_lints = lints.get("workspace") is True
    if name in ALLOWED_UNSAFE_CRATES:
        if rust_lints.get("unsafe_op_in_unsafe_fn") != "deny":
            manifest_policy.append({"crate": name, "issue": "unsafe boundary lacks unsafe_op_in_unsafe_fn=deny"})
        if rust_lints.get("unsafe_code") != "allow":
            manifest_policy.append({"crate": name, "issue": "unsafe boundary lacks explicit unsafe_code=allow"})
    elif not workspace_lints:
        manifest_policy.append({"crate": name, "issue": "ordinary crate does not inherit workspace lints"})

    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep, spec in data.get(section, {}).items():
            direct_dependencies[dep].append({"crate": name, "section": section, "spec": spec})

adr_ids = defaultdict(list)
for path in TRACKED:
    if len(path.parts) >= 3 and path.parts[:2] == ("docs", "adr") and path.suffix == ".md":
        text = (ROOT / path).read_text(encoding="utf-8")
        if text.startswith("# Moved:"):
            continue
        match = re.match(r"^(?:ADR-)?(\d{4})-", path.name)
        if match:
            adr_ids[match.group(1)].append(str(path))
adr_collisions = {key: value for key, value in sorted(adr_ids.items()) if len(value) > 1}

large_rust = []
for path in RUST:
    size = (ROOT / path).stat().st_size
    if size >= 50_000:
        large_rust.append({"path": str(path), "bytes": size, "lines": len(lines(path))})
large_rust.sort(key=lambda item: item["bytes"], reverse=True)

stale_docs = {}
for name, pattern in STALE_DOC_PATTERNS.items():
    hits = findings(pattern, MARKDOWN)
    if hits:
        stale_docs[name] = hits

stale_navigation_adr_references = []
for path in TRACKED:
    if path == Path("scripts/post_r4_audit.py"):
        continue
    if path.suffix.lower() not in {".md", ".rs", ".py", ".toml", ".yml", ".yaml"}:
        continue
    for number, line in enumerate(lines(path), start=1):
        if "0108-engine-document-navigation-transactions" in line:
            stale_navigation_adr_references.append(
                {"path": str(path), "line": number, "text": line.strip()[:240]}
            )

extensions = Counter(path.suffix.lower() or "<none>" for path in TRACKED)
line_counts = Counter()
for path in TRACKED:
    if path.suffix.lower() in {".rs", ".py", ".md", ".toml", ".yml", ".yaml"}:
        try:
            line_counts[path.suffix.lower()] += len(lines(path))
        except UnicodeDecodeError:
            pass

summary = {
    "tracked_files": len(TRACKED),
    "rust_files": len(RUST),
    "production_rust_files": len(PRODUCTION_RUST),
    "line_counts": dict(sorted(line_counts.items())),
    "extensions": dict(extensions.most_common()),
    "production_pattern_counts": {name: len(items) for name, items in prod_findings.items()},
    "all_rust_pattern_counts": {name: len(items) for name, items in all_findings.items()},
    "unsafe_outside_boundary": unsafe_outside_boundary,
    "manifest_policy": manifest_policy,
    "adr_collisions": adr_collisions,
    "large_rust": large_rust,
    "stale_docs": stale_docs,
    "stale_navigation_adr_references": stale_navigation_adr_references,
}

print("=== POST-R4 REPOSITORY AUDIT SUMMARY ===")
print(json.dumps(summary, indent=2, sort_keys=True))

for category in ("unwrap", "expect", "panic", "todo", "unimplemented", "unsafe_block", "unsafe_fn", "allow_attr"):
    print(f"\n=== PRODUCTION {category.upper()} ({len(prod_findings[category])}) ===")
    for item in prod_findings[category]:
        print(f"{item['path']}:{item['line']}: {item['text']}")

print("\n=== DIRECT DEPENDENCIES USED BY MULTIPLE CRATES ===")
for dep, users in sorted(direct_dependencies.items()):
    if len(users) > 1 and not dep.startswith("rarog-"):
        print(dep)
        for user in users:
            print(f"  {user['crate']} [{user['section']}]: {user['spec']}")

if unsafe_outside_boundary:
    raise SystemExit("unsafe Rust escaped the two explicit native/runtime boundaries")
if manifest_policy:
    raise SystemExit("crate lint policy is incomplete: " + json.dumps(manifest_policy))
if adr_collisions:
    raise SystemExit("accepted ADR identifiers are not unique: " + json.dumps(adr_collisions))
if stale_docs:
    raise SystemExit("stale security/milestone documentation remains: " + json.dumps(stale_docs))
if stale_navigation_adr_references:
    raise SystemExit(
        "legacy engine-navigation ADR path remains referenced: "
        + json.dumps(stale_navigation_adr_references)
    )
