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

The core engine workload still runs primarily in the embedding process. R4 established separate bounded Host/Site process authority, child lifecycle, IPC and Windows containment infrastructure without claiming wholesale engine-workload migration into Site children; crate boundaries continue to mirror the security/process boundaries required for that migration.

### R4 process identity and site assignment boundary

R4 established a process-placement-independent topology in `rarog-process`. The Host owns engine-level Host/Site process identities and assigns a schemeful `SiteIdentity` to a bounded Site-process identity. These identities are not OS process IDs, handles or transport endpoints; concrete Windows launch/sandbox state remains isolated behind the platform/native boundary.

The assignment contract is fail-closed:

- one live schemeful site may reuse its existing Site-process assignment;
- a distinct schemeful site receives a distinct Site-process identity;
- opaque site identities remain isolated unless the exact opaque identity is propagated and reused;
- exhausting the configured Site-process budget returns an error instead of sharing authority with another site;
- retiring a Site process removes its site assignment, and replacement allocation receives a fresh identity rather than reusing stale authority.

This identity/lifetime contract underpins the completed R4 IPC, capability-brokering and crash-recovery foundations without claiming that the engine workload itself already runs wholesale in Site children. See ADR-0102.

### R4 IPC protocol boundary

`rarog-ipc` defines the transport-independent Host↔Site protocol envelope and a deterministic fixed-header wire representation. Every envelope carries the current protocol version, explicit endpoint roles, a request/reply/event kind and a bounded owned payload. Request identities are correlation values only; they do not grant process or capability authority.

The initial channel model is bounded in both message count and queued payload bytes. Enqueue returns explicit backpressure errors rather than growing without limit. Disconnect discards queued work and makes subsequent send/receive operations fail until a new channel object is established. The 24-byte Rarog wire header validates magic/version/roles/kind/correlation/payload length before payload allocation and rejects malformed, truncated or trailing data.

Direct Site↔Site and Host↔Host routes are rejected by the portable protocol. Site-process authority is not accepted from a self-asserted wire or payload field: Host control-plane/platform code binds a channel/transport endpoint to the `SiteProcessId` it launched and capability checks use that Host-owned binding.

The portable protocol crate contains no named-pipe handles, OS endpoints or Windows types. Windows transport selection remains behind the platform boundary while the Rarog-owned codec defines the bytes that cross it. See ADR-0103 and ADR-0110.

### R4 capability broker boundary

`rarog-broker` owns Host-issued capability identities independently from IPC request identities and OS handles. A `CapabilityId` received from a Site process is only an untrusted reference. Authorization succeeds only when the Host broker still has a live grant for that identity, the authenticated channel owner matches the grant's `SiteProcessId`, and the requested capability class matches exactly.

Capability identities are monotonically allocated and never reused after revocation. The broker is bounded by an explicit grant count. Capacity or identity exhaustion fails closed. Revoking a Site process removes every capability owned by that process so crash/replacement handling can invalidate authority before assigning a fresh process identity.

The initial concrete classes are Network and Clipboard because both already have Rarog-owned policy/platform boundaries. A Network grant reaches the Host-owned Fetch/network transport boundary after Fetch has produced a bounded `NetworkRequest`; the broker route does not own or widen Fetch CORS/origin/credentials/redirect policy. Clipboard access reaches `PlatformClipboardService` only after exact process/capability/class authorization. See ADR-0104 and ADR-0107.

### R4 Host control-plane boundary

`rarog-host` composes the portable R4 process topology, IPC and capability broker without exposing their mutable ownership to Site-side code. A Host control plane owns each logical Site instance, binds one bounded IPC channel to its Host-assigned `SiteProcessId`, and grants capabilities only to currently live Site-process identities.

Inbound transport plumbing must supply the Host-owned Site-process binding separately from the decoded IPC envelope. The envelope's source/destination roles are checked against that binding path; payload values do not select another process or authority owner.

When a Site process is reported lost, the Host control plane disconnects and discards its queued IPC work, revokes every capability owned by that process, quarantines any private backend Network tickets owned by that process, then retires its topology identity. Recovery allocates a fresh Site-process identity and a fresh empty channel. Stale process IDs and capability references therefore remain invalid even when the same schemeful site is recovered.

On Windows, `WindowsSiteProcess` binds a real child lifetime to a live Host-produced `SiteLease`. Non-blocking observation, blocking wait and explicit termination all feed the child's exit into `HostControlPlane::process_lost` once. Concrete process creation is delegated to `rarog-platform-windows-native`, which atomically installs the bounded R4 exploit-mitigation, child-process restriction and per-Site Job Object policy through extended creation attributes before Site code can execute. Native handles and Job Object state stay inside that dedicated low-level crate; `rarog-platform-windows` consumes only its safe `SandboxedChild`/evidence API. Dropping a live child remains emergency containment, while coherent recovery uses explicit Host-reporting paths. See ADR-0108 and ADR-0111.

Privileged Network operations never expose backend `NetworkTicket` values as Site authority. `rarog-host` allocates a separate bounded monotonic `NetworkOperationId`, binds it to the authenticated Site process and exact Network capability, and validates that binding before poll/cancel reaches the backend. Completion and explicit cancellation finish the tracked operation normally. Capability revocation and process loss remove Site-visible authority immediately but retain the private ticket in a Host cancellation quarantine; active and quarantined work share the same configured operation budget until `cancel_pending_network_operations` confirms backend cancellation. Repeated revocation therefore cannot manufacture fresh Host capacity while old backend work remains outstanding. Clipboard reads/writes similarly authorize the exact Clipboard capability before touching the platform service and revalidate text against the concrete service limit. Rejected references therefore cannot cause privileged backend side effects. See ADR-0107.

Document navigation is bound at the Host boundary rather than inside Web-controlled payloads. A `DocumentSiteBinding` stores the owning `SiteIdentity` and Host-assigned `SiteProcessId`. Initial and same-site navigation use the bounded topology normally; cross-site or cross-scheme navigation replaces the document binding with a distinct Site-process identity. Opaque URLs create a fresh identity for a new environment, while inherited opaque environments pass their stored `SiteIdentity` explicitly so identity is never accidentally regenerated. A stale binding is rejected before a target process is allocated. See ADR-0106.

The production-facing Host navigation boundary additionally wraps document bindings in bounded monotonic `NavigationContextId` values. Embedders retain the context identity and Site identity, not authoritative `SiteProcessId` values. The Host reference-counts contexts per Site process, keeps a shared same-site process live until the last context leaves, and retires an unreferenced source through disconnect → operation/capability revocation → process retirement. Under a one-slot process budget, an unshared source may be retired before cross-site target allocation; if target creation then fails, the context is explicitly invalidated rather than silently retaining or sharing stale authority.

Privileges used by a navigation context are represented by `NavigationContextCapability`, which binds the Host-issued capability to the exact context in addition to the broker's process/class ownership. This prevents two same-site contexts sharing one Site process from treating each other's numeric capability references as authority. Context-scoped Network/Clipboard entry points resolve process authority from Host state before delegating to the existing broker-gated routes; cross-site navigation, context close and process loss make stale context capability/operation references unusable. See ADR-0109.

This remains a portable authority model. The Windows launch/loss adapter owns concrete child-process lifecycle and feeds observed loss back into Host revocation/retirement, while the Windows IPC adapter binds accepted local endpoints to live Host-produced `SiteLease` values separately from decoded wire fields. Neither child PID nor envelope contents select Rarog process authority. See ADR-0105, ADR-0108 and ADR-0110.

### R5 Storage-process and origin-storage foundation

R5 extends the process identity space with one Host-owned `StorageProcessId`. It is distinct from Site-process identity, is not an OS PID/handle, does not consume the Site-process budget and is never reused after retirement. On Windows, `WindowsStorageProcess` binds the Host-issued Storage generation to the existing sandboxed child boundary; observed loss revokes every Storage-class grant before retiring the logical Storage identity, while navigation contexts and unrelated capabilities remain live. Native PID/HANDLE/Job Object state stays behind `rarog-platform-windows-native`. See ADR-0116.

`rarog-storage` owns the portable storage-process state model. Persistent Web storage is keyed by the exact Rarog `Origin`, not by schemeful site, and opaque origins are rejected from persistent storage by default. Retained persistent origin identities are bounded, and key/value sizes, origin count, entries per origin, per-origin bytes and global bytes are all explicitly bounded. Failed quota or allocation checks leave committed state unchanged.

URL-backed navigation contexts retain the exact current `Origin` alongside their schemeful-site/process binding. Storage capabilities are bound to that exact origin at grant time, and Host storage routes re-check context, process, class, exact origin and the live Storage-process identity before data access. Same-site cross-origin navigation revokes Storage capabilities; a site-only or opaque-origin context cannot mint persistent Storage authority. Lower-level site-only navigation APIs remain available for R4 topology tests but are explicitly originless for persistent storage. See ADR-0113 and ADR-0114.

The Host-to-Storage request layer carries only typed bounded get/put/remove/clear operations. The Host-owned pending queue keeps queued and in-flight bytes charged, request IDs are monotonic correlation values rather than authority, and completion requires exact agreement with the live Storage-process identity and operation kind. Site code never selects filesystem paths, backend handles or storage authority through the request payload. See ADR-0115.

Storage transactions are bound to one exact persistent origin and the current Storage-process identity. Read-only transactions may overlap while writers wait behind conflicting work; bounded staging is applied to a fallibly cloned candidate state and becomes committed state only after every mutation succeeds. Failed commit aborts the transaction without partially mutating committed state. Durability is an explicit backend-facing preference, not a promise that this portable layer performs filesystem flushes.

Storage checkpoints use a deterministic versioned bounded wire contract with canonical ordering. Decode validates framing, lengths, origin identity and current `StorageLimits` before a fully reconstructed candidate replaces live state, so malformed, oversized, duplicate, non-canonical or otherwise invalid checkpoints cannot partially restore state. Serialized metadata never overrides the current Storage-process identity or Host authority. See ADR-0117.

These foundations do not claim Web Storage or IndexedDB API completeness, a filesystem-backed persistence backend, cross-process database locking semantics, or an authenticated Windows Storage request transport. Site code receives no filesystem authority from this layer.

### R5 Worker identity and lifecycle foundation

`rarog-workers` owns a dependency-free portable worker identity, ownership and live-lifecycle boundary. Worker identities are distinct from Host/Site/Storage process IDs, OS PIDs/threads, scheduler work IDs and script-engine objects. Each `WorkerRegistry` receives a process-local non-zero scope and allocates monotonic non-zero worker serials within it, so ordinary identities do not alias across independent registries and retired identities are not reused. `WorkerId` remains only a reference; it grants no Host or platform authority. See ADR-0118.

A registry is bounded independently by total live workers, direct children per owner and ownership depth. A root owner is an external identity supplied by the integrating layer; nested workers are owned by an exact parent worker. Only a `Running` parent may create children. Live workers move through explicit `Created`, `Running` and `Closing` states; closing a worker cascades to its descendants, while retirement removes the bounded subtree and releases parent child capacity. Destruction of an external root owner force-retires only the trees rooted in that owner. Ownership traversal is iterative rather than recursively consuming the native stack.

The identity/lifecycle sub-layer deliberately remains scheduler- and script-independent and does not attach workers to navigation contexts, message queues, Service Worker registrations, Fetch interception or OS thread/process objects. Execution is composed above it through the boundary below; the identity itself still grants no execution or Host authority, and this remains a scoped R5 foundation rather than a claim of Worker API completeness.

### R5 Dedicated worker execution ownership

`WorkerExecution` composes the existing `rarog-scheduler::EventLoopScheduler` and `rarog-script::ScriptRuntime` above the dependency-free identity/lifecycle layer. One execution owner is correlated with one exact `WorkerId`, exclusively borrows one Script runtime for its lifetime and owns one exact Rarog `RealmId` until explicit shutdown or drop. Construction and every queue, checkpoint and execution-driving operation receives the exact `WorkerRegistry` and requires that the worker is still present and `Running`; foreign, retired, `Created` or `Closing` identities are rejected before executable work is touched. The per-operation registry borrow lets lifecycle transitions revoke execution immediately without turning `WorkerId` into a capability.

Worker task and microtask ordering is delegated to the existing scheduler rather than duplicated. Queue count/backpressure remains governed by `SchedulerLimits`. Script source is checked against the worker realm's byte limit before ownership is copied into scheduler storage, so attacker-influenced source cannot bypass the configured per-item bound merely by enqueueing it.

Evaluation occurs only through the Rarog `ScriptRuntime` API. JavaScript throws remain normal `EvaluationOutcome` completions; Script/backend errors complete the exact scheduler work item before propagation so stale active work cannot wedge the worker loop. Explicit shutdown destroys the exact realm and reports teardown errors, while drop performs best-effort realm destruction and discards pending owned work without evaluating it.

Worker message delivery is layered above the same execution/scheduler owner rather than introducing a second event loop. `WorkerMessageMailbox` owns bounded Rarog-native structured payloads and FIFO transport state; scheduler tasks carry only an exact `WorkerMessageId` marker. A payload remains charged against mailbox message/byte budgets while queued, scheduled and selected, and capacity is released only by exact completion or lifecycle discard. `WorkerMessageId` and `WorkerId` remain references rather than authority.

Message routes are validated against the current exact `WorkerRegistry`: supported portable routes are an external root owner to its direct root worker, the reverse route, and direct parent/child worker pairs. Both worker endpoints must still be `Running`; foreign, retired, `Created` or `Closing` endpoints are rejected before payload access. Nested structured values are bounded by explicit depth, item, string/byte, per-message, queued-message and aggregate queued-byte limits. Accounting uses checked arithmetic and fails closed; payload ownership is established only after the applicable limits and route checks succeed.

`WorkerExecution::schedule_next_message` admits at most the oldest deliverable message for that worker into the existing `EventLoopScheduler`, preserving FIFO and scheduler backpressure. The mailbox retains the payload and its byte charge until `complete_message`; `message_for_delivery` revalidates worker liveness and the exact scheduled `(WorkerId, WorkerMessageId, TaskId)` tuple before exposing the Rarog-owned payload. Lifecycle revocation therefore blocks selected payload access, while explicit mailbox cleanup can discard stale work and recover capacity without restoring authority. See ADR-0119 and ADR-0120.

### R5 Service Worker registration, scope and lifecycle foundation

`ServiceWorkerRegistry` in `rarog-workers` owns the portable Service Worker registration/version state that precedes Fetch interception. Each registry allocates independent scoped monotonic `ServiceWorkerRegistrationId` and `ServiceWorkerVersionId` references; neither identity grants Host, process, network, storage or platform authority. Every registration in this bounded foundation is tied to one exact non-opaque Rarog `Origin` and one canonical fragmentless `WebUrl` scope. Scope and script URLs must be same-origin HTTP(S), reject ASCII-case-insensitive `%2f`/`%5c` path escapes, and are checked against the retained canonical URL-byte limit before registry ownership. Registration count is bounded globally and per origin, and live version count is independently bounded.

One exact canonical scope maps to one registration in this current exact-origin model. Re-registering it creates a fresh installing version, replacing only a superseded installing attempt; an existing waiting version remains available until the replacement install succeeds, at which point that previous waiting identity is retired and the new version becomes waiting. An existing active version is preserved until activation begins. Activation follows the Service Worker slot transition: a waiting version cannot replace an active version that is still `Activating`; once eligible, beginning activation retires the previous activated identity, promotes the waiting version into the active slot as `Activating`, and finishing activation advances only that exact active version to `Activated`. Invalid or stale version/slot transitions fail before mutation; discard and unregister paths release retained capacity deterministically.

Registration matching is data-only: for a canonical same-origin client URL, the registry returns the registration whose serialized scope is the longest prefix. The returned registration ID is still only a reference and cannot authorize request interception. The standards registration key also includes a storage key; secure-context/client authorization, storage-key partitioning and response-derived maximum-scope enforcement such as `Service-Worker-Allowed` remain later integration work rather than being guessed inside this portable foundation. See ADR-0121.

### R5 Host-authorized Service Worker Fetch interception

`HostControlPlane` is the authority boundary for the current R5 Service Worker Fetch slice. URL-backed navigation contexts retain a bounded canonical fragmentless `WebUrl` alongside their exact current origin and Site binding; URL-less Site-only contexts are deliberately ineligible. Navigation clears any previous Service Worker controller and pending dispatches before a new controller can be selected. The Host alone selects a controller by matching its retained current client URL against `ServiceWorkerRegistry`; registration, version and controller identities remain references rather than capabilities.

Every interception preparation first revalidates the exact live navigation-context Network capability and requires `FetchRequest.origin` to equal the Host-owned current client origin. The request URL itself may be cross-origin for a controlled subresource. An uncontrolled request returns its owned bounded request for explicit network fallback. A controller whose exact active version is still `Activating` returns an owned wait state and cannot touch the backend; only `Activated` allocates a bounded Host-owned `ServiceWorkerFetchDispatchId`. Pending dispatches retain the exact bounded request and remain charged until completion, explicit discard or authority cleanup.

Dispatch access and completion revalidate context/process/capability ownership plus the exact registration active slot and version lifecycle. Navigation, capability revocation, context close, Site-process loss and stale-controller reaping revoke pending dispatch authority deterministically. A Service Worker response is checked again against the originating request's response/header limits before capacity is released; an invalid response leaves the dispatch charged. Explicit fallback consumes the retained request into the existing Host `NetworkOperationId` / `NetworkCapability` route, so backend `NetworkTicket` values remain private and Service Worker IDs never become network authority. See ADR-0122.

This R5 interception boundary does not claim script-visible `FetchEvent`/`respondWith`, navigation interception completeness, Cache Storage, navigation preload, Service Worker router rules, soft-update timing, full storage-key partitioning, trustworthy-origin/secure-context completeness, complete Service Worker client/controller handoff, push/sync, SharedWorker semantics, generic Web structured-clone completeness, transferables or OS thread/process isolation. Those remain separate work; R6 compatibility qualification is explicitly out of scope.

### R5 WebSocket semantic contracts

`rarog-websocket` owns the first portable WebSocket boundary independently from Host/network transport. `WebSocketUrl` wraps Rarog `WebUrl`, bounds raw and canonical serialized input, maps HTTP(S) constructor schemes to WS(S), rejects fragments and unsupported schemes, exposes `wss` security state and derives the protocol resource name from path plus query. The crate depends only on `rarog-url`; backend socket, TLS, HTTP, platform and Host types are absent.

Opening-handshake intent is semantic data only. `WebSocketProtocols` preserves requested subprotocol order while enforcing a bounded count, bounded bytes per value, non-empty HTTP-token grammar and case-sensitive duplicate rejection before values are retained. `WebSocketHandshakeIntent` owns only the validated canonical URL plus requested protocols; random keys, raw Upgrade headers, extension negotiation and backend connection identity remain later integration work.

`WebSocketReadyState` records the four Web-facing connecting/open/closing/closed states, while `WebSocketLifecycle` now owns their portable legal transition discipline: `Connecting -> Open -> Closing -> Closed`, with `Closed` terminal. `WebSocketMessage` privately owns bounded text or binary application data; UTF-8 text is charged by encoded bytes and oversize input is rejected before copying. These are message-level contracts, not WebSocket frames: opcodes, fragmentation, masks and native sockets remain absent; bounded queue ownership is layered above this payload contract. See ADR-0123 and ADR-0126.

### R5 Host-authorized WebSocket transport ownership

`HostControlPlane` is the authority boundary for the current WebSocket transport slice. `rarog-websocket::WebSocketTransport` consumes only a validated `WebSocketHandshakeIntent` plus the exact client `Origin` selected from Host-owned navigation state and returns a Rarog `WebSocketTransportTicket`. The ticket is private backend correlation state rather than Web or Site authority; concrete sockets, TLS sessions and platform handles remain behind the replaceable transport adapter.

A live WebSocket is represented outside the backend by a separate scoped monotonic `WebSocketConnectionId`. The Host binds that reference to the exact navigation context, current Site process, exact Network capability, Host-authenticated client origin and private transport ticket. Context/class/process/broker authority, client-origin availability and the configured connection budget are validated before the transport is touched. The WebSocket target may be cross-origin, but target URL or subprotocol data cannot select a different client identity or mint network authority.

Active connections and tickets awaiting backend abort share one bounded Host budget. Capability revocation, navigation-context close, cross-site replacement and Site-process loss remove Host-visible connection authority and quarantine the backend ticket until explicit abort succeeds. Same-site navigation also quarantines every connection owned by the old document even when its Network capability remains live. A failed abort keeps the ticket charged, and backend ticket reuse across live or quarantined work fails closed rather than aliasing two Host connection references. Independent Host instances allocate distinct connection-ID scopes. See ADR-0124.

### R5 bounded WebSocket application queues

Each live Host WebSocket connection now owns one `WebSocketMessageQueues` value with independent FIFO inbound and outbound budgets. `WebSocketQueueLimits` bounds retained message count and aggregate bytes separately in each direction. Enqueue accounting uses checked addition before mutation, dequeue uses checked subtraction, and rejected operations leave both contents and counters unchanged. Empty messages remain valid: when inbound bytes are fully charged but message-count capacity remains, the receive bound is explicitly zero rather than being confused with a count-full queue.

Host queue access remains behind the exact navigation-context Network capability and `WebSocketConnectionId` binding from ADR-0124. Outbound transport handoff borrows the oldest queued message and releases it only after `WebSocketTransportSend::Accepted`; explicit backpressure retains the exact payload and byte charge. A transport error is terminal under ADR-0126: ordinary connection authority and application queues are revoked while the private ticket remains charged for cleanup. Inbound polling does not touch the backend when message-count capacity is exhausted, otherwise it supplies the exact remaining byte budget and revalidates the returned message before retaining it. A backend that exceeds the advertised receive bound likewise fails the connection closed and quarantines the private transport ticket rather than accepting oversized data.

Navigation, capability revocation, context close and Site-process loss remove the live connection and therefore discard its application queues, while pending backend cleanup remains charged independently through the existing transport-ticket quarantine. Queue state never widens connection authority and one connection cannot consume or expose another connection's FIFO. See ADR-0125.

### R5 WebSocket close, error and backpressure lifecycle

`WebSocketCloseIntent` owns validated local close metadata before transport access. Close reasons are bounded to 123 UTF-8 bytes; a non-empty reason requires an explicit code, and the current local contract accepts code 1000 or application/private codes 3000 through 4999. Host construction completes the deterministic local `Connecting -> Open` transition before `WebSocketTransport::start`, and a connection becomes Host-visible only after backend start succeeds.

A close request revalidates exact navigation-context Network authority, records one immutable close intent and moves the live connection to `Closing` without touching the backend. Closing immediately rejects new application-message enqueue and new inbound backend polling, while already-retained inbound data remains consumable and already-retained outbound FIFO data remains eligible for lossless flush. The backend close handshake begins only after that outbound FIFO drains. Send or close-start backpressure is retryable and preserves the live connection, exact intent and retained charges.

`WebSocketTransportCloseStart::Started` marks backend close ownership; close polling then returns `Pending` or the terminal `Closed`. Confirmed graceful closure clears retained application queues, removes Host-visible connection state and releases connection capacity without an unnecessary abort. By contrast, any transport error from send, receive, close start or close poll is terminal: Host state moves to Closed, queues are discarded, ordinary connection authority is removed and the private transport ticket enters the existing pending-abort quarantine. Live and quarantined tickets share the same configured connection budget, so backend failure cannot manufacture fresh capacity before cleanup succeeds. Navigation, capability revocation, context close and Site-process loss remain stronger authority-loss paths and converge on the same cleanup quarantine. See ADR-0126.

This completes the selected R5 WebSocket backlog boundary. WebSocket frame parsing/masking, ping/pong scheduling, compression/extensions, complete peer close-frame metadata, selected-protocol negotiation, real HTTP Upgrade/TCP/TLS behavior, DOM/WebIDL exposure, WPT qualification and R6 compatibility qualification remain outside this foundation.

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
  ├─ supported geometry change → retained subtree or root-flow relayout
  ├─ ordinary CharacterData change → retained text-node refresh + flow-aware fragment rebuild
  ├─ covered insertion/reparent/detach → retained structural-root refresh with stable existing LayoutNodeId identity
  ├─ connected stylesheet-source change → rebuild StyleSet + global retained-style revalidation
  └─ unprovable formatting/membership/history case → deterministic full rebuild
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

`rarog-html` exposes a decoded streaming-input contract independently of the parser backend. `StreamingInput` accepts UTF-8 chunks and closes explicitly; source spans in parser diagnostics are UTF-8 byte offsets in that decoded stream. Transport bytes and encoding detection/decoding stay outside this interface.

Recoverable syntax problems produce deterministic `ParseDiagnostic` records with a code, severity, source span and message. Contract failures that prevent parsing from starting or completing use `Result::Err`. The canonical entry points are `parse`, `parse_with_diagnostics` and `parse_stream`; `parse_standards*` names remain compatibility aliases rather than a separate parser path.

R1 routes parsing through the standards-oriented adapter backed by `html5ever`, then normalizes its result into Rarog-owned DOM identities and invariants. Streaming input is still buffered until close rather than incrementally tokenized across calls, but backend token/node types do not leak into DOM, layout or engine callers. The adapter boundary therefore preserves replaceability while the parser behavior follows the standards-oriented path. See ADR-0101 and ADR-0025.

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

The original R0 style-source/cascade boundary remains the ownership model, while R1 added standards-oriented CSS syntax parsing, importance, inheritance, CSS-wide values, selector combinators, attribute selectors, pseudo-classes and namespace-aware matching. This remains a bounded implementation rather than a claim of CSS Cascade or selector completeness.

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

`StyleSharingKey` captures local selector/cascade inputs such as namespace, tag, ID, canonicalized classes and inline style. Local style sharing is considered safe only while the active rule set has no relational dependencies that invalidate the key. Rarog does not install a process-global computed-style cache; any future broader cache must be bounded to a document/style-set lifetime and must expand or disable its key when additional contextual inputs become observable. See ADR-0026.

## R0 observability and benchmark harness

Full bootstrap renders expose `RenderObservability` without feeding timing data into deterministic render identity. `RenderTimings` records wall-clock durations for decoded HTML parsing, style-source construction, Layout Tree construction, Fragment Tree construction, display-list/damage construction, rasterization, and the enclosing render. `RenderCounters` records DOM nodes, layout nodes, fragments, display commands, and damage rectangles. Layout Tree construction currently includes per-element computed-style resolution because R0 resolves styles while deriving layout nodes.

Stateful updates expose elapsed wall-clock time alongside the existing `IncrementalMode`, dirty-node count and patched-node count. These values are diagnostics only: CI does not enforce thresholds and the project makes no cross-machine performance claims from them. Allocator-backed peak/persistent byte accounting is deliberately deferred rather than publishing misleading estimates.

`cargo run -p rarog-engine --example r0_bench --release -- <iterations>` runs fixed full-render, paint-only, subtree-relayout and flow-relayout scenarios. Setup for each incremental sample is excluded from the reported update duration through the engine's own timing boundary. The harness is intended to detect gross regressions during development and to provide a stable place for later benchmark methodology, not to publish competitive numbers. See ADR-0028.

## Incremental reuse

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

Layout nodes expose `IntrinsicSizes { min_content, max_content }`. Text is represented as a backend-neutral `TextRun`; R1 connects production OpenType shaping, Windows font selection, Unicode-aware line breaking, grapheme boundaries and bidi/fallback metadata behind Rarog-owned contracts. These remain scoped foundations rather than claims of complete CSS text layout or Unicode conformance.

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

This remains a geometry foundation, **not** a claim of CSS box-model compliance. R1 added scoped margin-collapsing, intrinsic-size and min/max sizing behavior plus explicit formatting-context boundaries; percentages, writing modes and broader formatting-context completeness remain later work.

## Image resource boundary

R1 introduces `rarog-resources` as the platform-neutral ownership boundary for decoded image data. The crate owns typed `ImageResourceId` values, revisioned `ImageResourceRef` snapshots, an explicit pending/ready/failed lifecycle, RGBA8 decoded pixels, and bounded per-store retention. A store limits resource count, pixels per decoded image, and total retained decoded pixels; IDs are monotonic within the store and are not reused after removal.

A ready reference includes both resource ID and revision. Replacing decoded pixels advances the revision and invalidates older references. This makes image content identity explicit in backend-neutral paint commands instead of relying on hidden mutable cache state. `rarog-paint` carries only `ImageResourceRef` plus destination geometry in `DisplayCommand::DrawImage`; software rasterization receives the resource store explicitly and treats missing, stale, pending or failed references as transparent/no-op content. Image revision changes therefore participate in ordinary display-list equality and damage tracking.

The DOM and Layout Tree do not own decoded pixel buffers, and no process-global image cache is introduced. R2 added Rarog-owned URL/origin/Fetch boundaries, and R3 added a bounded asynchronous image-decode queue that feeds this resource store. Image-format codec integration, HTML replaced-element semantics and responsive images remain future work; transport and decoder-specific types still do not leak into layout/paint.

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

The Windows boundary began empty in R0. R1 added system-font integration, R2 added normalized input/IME and clipboard services, and R3 added target-specific GPU selection/surface presentation behind compositor/platform adapters. R4 adds Site-process lifecycle, bounded local IPC and a concrete bounded sandbox/process-containment policy. `rarog-platform-windows` remains safe Rust under the workspace lint; unavoidable Win32 process/Job Object handle operations are isolated in `rarog-platform-windows-native`, whose public API exposes no HANDLE, PID, token or Job Object type. `WindowsPlatformHost::try_new` still succeeds only on a Windows compilation target, while both Windows crates remain buildable on Linux for portability CI. Accessibility remains R5 work. See ADR-0030, ADR-0108, ADR-0110 and ADR-0111.

## Engine and embedder boundary

R0 exposes `Engine` and `View` above `RenderSession`. `Engine` owns shared host policy, UI-neutral event delivery, resource budgets and stable `ViewId` allocation; each `View` owns one loaded inline document and the render session derived from it. This keeps browser-shell ownership out of DOM/layout/paint crates and gives later process isolation a stable host-facing seam.

The original R0 `NavigationRequest` and `ResourceRequest` forwarding seam remains available: `HostPolicy` can return `Blocked` or `ForwardToEmbedder`, and UI-neutral `ViewEvent` values remain independent of a toolkit or Windows API. Document navigation additionally has a Rarog-owned transaction boundary. `View::begin_navigation` canonicalizes an HTTP(S) target, constructs the high-level document `FetchRequest`, allocates a monotonic per-View navigation identity and exports only its bounded `NetworkRequest` transport projection. The embedder/Host maps that identity to an authorized R4 Host network operation; raw network capabilities, backend tickets, Host operation identities and process/capability authority do not enter `rarog-engine`. Superseded or cancelled transactions reject stale completion before document replacement.

A matching `FetchResponse` is returned to the engine for commit policy. The transport is one-exchange-only and must surface redirects instead of following them; a changed backend final URL is rejected as a policy-boundary violation. The initial completion slice accepts only bounded, valid UTF-8 `text/html` responses with an explicit UTF-8 charset and fails explicitly for redirects, unsupported encodings/media types and no-document responses. Full redirect handling and HTML byte encoding sniffing extend the Rarog-owned Fetch/navigation layer later rather than being delegated to an embedder. Local `View::load_html` remains an inline-document embedding path; its `BaseUrl` is not treated as canonical Web security identity. See ADR-0058 and ADR-0112.

`ResourceBudget` now enforces document-source bytes, viewport pixels, DOM node count/depth, text scalars, CSS rules, fragment count and display-command count. The viewport limit cannot exceed the lower-level framebuffer safety cap. Separate resource stores and queues add their own image/font/input bounds. Resident-memory, graphics-cache, background-CPU and lifecycle accounting remain future extensions rather than invented estimates. `View::render` creates a stateful render session on first use, reuses it for an unchanged viewport, and performs a deterministic full session rebuild when the viewport changes. See ADR-0029.

## Script architecture

R2 integrates SpiderMonkey through one replaceable abstraction:

```text
DOM/Web APIs
    ↓ WebIDL bindings
Rarog Script API
    ↓
SpiderMonkey adapter
```

No engine crate outside the script adapter should depend directly on SpiderMonkey APIs.

## Resource model

The current engine execution path still runs primarily in the embedding process, but its externally influenced structures are explicitly bounded rather than relying on estimated accounting. `ResourceBudget` covers document/render complexity, while image, font, scheduler, event, clipboard/input and compositor queues/stores enforce subsystem-specific limits. R4 additionally established bounded per-Site process/IPC/capability authority; that does not imply that all engine execution or resident memory has already moved into Site children.

Long-term per-site resident-memory, graphics-cache, background-CPU and broader lifecycle accounting remain future work. Those policies must preserve the completed R4 authority boundaries regardless of lifecycle state.

## Security model

R4 Site-process authority cannot directly select privileged OS resources. The Host/Broker issues bounded revocable `Network`, `Clipboard` and R5 `Storage` capability classes to a live Host-owned `SiteProcessId`; production navigation routes additionally bind those grants to the exact `NavigationContextId`. Storage adds a stricter exact-origin binding at grant/use time and checks the live Rarog-owned Storage-process identity before touching storage state. Network origin, credentials, redirect and related Web policy remain owned by the Fetch/navigation layer rather than being inferred from the capability ID.

Future capabilities such as camera, file access or screen capture must add their own explicitly scoped authority and broker checks before any platform side effect. They are examples of later policy design, not capabilities implemented by R4.

On Windows, the R4 process boundary already applies the bounded creation-time mitigation, child-process restriction and Job Object containment policy behind `rarog-platform-windows-native`. This is a concrete containment baseline, not an AppContainer or Chromium-equivalent sandbox claim.

## Compatibility model

Two independent test tracks are mandatory:

### Standards

- Web Platform Tests (WPT)
- ECMAScript/Test262 through the selected JS engine
- WebDriver/WebDriver BiDi tests

### Real Web

`rarog-web-corpus` will maintain reproducible scenarios for popular sites and applications. Compatibility fixes must live in a separately versioned `rarog-compat` subsystem rather than site-name branches in layout/DOM code.

## CI platform policy

Windows is the primary CI platform lane. It runs format, compile checks, Clippy, workspace tests, dedicated R4 child-lifecycle/sandbox/IPC-wire/IPC-transport gates, the R0/P1/R0.1/R1/R2/R3/R4 gates and the bootstrap render. A dedicated Windows SpiderMonkey feature lane runs check, Clippy and adapter tests.

Linux remains the portability lane so accidental Windows-only dependencies in engine-core crates are caught early. It runs workspace checks/tests, the milestone gates through R4, bootstrap render and fuzz-target compilation; a dedicated Linux SpiderMonkey feature lane runs check, Clippy and adapter tests. Rust 1.85 has a separate MSRV job, and dependency advisories are checked by the RustSec workflow.

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

R0 established scalar-indexed `ShapingRun` segments by intersecting logical bidi runs with grapheme-safe font fallback runs. Every shaping segment has exactly one source range, one `FontFaceId`, and one `BidiLevel`/direction, and adjacent segments with identical shaping state are coalesced. R1 connected this handoff to the production `rarog-text-opentype` backend without changing DOM, source ranges, line breaking, fragment identity or retained paint.

### Shaping backend boundary

R0 separated shaping segmentation from shaping execution. `ShapingBackend` receives one `ShapingRun` plus its selected `FontFace` and returns a `ShapedRun` containing positioned glyph IDs, per-glyph advances/offsets and scalar-indexed source-cluster mapping. `FixedTextShaper` remains a deterministic fallback/test backend, while R1's OpenType adapter supplies production glyph shaping without owning bidi, font fallback, source identity, fragmentation or retained-paint policy.

### Shaping request metadata

R0 established backend-neutral shaping metadata in `ShapingRequest`. Every request preserves one resolved `ShapingRun` and adds script classification, a normalized language tag, OpenType feature settings and variation-axis coordinates. Bootstrap requests infer script deterministically from the scalar-indexed source range and default language to `und`; R1's OpenType adapter consumes configured feature/variation metadata behind the same `ShapingBackend` boundary.

