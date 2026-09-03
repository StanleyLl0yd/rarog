from pathlib import Path

path = Path("crates/rarog-paint/src/lib.rs")
s = path.read_text()

start = s.find("    fn rasterize_internal(&mut self, list: &DisplayList, images: Option<&ImageResourceStore>) {")
end = s.find("    pub fn rasterize_damage(\n", start)
if start < 0 or end < 0:
    raise SystemExit("rasterize_internal block markers missing")
old = s[start:end]
body_start = old.find("        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);\n")
if body_start < 0:
    raise SystemExit("rasterize_internal body marker missing")
body = old[body_start:]
body = body.replace(
    "        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);\n",
    '''        let framebuffer = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);\n        let framebuffer_clip = intersection(framebuffer, initial_clip)\n            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));\n''',
    1,
)
replacement = '''    fn rasterize_internal(&mut self, list: &DisplayList, images: Option<&ImageResourceStore>) {
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);
        self.rasterize_clipped_internal(list, images, framebuffer_clip);
    }

    fn rasterize_clipped_internal(
        &mut self,
        list: &DisplayList,
        images: Option<&ImageResourceStore>,
        initial_clip: Rect,
    ) {
''' + body + '''
'''
s = s[:start] + replacement + s[end:]

start = s.find("    fn rasterize_damage_internal(\n")
end = s.find("    fn draw_image(\n", start)
if start < 0 or end < 0:
    raise SystemExit("rasterize_damage_internal block markers missing")
replacement = '''    fn rasterize_damage_internal(
        &mut self,
        list: &DisplayList,
        damage: &DamageRegion,
        background: Color,
        images: Option<&ImageResourceStore>,
    ) {
        let framebuffer = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);
        for damaged in &damage.rects {
            let Some(damaged) = intersection(*damaged, framebuffer) else {
                continue;
            };
            self.clear_rect(damaged, background);
            self.rasterize_clipped_internal(list, images, damaged);
        }
    }

'''
s = s[:start] + replacement + s[end:]

test_marker = '''    #[test]
    fn damage_clear_overwrites_with_transparent_background() {
'''
tests = '''    #[test]
    fn structural_damage_replay_is_clipped_to_damage_rect() {
        let list = DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
                DisplayItemId::test(4),
                DisplayItemId::test(5),
                DisplayItemId::test(6),
                DisplayItemId::test(7),
            ],
            commands: vec![
                DisplayCommand::PushTransform {
                    transform: Transform2D::translation(2.0, 0.0),
                },
                DisplayCommand::PushClip {
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                },
                DisplayCommand::PushOpacity {
                    opacity: Opacity::new(0.5).unwrap(),
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                    color: Color::BLACK,
                },
                DisplayCommand::PopOpacity,
                DisplayCommand::PopClip,
                DisplayCommand::PopTransform,
            ],
        };
        let mut framebuffer = Framebuffer::new(
            Size {
                width: 8.0,
                height: 4.0,
            },
            Color::BLACK,
        );
        let damage = DamageRegion {
            rects: vec![Rect::new(2.0, 0.0, 2.0, 2.0)],
        };

        framebuffer.rasterize_damage(&list, &damage, Color::WHITE);

        assert_eq!(framebuffer.pixels[0], Color::BLACK);
        assert_eq!(framebuffer.pixels[2], Color::rgb(127, 127, 127));
        assert_eq!(framebuffer.pixels[4], Color::BLACK);
        assert_eq!(framebuffer.pixels[18], Color::rgb(127, 127, 127));
        assert_eq!(framebuffer.pixels[20], Color::BLACK);
    }

    #[test]
    fn structural_damage_incremental_replay_matches_full_raster() {
        let make = |translation: f32| DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
                DisplayItemId::test(4),
                DisplayItemId::test(5),
            ],
            commands: vec![
                DisplayCommand::PushTransform {
                    transform: Transform2D::translation(translation, 0.0),
                },
                DisplayCommand::PushOpacity {
                    opacity: Opacity::new(0.5).unwrap(),
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                    color: Color::BLACK,
                },
                DisplayCommand::PopOpacity,
                DisplayCommand::PopTransform,
            ],
        };
        let before = make(0.0);
        let after = make(3.0);
        let damage = DamageRegion::between(Some(&before), &after);
        let size = Size {
            width: 8.0,
            height: 4.0,
        };

        let mut incremental = Framebuffer::new(size, Color::WHITE);
        incremental.rasterize(&before);
        incremental.rasterize_damage(&after, &damage, Color::WHITE);

        let mut full = Framebuffer::new(size, Color::WHITE);
        full.rasterize(&after);

        assert_eq!(incremental.pixels, full.pixels);
    }

'''
if test_marker not in s:
    raise SystemExit("damage test insertion marker missing")
s = s.replace(test_marker, tests + test_marker, 1)

path.write_text(s)
