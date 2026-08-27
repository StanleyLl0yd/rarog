from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"marker not found in {path}: {old[:80]!r}")
    file.write_text(text.replace(old, new, 1))

replace(
    "README.md",
    "   ├─ vertical-footprint geometry change → retain Layout Tree + rebuild Fragment Tree\n",
    "   ├─ vertical-footprint geometry change → retain Layout Tree + flow-aware suffix relayout\n",
)
replace(
    "README.md",
    "- a stateful `RenderSession` with paint-only reuse, footprint-safe subtree Fragment relayout, whole-Fragment-Tree geometry fallback from a retained Layout Tree, and conservative full rebuild for structural/text/display-membership changes;",
    "- a stateful `RenderSession` with paint-only reuse, footprint-safe subtree Fragment relayout, flow-aware vertical suffix relayout from a retained Layout Tree, whole-Fragment-Tree fallback when local flow mapping is not provably safe, and conservative full rebuild for structural/text/display-membership changes;",
)
replace(
    "README.md",
    "The incremental experiment is intentionally narrow. It now proves paint-only retained updates, subtree-local Fragment relayout for geometry changes that preserve vertical flow footprint, whole-Fragment-Tree relayout when vertical flow may move siblings, and damage-scoped software raster updates. It does **not** yet claim general CSS incremental reflow, fragmentation-aware retained painting, standards-complete invalidation or measured performance gains.",
    "The incremental experiment is intentionally narrow. It now proves paint-only retained updates, subtree-local Fragment relayout for geometry changes that preserve vertical flow footprint, ancestor/sibling-aware suffix reflow for vertical-footprint changes in the current root block flow, conservative whole-Fragment-Tree fallback when that mapping is not safe, and damage-scoped software raster updates. It does **not** yet claim general CSS incremental reflow, nested formatting-context-local propagation, fragmentation-aware retained painting, standards-complete invalidation or measured performance gains.",
)
replace(
    "README.md",
    "cargo test -p rarog-engine vertical_geometry_change_uses_full_fragment_relayout",
    "cargo test -p rarog-engine vertical_geometry_change_reflows_ancestors_and_following_siblings",
)

replace(
    "docs/ARCHITECTURE.md",
    "  ├─ vertical-footprint geometry change → retain Layout Tree + rebuild Fragment Tree\n",
    "  ├─ vertical-footprint geometry change → retain Layout Tree + flow-aware suffix relayout\n",
)
replace(
    "docs/ARCHITECTURE.md",
    "5. if height or vertical margin/padding/border can move following siblings, retain the Layout Tree but rebuild the whole Fragment Tree;\n6. structural mutations, text changes, display-membership changes or unprovable cases use the deterministic full-rebuild fallback.",
    "5. if height or vertical margin/padding/border can move following siblings, retain the Layout Tree, find the earliest root block-flow child containing a dirty node, preserve the preceding Fragment prefix, and rebuild that child plus all following siblings;\n6. if the dirty nodes cannot be mapped safely to the current root flow, fall back to whole-Fragment-Tree geometry relayout; structural mutations, text changes, display-membership changes or other unprovable cases use the deterministic full-rebuild fallback.",
)
replace(
    "docs/ARCHITECTURE.md",
    "This proves narrower retained work boundaries and pixel-equivalent damage rasterization, **not a measured end-to-end performance win**. General ancestor/sibling reflow, fragmentation-aware retained painting, stacking/clip/transform-aware damage and compositor integration remain later work.",
    "This proves narrower retained work boundaries and pixel-equivalent damage rasterization, **not a measured end-to-end performance win**. Nested formatting-context-local reflow, fragmentation-aware retained painting, stacking/clip/transform-aware damage and compositor integration remain later work.",
)
replace(
    "docs/ARCHITECTURE.md",
    "The stateful incremental tests add invariants for paint-only geometry preservation, footprint-safe subtree relayout, whole-Fragment-Tree fallback when vertical flow can move siblings, retained display-list replacement, and damage-scoped raster output equivalence with a full reraster.",
    "The stateful incremental tests add invariants for paint-only geometry preservation, footprint-safe subtree relayout, root-flow ancestor/sibling-aware vertical reflow with full-render equivalence, conservative whole-Fragment-Tree fallback, retained display-list replacement, and damage-scoped raster output equivalence with a full reraster.",
)
replace(
    "docs/ARCHITECTURE.md",
    "See ADR-0009.",
    "See ADR-0009 and ADR-0010.",
)

replace(
    "docs/R0-BACKLOG.md",
    "- [ ] ancestor/sibling-aware local reflow for vertical-footprint changes",
    "- [x] first ancestor/sibling-aware local reflow for vertical-footprint changes in the root block-flow context",
)
replace(
    "docs/R0-BACKLOG.md",
    "Incremental frames must additionally expose which path ran (`unchanged`, `paint-only reuse`, `subtree relayout`, `geometry relayout`, `full rebuild`) and how many nodes were dirtied/patched.",
    "Incremental frames must additionally expose which path ran (`unchanged`, `paint-only reuse`, `subtree relayout`, `flow relayout`, `geometry relayout`, `full rebuild`) and how many nodes were dirtied/patched.",
)
replace(
    "docs/R0-BACKLOG.md",
    "The stateful R0 path must also prove paint-only reuse, footprint-safe subtree relayout, deterministic whole-Fragment-Tree fallback for geometry that can move siblings, and a deterministic full rebuild for structural changes.",
    "The stateful R0 path must also prove paint-only reuse, footprint-safe subtree relayout, ancestor/sibling-aware root-flow reflow for vertical-footprint changes with full-render equivalence, conservative whole-Fragment-Tree geometry fallback, and a deterministic full rebuild for structural changes.",
)

replace(
    ".github/workflows/ci.yml",
    "cargo test -p rarog-engine vertical_geometry_change_uses_full_fragment_relayout",
    "cargo test -p rarog-engine vertical_geometry_change_reflows_ancestors_and_following_siblings",
)
replace(
    ".github/workflows/ci.yml",
    "cargo test -p rarog-engine vertical_geometry_change_uses_full_fragment_relayout",
    "cargo test -p rarog-engine vertical_geometry_change_reflows_ancestors_and_following_siblings",
)
