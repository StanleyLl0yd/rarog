# Contributing to Rarog

Rarog is architecture-first and measurement-driven.

Before adding a subsystem:

1. identify its trust boundary;
2. define ownership/lifetime independently of process placement;
3. define measurable correctness/performance criteria;
4. avoid site-specific behavior in standards code;
5. prefer a narrow adapter over leaking a third-party API across crates.

Changes that knowingly reduce site isolation, origin isolation or capability boundaries for performance are not accepted as normal optimizations.
