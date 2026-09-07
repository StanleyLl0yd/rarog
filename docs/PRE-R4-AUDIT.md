# Mandatory pre-R4 repository audit and refactor gate

Status: **pending**.

R3 is complete before this gate begins. R4 is blocked until this document is marked **complete**.

The authoritative execution protocol is `agent/AUDIT_REFACTOR.md`; the repository-specific invariants in `../AGENTS.md` remain mandatory.

## Scope

This is a behavior-preserving repository-wide audit and implementation pass, not a feature milestone.

Do not begin Host/Site processes, IPC, sandboxing, capability-broker, site-isolation or other R4 feature work while this gate is pending.

The audit covers the entire repository:

- production crates and public/internal APIs;
- tests, fixtures, fuzz targets and deterministic gates;
- build scripts, workspace configuration and CI;
- platform-specific integration and backend boundaries;
- dependencies and development dependencies;
- documentation and ADRs;
- legacy, compatibility and migration leftovers;
- allocations, copies, repeated passes and practical hot-path costs;
- ownership, state duplication, error handling and redundant validation.

## Required phases

- [ ] Capture a reproducible pre-refactor baseline: repository/file/source/dependency/test statistics where reliable, plus the complete available verification result.
- [ ] Complete the first full repository inspection and classify candidates as deletion, consolidation, simplification, architecture reduction, dependency cleanup or practical performance optimization.
- [ ] Validate indirect/convention/platform/generated uses before every removal.
- [ ] Add minimal regression coverage first wherever a justified change lacks sufficient behavioral protection.
- [ ] Implement safe candidates in small coherent PRs with relevant verification after each group.
- [ ] Review every production and development dependency for actual use, overlap and continuing necessity.
- [ ] Review architecture boundaries for obsolete wrappers, duplicate state, speculative abstractions and responsibility fragmentation without weakening future trust/process boundaries.
- [ ] Complete the mandatory second full repository pass after the first refactoring pass.
- [ ] Resolve or explicitly document every remaining audit candidate that cannot be proven safe to change.
- [ ] Run the complete available final verification suite, including Windows-primary, Linux portability, MSRV, prior milestone gates, R3 exit gate, SpiderMonkey jobs and fuzz-target compilation.
- [ ] Produce the final audit report required by `agent/AUDIT_REFACTOR.md`, including removed/consolidated/simplified/dependency/legacy/intentionally-unchanged/verification/limitations and reliable before/after statistics.
- [ ] Mark this document complete only after the final audit/refactor state is merged and post-merge `main` CI is green.

## Completion rule

R4 may start only when:

1. this document says `Status: **complete**.`;
2. every checklist item above is checked;
3. the final audit report exists in the repository;
4. post-merge `main` CI is green.

Until then, the roadmap transition is R3 → pre-R4 audit/refactor, not R3 → R4.
