# Security Policy

Rarog treats Web content as hostile input.

The bootstrap renderer is experimental and is **not suitable for processing untrusted content as a security boundary**. Production security claims begin only after the multi-process sandbox/capability milestones are implemented and reviewed.

Security architecture principles:

- site isolation is mandatory for production;
- privileged OS access is brokered;
- capabilities are scoped and revocable;
- parser/rendering failures must not grant privilege;
- unsafe Rust, when eventually unavoidable, is isolated and audited.
