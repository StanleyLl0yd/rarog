use rarog_css::{ComputedStyle, DirtyFlags, InvalidationSet, StyleSet, computed_style};
use rarog_dom::{Document, MutationKind, NodeId};
use rarog_layout::{Fragment, LayoutNode, LayoutOutput, layout_document_with_styles};
use rarog_paint::{DamageRegion, DisplayList, Framebuffer, build_display_list};
use rarog_types::{Color, Size};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub viewport: Size,
    pub background: Color,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            viewport: Size {
                width: 1024.0,
                height: 768.0,
            },
            background: Color::WHITE,
        }
    }
}

pub struct RenderOutput {
    pub document: Document,
    pub styles: StyleSet,
    pub layout: LayoutOutput,
    pub display_list: DisplayList,
    pub damage: DamageRegion,
    pub framebuffer: Framebuffer,
}

impl RenderOutput {
    pub fn deterministic_signature_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        hash = fnv1a(hash, self.document.snapshot().as_bytes());
        hash = fnv1a(hash, self.styles.snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.tree.style_snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.tree.snapshot().as_bytes());
        hash = fnv1a(hash, self.layout.fragments.snapshot().as_bytes());
        hash = fnv1a(hash, self.display_list.snapshot().as_bytes());
        fnv1a(hash, &self.framebuffer.stable_hash64().to_le_bytes())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyState {
    entries: BTreeMap<NodeId, DirtyFlags>,
    through_generation: u64,
}

impl DirtyState {
    pub fn clean_at(generation: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            through_generation: generation,
        }
    }

    pub fn through_generation(&self) -> u64 {
        self.through_generation
    }

    pub fn entries(&self) -> &BTreeMap<NodeId, DirtyFlags> {
        &self.entries
    }

    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capture(&mut self, document: &Document) {
        let delta = InvalidationSet::from_document_since(document, self.through_generation);
        for (node, flags) in delta.entries {
            let current = self.entries.entry(node).or_default();
            current.style |= flags.style;
            current.layout |= flags.layout;
            current.paint |= flags.paint;
        }
        self.through_generation = delta.through_generation;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncrementalMode {
    Unchanged,
    PaintOnlyReuse,
    FullRebuild,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncrementalReport {
    pub mode: IncrementalMode,
    pub from_generation: u64,
    pub through_generation: u64,
    pub dirty_nodes: usize,
    pub patched_nodes: usize,
}

pub struct RenderSession {
    options: RenderOptions,
    document: Document,
    styles: StyleSet,
    layout: LayoutOutput,
    display_list: DisplayList,
    damage: DamageRegion,
    framebuffer: Framebuffer,
    dirty: DirtyState,
}

impl RenderSession {
    pub fn new(source: &str, options: RenderOptions) -> Self {
        let output = render_html(source, options);
        let generation = output.document.generation();
        Self {
            options,
            document: output.document,
            styles: output.styles,
            layout: output.layout,
            display_list: output.display_list,
            damage: output.damage,
            framebuffer: output.framebuffer,
            dirty: DirtyState::clean_at(generation),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn styles(&self) -> &StyleSet {
        &self.styles
    }

    pub fn layout(&self) -> &LayoutOutput {
        &self.layout
    }

    pub fn display_list(&self) -> &DisplayList {
        &self.display_list
    }

    pub fn damage(&self) -> &DamageRegion {
        &self.damage
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    pub fn dirty_state(&self) -> &DirtyState {
        &self.dirty
    }

    pub fn update(&mut self) -> IncrementalReport {
        let from_generation = self.dirty.through_generation();
        let mutations = self
            .document
            .mutation_records_since(from_generation)
            .map(|record| record.kind.clone())
            .collect::<Vec<_>>();
        self.dirty.capture(&self.document);
        let through_generation = self.dirty.through_generation();
        let dirty_nodes = self.dirty.entries().len();

        if mutations.is_empty() || dirty_nodes == 0 {
            self.damage = DamageRegion::default();
            self.dirty.clear();
            return IncrementalReport {
                mode: IncrementalMode::Unchanged,
                from_generation,
                through_generation,
                dirty_nodes,
                patched_nodes: 0,
            };
        }

        let mut style_candidates = BTreeSet::new();
        let mut requires_full_rebuild = false;
        for mutation in &mutations {
            match mutation {
                MutationKind::Attribute { node, name }
                    if matches!(name.as_str(), "id" | "class" | "style") =>
                {
                    style_candidates.insert(*node);
                }
                MutationKind::Attribute { .. } => {}
                MutationKind::NodeCreated { .. }
                | MutationKind::ChildAdded { .. }
                | MutationKind::Reparented { .. }
                | MutationKind::CharacterData { .. } => {
                    requires_full_rebuild = true;
                }
            }
        }

        let new_styles = StyleSet::for_document(&self.document);
        let mut paint_updates = Vec::new();

        if !requires_full_rebuild {
            for node in style_candidates {
                let Some(old_style) = layout_style_for_dom(&self.layout.tree.root, node) else {
                    requires_full_rebuild = true;
                    break;
                };
                let new_style = computed_style(&self.document, node, &new_styles);
                if layout_style_changed(old_style, new_style) {
                    requires_full_rebuild = true;
                    break;
                }
                if paint_style_changed(old_style, new_style) {
                    paint_updates.push((node, new_style));
                }
            }
        }

        let mode;
        let patched_nodes;
        if requires_full_rebuild {
            self.full_rebuild(new_styles);
            mode = IncrementalMode::FullRebuild;
            patched_nodes = 0;
        } else if paint_updates.is_empty() {
            self.styles = new_styles;
            self.damage = DamageRegion::default();
            mode = IncrementalMode::Unchanged;
            patched_nodes = 0;
        } else {
            let previous_display_list = self.display_list.clone();
            patched_nodes = paint_updates.len();
            for (node, style) in paint_updates {
                patch_layout_style(&mut self.layout.tree.root, node, style);
                patch_fragment_style(&mut self.layout.fragments.root, node, style);
            }
            self.styles = new_styles;
            self.display_list = build_display_list(&self.layout.fragments);
            self.damage = DamageRegion::between(Some(&previous_display_list), &self.display_list);
            self.framebuffer = Framebuffer::new(self.options.viewport, self.options.background);
            self.framebuffer.rasterize(&self.display_list);
            mode = IncrementalMode::PaintOnlyReuse;
        }

        self.dirty.clear();
        IncrementalReport {
            mode,
            from_generation,
            through_generation,
            dirty_nodes,
            patched_nodes,
        }
    }

    fn full_rebuild(&mut self, styles: StyleSet) {
        let previous_display_list = self.display_list.clone();
        let layout = layout_document_with_styles(&self.document, &styles, self.options.viewport);
        let display_list = build_display_list(&layout.fragments);
        let damage = DamageRegion::between(Some(&previous_display_list), &display_list);
        let mut framebuffer = Framebuffer::new(self.options.viewport, self.options.background);
        framebuffer.rasterize(&display_list);

        self.styles = styles;
        self.layout = layout;
        self.display_list = display_list;
        self.damage = damage;
        self.framebuffer = framebuffer;
    }
}

pub fn render_html(source: &str, options: RenderOptions) -> RenderOutput {
    render_html_against(source, options, None)
}

pub fn render_html_against(
    source: &str,
    options: RenderOptions,
    previous_display_list: Option<&DisplayList>,
) -> RenderOutput {
    let document = rarog_html::parse(source);
    let styles = StyleSet::for_document(&document);
    let layout = layout_document_with_styles(&document, &styles, options.viewport);
    let display_list = build_display_list(&layout.fragments);
    let damage = DamageRegion::between(previous_display_list, &display_list);
    let mut framebuffer = Framebuffer::new(options.viewport, options.background);
    framebuffer.rasterize(&display_list);

    RenderOutput {
        document,
        styles,
        layout,
        display_list,
        damage,
        framebuffer,
    }
}

fn layout_style_for_dom(node: &LayoutNode, dom_node: NodeId) -> Option<ComputedStyle> {
    if node.dom_node == Some(dom_node) {
        return Some(node.style);
    }
    node.children
        .iter()
        .find_map(|child| layout_style_for_dom(child, dom_node))
}

fn patch_layout_style(node: &mut LayoutNode, dom_node: NodeId, style: ComputedStyle) {
    if node.dom_node == Some(dom_node) {
        node.style = style;
    }
    for child in &mut node.children {
        patch_layout_style(child, dom_node, style);
    }
}

fn patch_fragment_style(fragment: &mut Fragment, dom_node: NodeId, style: ComputedStyle) {
    if fragment.dom_node == Some(dom_node) {
        fragment.style = style;
    }
    for child in &mut fragment.children {
        patch_fragment_style(child, dom_node, style);
    }
}

fn layout_style_changed(before: ComputedStyle, after: ComputedStyle) -> bool {
    before.width != after.width
        || before.height != after.height
        || before.margin != after.margin
        || before.border_width != after.border_width
        || before.padding != after.padding
        || before.display_none != after.display_none
}

fn paint_style_changed(before: ComputedStyle, after: ComputedStyle) -> bool {
    before.background != after.background || before.border_color != after.border_color
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_dom::NodeKind;

    const DETERMINISTIC_FIXTURE: &str = "<style>.card { width:80px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\"hero\" class=\"card\">Rarog</div>";

    fn deterministic_options() -> RenderOptions {
        RenderOptions {
            viewport: Size {
                width: 160.0,
                height: 90.0,
            },
            background: Color::WHITE,
        }
    }

    fn first_element(document: &Document) -> NodeId {
        *document
            .children(document.root())
            .iter()
            .find(|node| matches!(document.node(**node).kind, NodeKind::Element(_)))
            .expect("fixture contains an element")
    }

    #[test]
    fn bootstrap_pipeline_produces_commands_and_fragments() {
        let output = render_html(
            "<html><body><div style=\"background:#ffffff;height:32px\">x</div></body></html>",
            RenderOptions::default(),
        );

        assert!(!output.display_list.commands.is_empty());
        assert!(!output.layout.fragments.root.children.is_empty());
        assert_eq!(output.framebuffer.width, 1024);
        assert_eq!(output.framebuffer.height, 768);
    }

    #[test]
    fn box_model_reaches_paint_without_layout_drawing_directly() {
        let output = render_html(
            "<div style=\"width:100px;height:20px;padding:10px;border-width:2px;\
             border-color:#000000;background:#ffffff\">x</div>",
            RenderOptions {
                viewport: Size {
                    width: 320.0,
                    height: 200.0,
                },
                background: Color::WHITE,
            },
        );

        let fragment = &output.layout.fragments.root.children[0];
        assert_eq!(fragment.boxes.content_box.size.width, 100.0);
        assert_eq!(fragment.boxes.border_box.size.width, 124.0);
        assert!(output.display_list.commands.len() >= 6);
    }

    #[test]
    fn author_stylesheet_cascade_reaches_rendering() {
        let output = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let fragment = &output.layout.fragments.root.children[0];

        assert_eq!(fragment.boxes.content_box.size.width, 80.0);
        assert_eq!(fragment.style.background, Color::rgb(0x11, 0x22, 0x33));
        assert_eq!(fragment.style.border_width.top, 2.0);
    }

    #[test]
    fn damage_is_empty_when_display_list_is_unchanged() {
        let first = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_html_against(
            DETERMINISTIC_FIXTURE,
            deterministic_options(),
            Some(&first.display_list),
        );

        assert!(second.damage.rects.is_empty());
    }

    #[test]
    fn dirty_state_persists_until_render_consumes_it() {
        let mut document = rarog_html::parse("<div id=\"hero\">Rarog</div>");
        let node = first_element(&document);
        let mut dirty = DirtyState::clean_at(document.generation());

        document.set_attribute(node, "class", "hot").unwrap();
        dirty.capture(&document);

        assert_eq!(dirty.through_generation(), document.generation());
        assert_eq!(
            dirty.entries().get(&node),
            Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
        );
        dirty.clear();
        assert!(dirty.is_clean());
    }

    #[test]
    fn paint_only_update_reuses_layout_and_fragment_geometry() {
        let mut session = RenderSession::new(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        let layout_before = session.layout().tree.snapshot();
        let fragments_before = session.layout().fragments.snapshot();
        let framebuffer_before = session.framebuffer().stable_hash64();

        session
            .document_mut()
            .set_attribute(
                node,
                "style",
                "width:80px;height:20px;background:#445566",
            )
            .unwrap();
        let report = session.update();

        assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
        assert_eq!(report.patched_nodes, 1);
        assert_eq!(session.layout().tree.snapshot(), layout_before);
        assert_eq!(session.layout().fragments.snapshot(), fragments_before);
        assert_ne!(session.framebuffer().stable_hash64(), framebuffer_before);
        assert!(!session.damage().rects.is_empty());
        assert!(session.dirty_state().is_clean());
    }

    #[test]
    fn geometry_change_falls_back_to_full_rebuild() {
        let mut session = RenderSession::new(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());

        session
            .document_mut()
            .set_attribute(
                node,
                "style",
                "width:96px;height:20px;background:#445566",
            )
            .unwrap();
        let report = session.update();

        assert_eq!(report.mode, IncrementalMode::FullRebuild);
        assert_eq!(
            session.layout().fragments.root.children[0]
                .boxes
                .content_box
                .size
                .width,
            96.0
        );
        assert!(session.dirty_state().is_clean());
    }

    #[test]
    fn unrelated_attribute_change_does_not_rebuild_render_state() {
        let mut session = RenderSession::new(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        let framebuffer_before = session.framebuffer().stable_hash64();

        session
            .document_mut()
            .set_attribute(node, "title", "bootstrap metadata")
            .unwrap();
        let report = session.update();

        assert_eq!(report.mode, IncrementalMode::Unchanged);
        assert_eq!(session.framebuffer().stable_hash64(), framebuffer_before);
        assert!(session.damage().rects.is_empty());
    }

    #[test]
    fn deterministic_render_snapshot_and_hash() {
        let first = render_html(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_html(DETERMINISTIC_FIXTURE, deterministic_options());

        assert_eq!(first.document.snapshot(), second.document.snapshot());
        assert_eq!(first.styles.snapshot(), second.styles.snapshot());
        assert_eq!(
            first.layout.tree.style_snapshot(),
            second.layout.tree.style_snapshot()
        );
        assert_eq!(first.layout.tree.snapshot(), second.layout.tree.snapshot());
        assert_eq!(
            first.layout.fragments.snapshot(),
            second.layout.fragments.snapshot()
        );
        assert_eq!(
            first.display_list.snapshot(),
            second.display_list.snapshot()
        );
        assert_eq!(
            first.framebuffer.stable_hash64(),
            second.framebuffer.stable_hash64()
        );
        assert_eq!(
            first.deterministic_signature_hash(),
            second.deterministic_signature_hash()
        );

        assert_eq!(
            first.framebuffer.stable_hash64(),
            13_219_555_538_035_458_927
        );
        assert_eq!(
            first.deterministic_signature_hash(),
            12_885_545_535_776_656_151
        );
    }
}
