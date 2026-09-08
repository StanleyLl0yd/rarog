# ADR-0111: Bounded Windows Site-process sandbox and native boundary

**Status:** Accepted  
**Date:** 2026-09-08

## Context

R4 has Host-owned Site-process authority, real Windows child lifetimes and bounded Windows local IPC. The remaining Windows hardening step must ensure a Site child cannot begin executing outside the first concrete Rarog containment policy, while keeping Win32 handle authority out of portable and ordinary platform crates.

A blanket dynamic-code prohibition is incompatible with the planned SpiderMonkey JIT. AppContainer also requires a dedicated executable/resource/IPC ACL contract that R4 does not yet have.

## Decision

The first R4 Windows Site-process sandbox is a bounded process-hardening baseline:

- DEP and SEHOP;
- mandatory image relocation, bottom-up ASLR and high-entropy ASLR;
- heap termination on corruption;
- strict handle checks;
- legacy extension-point disable;
- child-process creation restriction;
- one Job Object per Site child with active-process limit one and kill-on-close containment.

The policy is installed through extended `CreateProcessW` attributes. The Job Object is supplied with `PROC_THREAD_ATTRIBUTE_JOB_LIST`, so the child is created inside containment instead of executing first and being assigned afterward. Handle inheritance is disabled.

Dynamic-code generation is intentionally not prohibited in this baseline so SpiderMonkey JIT remains architecturally possible.

Unavoidable Win32 process, attribute-list, mitigation-query and Job Object calls live in the dedicated `rarog-platform-windows-native` crate. That crate exposes a safe `SandboxedChild` and bounded `SandboxEvidence` API. Native HANDLE/PID/token/Job Object types never cross into `rarog-platform-windows`, `rarog-host`, IPC or engine contracts.

Ordinary workspace crates retain `unsafe_code = "forbid"`. The native crate alone allows audited unsafe blocks and denies unsafe operations inside unsafe functions unless explicitly wrapped.

## Validation

Windows-primary CI runs a concrete sandbox gate that:

- creates a child under the policy;
- queries supported mitigation policy state and Job Object limits;
- verifies active-process limit one and kill-on-close;
- verifies nested child creation is denied.

The existing high-level Windows Site-process lifecycle regression also requires sandbox evidence before proving process loss revokes Host authority and recovery allocates fresh Rarog identity.

## Consequences

R4 gains a real Windows process-containment boundary without making OS process details authoritative and without disabling a future JIT.

This is not a Chromium-equivalent sandbox. R4 does not claim AppContainer/resource ACL isolation, complete broker coverage for future Web-platform capabilities, or wholesale migration of current engine execution into Site children.
