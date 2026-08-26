use rarog_dom::{Document, ElementData, NodeId, NodeKind};
use std::collections::BTreeMap;

/// Bootstrap-only HTML parser.
///
/// This exists solely to exercise the v0.1 end-to-end pipeline. It is not a
/// standards implementation and is intentionally replaceable behind `parse`.
pub fn parse(input: &str) -> Document {
    let mut doc = Document::new();
    let mut stack: Vec<NodeId> = vec![doc.root()];
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(close_rel) = input[i..].find('>') {
                let end = i + close_rel;
                let raw = input[i + 1..end].trim();
                if raw.starts_with('!') {
                    i = end + 1;
                    continue;
                }
                if raw.starts_with('/') {
                    if stack.len() > 1 {
                        stack.pop();
                    }
                } else {
                    let self_closing = raw.ends_with('/');
                    let inner = raw.trim_end_matches('/').trim();
                    let (tag, attrs) = parse_tag(inner);
                    if !tag.is_empty() {
                        let parent = *stack.last().expect("document stack is never empty");
                        let id = doc.append(
                            parent,
                            NodeKind::Element(ElementData {
                                tag_name: tag,
                                attributes: attrs,
                            }),
                        );
                        if !self_closing && !matches_void(doc.node(id)) {
                            stack.push(id);
                        }
                    }
                }
                i = end + 1;
            } else {
                break;
            }
        } else {
            let next = input[i..].find('<').map(|n| i + n).unwrap_or(bytes.len());
            let text = input[i..next]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                let parent = *stack.last().expect("document stack is never empty");
                doc.append(parent, NodeKind::Text(text));
            }
            i = next;
        }
    }
    doc
}

fn parse_tag(input: &str) -> (String, BTreeMap<String, String>) {
    let mut chars = input.char_indices().peekable();
    let mut tag_end = input.len();
    while let Some((idx, ch)) = chars.next() {
        if ch.is_whitespace() {
            tag_end = idx;
            break;
        }
    }
    let tag = input[..tag_end].to_ascii_lowercase();
    let mut attrs = BTreeMap::new();
    let rest = input[tag_end..].trim();
    let mut cursor = 0usize;
    while cursor < rest.len() {
        while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= rest.len() {
            break;
        }
        let name_start = cursor;
        while cursor < rest.len() {
            let b = rest.as_bytes()[cursor];
            if b == b'=' || b.is_ascii_whitespace() {
                break;
            }
            cursor += 1;
        }
        let name = rest[name_start..cursor].to_ascii_lowercase();
        while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let mut value = String::new();
        if cursor < rest.len() && rest.as_bytes()[cursor] == b'=' {
            cursor += 1;
            while cursor < rest.len() && rest.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < rest.len()
                && (rest.as_bytes()[cursor] == b'"' || rest.as_bytes()[cursor] == b'\'')
            {
                let quote = rest.as_bytes()[cursor];
                cursor += 1;
                let start = cursor;
                while cursor < rest.len() && rest.as_bytes()[cursor] != quote {
                    cursor += 1;
                }
                value = rest[start..cursor].to_string();
                if cursor < rest.len() {
                    cursor += 1;
                }
            } else {
                let start = cursor;
                while cursor < rest.len() && !rest.as_bytes()[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                value = rest[start..cursor].to_string();
            }
        }
        if !name.is_empty() {
            attrs.insert(name, value);
        }
    }
    (tag, attrs)
}

fn matches_void(node: &rarog_dom::Node) -> bool {
    match &node.kind {
        NodeKind::Element(el) => matches!(
            el.tag_name.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link"
        ),
        _ => false,
    }
}
