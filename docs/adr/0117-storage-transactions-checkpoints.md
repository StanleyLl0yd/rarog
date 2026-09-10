# ADR-0117: Bounded storage transactions and portable checkpoints

- Status: Accepted
- Date: 2026-09-10

## Context

ADR-0113 established exact-origin bounded persistent storage, ADR-0114 bound Storage authority to Host-owned navigation context state, ADR-0115 added bounded Host-to-Storage request authority, and ADR-0116 established the concrete Windows Storage-process lifecycle without making OS process objects Web authority.

R5 still needs two portable persistence foundations before higher-level Web storage APIs can safely build on the Storage utility process: a transaction lifecycle that cannot grow unbounded or partially commit on failure, and a checkpoint representation that can cross a backend/process restart boundary without trusting serialized authority or bypassing current limits.

The portable layer must also distinguish a caller's durability preference from a concrete backend guarantee. An in-memory transaction manager cannot truthfully promise `fsync`, database journal semantics or cross-process locking simply because a strict durability mode was requested.

## Decision

`rarog-storage` owns a bounded `StorageTransactionManager` tied to exactly one current `StorageProcessId` and configured `StorageLimits`.

- Each transaction retains one exact persistent `Origin`; opaque or oversized persistent-origin identities are rejected before retention.
- Transaction IDs are monotonic non-zero values and are not reused after commit, abort or identity-space exhaustion.
- Transactions are explicitly `ReadOnly` or `ReadWrite` and begin in `Waiting` or `Active` state according to exact-origin conflicts.
- Multiple older/read-only transactions may coexist for an origin, while a conflicting writer and later conflicting work remain ordered behind already waiting transactions rather than bypassing them.
- The number of retained transactions, mutations per transaction, staged bytes per transaction and total staged bytes are independently bounded. Default bounds are 256 retained transactions, 1024 mutations per transaction, 4 MiB staged per transaction and 16 MiB staged in total.
- Key/value limits and transaction staging budgets are validated before staging allocations. Retained origin/key/value data uses fallible allocation paths; storage quota is revalidated atomically when the staged mutations are applied to the commit candidate.
- Read-only transactions cannot stage mutations.
- Commit first builds a fallibly cloned candidate `StorageProcessState`, applies every staged mutation to that candidate under the current storage limits, and swaps it into committed state only after all mutations succeed. A failed commit aborts the transaction and leaves the prior committed state intact.
- Explicit abort and successful/failed completion release staged accounting and may activate newly unblocked waiters without reusing stale transaction IDs.
- `StorageDurabilityHint::{Default, Strict, Relaxed}` is returned with successful commit as a backend-facing policy hint. The portable manager does not claim filesystem flush, journal, locking or crash-durability semantics.

Persistent origin identity itself is bounded at 4096 bytes before retention. The request layer uses the same persistent-origin identity validation so queued requests cannot retain origins that storage state or transactions would reject.

`rarog-storage` also defines a deterministic versioned checkpoint contract. Version 1 uses the fixed `RAROGST1` magic and a canonical ordering for origins and keys.

- Checkpoint encoding and decoding are bounded by `StorageCheckpointLimits`; the default maximum encoded checkpoint is 320 MiB. The storage state default remains 256 MiB, leaving bounded serialization/framing overhead without widening the live-state quota.
- Counts and lengths are checked before dependent allocation; fallible reserve/allocation paths are used where checkpoint data can scale with input.
- Decoding rejects bad magic, unsupported versions, truncation, invalid UTF-8/host tags, invalid or oversized persistent origins, non-canonical order, duplicate logical entries, length overflow and trailing data.
- Restore targets the caller-supplied current `StorageProcessId` and current `StorageLimits`; no serialized field can mint or replace Host/process authority.
- Restore reconstructs a candidate state through the same bounded state mutation API and replaces live state only after the complete checkpoint validates and fits current limits. Failure therefore cannot partially restore state.

The checkpoint format is a portable Rarog state contract, not an authorization token and not a concrete filesystem/database format. A future persistence backend may wrap or replace its physical representation, but must preserve the same authority, limit and transactional-restore invariants at the Rarog boundary.

## Validation

Coverage must demonstrate:

- exact-origin isolation and persistent-origin identity bounds;
- bounded transaction count, mutation count and staged-byte accounting;
- writer/read-only scheduling including fairness against already waiting conflicting work;
- stale/unknown/aborted transaction IDs rejected;
- wrong-Storage-process use rejected before state access;
- read-only mutation attempts rejected;
- failed commit preserves committed state and releases transaction accounting;
- successful commit reports the requested durability hint without claiming backend durability;
- deterministic checkpoint bytes independent of hash-map iteration order;
- checkpoint size/version/framing/canonical-order validation;
- malformed, duplicate, trailing or oversized checkpoint input rejected before live-state replacement;
- restore revalidates current storage quotas and current process identity transactionally.

## Consequences

R5 now has a portable bounded transaction and recovery-state contract that future Web Storage/IndexedDB and persistence backends can build on without making allocation success, serialized metadata or durability labels into authority.

This ADR does not claim Web Storage or IndexedDB API completeness, IndexedDB schema/version-change semantics, filesystem-backed persistence, `fsync` behavior, database journaling, multi-process database locking or an authenticated Windows Storage request transport. Those require separate implementation and validation.