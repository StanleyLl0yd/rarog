from pathlib import Path

path = Path("crates/rarog-paint/src/lib.rs")
s = path.read_text()
start = s.find("    #[test]\n    fn structural_damage_replay_is_clipped_to_damage_rect() {")
end = s.find("    #[test]\n    fn structural_damage_incremental_replay_matches_full_raster() {", start)
if start < 0 or end < 0:
    raise SystemExit("structural damage clipping test marker missing")
replacement = '''    #[test]
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
                    opacity: Opacity::ONE,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 4.0, 4.0),
                    color: Color::rgb(255, 0, 0),
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
        assert_eq!(framebuffer.pixels[2], Color::rgb(255, 0, 0));
        assert_eq!(framebuffer.pixels[3], Color::rgb(255, 0, 0));
        assert_eq!(framebuffer.pixels[4], Color::BLACK);
        assert_eq!(framebuffer.pixels[10], Color::rgb(255, 0, 0));
        assert_eq!(framebuffer.pixels[18], Color::BLACK);
        assert_eq!(framebuffer.pixels[20], Color::BLACK);
    }

'''
s = s[:start] + replacement + s[end:]
path.write_text(s)
