from pathlib import Path

path = Path("crates/rarog-paint/src/lib.rs")
s = path.read_text()

old_clipped = '''        let x0 = clipped.origin.x.floor().max(0.0) as u32;
        let y0 = clipped.origin.y.floor().max(0.0) as u32;
        let x1 = (clipped.origin.x + clipped.size.width)
            .ceil()
            .clamp(0.0, self.width as f32) as u32;
        let y1 = (clipped.origin.y + clipped.size.height)
            .ceil()
            .clamp(0.0, self.height as f32) as u32;
        for y in y0..y1 {
'''
new_clipped = '''        let (x0, y0, x1, y1) = self.pixel_bounds(clipped);
        for y in y0..y1 {
'''
if s.count(old_clipped) != 1:
    raise SystemExit(f"expected one clipped pixel-bound block, found {s.count(old_clipped)}")
s = s.replace(old_clipped, new_clipped, 1)

old_rect = '''        let x0 = rect.origin.x.floor().max(0.0) as u32;
        let y0 = rect.origin.y.floor().max(0.0) as u32;
        let x1 = (rect.origin.x + rect.size.width)
            .ceil()
            .clamp(0.0, self.width as f32) as u32;
        let y1 = (rect.origin.y + rect.size.height)
            .ceil()
            .clamp(0.0, self.height as f32) as u32;
        for y in y0..y1 {
'''
new_rect = '''        let (x0, y0, x1, y1) = self.pixel_bounds(rect);
        for y in y0..y1 {
'''
if s.count(old_rect) != 2:
    raise SystemExit(f"expected two rect pixel-bound blocks, found {s.count(old_rect)}")
s = s.replace(old_rect, new_rect, 2)

anchor = '''    fn draw_image(
        &mut self,
'''
helper = '''    fn pixel_bounds(&self, rect: Rect) -> (u32, u32, u32, u32) {
        let x0 = rect.origin.x.floor().max(0.0) as u32;
        let y0 = rect.origin.y.floor().max(0.0) as u32;
        let x1 = (rect.origin.x + rect.size.width)
            .ceil()
            .clamp(0.0, self.width as f32) as u32;
        let y1 = (rect.origin.y + rect.size.height)
            .ceil()
            .clamp(0.0, self.height as f32) as u32;
        (x0, y0, x1, y1)
    }

'''
if s.count(anchor) != 1:
    raise SystemExit("draw_image anchor mismatch")
s = s.replace(anchor, helper + anchor, 1)
path.write_text(s)
