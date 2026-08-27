from pathlib import Path

paint = Path('crates/rarog-paint/src/lib.rs')
text = paint.read_text()

text = text.replace(
'''pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
}

impl DisplayCommand {
    pub fn bounds(self) -> Rect {
        match self {
            Self::FillRect { rect, .. } | Self::TextPlaceholder { rect, .. } => rect,
        }
    }
}
''',
'''pub enum DisplayCommand {
    FillRect { rect: Rect, color: Color },
    TextPlaceholder { rect: Rect, color: Color },
    PushClip { rect: Rect },
    PopClip,
}

impl DisplayCommand {
    pub fn bounds(self) -> Option<Rect> {
        match self {
            Self::FillRect { rect, .. }
            | Self::TextPlaceholder { rect, .. }
            | Self::PushClip { rect } => Some(rect),
            Self::PopClip => None,
        }
    }

    fn is_clip(self) -> bool {
        matches!(self, Self::PushClip { .. } | Self::PopClip)
    }
}
''',
1,
)

text = text.replace(
'''                DisplayCommand::TextPlaceholder { rect, color } => output.push_str(&format!(
                    "{}|text-placeholder|{}|{:02x}{:02x}{:02x}{:02x}\\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect),
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
''',
'''                DisplayCommand::TextPlaceholder { rect, color } => output.push_str(&format!(
                    "{}|text-placeholder|{}|{:02x}{:02x}{:02x}{:02x}\\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect),
                    color.r,
                    color.g,
                    color.b,
                    color.a
                )),
                DisplayCommand::PushClip { rect } => output.push_str(&format!(
                    "{}|push-clip|{}\\n",
                    display_item_id_snapshot(*id),
                    rect_snapshot(*rect)
                )),
                DisplayCommand::PopClip => output.push_str(&format!(
                    "{}|pop-clip\\n",
                    display_item_id_snapshot(*id)
                )),
''',
1,
)

text = text.replace(
'''            for command in &current.commands {
                damage.push(command.bounds());
            }
            return damage;
''',
'''            for command in &current.commands {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
''',
1,
)

text = text.replace(
'''        let previous_items = indexed_commands(previous);
        let current_items = indexed_commands(current);
''',
'''        if previous.commands.iter().copied().any(DisplayCommand::is_clip)
            || current.commands.iter().copied().any(DisplayCommand::is_clip)
        {
            let mut damage = Self::default();
            for command in previous.commands.iter().chain(&current.commands) {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            return damage;
        }

        let previous_items = indexed_commands(previous);
        let current_items = indexed_commands(current);
''',
1,
)

text = text.replace(
'''            if let Some(command) = before {
                damage.push(command.bounds());
            }
            if let Some(command) = after {
                damage.push(command.bounds());
            }
''',
'''            if let Some(command) = before {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
            if let Some(command) = after {
                if let Some(bounds) = command.bounds() {
                    damage.push(bounds);
                }
            }
''',
1,
)

text = text.replace(
'''    pub fn rasterize(&mut self, list: &DisplayList) {
        for command in &list.commands {
            match *command {
                DisplayCommand::FillRect { rect, color }
                | DisplayCommand::TextPlaceholder { rect, color } => self.fill_rect(rect, color),
            }
        }
    }
''',
'''    pub fn rasterize(&mut self, list: &DisplayList) {
        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);
        let mut clips = vec![framebuffer_clip];
        for command in &list.commands {
            match *command {
                DisplayCommand::FillRect { rect, color }
                | DisplayCommand::TextPlaceholder { rect, color } => {
                    if let Some(clipped) = intersection(rect, *clips.last().expect("clip stack")) {
                        self.fill_rect(clipped, color);
                    }
                }
                DisplayCommand::PushClip { rect } => {
                    let current = *clips.last().expect("clip stack");
                    clips.push(intersection(current, rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)));
                }
                DisplayCommand::PopClip => {
                    if clips.len() > 1 {
                        clips.pop();
                    }
                }
            }
        }
    }
''',
1,
)

text = text.replace(
'''    pub fn rasterize_damage(
        &mut self,
        list: &DisplayList,
        damage: &DamageRegion,
        background: Color,
    ) {
        for damaged in &damage.rects {
''',
'''    pub fn rasterize_damage(
        &mut self,
        list: &DisplayList,
        damage: &DamageRegion,
        background: Color,
    ) {
        if list.commands.iter().copied().any(DisplayCommand::is_clip) {
            self.fill_rect(
                Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
                background,
            );
            self.rasterize(list);
            return;
        }
        for damaged in &damage.rects {
''',
1,
)

text = text.replace(
'''                let (rect, color) = match *command {
                    DisplayCommand::FillRect { rect, color }
                    | DisplayCommand::TextPlaceholder { rect, color } => (rect, color),
                };
''',
'''                let (rect, color) = match *command {
                    DisplayCommand::FillRect { rect, color }
                    | DisplayCommand::TextPlaceholder { rect, color } => (rect, color),
                    DisplayCommand::PushClip { .. } | DisplayCommand::PopClip => continue,
                };
''',
1,
)

module_end = text.rfind('\n}')
insert = r'''

    #[test]
    fn nested_clip_commands_constrain_rasterization() {
        let mut list = DisplayList::default();
        list.push(DisplayItemId::test(1), DisplayCommand::PushClip { rect: Rect::new(1.0, 1.0, 4.0, 4.0) });
        list.push(DisplayItemId::test(2), DisplayCommand::FillRect { rect: Rect::new(0.0, 0.0, 8.0, 8.0), color: Color::BLACK });
        list.push(DisplayItemId::test(3), DisplayCommand::PushClip { rect: Rect::new(3.0, 0.0, 4.0, 4.0) });
        list.push(DisplayItemId::test(4), DisplayCommand::FillRect { rect: Rect::new(0.0, 0.0, 8.0, 8.0), color: Color::rgb(255, 0, 0) });
        list.push(DisplayItemId::test(5), DisplayCommand::PopClip);
        list.push(DisplayItemId::test(6), DisplayCommand::PopClip);

        let mut framebuffer = Framebuffer::new(Size::new(8.0, 8.0), Color::WHITE);
        framebuffer.rasterize(&list);

        assert_eq!(framebuffer.pixels[(2 * 8 + 2) as usize], Color::BLACK);
        assert_eq!(framebuffer.pixels[(2 * 8 + 3) as usize], Color::rgb(255, 0, 0));
        assert_eq!(framebuffer.pixels[0], Color::WHITE);
        assert_eq!(framebuffer.pixels[(6 * 8 + 6) as usize], Color::WHITE);
    }

    #[test]
    fn clip_display_commands_have_deterministic_snapshots() {
        let mut list = DisplayList::default();
        list.push(DisplayItemId::test(7), DisplayCommand::PushClip { rect: Rect::new(1.0, 2.0, 3.0, 4.0) });
        list.push(DisplayItemId::test(8), DisplayCommand::PopClip);
        assert_eq!(list.snapshot(), "7:7:0|push-clip|1.0,2.0,3.0,4.0\n8:8:0|pop-clip\n");
    }

    #[test]
    fn damage_raster_with_clips_matches_full_raster() {
        let mut previous = DisplayList::default();
        previous.push(DisplayItemId::test(1), DisplayCommand::PushClip { rect: Rect::new(1.0, 1.0, 4.0, 4.0) });
        previous.push(DisplayItemId::test(2), DisplayCommand::FillRect { rect: Rect::new(0.0, 0.0, 8.0, 8.0), color: Color::BLACK });
        previous.push(DisplayItemId::test(3), DisplayCommand::PopClip);

        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect { rect: Rect::new(0.0, 0.0, 8.0, 8.0), color: Color::rgb(0, 255, 0) };
        let damage = DamageRegion::between(Some(&previous), &current);

        let mut incremental = Framebuffer::new(Size::new(8.0, 8.0), Color::WHITE);
        incremental.rasterize(&previous);
        incremental.rasterize_damage(&current, &damage, Color::WHITE);

        let mut full = Framebuffer::new(Size::new(8.0, 8.0), Color::WHITE);
        full.rasterize(&current);
        assert_eq!(incremental.pixels, full.pixels);
    }
'''
text = text[:module_end] + insert + text[module_end:]
paint.write_text(text)

backlog = Path('docs/R0-BACKLOG.md')
text = backlog.read_text().replace('- [ ] clip commands', '- [x] clip commands with nested software-raster clip-stack semantics and conservative damage fallback', 1)
backlog.write_text(text)

architecture = Path('docs/ARCHITECTURE.md')
text = architecture.read_text()
needle = 'The display list remains backend-neutral.'
pos = text.find(needle)
if pos < 0:
    raise SystemExit('architecture paint section marker not found')
paragraph_end = text.find('\n\n', pos)
addition = '\n\nClip commands are explicit backend-neutral display-list operations. R0 rasterization maintains a nested rectangular clip stack. Damage-scoped rasterization conservatively falls back to a full framebuffer refresh whenever clips are present; clip-aware retained damage remains intentionally deferred until stacking and fragmentation semantics are defined.'
text = text[:paragraph_end] + addition + text[paragraph_end:]
architecture.write_text(text)
