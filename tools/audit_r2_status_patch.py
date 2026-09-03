from pathlib import Path

readme = Path("README.md")
s = readme.read_text()
old = "**R2 — Flight** is the next development milestone and introduces WebIDL, the replaceable script-runtime boundary, events/event-loop work, Fetch/URL/origin foundations and Windows input/IME/clipboard adapters."
new = "**R2 — Flight is in progress**: the Rarog-owned normalized WebIDL IR and parser frontend boundary are established; the standards-oriented WebIDL parser adapter, replaceable script-runtime boundary, events/event-loop work, Fetch/URL/origin foundations and Windows input/IME/clipboard adapters remain active R2 work."
if s.count(old) != 1:
    raise SystemExit("README R2 intro anchor mismatch")
s = s.replace(old, new, 1)

old = "- `rarog-text-opentype` — production OpenType shaping adapter behind Rarog-owned contracts\n- `rarog-paint` — retained structural display list, stable IDs, damage tracking and software rasterizer"
new = "- `rarog-text-opentype` — production OpenType shaping adapter behind Rarog-owned contracts\n- `rarog-webidl` — Rarog-owned normalized WebIDL IR and parser frontend boundary\n- `rarog-paint` — retained structural display list, stable IDs, damage tracking and software rasterizer"
if s.count(old) != 1:
    raise SystemExit("README workspace anchor mismatch")
s = s.replace(old, new, 1)

old = "**R0 — Ember and R1 — Flame are complete. R2 — Flight is next.**"
new = "**R0 — Ember and R1 — Flame are complete. R2 — Flight is in progress.**"
if s.count(old) != 1:
    raise SystemExit("README status anchor mismatch")
s = s.replace(old, new, 1)
readme.write_text(s)

roadmap = Path("docs/ROADMAP.md")
s = roadmap.read_text()
old = "## R2 — Flight — next"
new = "## R2 — Flight — in progress"
if s.count(old) != 1:
    raise SystemExit("ROADMAP R2 heading anchor mismatch")
roadmap.write_text(s.replace(old, new, 1))
