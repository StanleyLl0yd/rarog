# Rarog v0.1 Architecture

## Mission

Rarog is an independent Web engine intended to make modern Web content cheaper to execute without reducing compatibility or weakening security.

Primary promise:

> **Modern Web without the cost of Chromium.**

Engineering motto:

> **Compatible without becoming Chromium.**

## Platform priority

Rarog is **Windows-first**.

The first production target is Windows 10/11, followed by other desktop platforms when the engine is mature enough to justify the porting work. This affects prioritization, test coverage and platform integration, but not the boundaries of the engine core.

Windows-specific APIs must stay behind platform adapters. The DOM, HTML, CSS, layout, script-facing Web platform and compatibility layers must not depend directly on Win32, WinRT, Direct3D or other Windows-only interfaces.

The first implementations of the following platform surfaces will therefore be Windows implementations:

- window/event integration;
- text and font platform integration;
- keyboard, mouse, touch and IME input;
- clipboard and drag-and-drop;
- accessibility bridge;
- sandbox/process hardening;
- GPU/compositor backend integration;
- file dialogs and OS capability brokering.

Zorya Browser is the reference host and will also target Windows first.

See ADR-0006.

## Architectural invariants

1. **Compatibility is the first product requirement.** Standards conformance and real-Web behavior are measured separately.
2. **Rust-first.** New engine-owned components use safe Rust by default. `unsafe` is forbidden at workspace level in bootstrap code and later isolated into audited platform crates where unavoidable.
3. **Host and Web content are different trust domains.** Web content must never directly own OS capabilities.
4. **Site isolation is not traded for RAM.** Resource savings come from compact processes, lifecycle management, sharing immutable state and explicit budgets.
5. **Rendering is incremental and task-graph oriented.** Work is invalidated at the smallest practical granularity and parallelized only where semantics allow it.
6. **Embedding is a first-class product.** Zorya is the reference browser, not the only possible host.
7. **The standards engine stays clean.** Site-specific compatibility behavior belongs to a separate, auditable compatibility subsystem.
8. **Dependencies are replaceable behind adapters.** SpiderMonkey, networking backends, graphics APIs and platform integrations must not leak throughout the Web platform implementation.
9. **DOM, layout and fragments have different identities and lifetimes.** DOM is mutable source state; layout nodes and fragments are derived snapshots and may be discarded or rebuilt at any time.
10. **Layout never paints directly.** Paint consumes derived fragments and emits a display list.
11. **Cascade and invalidation are explicit data flows.** DOM/style mutations produce dirty information; they do not silently mutate layout or paint state behind subsystem boundaries.
12. **Determinism is an R0 correctness requirement.** Equivalent input on the same architecture/toolchain must produce equivalent snapshots, display items and framebuffer hashes.
13. **Incremental rendering is an optimization with a full-rebuild fallback.** Reuse is allowed only when the engine can prove that the affected derived state remains valid.

## Long-term process topology

```text
Host application (Zorya / Rarog View embedder)
                  │
                  ▼
          Rarog Host Process
      policy · navigation · broker
        │         │          │
        │         │          ├───────────────┐
        ▼         ▼                          ▼
   Site Proc A  Site Proc B              Utility Procs
   DOM/JS/style DOM/JS/style          network/storage/media
        │         │                          │
        └────┬────┘                          │
             ▼                               │
       Compositor/GPU ◄──────────────────────┘
```

The v0.1 bootstrap runs in one process, but its crate boundaries intentionally mirror future security/process boundaries.

## Rendering model

```text
bytes
  ↓
HTML tokenizer/tree builder
  ↓
mutable DOM + generation-ordered mutation records
  ↓
stylesheet sources / selector matching / cascade
  ↓
computed style + invalidation keys
  ↓
persistent engine dirty state
  ├─ paint-only computed-style change → reuse geometry + retained paint update
  ├─ footprint-safe geometry change → subtree Fragment relayout
  ├─ vertical-footprint geometry change → retain Layout Tree + flow-aware suffix relayout
  └─ structure/text/display-membership change → deterministic full rebuild
  ↓
derived Layout Tree
  ↓
derived Fragment Tree
  ↓
stable display-item IDs + damage comparison
  ↓
compositor / raster backend
  ↓
pixels + deterministic hash
```

## DOM mutation boundary

`rarog-dom` owns tree invariants. Callers do not directly repair parent/child relationships after a mutation.

The R0 mutation surface establishes these rules:

- the document root cannot be reparented or detached;
- text nodes cannot have children;
- a mutation that would create a cycle is rejected;
- reparenting updates both the old and new parent relationships;
- element/text changes advance the document generation only when state actually changes;
- detached nodes are valid DOM objects;
- `validate_invariants` is available for deterministic tests and debug checks.

Each accepted mutation also records a generation-ordered `MutationRecord`. The record describes the minimum semantic change — node creation, child insertion/reparenting, attribute change or character-data change — without importing CSS/layout types into the DOM crate. Downstream invalidation code consumes these records through a generation boundary. `Document` also tracks a mutation-history floor; once the active engine consumer has advanced through a generation, older records are pruned so a long-lived document does not retain an unbounded journal. Requests older than the retained floor fail loudly instead of silently producing incomplete invalidation input. `RenderSession` owns that checkpoint: its public mutation surface is a `DocumentEditor` that exposes DOM mutations but not journal pruning, so an embedder cannot invalidate the session's dirty-generation contract behind the engine.

This keeps the direction of dependency clear:

```text
DOM mutation record
      ↓
style/layout invalidation policy
      ↓
persistent dirty state
      ↓
incremental reuse or deterministic rebuild
```

The DOM does not know which selectors, layout nodes or paint items depend on a mutation.

### Element names, namespaces and atoms

R0 stores an explicit `Namespace` on every `ElementData` and represents the local element name with an immutable `Atom`. The bootstrap HTML parser assigns `Namespace::Html` only; SVG/MathML tree-building and namespace switching remain standards-parser work. Non-HTML namespaces can already be represented by the DOM without encoding namespace state into tag-name strings.

`Atom` is the semantic boundary for frequently repeated engine-owned names. Its R0 storage is a cheap cloneable `Arc<str>` handle, not a process-global interning table. The long-term strategy is document/process-scoped canonical interning behind the same boundary once measurements justify it. Text-node contents and attribute values remain ordinary owned strings. A process-global immortal string table is intentionally rejected because it conflicts with bounded lifetimes, site isolation and explicit resource budgets. See ADR-0024.

## HTML parsing boundary

`rarog-html` exposes a decoded streaming-input contract independently of the bootstrap parser implementation. `StreamingInput` accepts UTF-8 chunks and closes explicitly; source spans in parser diagnostics are UTF-8 byte offsets in that decoded stream. Transport bytes and encoding detection/decoding stay outside this R0 interface.

Recoverable syntax problems produce deterministic `ParseDiagnostic` records with a code, severity, source span and message. Contract failures that prevent parsing from starting or completing use `Result::Err`. The legacy `parse(&str) -> Document` entry point remains a convenience wrapper, while `parse_with_diagnostics` and `parse_stream` expose the reporting boundary.

The R0 implementation buffers chunks until end of input and then runs the bootstrap parser. This proves ownership and reporting contracts only; it does **not** claim incremental tokenization or WHATWG HTML conformance. R1 replaces the bootstrap algorithm behind the adapter with a standards-oriented tokenizer/tree builder without leaking implementation-specific token or node types into DOM/layout callers. See ADR-0025.

## Style source, selector and cascade boundary

R0 has explicit bootstrap representations for:

- `StyleSourceId` and source labels;
- cascade origin (`UserAgent`, `Author`, `Inline`);
- `CascadeLayer` data even though `@layer` parsing is not implemented yet;
- simple selector components: type, ID and class;
- selector specificity;
- typed bootstrap `PropertyId` / `PropertyValue` pairs;
- stylesheet rule source order.

The current cascade priority is deterministic and compares:

```text
origin → layer → specificity → sheet order → rule order → declaration order
```

The bootstrap document style set contains a tiny Rarog user-agent sheet, author `<style>` elements in document order and inline `style` declarations. This is an architectural foundation, **not CSS Cascade compliance**. Importance, inheritance, CSS-wide values, selector combinators, pseudo-classes, namespaces and standards parsing remain later work.

### Invalidation primitives

Selectors expose a `SelectorInvalidationKey` containing the tag/ID/class keys that can make the selector relevant. `InvalidationSet::from_document_since` converts DOM mutation records into conservative dirty flags:

```text
style dirty
layout dirty
paint dirty
```

For the current simple-selector bootstrap:

- `id`, `class` and `style` attribute changes invalidate style and downstream layout/paint for the changed element;
- character-data changes invalidate layout/paint and affected ancestors;
- child insertion/reparenting invalidates the moved/inserted node and ancestor geometry;
- a stylesheet-source change can invalidate the connected document subtree.

These flags are deliberately conservative. `rarog-engine` persists them in `DirtyState` across DOM generations until a render update consumes them.

### Relational invalidation and style sharing

R0 now has an explicit `SelectorInvalidationDependencies` boundary for selector relationships that can make a mutation affect nodes other than the mutated element. A dependency records the local trigger key plus a conservative scope: descendants or following siblings. The bootstrap CSS parser still accepts only simple selectors, so it produces no relational dependencies itself; a future standards parser can populate the same rule-level dependency metadata without changing the DOM mutation journal or engine dirty-state API.

Attribute invalidation deliberately keys on the changed attribute category (`id` or `class`) rather than only the post-mutation value. This is necessary because the R0 mutation journal does not retain old attribute values: removing a trigger must invalidate the same dependent nodes as adding it. Structural insert/reparent operations conservatively invalidate affected descendant or sibling subtrees when the corresponding dependency scope exists.

`StyleSharingKey` captures the local inputs that are sufficient for the current bootstrap selector/cascade model: namespace, tag, ID, canonicalized classes and inline style. Local style sharing is considered safe only while the active rule set has no relational dependencies. R0 does not install a process-global computed-style cache; any future cache must be bounded to a document/style-set lifetime and must expand or disable its key when inheritance, pseudo-state, relational selectors or other contextual inputs become observable. See ADR-0026.

## R0 observability and benchmark harness

Full bootstrap renders expose `RenderObservability` without feeding timing data into deterministic render identity. `RenderTimings` records wall-clock durations for decoded HTML parsing, style-source construction, Layout Tree construction, Fragment Tree construction, display-list/damage construction, rasterization, and the enclosing render. `RenderCounters` records DOM nodes, layout nodes, fragments, display commands, and damage rectangles. Layout Tree construction currently includes per-element computed-style resolution because R0 resolves styles while deriving layout nodes.

Stateful updates expose elapsed wall-clock time alongside the existing `IncrementalMode`, dirty-node count and patched-node count. These values are diagnostics only: CI does not enforce thresholds and the project makes no cross-machine performance claims from them. Allocator-backed peak/persistent byte accounting is deliberately deferred rather than publishing misleading estimates.

`cargo run -p rarog-engine --example r0_bench --release -- <iterations>` runs fixed full-render, paint-only, subtree-relayout and flow-relayout scenarios. Setup for each incremental sample is excluded from the reported update duration through the engine's own timing boundary. The harness is intended to detect gross regressions during development and to provide a stable place for later benchmark methodology, not to publish competitive numbers. See ADR-0028.

## First incremental reuse experiment

R0 now has a stateful `RenderSession` that owns the current document, styles, Layout Tree, Fragment Tree, display list, framebuffer and persistent dirty state.

The current reuse path is intentionally conservative:

1. collect DOM mutations since the last consumed generation and accumulate dirty entries;
2. recompute affected element styles for `id`, `class` and inline `style` mutations;
3. patch paint-only changes onto retained Layout/Fragment state;
4. for geometry changes that preserve vertical flow footprint, rebuild only the affected Fragment subtree from its parent's content-box containing block;
5. if height or vertical margin/padding/border can move following siblings, retain the Layout Tree, find the earliest root block-flow child containing a dirty node, preserve the preceding Fragment prefix, and rebuild that child plus all following siblings;
6. if the dirty nodes cannot be mapped safely to the current root flow, fall back to whole-Fragment-Tree geometry relayout; structural mutations, text changes, display-membership changes or other unprovable cases use the deterministic full-rebuild fallback.

The geometry-affecting comparison includes width, height, margin, border width, padding and `display`. Background and border color remain paint-only values. The current subtree-safety rule deliberately treats only vertical footprint as the hard flow boundary because the bootstrap text path does not wrap yet. This rule must become formatting-context-aware before it can represent general CSS incremental reflow.

Paint now retains unaffected display-list ranges when an affected fragment subtree already has a stable command range. If that range cannot be patched safely, the engine regenerates the display list. Damage is still derived by stable display-item identity. The persistent software framebuffer is then cleared and rerasterized only inside the resulting damage rectangles, with commands clipped to each damaged rectangle.

This proves narrower retained work boundaries and pixel-equivalent damage rasterization, **not a measured end-to-end performance win**. Nested formatting-context-local reflow, fragmentation-aware retained painting, stacking/clip/transform-aware damage and compositor integration remain later work.

See ADR-0009 and ADR-0010.

## Layout and Fragment Tree

R0 has three distinct representations:

```text
DOM NodeId
   │ source relationship
   ▼
LayoutNodeId
   │ can produce one or more fragments later
   ▼
FragmentId
```

The IDs are intentionally different types. Numeric equality has no semantic meaning across these domains.

A `LayoutTree` is derived from DOM + computed style. It may contain anonymous layout nodes later and therefore stores its DOM source as optional metadata rather than treating DOM identity as layout identity.

A `FragmentTree` is the geometry snapshot consumed by paint. Today the bootstrap mostly produces one fragment per layout node. The API does **not** depend on that assumption; later inline fragmentation, pagination, multicolumn layout and generated/anonymous boxes may produce multiple fragments for one layout node.

Derived does not mean that every frame must rebuild these structures. ADR-0009 allows an existing derived snapshot to be reused when the engine proves that its geometry remains valid; otherwise it remains freely disposable/rebuildable.

See ADR-0007.

## Containing blocks, intrinsic sizing and text runs

R0 now passes an explicit `ContainingBlock` through fragment construction instead of coupling layout to raw x/available-width arguments. A containing block carries an origin and available size, and nested block content becomes the containing block for descendants. This is a bootstrap foundation for later formatting-context-specific containing-block rules, not CSS containing-block compliance.

Layout nodes also expose `IntrinsicSizes { min_content, max_content }`. Text is represented as a backend-neutral `TextRun` carrying bootstrap advance and line-height metrics; its intrinsic sizes distinguish the longest unbreakable word from the full run advance. No shaping backend, font selection or Unicode line-breaking contract is implied yet.

## Box model foundation

Each box fragment carries four explicit rectangles:

```text
margin box
└─ border box
   └─ padding box
      └─ content box
```

R0 supports bootstrap values for:

- `width` / `height`;
- `margin` and individual margin edges;
- `padding` and individual padding edges;
- `border-width` and individual border-width edges;
- `border-color`;
- background color;
- `display: none` / `display: block` for bootstrap cascade decisions.

This is a geometry foundation, **not** a claim of CSS box-model compliance. Margin collapsing, intrinsic sizing, min/max constraints, percentages, writing modes and formatting-context-specific behavior remain later work.

## Paint identity and damage tracking

The display list remains backend-neutral. R0 `DisplayItemId` values now contain three explicit components: source identity, Fragment identity and paint-command slot. This prevents two fragments produced from the same DOM/layout source from colliding once fragmentation begins. Generated display lists assert ID uniqueness, and damage comparison rejects duplicate IDs instead of silently overwriting them in its index. Fragment identity is still snapshot-oriented in R0; retained/stable fragment ordinals remain a later fragmentation concern.

Clip commands are explicit backend-neutral display-list operations. R0 rasterization maintains a nested rectangular clip stack. Damage-scoped rasterization conservatively falls back to a full framebuffer refresh whenever clips are present; clip-aware retained damage remains intentionally deferred until stacking and fragmentation semantics are defined.

Stacking contexts, transforms and opacity are represented as explicit balanced display-list scopes. `Transform2D` is a backend-neutral affine transform and `Opacity` is a clamped scalar. The R0 software raster path applies nested transforms to rectangular paint bounds, intersects transformed clips in device space and source-over blends opacity-modulated colors. This remains a bootstrap raster model: it does not define CSS transform-origin, stacking order, isolation groups or compositor surfaces.

Retained display-list replacement operates on exact contiguous command ranges rather than unordered ID sets. A patch is accepted only when the live range still contains the exact previous commands, the range begins and ends in the same outer structural scope state, and the replacement/result preserve unique IDs and balanced clip/stacking/transform/opacity scopes. Because display-item identity includes fragment ordinal, one fragment can be patched inside nested stacking/clip scopes without colliding with sibling fragments from the same source node.

Fragment identity is explicitly one-to-many with layout identity. A layout node may emit multiple fragments, each carrying a stable ordinal within that source node. The R0 proof case uses bootstrap fixed-advance text fragmentation in narrow containing blocks; it is an architectural multiplicity test, not a standards line-breaking implementation. Display-item identity uses the fragment ordinal rather than the ephemeral FragmentId so multiple fragments remain distinct without coupling retained paint to snapshot allocation order.

Text fragmentation now records explicit source-character `TextRange` values and `LineBox` geometry. Line breaking is isolated behind the `LineBreaker` abstraction; R0 uses a deterministic fixed-advance implementation so future shaping, font metrics, bidi, and standards line breaking can replace policy without changing fragment identity or retained-paint contracts.

R1 extends this boundary with a conservative inline-formatting-context foundation. Eligible unsized inline containers can fragment across line boxes instead of being forced into one atomic box; first/middle/last fragments slice horizontal margin, border and padding edges, nested single-leaf inline owner chains preserve real fragment-tree ownership, and eligible pure-inline subtrees can now stream multiple nested and sibling text leaves through the same line sequence while producing at most one fragment per owner per line. Shared owner paths are reused line-locally, owner fragment ordinals remain stable across line continuation, and first/middle/last horizontal edges are applied over the complete descendant span rather than per text leaf. Unsupported inline structures, empty nested owners and explicit sizing keep the existing atomic fallback rather than receiving approximate semantics. This completes the scoped R1 inline-formatting-context foundation; it is not a claim of complete CSS inline formatting or fragmentation behavior.

Text measurement is separated from layout through `TextShaper`, `ShapedText`, `GlyphCluster`, and `FontMetrics`. The bootstrap shaper emits one fixed-advance cluster per source character, while line breaking consumes cluster advances rather than assuming character width. This keeps shaping/font selection replaceable and makes variable-width or multi-codepoint clusters possible without redesigning the fragment contract.

Line breaking now consumes explicit Unicode-aware break opportunities. R0 recognizes mandatory Unicode separators, breakable Unicode whitespace, hyphen opportunities, non-breaking spaces, and basic CJK ideographic boundaries. This is intentionally a deterministic UAX #14-oriented bootstrap subset, not a claim of full Unicode Line Breaking Algorithm conformance.

Grapheme safety is enforced before shaping and line breaking: `TextRange` remains scalar-index based, while `GlyphCluster` may cover multiple scalar values. The deterministic R0 classifier keeps combining marks, variation selectors, emoji modifiers, CRLF, regional-indicator pairs, and basic emoji ZWJ sequences indivisible. This is a UAX #29-oriented bootstrap subset rather than full conformance.

Damage is computed by comparing previous and current display lists by item ID:

- unchanged item and command → no damage;
- changed item → old and new command bounds are damaged;
- removed item → old bounds are damaged;
- new item → new bounds are damaged.

The current `DamageRegion` intentionally stores conservative rectangles without advanced coalescing. For structural display lists it derives conservative device-space paint bounds through transform and clip scopes; structural damage rasterization still uses a full-frame refresh so correctness does not depend on partial replay across compositing scopes. R0 can replace the stable command range belonging to an affected fragment subtree and preserve unrelated commands; if a stable previous range or structural proof does not exist, it falls back to display-list regeneration. Occlusion, CSS stacking-order semantics, isolated opacity groups and compositor damage remain later work.

See ADR-0008.

## Deterministic R0 snapshots

The R0 pipeline exposes deterministic textual snapshots for:

- DOM arena state;
- stylesheet/source/rule structure;
- computed styles carried by the Layout Tree;
- Layout Tree identity/shape;
- Fragment Tree geometry;
- display-item IDs and commands.

CSS bootstrap length parsing rejects non-finite values before they enter computed geometry. The software framebuffer enforces a checked R0 pixel budget before allocation, and the public render/session construction boundary returns a `RenderError` rather than panicking for invalid or oversized viewports. The framebuffer exposes a stable 64-bit FNV-1a hash over dimensions and RGBA pixels. `rarog-engine` combines the textual snapshots and framebuffer hash into a deterministic render-signature hash used as a regression gate.

This is not a cryptographic hash and must never be used for security decisions. It is a small deterministic regression fingerprint for R0.

The stateful incremental tests add invariants for paint-only geometry preservation, footprint-safe subtree relayout, root-flow ancestor/sibling-aware vertical reflow with full-render equivalence, conservative whole-Fragment-Tree fallback, retained display-list replacement, and damage-scoped raster output equivalence with a full reraster.

## Important separation

- DOM is mutable script-visible state.
- Stylesheets/selectors/cascade produce computed style; they do not own layout objects.
- Dirty state belongs to engine orchestration and is derived from DOM generations/invalidation policy.
- Layout Tree is derived state and must remain safely disposable/rebuildable even when a valid snapshot is reused.
- Fragment Tree is derived geometry and must remain safely disposable/rebuildable even when a valid snapshot is reused.
- Paint output is a display list, not direct drawing from layout code.
- Damage is derived from display-list differences, not from layout drawing side effects.
- The compositor consumes snapshots; it does not mutate DOM/layout.
- Platform code consumes engine output through adapters; core Web semantics do not depend on Windows APIs.

This separation is required for later incremental invalidation, parallelism, process isolation, GPU composition and crash recovery.

## Platform host boundary

R0 isolates host-platform integration behind two crate layers. `rarog-platform` owns the platform-neutral `PlatformHost` and `PlatformCapabilities` contract consumed by `rarog-engine`. `rarog-platform-windows` is the first target-specific host boundary; engine core never depends on that Windows crate.

`EngineBuilder` accepts a platform host and defaults to `NullPlatformHost`, so headless tests and portability lanes do not need to impersonate a desktop integration. The engine exposes only the host name and capability data. No Win32, WinRT, DirectWrite, Direct3D, HWND, COM, or other Windows-specific type enters DOM/HTML/CSS/layout/paint or the embedder API.

The Windows boundary deliberately advertises no concrete service capability in R0. Window/events, font/text, input/IME, accessibility, sandbox/process, and GPU/compositor adapters become capabilities only when real implementations exist. `WindowsPlatformHost::try_new` succeeds only on a Windows compilation target, while the crate itself remains buildable on Linux for portability CI. See ADR-0030.

## Engine and embedder boundary

R0 exposes `Engine` and `View` above `RenderSession`. `Engine` owns shared host policy, UI-neutral event delivery, resource budgets and stable `ViewId` allocation; each `View` owns one loaded inline document and the render session derived from it. This keeps browser-shell ownership out of DOM/layout/paint crates and gives later process isolation a stable host-facing seam.

Navigation and subresource loading are contracts only in R0. `NavigationRequest` and `ResourceRequest` are checked by `HostPolicy` and return either `Blocked` or `ForwardToEmbedder`; the engine performs no network I/O. The same actions are surfaced as `ViewEvent` values through `EventSink`, which has no dependency on a UI toolkit or Windows API. An embedder can therefore decide how to obtain bytes and then call `View::load_html` with decoded text and an opaque `BaseUrl`.

`ResourceBudget` begins with enforced document-source and viewport-pixel limits. The viewport limit cannot exceed the lower-level framebuffer safety cap. Memory/cache/background CPU and lifecycle budgets remain future extensions rather than invented R0 accounting. `View::render` creates a stateful render session on first use, reuses it for an unchanged viewport, and performs a deterministic full session rebuild when the viewport changes. See ADR-0029.

## Script architecture

Rarog 1.x should initially integrate SpiderMonkey through one replaceable abstraction:

```text
DOM/Web APIs
    ↓ WebIDL bindings
Rarog Script API
    ↓
SpiderMonkey adapter
```

No engine crate outside the script adapter should depend directly on SpiderMonkey APIs.

## Resource model

Every top-level site receives a `ResourceBudget` containing at minimum:

- resident memory target;
- decoded image cache target;
- graphics cache target;
- background CPU allowance;
- timer/rendering policy;
- lifecycle state.

Lifecycle states:

```text
Active → VisibleIdle → Background → Frozen → Discardable
```

Security boundaries are preserved regardless of lifecycle state.

## Security model

A future site process can request operations but cannot directly access privileged OS resources. The Host/Broker issues scoped capabilities, for example:

```text
CameraCapability(origin, device, expiry)
FileReadCapability(origin, path-scope, expiry)
ClipboardReadCapability(origin, expiry)
ScreenCaptureCapability(origin, target, expiry)
```

Capabilities are origin-bound, operation-bound and revocable.

On Windows, sandbox/process primitives will be implemented behind the Host/Broker platform layer rather than exposed to Web-facing crates.

## Compatibility model

Two independent test tracks are mandatory:

### Standards

- Web Platform Tests (WPT)
- ECMAScript/Test262 through the selected JS engine
- WebDriver/WebDriver BiDi tests

### Real Web

`rarog-web-corpus` will maintain reproducible scenarios for popular sites and applications. Compatibility fixes must live in a separately versioned `rarog-compat` subsystem rather than site-name branches in layout/DOM code.

## CI platform policy

Windows is the primary CI platform lane. It must run format, compile checks, Clippy, workspace tests, the deterministic-render gate and the bootstrap render.

Linux remains a portability lane from R0 onward so accidental Windows-only dependencies in engine-core crates are caught early. It also runs the workspace tests and bootstrap render; deterministic behavior is therefore exercised on both lanes even though Windows is the product-priority gate.

When macOS support becomes an active target it should gain an equivalent portability lane, but absence of a macOS lane must not block Windows-first engine progress.

## Why the bootstrap renderer is deliberately small

The first milestone proves the interfaces:

```text
parse → DOM → style/cascade → dirty state → Layout Tree → Fragment Tree → display list/damage → framebuffer
```

It is not a standards claim. A small end-to-end pipeline lets us replace parsing, selector, cascade, layout and raster implementations without rewriting host and test infrastructure.

### Bidirectional text foundation

R0 now exposes explicit `TextDirection`, `BidiLevel`, and `BidiRun` values. Paragraph direction is derived from the first strong character and mixed strong-direction spans are represented as scalar-indexed runs. `visual_bidi_runs()` performs deterministic level-based run reordering while leaving grapheme, shaping, line-breaking, fragment, and retained-paint identities unchanged. This is a UAX #9-oriented bootstrap boundary, not full Unicode Bidirectional Algorithm conformance.

### Font fallback foundation

R0 now models font selection explicitly through `FontFaceId`, `FontFamily`, `FontFace`, `FontFallbackChain`, and scalar-indexed `FontRun` values. Fallback selection occurs only on grapheme-cluster boundaries, so combining sequences and emoji ZWJ clusters cannot be split between faces. The deterministic bootstrap chain covers Latin/Cyrillic, Hebrew/Arabic, CJK, emoji, and a mandatory LastResort face. These are architectural coverage classes rather than bundled fonts; a platform font database and real shaping backend can replace the selector without changing source, bidi, fragment, or retained-paint identities.

### Shaping segmentation foundation

R0 now derives scalar-indexed `ShapingRun` segments by intersecting logical bidi runs with grapheme-safe font fallback runs. Every shaping segment has exactly one source range, one `FontFaceId`, and one `BidiLevel`/direction, and adjacent segments with identical shaping state are coalesced. This is the handoff boundary for a future OpenType shaper: a real backend can shape each segment independently without changing DOM, source ranges, line breaking, fragment identity, or retained paint.

### Shaping backend boundary

R0 now separates shaping segmentation from shaping execution. `ShapingBackend` receives one `ShapingRun` plus its selected `FontFace` and returns a `ShapedRun` containing positioned glyph IDs, per-glyph advances/offsets, and scalar-indexed source-cluster mapping. The deterministic `FixedTextShaper` implements this contract as the bootstrap backend while the existing aggregate `ShapedText` remains the line-breaking input. A future OpenType backend can therefore replace glyph generation without owning bidi, font fallback, source identity, fragmentation, or retained-paint policy.

### Shaping request metadata

R0 now carries backend-neutral shaping metadata in `ShapingRequest`. Every request preserves one resolved `ShapingRun` and adds script classification, a normalized language tag, OpenType feature settings, and variation-axis coordinates. Bootstrap requests infer script deterministically from the scalar-indexed source range and default language to `und`; feature and variation vectors are empty unless the caller explicitly configures them. The deterministic bootstrap backend accepts this metadata but intentionally ignores feature/variation semantics, leaving those decisions to a future OpenType implementation behind the same `ShapingBackend` boundary.

