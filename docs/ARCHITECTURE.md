# Rarog v0.1 Architecture

## Mission

Rarog is an independent Web engine intended to make modern Web content cheaper to execute without reducing compatibility or weakening security.

Primary promise:

> **Modern Web without the cost of Chromium.**

Engineering motto:

> **Compatible without becoming Chromium.**

## Architectural invariants

1. **Compatibility is the first product requirement.** Standards conformance and real-Web behavior are measured separately.
2. **Rust-first.** New engine-owned components use safe Rust by default. `unsafe` is forbidden at workspace level in bootstrap code and later isolated into audited platform crates where unavoidable.
3. **Host and Web content are different trust domains.** Web content must never directly own OS capabilities.
4. **Site isolation is not traded for RAM.** Resource savings come from compact processes, lifecycle management, sharing immutable state and explicit budgets.
5. **Rendering is incremental and task-graph oriented.** Work is invalidated at the smallest practical granularity and parallelized only where semantics allow it.
6. **Embedding is a first-class product.** Zorya is the reference browser, not the only possible host.
7. **The standards engine stays clean.** Site-specific compatibility behavior belongs to a separate, auditable compatibility subsystem.
8. **Dependencies are replaceable behind adapters.** SpiderMonkey, networking backends, graphics APIs and platform integrations must not leak throughout the Web platform implementation.

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
DOM arena
  ↓
style matching + cascade
  ↓
layout tree / fragments
  ↓
display list
  ↓
compositor / raster backend
  ↓
pixels
```

### Important separation

- DOM is mutable script-visible state.
- Layout output is derived state and must be disposable/rebuildable.
- Paint output is a display list, not direct drawing from layout code.
- The compositor consumes snapshots; it does not mutate DOM/layout.

This separation is required for later parallelism, process isolation, GPU composition and crash recovery.

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

## Compatibility model

Two independent test tracks are mandatory:

### Standards

- Web Platform Tests (WPT)
- ECMAScript/Test262 through the selected JS engine
- WebDriver/WebDriver BiDi tests

### Real Web

`rarog-web-corpus` will maintain reproducible scenarios for popular sites and applications. Compatibility fixes must live in a separately versioned `rarog-compat` subsystem rather than site-name branches in layout/DOM code.

## Why the bootstrap renderer is deliberately small

The first milestone proves the interfaces:

```text
parse → DOM → style → layout → display list → framebuffer
```

It is not a standards claim. A small end-to-end pipeline lets us change parsing/layout implementations without rewriting host, paint and test infrastructure.
