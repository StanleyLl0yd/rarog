from pathlib import Path

path = Path('crates/rarog-paint/src/lib.rs')
text = path.read_text()
old = '''    list.command_ids
        .splice(range.start..range.end, current.command_ids.iter().copied());
    list.commands
        .splice(range.start..range.end, current.commands.iter().copied());

    list.has_unique_ids() && list.has_balanced_structure()
}
'''
new = '''    let mut candidate = list.clone();
    candidate
        .command_ids
        .splice(range.start..range.end, current.command_ids.iter().copied());
    candidate
        .commands
        .splice(range.start..range.end, current.commands.iter().copied());

    if !candidate.has_unique_ids() || !candidate.has_balanced_structure() {
        return false;
    }
    *list = candidate;
    true
}
'''
if old not in text:
    raise SystemExit('atomic replacement marker not found')
text = text.replace(old, new, 1)

marker = '''    #[test]
    fn retained_patch_preserves_balanced_structural_scopes() {
'''
pos = text.find(marker)
if pos < 0:
    raise SystemExit('retained scope test marker not found')
insert = r'''    #[test]
    fn retained_patch_failure_is_atomic() {
        let mut list = DisplayList {
            command_ids: vec![DisplayItemId::test(1), DisplayItemId::test(2)],
            commands: vec![
                DisplayCommand::FillRect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
                DisplayCommand::FillRect {
                    rect: Rect::new(1.0, 0.0, 1.0, 1.0),
                    color: Color::BLACK,
                },
            ],
        };
        let original = list.clone();
        let previous = DisplayList {
            command_ids: vec![DisplayItemId::test(1)],
            commands: vec![list.commands[0]],
        };
        let current = DisplayList {
            command_ids: vec![DisplayItemId::test(2)],
            commands: vec![DisplayCommand::FillRect {
                rect: Rect::new(0.0, 0.0, 2.0, 1.0),
                color: Color::BLACK,
            }],
        };

        assert!(!replace_display_items(&mut list, &previous, &current));
        assert_eq!(list, original);
    }

'''
text = text[:pos] + insert + text[pos:]
path.write_text(text)
