from pathlib import Path

path = Path("crates/rarog-paint/src/lib.rs")
s = path.read_text()

start = s.find("    fn is_structural(self) -> bool {")
if start >= 0:
    end = s.find("    }\n", start)
    if end < 0:
        raise SystemExit("is_structural end missing")
    end += len("    }\n")
    s = s[:start] + s[end:]
elif "fn is_structural(self)" in s:
    raise SystemExit("unexpected is_structural shape")

start = s.find("fn effective_paint_bounds(list: &DisplayList) -> Vec<Rect> {")
if start >= 0:
    end_marker = "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum FramebufferError"
    end = s.find(end_marker, start)
    if end < 0:
        raise SystemExit("effective_paint_bounds end marker missing")
    s = s[:start] + s[end:]

path.write_text(s)
