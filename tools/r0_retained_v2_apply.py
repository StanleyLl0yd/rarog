from pathlib import Path

paint = Path('crates/rarog-paint/src/lib.rs')
text = paint.read_text()

old = '''fn replace_display_items(\n    list: &mut DisplayList,\n    previous: &DisplayList,\n    current: &DisplayList,\n) -> bool {\n    if previous.command_ids.is_empty() {\n        return current.command_ids.is_empty();\n    }\n\n    let removed = previous\n        .command_ids\n        .iter()\n        .copied()\n        .collect::<BTreeSet<_>>();\n    if !list.command_ids.iter().any(|id| removed.contains(id)) {\n        return false;\n    }\n\n    let mut command_ids = Vec::with_capacity(\n        list.command_ids.len() - previous.command_ids.len().min(list.command_ids.len())\n            + current.command_ids.len(),\n    );\n    let mut commands = Vec::with_capacity(command_ids.capacity());\n    let mut inserted = false;\n\n    for (id, command) in list\n        .command_ids\n        .iter()\n        .copied()\n        .zip(list.commands.iter().copied())\n    {\n        if removed.contains(&id) {\n            if !inserted {\n                command_ids.extend_from_slice(&current.command_ids);\n                commands.extend_from_slice(&current.commands);\n                inserted = true;\n            }\n            continue;\n        }\n        command_ids.push(id);\n        commands.push(command);\n    }\n\n    if !inserted {\n        return false;\n    }\n    list.command_ids = command_ids;\n    list.commands = commands;\n    true\n}\n'''

new = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct DisplayRange {\n    pub start: usize,\n    pub end: usize,\n}\n\nimpl DisplayRange {\n    pub fn len(self) -> usize {\n        self.end.saturating_sub(self.start)\n    }\n\n    pub fn is_empty(self) -> bool {\n        self.start >= self.end\n    }\n}\n\nimpl DisplayList {\n    pub fn contiguous_range_for_ids(&self, ids: &[DisplayItemId]) -> Option<DisplayRange> {\n        if ids.is_empty() || ids.len() > self.command_ids.len() {\n            return None;\n        }\n        let start = self\n            .command_ids\n            .windows(ids.len())\n            .position(|window| window == ids)?;\n        Some(DisplayRange {\n            start,\n            end: start + ids.len(),\n        })\n    }\n}\n\nfn replace_display_items(\n    list: &mut DisplayList,\n    previous: &DisplayList,\n    current: &DisplayList,\n) -> bool {\n    if previous.command_ids.is_empty() {\n        return current.command_ids.is_empty();\n    }\n    if !previous.has_balanced_structure() || !current.has_balanced_structure() {\n        return false;\n    }\n\n    let Some(range) = list.contiguous_range_for_ids(&previous.command_ids) else {\n        return false;\n    };\n    if range.len() != previous.command_ids.len() {\n        return false;\n    }\n\n    list.command_ids\n        .splice(range.start..range.end, current.command_ids.iter().copied());\n    list.commands\n        .splice(range.start..range.end, current.commands.iter().copied());\n\n    list.has_unique_ids() && list.has_balanced_structure()\n}\n'''

if old not in text:
    raise SystemExit('retained replacement marker not found')
text = text.replace(old, new, 1)

marker = '''    #[test]\n    fn retained_display_patch_preserves_unrelated_items() {\n'''
pos = text.find(marker)
if pos < 0:
    raise SystemExit('retained test marker not found')
insert = r'''    #[test]
    fn retained_range_requires_exact_contiguous_ids() {
        let list = DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(2.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        assert_eq!(
            list.contiguous_range_for_ids(&[DisplayItemId::test(2), DisplayItemId::test(3)]),
            Some(DisplayRange { start: 1, end: 3 })
        );
        assert_eq!(
            list.contiguous_range_for_ids(&[DisplayItemId::test(1), DisplayItemId::test(3)]),
            None
        );
    }

    #[test]
    fn retained_patch_rejects_noncontiguous_previous_items() {
        let mut list = DisplayList {
            command_ids: vec![
                DisplayItemId::test(1),
                DisplayItemId::test(2),
                DisplayItemId::test(3),
            ],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(2.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        let previous = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(3)],
            commands: vec![list.commands[0], list.commands[2]],
        };
        let current = previous.clone();
        assert!(!replace_display_items(&mut list, &previous, &current));
    }

    #[test]
    fn retained_patch_preserves_balanced_structural_scopes() {
        let mut list = DisplayList::default();
        list.push(
            DisplayItemId::test(1),
            DisplayCommand::PushStackingContext {
                id: StackingContextId(1),
            },
        );
        list.push(
            DisplayItemId::test(2),
            DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                color: Color::BLACK,
            },
        );
        list.push(DisplayItemId::test(3), DisplayCommand::PopStackingContext);

        let previous = list.clone();
        let mut current = previous.clone();
        current.commands[1] = DisplayCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 3.0, 2.0),
            color: Color::BLACK,
        };
        assert!(replace_display_items(&mut list, &previous, &current));
        assert!(list.has_balanced_structure());
        assert_eq!(list, current);
    }

'''
text = text[:pos] + insert + text[pos:]
paint.write_text(text)

backlog = Path('docs/R0-BACKLOG.md')
text = backlog.read_text().replace(
    '- [x] retained display-list replacement experiment for affected fragment subtrees',
    '- [x] retained display-list replacement experiment for affected fragment subtrees\n- [x] retained display-list v2 uses exact contiguous ranges and preserves clip/stacking scope balance',
    1,
)
backlog.write_text(text)

architecture = Path('docs/ARCHITECTURE.md')
text = architecture.read_text()
needle = 'Stacking contexts are represented as explicit balanced display-list scopes'
pos = text.find(needle)
if pos < 0:
    raise SystemExit('stacking architecture marker not found')
paragraph_end = text.find('\n\n', pos)
addition = '\n\nRetained display-list replacement operates on exact contiguous command ranges rather than unordered ID sets. A patch is accepted only when the previous range is contiguous and both replacement and resulting lists preserve unique IDs and balanced structural scopes.'
text = text[:paragraph_end] + addition + text[paragraph_end:]
architecture.write_text(text)
