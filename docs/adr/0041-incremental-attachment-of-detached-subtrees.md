# ADR-0041 — Incremental attachment of detached subtrees

Status: Accepted

## Context

R1 structural retention now handles connected child insertion, reparenting and detach by rebuilding only affected retained layout roots. One deliberate fallback remained: a node created with `Document::create_node` while detached would force a full rebuild if it was later attached before the next render update, because the final connected state made its earlier `NodeCreated` mutation appear structural by itself.

That is unnecessarily conservative. Public DOM mutation APIs cannot make a detached node connected without a later `Reparented` mutation (or an equivalent connected structural mutation). The connected parent from that structural mutation already provides the retained layout root needed to materialize the new subtree.

## Decision

Treat `NodeCreated` as identity/allocation history rather than an independent connected-layout invalidation.

During an incremental update:

- collect created nodes that are connected in the final DOM;
- require every such node to be contained by one of the structural relayout roots discovered from the same mutation batch;
- if any connected created node is not covered by a known structural root, keep the deterministic full-rebuild fallback;
- when the structural root is refreshed, allow the normal layout builder to allocate fresh `LayoutNodeId` values for newly materialized nodes while retaining existing IDs outside and around the attachment;
- consume style and text dirty candidates that are inside a successfully refreshed structural root, because final-DOM subtree reconstruction has already applied those mutations;
- keep connected `<style>` source changes on the stylesheet/full-rebuild path.

## Consequences

A detached subtree may now be created, styled, populated with additional detached nodes/text, and attached to an existing connected parent within one update generation without rebuilding the whole Layout Tree.

The parent and unaffected prefix retain layout/fragment identity, the attached nodes receive new layout identity, flow-aware fragment relayout and retained display-list suffix replacement remain available, and output must continue to match a fresh render.

The coverage check is intentionally defensive: if future DOM APIs introduce a way for `NodeCreated` state to become visible without a structural mutation that names a retained root, the engine will fall back rather than silently under-invalidate.
