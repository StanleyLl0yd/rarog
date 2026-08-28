use rarog_dom::{Document, ElementData, MutationHistoryError, NodeKind};

#[test]
fn foreign_node_reads_are_rejected_without_panicking() {
    let mut source = Document::new();
    let foreign = source
        .append_new(source.root(), NodeKind::Element(ElementData::html("div")))
        .expect("fixture node is valid");
    let target = Document::new();

    assert!(target.node(foreign).is_none());
    assert!(target.children(foreign).is_none());
}

#[test]
fn pruned_mutation_history_is_reported_without_panicking() {
    let mut document = Document::new();
    document
        .append_new(document.root(), NodeKind::Element(ElementData::html("div")))
        .expect("fixture node is valid");
    let floor = document.generation();
    document.prune_mutations_through(floor);

    assert_eq!(
        document.mutation_records_since(floor - 1).err(),
        Some(MutationHistoryError::RequestedBeforeFloor {
            requested: floor - 1,
            floor,
        })
    );
}
