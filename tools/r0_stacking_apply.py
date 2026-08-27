from pathlib import Path

paint = Path('crates/rarog-paint/src/lib.rs')
text = paint.read_text()

text = text.replace(
'''#[derive(Clone, Copy, Debug, PartialEq)]\npub enum DisplayCommand {\n    FillRect { rect: Rect, color: Color },\n    TextPlaceholder { rect: Rect, color: Color },\n    PushClip { rect: Rect },\n    PopClip,\n}\n\nimpl DisplayCommand {\n    pub fn bounds(self) -> Option<Rect> {\n        match self {\n            Self::FillRect { rect, .. }\n            | Self::TextPlaceholder { rect, .. }\n            | Self::PushClip { rect } => Some(rect),\n            Self::PopClip => None,\n        }\n    }\n\n    fn is_clip(self) -> bool {\n        matches!(self, Self::PushClip { .. } | Self::PopClip)\n    }\n}\n''',
'''#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct StackingContextId(pub u64);\n\n#[derive(Clone, Copy, Debug, PartialEq)]\npub enum DisplayCommand {\n    FillRect { rect: Rect, color: Color },\n    TextPlaceholder { rect: Rect, color: Color },\n    PushClip { rect: Rect },\n    PopClip,\n    PushStackingContext { id: StackingContextId },\n    PopStackingContext,\n}\n\nimpl DisplayCommand {\n    pub fn bounds(self) -> Option<Rect> {\n        match self {\n            Self::FillRect { rect, .. }\n            | Self::TextPlaceholder { rect, .. }\n            | Self::PushClip { rect } => Some(rect),\n            Self::PopClip | Self::PushStackingContext { .. } | Self::PopStackingContext => None,\n        }\n    }\n\n    fn is_structural(self) -> bool {\n        matches!(\n            self,\n            Self::PushClip { .. }\n                | Self::PopClip\n                | Self::PushStackingContext { .. }\n                | Self::PopStackingContext\n        )\n    }\n}\n''',
1,
)

text = text.replace(
'''    pub fn has_unique_ids(&self) -> bool {\n        self.command_ids\n            .iter()\n            .copied()\n            .collect::<BTreeSet<_>>()\n            .len()\n            == self.command_ids.len()\n    }\n\n    pub fn snapshot(&self) -> String {\n''',
'''    pub fn has_unique_ids(&self) -> bool {\n        self.command_ids\n            .iter()\n            .copied()\n            .collect::<BTreeSet<_>>()\n            .len()\n            == self.command_ids.len()\n    }\n\n    pub fn has_balanced_structure(&self) -> bool {\n        #[derive(Clone, Copy, PartialEq, Eq)]\n        enum Scope {\n            Clip,\n            Stacking,\n        }\n\n        let mut scopes = Vec::new();\n        for command in &self.commands {\n            match command {\n                DisplayCommand::PushClip { .. } => scopes.push(Scope::Clip),\n                DisplayCommand::PushStackingContext { .. } => scopes.push(Scope::Stacking),\n                DisplayCommand::PopClip => {\n                    if scopes.pop() != Some(Scope::Clip) {\n                        return false;\n                    }\n                }\n                DisplayCommand::PopStackingContext => {\n                    if scopes.pop() != Some(Scope::Stacking) {\n                        return false;\n                    }\n                }\n                DisplayCommand::FillRect { .. } | DisplayCommand::TextPlaceholder { .. } => {}\n            }\n        }\n        scopes.is_empty()\n    }\n\n    pub fn snapshot(&self) -> String {\n''',
1,
)

text = text.replace(
'''                DisplayCommand::PopClip => {\n                    output.push_str(&format!("{}|pop-clip\\n", display_item_id_snapshot(*id)))\n                }\n''',
'''                DisplayCommand::PopClip => {\n                    output.push_str(&format!("{}|pop-clip\\n", display_item_id_snapshot(*id)))\n                }\n                DisplayCommand::PushStackingContext { id: context } => output.push_str(&format!(\n                    "{}|push-stacking-context|{}\\n",\n                    display_item_id_snapshot(*id),\n                    context.0\n                )),\n                DisplayCommand::PopStackingContext => output.push_str(&format!(\n                    "{}|pop-stacking-context\\n",\n                    display_item_id_snapshot(*id)\n                )),\n''',
1,
)

text = text.replace(
'''    assert!(list.has_unique_ids(), "display item IDs must be unique");\n    list\n''',
'''    assert!(list.has_unique_ids(), "display item IDs must be unique");\n    assert!(\n        list.has_balanced_structure(),\n        "display list structural scopes must be balanced"\n    );\n    list\n''',
1,
)

text = text.replace('DisplayCommand::is_clip)', 'DisplayCommand::is_structural)')

text = text.replace(
'''    pub fn rasterize(&mut self, list: &DisplayList) {\n        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);\n''',
'''    pub fn rasterize(&mut self, list: &DisplayList) {\n        assert!(\n            list.has_balanced_structure(),\n            "display list structural scopes must be balanced"\n        );\n        let framebuffer_clip = Rect::new(0.0, 0.0, self.width as f32, self.height as f32);\n''',
1,
)

text = text.replace(
'''                DisplayCommand::PopClip => {\n                    if clips.len() > 1 {\n                        clips.pop();\n                    }\n                }\n''',
'''                DisplayCommand::PopClip => {\n                    if clips.len() > 1 {\n                        clips.pop();\n                    }\n                }\n                DisplayCommand::PushStackingContext { .. }\n                | DisplayCommand::PopStackingContext => {}\n''',
1,
)

text = text.replace(
'''                    DisplayCommand::PushClip { .. } | DisplayCommand::PopClip => continue,\n''',
'''                    DisplayCommand::PushClip { .. }\n                    | DisplayCommand::PopClip\n                    | DisplayCommand::PushStackingContext { .. }\n                    | DisplayCommand::PopStackingContext => continue,\n''',
1,
)

module_end = text.rfind('\n}')
insert = r'''

    #[test]
    fn stacking_context_commands_have_deterministic_snapshots() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(20),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(42),
            },
        );
        list.push(DisplayItemId::test(21), DisplayCommand::PopStackingContext);

        assert_eq!(
            list.snapshot(),
            "20:20:0|push-stacking-context|42\n21:21:0|pop-stacking-context\n"
        );
        assert!(list.has_balanced_structure());
    }

    #[test]
    fn structural_scopes_must_be_properly_nested() {
        let mut valid = DisplayList::default();
        valid.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        valid.push(
            DisplayItemId::test(2),
            DisplayCommand::PushClip {
                rect: Rect::new(0.0, 0.0, 4.0, 4.0),
            },
        );
        valid.push(DisplayItemId::test(3), DisplayCommand::PopClip);
        valid.push(DisplayItemId::test(4), DisplayCommand::PopStackingContext);
        assert!(valid.has_balanced_structure());

        let mut invalid = valid.clone();
        invalid.commands.swap(2, 3);
        assert!(!invalid.has_balanced_structure());
    }

    #[test]
    fn stacking_context_boundaries_preserve_current_raster_output() {
        let rect = Rect::new(1.0, 1.0, 3.0, 3.0);
        let mut plain = DisplayList::default();
        plain.push(
            DisplayItemId::test(1),
            DisplayCommand::FillRect {
                rect,
                color: Color::BLACK,
            },
        );

        let mut stacked = DisplayList::default();
        stacked.push(
            DisplayItemId::test(2),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(9),
            },
        );
        stacked.push(
            DisplayItemId::test(3),
            DisplayCommand::FillRect {
                rect,
                color: Color::BLACK,
            },
        );
        stacked.push(DisplayItemId::test(4), DisplayCommand::PopStackingContext);

        let size = Size { width: 6.0, height: 6.0 };
        let mut plain_fb = Framebuffer::new(size, Color::WHITE);
        plain_fb.rasterize(&plain);
        let mut stacked_fb = Framebuffer::new(size, Color::WHITE);
        stacked_fb.rasterize(&stacked);
        assert_eq!(plain_fb.pixels, stacked_fb.pixels);
    }

    #[test]
    fn damage_raster_with_stacking_contexts_matches_full_raster() {
        let mut previous = DisplayList::default();
        previous.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        previous.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(1.0, 1.0, 4.0, 4.0),
                color: Color::BLACK,
            },
        );
        previous.push(DisplayItemId::test(3), DisplayCommand::PopStackingContext);

        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect {
            rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            color: Color::rgb(0, 0, 255),
        };
        let damage = DamageRegion::between(Some(&previous), &current);

        let size = Size { width: 6.0, height: 6.0 };
        let mut incremental = Framebuffer::new(size, Color::WHITE);
        incremental.rasterize(&previous);
        incremental.rasterize_damage(&current, &damage, Color::WHITE);
        let mut full = Framebuffer::new(size, Color::WHITE);
        full.rasterize(&current);
        assert_eq!(incremental.pixels, full.pixels);
    }
'''
text = text[:module_end] + insert + text[module_end:]
paint.write_text(text)

backlog = Path('docs/R0-BACKLOG.md')
text = backlog.read_text().replace(
    '- [ ] stacking-context representation',
    '- [x] stacking-context representation with explicit balanced display-list scopes',
    1,
)
backlog.write_text(text)

architecture = Path('docs/ARCHITECTURE.md')
text = architecture.read_text()
needle = 'Clip commands are explicit backend-neutral display-list operations.'
pos = text.find(needle)
if pos < 0:
    raise SystemExit('clip architecture marker not found')
paragraph_end = text.find('\n\n', pos)
addition = '\n\nStacking contexts are represented as explicit balanced display-list scopes with stable context IDs. R0 does not yet assign CSS stacking order, opacity, transforms, or compositing behavior to these scopes; the representation exists so those semantics can be added without changing the display-list contract again.'
text = text[:paragraph_end] + addition + text[paragraph_end:]
architecture.write_text(text)
