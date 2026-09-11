from pathlib import Path

path = Path("docs/ARCHITECTURE.md")
text = path.read_text()
heading = "### R5 Canvas output revisions and render invalidation"
new_heading = "### R5 bounded WebGL semantic ownership and context loss"

if new_heading in text:
    raise SystemExit(0)

start = text.find(heading)
if start < 0:
    raise SystemExit("Canvas output architecture section not found")
next_heading = text.find("\n### ", start + len(heading))
if next_heading < 0:
    next_heading = len(text)

section = '''

### R5 bounded WebGL semantic ownership and context loss

- `rarog-canvas` remains authoritative for exact Canvas surface identity and rendering-context exclusivity. WebGL acquires only a scoped monotonic external-context lease bound to one exact live `CanvasSurfaceId`; a live 2D context and a live WebGL lease are mutually exclusive.
- `rarog-webgl` owns portable scoped monotonic context, buffer and texture identities plus explicit bounds for live contexts, per-context/global resources, buffer bytes, texture dimensions/pixels and aggregate resource budgets. All aggregate accounting uses checked arithmetic before retained-state mutation.
- WebGL resources belong to exactly one context. Foreign-context, stale and cross-registry identities fail closed and are never reused after retirement.
- Context loss transitions semantic state to `Lost`, retires all resource authority and releases accounted buffer/texture budgets, but retains the exact Canvas external-context lease until deterministic context destruction. Destruction revalidates and releases that lease before context capacity is recovered.
- WebGL semantic contracts expose no GPU device/queue/buffer/texture/view, shader/program/pipeline, command encoder, D3D/DXGI, `wgpu`, platform handle or native pointer authority. The replaceable graphics-backend boundary is intentionally the next R5 Canvas/WebGL slice.
- See [ADR-0133](adr/0133-webgl-ownership-loss.md).
'''

path.write_text(text[:next_heading] + section + text[next_heading:])
