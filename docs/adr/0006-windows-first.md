# ADR-0006: Windows-first platform strategy

- Status: Accepted
- Date: 2026-08-26

## Context

Rarog needs a concrete first desktop target so platform integration, performance work and user-facing validation do not become an abstract lowest-common-denominator exercise.

The planned reference browser, Zorya, is also intended to reach users on Windows first.

At the same time, coupling Web semantics directly to Win32/WinRT/Direct3D would make later ports expensive and would contaminate engine architecture with host-platform assumptions.

## Decision

Windows 10/11 is the primary production target for Rarog and Zorya.

Windows receives the first production-quality implementations of window/event integration, text/font integration, input/IME, accessibility, sandbox/process hardening, capability brokering and GPU/compositor platform integration.

Engine-core crates remain platform-neutral. Windows APIs must be isolated behind narrow platform adapters.

CI treats Windows as the primary lane and Linux as an early portability lane. A macOS lane is added when macOS becomes an active target.

## Consequences

- Product decisions can optimize for a real first platform.
- Windows regressions block primary-platform readiness.
- Engine-core architecture must not use Windows-only types as Web-facing primitives.
- Portability is continuously checked without requiring feature parity on every OS from day one.
