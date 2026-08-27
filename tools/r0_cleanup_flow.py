from pathlib import Path

path = Path("crates/rarog-layout/src/lib.rs")
text = path.read_text()
start = "pub fn relayout_fragment_flow("
positions = []
offset = 0
while True:
    index = text.find(start, offset)
    if index < 0:
        break
    positions.append(index)
    offset = index + len(start)

if len(positions) < 2:
    raise SystemExit("duplicate flow helper not found")

second = positions[1]
end_marker = "fn find_layout_node("
end = text.find(end_marker, second)
if end < 0:
    raise SystemExit("layout helper end marker not found")

text = text[:second] + text[end:]
path.write_text(text)
