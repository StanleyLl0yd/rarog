from pathlib import Path

path = Path("crates/rarog-engine/src/lib.rs")
text = path.read_text(encoding="utf-8")
replacements = [
(
"""        let mutations = self
            .document
            .mutation_records_since(from_generation)
            .map(|record| record.kind.clone())
            .collect::<Vec<_>>();""",
"""        let (mutations, mutation_history_lost) =
            match self.document.mutation_records_since(from_generation) {
                Ok(records) => (
                    records
                        .map(|record| record.kind.clone())
                        .collect::<Vec<_>>(),
                    false,
                ),
                Err(_) => (Vec::new(), true),
            };""",
),
(
"""        if mutations.is_empty() || dirty_nodes == 0 {""",
"""        if !mutation_history_lost && (mutations.is_empty() || dirty_nodes == 0) {""",
),
(
"""        let mut requires_full_rebuild = false;""",
"""        let mut requires_full_rebuild = mutation_history_lost;""",
),
(
"""        *document
            .children(document.root())
            .iter()
            .find(|node| matches!(&document.node(**node).kind, NodeKind::Element(_)))
            .expect("fixture contains an element")""",
"""        *document
            .children(document.root())
            .expect("document root is valid")
            .iter()
            .find(|node| {
                document
                    .node(**node)
                    .is_some_and(|node| matches!(&node.kind, NodeKind::Element(_)))
            })
            .expect("fixture contains an element")""",
),
(
"""            if let NodeKind::Element(element) = &document.node(node).kind
                && element.attributes.get("id").map(String::as_str) == Some(id)
            {
                return Some(node);
            }
            document
                .children(node)
                .iter()
                .find_map(|child| find(document, *child, id))""",
"""            if let Some(dom_node) = document.node(node)
                && let NodeKind::Element(element) = &dom_node.kind
                && element.attributes.get("id").map(String::as_str) == Some(id)
            {
                return Some(node);
            }
            document
                .children(node)
                .unwrap_or(&[])
                .iter()
                .find_map(|child| find(document, *child, id))""",
),
]
for old, new in replacements:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one pattern, found {count}: {old[:80]!r}")
    text = text.replace(old, new)
path.write_text(text, encoding="utf-8")
