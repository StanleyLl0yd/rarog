use rarog_dom::{Document, ElementData, NodeKind};

#[test]
fn foreign_node_reads_are_rejected_without_panicking() {
    let mut source = Document::new();
    let foreign = source
        .append_new(
            source.root(),
            NodeKind::Element(ElementData::html("div")),
        )
        .expect("fixture node is valid");
    let target = Document::new();

    assert!(target.node(foreign).is_none());
    assert!(target.children(foreign).is_none());
}
