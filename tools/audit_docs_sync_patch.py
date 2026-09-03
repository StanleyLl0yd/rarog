from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected exactly one match in {path}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "docs/METRICS.md",
    "Paint retains unaffected display-list ranges when a replacement is structurally valid. The persistent software framebuffer is then updated inside damage rectangles for non-structural display lists. Structural clip/stacking/transform/opacity scopes currently force conservative full-frame raster refreshes where damage-scoped replay is not yet proven safe.",
    "Paint retains unaffected display-list ranges when a replacement is structurally valid. The persistent software framebuffer is updated only inside the resulting damage rectangles. Damage replay reconstructs clip, stacking, transform and opacity scopes while clipping raster work to each damaged rectangle, and structural damage is derived from effective per-item transform, clip, opacity and paint-order state rather than invalidating every paint bound.",
)
replace_once(
    "docs/METRICS.md",
    "- full-frame raster fallbacks caused by structural display scopes;",
    "- partial-damage replay involving structural display scopes;",
)

replace_once(
    "CONTRIBUTING.md",
    "cargo test -p rarog-engine --test r01_correctness\ncargo run -p rarog-shell -- examples/hello.html rarog.ppm",
    "cargo test -p rarog-engine --test r01_correctness\ncargo test -p rarog-engine --test r1_exit\ncargo run -p rarog-shell -- examples/hello.html rarog.ppm",
)
replace_once(
    "CONTRIBUTING.md",
    "When changing invalidation or incremental rendering, add tests for both the reuse path and the conservative fallback. A paint-only mutation should prove which derived state was reused; a geometry/structure mutation should prove that the fallback still produces correct deterministic output. The dedicated `r01_correctness` integration target is the required high-level regression gate; unit tests remain the place for narrow subsystem invariants.",
    "When changing invalidation or incremental rendering, add tests for both the retained path and the conservative fallback. A paint-only mutation should prove which derived state was reused; text, geometry and structural mutations should prove retained identities where the implementation claims reuse and deterministic equivalence when a fallback is required. The dedicated `r01_correctness` integration target remains the high-level render-correctness gate, while `r1_exit` protects the current R1 standards-oriented path; unit tests remain the place for narrow subsystem invariants.",
)

replace_once(
    "fuzz/README.md",
    "- `html_parse` feeds arbitrary UTF-8 HTML into the bootstrap parser and asserts that the resulting DOM preserves native invariants.\n- `render_html` feeds arbitrary UTF-8 HTML through the full render boundary with a small viewport. Any controlled `RenderError` is acceptable; a panic, abort, invariant failure, or memory-safety failure is not.",
    "- `html_parse` feeds arbitrary UTF-8 HTML into the standards-oriented parser adapter and asserts that the resulting DOM preserves native invariants.\n- `css_stylesheet` feeds arbitrary UTF-8 CSS into stylesheet parsing so malformed selector/declaration input must remain controlled and non-panicking.\n- `render_html` feeds arbitrary UTF-8 HTML through the full render boundary with a small viewport. Any controlled `RenderError` is acceptable; a panic, abort, invariant failure, or memory-safety failure is not.",
)
replace_once(
    "fuzz/README.md",
    "cargo fuzz run html_parse -- -max_len=1048576\ncargo fuzz run render_html -- -max_len=1048576",
    "cargo fuzz run html_parse -- -max_len=1048576\ncargo fuzz run css_stylesheet -- -max_len=1048576\ncargo fuzz run render_html -- -max_len=1048576",
)

arch = Path("docs/ARCHITECTURE.md")
text = arch.read_text()

render_re = re.compile(
    r"computed style \+ invalidation keys\n  ↓\npersistent engine dirty state\n.*?  ↓\nderived Layout Tree",
    re.S,
)
render_new = """computed style + invalidation keys
  ↓
persistent engine dirty state
  ├─ paint-only computed-style change → reuse geometry + retained paint update
  ├─ supported geometry change → retained subtree or root-flow relayout
  ├─ ordinary CharacterData change → retained text-node refresh + flow-aware fragment rebuild
  ├─ covered insertion/reparent/detach → retained structural-root refresh with stable existing LayoutNodeId identity
  ├─ connected stylesheet-source change → rebuild StyleSet + global retained-style revalidation
  └─ unprovable formatting/membership/history case → deterministic full rebuild
  ↓
derived Layout Tree"""
text, count = render_re.subn(render_new, text, count=1)
if count != 1:
    raise SystemExit("rendering model section anchor mismatch")

html_re = re.compile(r"## HTML parsing boundary\n.*?\n## Style source, selector and cascade boundary", re.S)
html_new = """## HTML parsing boundary

`rarog-html` exposes a decoded streaming-input contract independently of the parser backend. `StreamingInput` accepts UTF-8 chunks and closes explicitly; source spans in parser diagnostics are UTF-8 byte offsets in that decoded stream. Transport bytes and encoding detection/decoding stay outside this interface.

Recoverable syntax problems produce deterministic `ParseDiagnostic` records with a code, severity, source span and message. Contract failures that prevent parsing from starting or completing use `Result::Err`. The canonical entry points are `parse`, `parse_with_diagnostics` and `parse_stream`; `parse_standards*` names remain compatibility aliases rather than a separate parser path.

R1 routes parsing through the standards-oriented adapter backed by `html5ever`, then normalizes its result into Rarog-owned DOM identities and invariants. Streaming input is still buffered until close rather than incrementally tokenized across calls, but backend token/node types do not leak into DOM, layout or engine callers. The adapter boundary therefore preserves replaceability while the parser behavior follows the standards-oriented path. See ADR-0014 and ADR-0025.

## Style source, selector and cascade boundary"""
text, count = html_re.subn(html_new, text, count=1)
if count != 1:
    raise SystemExit("HTML parsing section anchor mismatch")

incremental_re = re.compile(r"## First incremental reuse experiment\n.*?\n## Layout and Fragment Tree", re.S)
incremental_new = """## Incremental reuse

`RenderSession` owns the current document, styles, Layout Tree, Fragment Tree, display list, framebuffer and persistent dirty state. The implementation remains conservative, but R1 now retains substantially more derived state than the original R0 experiment:

1. ordinary paint-only style changes patch retained layout/fragment styles and affected display items;
2. footprint-safe geometry changes may relayout an affected Fragment subtree;
3. vertical-flow changes and ordinary CharacterData mutations retain the Layout Tree and rebuild the affected root-flow suffix;
4. covered child insertion, reparent, detach and detached-subtree attachment refresh retained structural roots while preserving existing `LayoutNodeId` identity where the DOM node survives;
5. connected stylesheet-source changes rebuild the `StyleSet`, globally revalidate retained computed styles, and retain layout for supported paint/geometry changes while formatting or visibility-membership boundaries remain conservative fallbacks;
6. complex inline/formatting-boundary cases can refresh a retained parent structural root instead of forcing an unconditional document rebuild;
7. missing mutation history, unsupported membership transitions or any state whose correctness cannot be proven still use the deterministic full-rebuild fallback.

Retained paint can replace structurally valid display-list ranges and preserve unaffected ranges across flow relayout. Damage comparison uses stable display-item identity plus effective transform, clip, opacity, image and paint-order state. Damage rasterization replays the display list through clip/stacking/transform/opacity scopes while clipping writes to each damaged rectangle, so the presence of structural display commands alone no longer forces a full-frame raster pass.

These mechanisms establish correctness-preserving retained boundaries; they do **not** by themselves establish an end-to-end performance claim. Measurements remain governed by `docs/METRICS.md` and the benchmark harness.

See ADR-0009, ADR-0010 and ADR-0035 through ADR-0046.

## Layout and Fragment Tree"""
text, count = incremental_re.subn(incremental_new, text, count=1)
if count != 1:
    raise SystemExit("incremental section anchor mismatch")

arch.write_text(text)
