from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


layout_path = Path("crates/rarog-layout/src/lib.rs")
layout = layout_path.read_text()
layout = replace_once(
    layout,
    '''        for node in nodes {
            match &node.kind {
                LayoutNodeKind::Box if fragmenting_inline_text_stream(node).is_some() => {
                    suppress_leading_margin = false;
                    if !line.active {
                        *cursor_y += pending_margin.resolved();
                        pending_margin = MarginStrut::default();
                    }
                    let stream = fragmenting_inline_text_stream(node)
                        .expect("fragmenting inline guard validated the text stream");
                    self.layout_inline_text_stream_flow(
                        stream,
                        InlineTextContainerFlow {
                            containing_block,
                            cursor_y,
                            line: &mut line,
                            fragments: &mut fragments,
                        },
                    );
                }
                LayoutNodeKind::Box if node.style.display_inline => {
''',
    '''        for node in nodes {
            if matches!(&node.kind, LayoutNodeKind::Box) {
                if let Some(stream) = fragmenting_inline_text_stream(node) {
                    suppress_leading_margin = false;
                    if !line.active {
                        *cursor_y += pending_margin.resolved();
                        pending_margin = MarginStrut::default();
                    }
                    self.layout_inline_text_stream_flow(
                        stream,
                        InlineTextContainerFlow {
                            containing_block,
                            cursor_y,
                            line: &mut line,
                            fragments: &mut fragments,
                        },
                    );
                    continue;
                }
            }

            match &node.kind {
                LayoutNodeKind::Box if node.style.display_inline => {
''',
    "single inline stream construction",
)
layout_path.write_text(layout)

paint_path = Path("crates/rarog-paint/src/lib.rs")
paint = paint_path.read_text()
paint = replace_once(
    paint,
    '''    let (Some(before_scopes), Some(after_scopes)) = (
        scope_stack_at(&list.commands, range.start),
        scope_stack_at(&list.commands, range.end),
    ) else {
        return false;
    };
    if before_scopes != after_scopes {
        return false;
    }
''',
    '''    let Some(before_scopes) = scope_stack_at(&list.commands, range.start) else {
        return false;
    };
    let mut after_scopes = before_scopes.clone();
    for command in &list.commands[range.start..range.end] {
        if !apply_scope_command(*command, &mut after_scopes) {
            return false;
        }
    }
    if before_scopes != after_scopes {
        return false;
    }
''',
    "single structural prefix scan",
)
paint_path.write_text(paint)
