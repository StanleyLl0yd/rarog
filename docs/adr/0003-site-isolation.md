# ADR-0003: Site isolation is mandatory

**Status:** Accepted

## Decision

Production Rarog separates mutually untrusted sites at a process boundary. Memory-efficiency work must not weaken this boundary.

## Consequence

The bootstrap may be single-process only as an implementation stage. Public APIs and ownership rules should avoid assumptions that DOM/layout/network/storage all share one address space.
