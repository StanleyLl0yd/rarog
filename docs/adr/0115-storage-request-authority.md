# ADR-0115: Bounded Host-to-Storage request authority

Status: accepted

## Context

ADR-0114 binds persistent storage authority to the Host-owned navigation context, exact origin, capability and live Storage-process identity. Moving storage execution into a utility process requires another boundary: Host must be able to hand an already-authorized operation to Storage without letting Site code choose an origin, filesystem path, database handle or backend object.

The handoff must also remain bounded while requests are queued or in flight. A numeric request ID alone cannot be completion authority, and a response for a cancelled, completed or different-process request must not release or mutate pending Host state.

## Decision

`rarog-storage` separates persistent state from its request lifecycle behind a thin crate facade. The Storage request protocol defines only four initial typed operations: get, put, remove and clear. A request contains a Host-supplied exact `Origin`, the intended Rarog-owned `StorageProcessId`, a monotonic `StorageRequestId` and bounded key/value data. No filesystem path, OS PID/handle, database handle or platform backend identifier exists in the contract.

`StorageRequestQueue` is Host-side authority. It is constructed for exactly one live Storage-process identity and applies explicit limits to each request, the number of pending requests and total accounted pending bytes. Origin serialization, key bytes and value bytes are included in request accounting. Opaque origins are rejected before tracking. Storage key/value limits are rechecked before enqueue so an oversized request cannot occupy pending authority only to fail later.

Taking a request from the outbound queue does not release its budget: bytes remain charged while the operation is in flight. Cancellation or a valid matching completion releases the pending entry. Request identities are monotonic and are not reused after cancellation or completion.

`execute_storage_request` validates the target `StorageProcessId` again before touching `StorageProcessState`. The response repeats the request identity, process identity and operation kind. Host completion requires an existing pending request plus exact process and operation agreement. Replayed, guessed, cancelled and already-completed response identities fail closed. A mismatched response leaves the valid pending request intact so a later valid completion can still be accepted.

This is a typed process-boundary contract, not a concrete byte wire format or Windows transport. Encoding, authenticated endpoint binding and Windows Storage-process launch/containment remain later R5 work.

## Consequences

Storage transport can be replaced without changing exact-origin Web security semantics or exposing storage backend authority to Site code. Backpressure survives the queued-to-in-flight transition instead of disappearing when bytes leave the local outbound queue.

The initial protocol intentionally covers only the operations needed to prove request authority and lifecycle. Web Storage/IndexedDB API shape, durable transactions and compatibility behavior are separate concerns.
