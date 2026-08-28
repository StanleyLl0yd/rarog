mod embedder;
pub use embedder::*;

use rarog_css::{ComputedStyle, DirtyFlags, InvalidationSet, StyleSet, computed_style};
use rarog_dom::{Document, MutationError, MutationKind, NodeId, NodeKind};
use rarog_layout::{
    Fragment, LayoutNode, LayoutOutput, build_layout_tree, fragment_for_dom,
    layout_document_with_styles, relayout_fragment_flow, relayout_fragment_subtree, relayout_tree,
};
use rarog_paint::{
    DamageRegion, DisplayList, Framebuffer, FramebufferError, build_display_list,
    replace_display_items_for_fragment,
};
use rarog_types::{Color, Size};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub viewport: Size,
    pub background: Color,
}

pub const DEFAULT_MAX_RENDER_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_DOM_NODES: usize = 65_536;
pub const DEFAULT_MAX_DOM_DEPTH: usize = 512;
pub const DEFAULT_MAX_TEXT_SCALARS: usize = 4_000_000;
pub const DEFAULT_MAX_CSS_RULES: usize = 100_000;
pub const DEFAULT_MAX_FRAGMENTS: usize = 131_072;
pub const DEFAULT_MAX_DISPLAY_COMMANDS: usize = 524_288;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderLimits {
    pub max_document_source_bytes: usize,
    pub max_dom_nodes: usize,
    pub max_dom_depth: usize,
    pub max_text_scalars: usize,
    pub max_css_rules: usize,
    pub max_fragments: usize,
    pub max_display_commands: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            max_document_source_bytes: DEFAULT_MAX_RENDER_SOURCE_BYTES,
            max_dom_nodes: DEFAULT_MAX_DOM_NODES,
            max_dom_depth: DEFAULT_MAX_DOM_DEPTH,
            max_text_scalars: DEFAULT_MAX_TEXT_SCALARS,
            max_css_rules: DEFAULT_MAX_CSS_RULES,
            max_fragments: DEFAULT_MAX_FRAGMENTS,
            max_display_commands: DEFAULT_MAX_DISPLAY_COMMANDS,
        }
    }
}

impl RenderLimits {
    pub fn is_valid(self) -> bool {
        self.max_document_source_bytes > 0
            && self.max_dom_nodes > 0
            && self.max_dom_depth > 0
            && self.max_text_scalars > 0
            && self.max_css_rules > 0
            && self.max_fragments > 0
            && self.max_display_commands > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    InvalidViewportSize,
    InvalidRenderLimits,
    DocumentSourceLimitExceeded { bytes: usize, limit: usize },
    DomNodeLimitExceeded { nodes: usize, limit: usize },
    DomDepthLimitExceeded { depth: usize, limit: usize },
    TextScalarLimitExceeded { scalars: usize, limit: usize },
    CssRuleLimitExceeded { rules: usize, limit: usize },
    FragmentLimitExceeded { fragments: usize, limit: usize },
    DisplayCommandLimitExceeded { commands: usize, limit: usize },
    Framebuffer(FramebufferError),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidViewportSize => {
                formatter.write_str("viewport dimensions must be non-negative")
            }
            Self::InvalidRenderLimits => formatter.write_str("render limits must be non-zero"),
            Self::DocumentSourceLimitExceeded { bytes, limit } => write!(
                formatter,
                "document source requires {bytes} bytes; limit is {limit}"
            ),
            Self::DomNodeLimitExceeded { nodes, limit } => write!(
                formatter,
                "document contains {nodes} nodes; limit is {limit}"
            ),
            Self::DomDepthLimitExceeded { depth, limit } => {
                write!(formatter, "document depth is {depth}; limit is {limit}")
            }
            Self::TextScalarLimitExceeded { scalars, limit } => write!(
                formatter,
                "document contains {scalars} text scalars; limit is {limit}"
            ),
            Self::CssRuleLimitExceeded { rules, limit } => write!(
                formatter,
                "document contains {rules} CSS rules; limit is {limit}"
            ),
            Self::FragmentLimitExceeded { fragments, limit } => write!(
                formatter,
                "layout produced {fragments} fragments; limit is {limit}"
            ),
            Self::DisplayCommandLimitExceeded { commands, limit } => write!(
                formatter,
                "paint produced {commands} display commands; limit is {limit}"
            ),
            Self::Framebuffer(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<FramebufferError> for RenderError {
    fn from(error: FramebufferError) -> Self {
        Self::Framebuffer(error)
    }
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderTimings {
    pub parse: Duration,
    pub style: Duration,
    pub layout_tree: Duration,
    pub fragment: Duration,
    pub paint_list: Duration,
    pub raster: Duration,
    pub total: Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderCounters {
    pub dom_nodes: usize,
    pub layout_nodes: usize,
    pub fragments: usize,
    pub display_commands: usize,
    pub damage_rects: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderObservability {
    pub timings: RenderTimings,
    pub counters: RenderCounters,
}

pub struct RenderOutput {
    pub document: Document,
    pub styles: StyleSet,
    pub layout: LayoutOutput,
    pub display_list: DisplayList,
    pub damage: DamageRegion,
    pub framebuffer: Framebuffer,
    pub observability: RenderObservability,
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

    pub fn capture(&mut self, document: &Document, styles: &StyleSet) {
        let delta = InvalidationSet::from_document_since_with_styles(
            document,
            self.through_generation,
            styles,
        );
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
    SubtreeRelayout,
    FlowRelayout,
    GeometryRelayout,
    FullRebuild,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncrementalReport {
    pub mode: IncrementalMode,
    pub from_generation: u64,
    pub through_generation: u64,
    pub dirty_nodes: usize,
    pub patched_nodes: usize,
    pub elapsed: Duration,
}

pub struct DocumentEditor<'a> {
    document: &'a mut Document,
}

impl DocumentEditor<'_> {
    pub fn create_node(&mut self, kind: NodeKind) -> Result<NodeId, MutationError> {
        self.document.create_node(kind)
    }

    pub fn append_new(&mut self, parent: NodeId, kind: NodeKind) -> Result<NodeId, MutationError> {
        self.document.append_new(parent, kind)
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), MutationError> {
        self.document.append_child(parent, child)
    }

    pub fn detach(&mut self, child: NodeId) -> Result<(), MutationError> {
        self.document.detach(child)
    }

    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), MutationError> {
        self.document.set_attribute(node, name, value)
    }

    pub fn remove_attribute(
        &mut self,
        node: NodeId,
        name: &str,
    ) -> Result<Option<String>, MutationError> {
        self.document.remove_attribute(node, name)
    }

    pub fn set_text(
        &mut self,
        node: NodeId,
        value: impl Into<String>,
    ) -> Result<(), MutationError> {
        self.document.set_text(node, value)
    }
}

pub struct RenderSession {
    options: RenderOptions,
    limits: RenderLimits,
    document: Document,
    styles: StyleSet,
    layout: LayoutOutput,
    display_list: DisplayList,
    damage: DamageRegion,
    framebuffer: Framebuffer,
    dirty: DirtyState,
    observability: RenderObservability,
}

impl RenderSession {
    pub fn new(source: &str, options: RenderOptions) -> Result<Self, RenderError> {
        Self::new_with_limits(source, options, RenderLimits::default())
    }

    pub fn new_with_limits(
        source: &str,
        options: RenderOptions,
        limits: RenderLimits,
    ) -> Result<Self, RenderError> {
        let mut output = render_html_with_limits(source, options, limits)?;
        let generation = output.document.generation();
        output.document.prune_mutations_through(generation);
        Ok(Self {
            options,
            limits,
            document: output.document,
            styles: output.styles,
            layout: output.layout,
            display_list: output.display_list,
            damage: output.damage,
            framebuffer: output.framebuffer,
            dirty: DirtyState::clean_at(generation),
            observability: output.observability,
        })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> DocumentEditor<'_> {
        DocumentEditor {
            document: &mut self.document,
        }
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

    pub fn observability(&self) -> RenderObservability {
        self.observability
    }

    pub fn resize(&mut self, viewport: Size) -> Result<(), RenderError> {
        validate_viewport_size(viewport)?;
        validate_document_limits(&self.document, self.limits)?;
        let total_started = Instant::now();

        let stage_started = Instant::now();
        let styles = StyleSet::for_document(&self.document);
        validate_style_limits(&styles, self.limits)?;
        let style = stage_started.elapsed();

        let stage_started = Instant::now();
        let tree = build_layout_tree(&self.document, &styles);
        let layout_tree = stage_started.elapsed();

        let stage_started = Instant::now();
        let fragments = relayout_tree(&tree, viewport);
        let fragment = stage_started.elapsed();
        let layout = LayoutOutput { tree, fragments };
        validate_layout_limits(&layout, self.limits)?;

        let stage_started = Instant::now();
        let display_list = build_display_list(&layout.fragments);
        validate_display_list_limits(&display_list, self.limits)?;
        let damage = DamageRegion::between(Some(&self.display_list), &display_list);
        let paint_list = stage_started.elapsed();

        let stage_started = Instant::now();
        let mut framebuffer = Framebuffer::try_new(viewport, self.options.background)?;
        framebuffer.rasterize(&display_list);
        let raster = stage_started.elapsed();

        let generation = self.document.generation();
        self.document.prune_mutations_through(generation);
        self.options.viewport = viewport;
        self.styles = styles;
        self.layout = layout;
        self.display_list = display_list;
        self.damage = damage;
        self.framebuffer = framebuffer;
        self.dirty = DirtyState::clean_at(generation);
        self.observability = RenderObservability {
            timings: RenderTimings {
                parse: Duration::ZERO,
                style,
                layout_tree,
                fragment,
                paint_list,
                raster,
                total: total_started.elapsed(),
            },
            counters: RenderCounters {
                dom_nodes: self.document.node_count(),
                layout_nodes: self.layout.tree.node_count(),
                fragments: self.layout.fragments.fragment_count(),
                display_commands: self.display_list.len(),
                damage_rects: self.damage.rects.len(),
            },
        };
        Ok(())
    }

    pub fn update(&mut self) -> Result<IncrementalReport, RenderError> {
        validate_document_limits(&self.document, self.limits)?;
        let update_started = Instant::now();
        let from_generation = self.dirty.through_generation();
        let (mutations, mutation_history_lost) =
            match self.document.mutation_records_since(from_generation) {
                Ok(records) => (
                    records
                        .map(|record| record.kind.clone())
                        .collect::<Vec<_>>(),
                    false,
                ),
                Err(_) => (Vec::new(), true),
            };
        self.dirty.capture(&self.document, &self.styles);
        let through_generation = self.dirty.through_generation();
        let dirty_nodes = self.dirty.entries().len();

        if !mutation_history_lost && (mutations.is_empty() || dirty_nodes == 0) {
            self.damage = DamageRegion::default();
            self.dirty.clear();
            self.document.prune_mutations_through(through_generation);
            return Ok(IncrementalReport {
                mode: IncrementalMode::Unchanged,
                from_generation,
                through_generation,
                dirty_nodes,
                patched_nodes: 0,
                elapsed: update_started.elapsed(),
            });
        }

        let mut style_candidates = self
            .dirty
            .entries()
            .iter()
            .filter_map(|(node, flags)| flags.style.then_some(*node))
            .collect::<BTreeSet<_>>();
        let mut requires_full_rebuild = mutation_history_lost;
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
        let mut style_updates = Vec::new();
        let mut geometry_changed = false;
        let mut subtree_relayout_safe = true;
        let mut flow_relayout_nodes = BTreeSet::new();

        if !requires_full_rebuild {
            for node in style_candidates {
                let Some(old_style) = layout_style_for_dom(&self.layout.tree.root, node) else {
                    requires_full_rebuild = true;
                    break;
                };
                let new_style = computed_style(&self.document, node, &new_styles);
                if old_style.display_none != new_style.display_none {
                    requires_full_rebuild = true;
                    break;
                }
                if old_style != new_style {
                    let layout_changed = layout_style_changed(old_style, new_style);
                    geometry_changed |= layout_changed;
                    if layout_changed && vertical_footprint_changed(old_style, new_style) {
                        subtree_relayout_safe = false;
                        flow_relayout_nodes.insert(node);
                    }
                    style_updates.push((node, new_style));
                }
            }
        }

        let mode;
        let patched_nodes;
        if requires_full_rebuild {
            self.full_rebuild(new_styles);
            mode = IncrementalMode::FullRebuild;
            patched_nodes = 0;
        } else if style_updates.is_empty() {
            self.styles = new_styles;
            self.damage = DamageRegion::default();
            mode = IncrementalMode::Unchanged;
            patched_nodes = 0;
        } else if geometry_changed && subtree_relayout_safe {
            let previous_display_list = self.display_list.clone();
            patched_nodes = style_updates.len();
            for &(node, style) in &style_updates {
                patch_layout_style(&mut self.layout.tree.root, node, style);
            }
            self.styles = new_styles;

            let mut subtree_applied = true;
            let mut retained_display = true;
            for &(node, _) in &style_updates {
                let previous_fragment = fragment_for_dom(&self.layout.fragments, node).cloned();
                if previous_fragment.is_none()
                    || !relayout_fragment_subtree(
                        &self.layout.tree,
                        &mut self.layout.fragments,
                        node,
                    )
                {
                    subtree_applied = false;
                    break;
                }
                let current_fragment = fragment_for_dom(&self.layout.fragments, node).cloned();
                let (Some(previous_fragment), Some(current_fragment)) =
                    (previous_fragment, current_fragment)
                else {
                    subtree_applied = false;
                    break;
                };
                retained_display &= replace_display_items_for_fragment(
                    &mut self.display_list,
                    &previous_fragment,
                    &current_fragment,
                );
            }

            if subtree_applied {
                if !retained_display {
                    self.display_list = build_display_list(&self.layout.fragments);
                }
                mode = IncrementalMode::SubtreeRelayout;
            } else {
                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);
                self.display_list = build_display_list(&self.layout.fragments);
                mode = IncrementalMode::GeometryRelayout;
            }
            self.damage = DamageRegion::between(Some(&previous_display_list), &self.display_list);
            self.framebuffer.rasterize_damage(
                &self.display_list,
                &self.damage,
                self.options.background,
            );
        } else if geometry_changed {
            let previous_display_list = self.display_list.clone();
            patched_nodes = style_updates.len();
            for &(node, style) in &style_updates {
                patch_layout_style(&mut self.layout.tree.root, node, style);
            }
            self.styles = new_styles;
            let flow_nodes = flow_relayout_nodes.into_iter().collect::<Vec<_>>();
            if relayout_fragment_flow(&self.layout.tree, &mut self.layout.fragments, &flow_nodes) {
                mode = IncrementalMode::FlowRelayout;
            } else {
                self.layout.fragments = relayout_tree(&self.layout.tree, self.options.viewport);
                mode = IncrementalMode::GeometryRelayout;
            }
            self.display_list = build_display_list(&self.layout.fragments);
            self.damage = DamageRegion::between(Some(&previous_display_list), &self.display_list);
            self.framebuffer.rasterize_damage(
                &self.display_list,
                &self.damage,
                self.options.background,
            );
        } else {
            let previous_display_list = self.display_list.clone();
            patched_nodes = style_updates.len();
            let mut retained_display = true;
            for &(node, style) in &style_updates {
                let previous_fragment = fragment_for_dom(&self.layout.fragments, node).cloned();
                patch_layout_style(&mut self.layout.tree.root, node, style);
                patch_fragment_style(&mut self.layout.fragments.root, node, style);
                let current_fragment = fragment_for_dom(&self.layout.fragments, node).cloned();
                match (previous_fragment, current_fragment) {
                    (Some(previous_fragment), Some(current_fragment)) => {
                        retained_display &= replace_display_items_for_fragment(
                            &mut self.display_list,
                            &previous_fragment,
                            &current_fragment,
                        );
                    }
                    _ => retained_display = false,
                }
            }
            self.styles = new_styles;
            if !retained_display {
                self.display_list = build_display_list(&self.layout.fragments);
            }
            self.damage = DamageRegion::between(Some(&previous_display_list), &self.display_list);
            self.framebuffer.rasterize_damage(
                &self.display_list,
                &self.damage,
                self.options.background,
            );
            mode = IncrementalMode::PaintOnlyReuse;
        }

        self.dirty.clear();
        self.document.prune_mutations_through(through_generation);
        validate_render_state_limits(&self.styles, &self.layout, &self.display_list, self.limits)?;
        Ok(IncrementalReport {
            mode,
            from_generation,
            through_generation,
            dirty_nodes,
            patched_nodes,
            elapsed: update_started.elapsed(),
        })
    }

    fn full_rebuild(&mut self, styles: StyleSet) {
        let previous_display_list = self.display_list.clone();
        let layout = layout_document_with_styles(&self.document, &styles, self.options.viewport);
        let display_list = build_display_list(&layout.fragments);
        let damage = DamageRegion::between(Some(&previous_display_list), &display_list);
        self.framebuffer
            .rasterize_damage(&display_list, &damage, self.options.background);

        self.styles = styles;
        self.layout = layout;
        self.display_list = display_list;
        self.damage = damage;
    }
}

pub fn render_html(source: &str, options: RenderOptions) -> Result<RenderOutput, RenderError> {
    render_html_with_limits(source, options, RenderLimits::default())
}

pub fn render_html_with_limits(
    source: &str,
    options: RenderOptions,
    limits: RenderLimits,
) -> Result<RenderOutput, RenderError> {
    render_html_against_with_limits(source, options, None, limits)
}

pub fn render_html_against(
    source: &str,
    options: RenderOptions,
    previous_display_list: Option<&DisplayList>,
) -> Result<RenderOutput, RenderError> {
    render_html_against_with_limits(
        source,
        options,
        previous_display_list,
        RenderLimits::default(),
    )
}

pub fn render_html_against_with_limits(
    source: &str,
    options: RenderOptions,
    previous_display_list: Option<&DisplayList>,
    limits: RenderLimits,
) -> Result<RenderOutput, RenderError> {
    validate_viewport_size(options.viewport)?;
    if !limits.is_valid() {
        return Err(RenderError::InvalidRenderLimits);
    }
    if source.len() > limits.max_document_source_bytes {
        return Err(RenderError::DocumentSourceLimitExceeded {
            bytes: source.len(),
            limit: limits.max_document_source_bytes,
        });
    }
    let total_started = Instant::now();

    let stage_started = Instant::now();
    let document = rarog_html::parse(source);
    let parse = stage_started.elapsed();
    validate_document_limits(&document, limits)?;

    let stage_started = Instant::now();
    let styles = StyleSet::for_document(&document);
    validate_style_limits(&styles, limits)?;
    let style = stage_started.elapsed();

    let stage_started = Instant::now();
    let tree = build_layout_tree(&document, &styles);
    let layout_tree = stage_started.elapsed();

    let stage_started = Instant::now();
    let fragments = relayout_tree(&tree, options.viewport);
    let fragment = stage_started.elapsed();
    let layout = LayoutOutput { tree, fragments };
    validate_layout_limits(&layout, limits)?;

    let stage_started = Instant::now();
    let display_list = build_display_list(&layout.fragments);
    validate_display_list_limits(&display_list, limits)?;
    let damage = DamageRegion::between(previous_display_list, &display_list);
    let paint_list = stage_started.elapsed();

    let stage_started = Instant::now();
    let mut framebuffer = Framebuffer::try_new(options.viewport, options.background)?;
    framebuffer.rasterize(&display_list);
    let raster = stage_started.elapsed();

    let observability = RenderObservability {
        timings: RenderTimings {
            parse,
            style,
            layout_tree,
            fragment,
            paint_list,
            raster,
            total: total_started.elapsed(),
        },
        counters: RenderCounters {
            dom_nodes: document.node_count(),
            layout_nodes: layout.tree.node_count(),
            fragments: layout.fragments.fragment_count(),
            display_commands: display_list.len(),
            damage_rects: damage.rects.len(),
        },
    };

    Ok(RenderOutput {
        document,
        styles,
        layout,
        display_list,
        damage,
        framebuffer,
        observability,
    })
}

fn validate_document_limits(document: &Document, limits: RenderLimits) -> Result<(), RenderError> {
    if !limits.is_valid() {
        return Err(RenderError::InvalidRenderLimits);
    }
    let nodes = document.node_count();
    if nodes > limits.max_dom_nodes {
        return Err(RenderError::DomNodeLimitExceeded {
            nodes,
            limit: limits.max_dom_nodes,
        });
    }
    let depth = document.max_depth();
    if depth > limits.max_dom_depth {
        return Err(RenderError::DomDepthLimitExceeded {
            depth,
            limit: limits.max_dom_depth,
        });
    }
    let scalars = document.text_scalar_count();
    if scalars > limits.max_text_scalars {
        return Err(RenderError::TextScalarLimitExceeded {
            scalars,
            limit: limits.max_text_scalars,
        });
    }
    Ok(())
}

fn validate_style_limits(styles: &StyleSet, limits: RenderLimits) -> Result<(), RenderError> {
    let rules = styles.rule_count();
    if rules > limits.max_css_rules {
        return Err(RenderError::CssRuleLimitExceeded {
            rules,
            limit: limits.max_css_rules,
        });
    }
    Ok(())
}

fn validate_layout_limits(layout: &LayoutOutput, limits: RenderLimits) -> Result<(), RenderError> {
    let fragments = layout.fragments.fragment_count();
    if fragments > limits.max_fragments {
        return Err(RenderError::FragmentLimitExceeded {
            fragments,
            limit: limits.max_fragments,
        });
    }
    Ok(())
}

fn validate_display_list_limits(
    display_list: &DisplayList,
    limits: RenderLimits,
) -> Result<(), RenderError> {
    let commands = display_list.len();
    if commands > limits.max_display_commands {
        return Err(RenderError::DisplayCommandLimitExceeded {
            commands,
            limit: limits.max_display_commands,
        });
    }
    Ok(())
}

fn validate_render_state_limits(
    styles: &StyleSet,
    layout: &LayoutOutput,
    display_list: &DisplayList,
    limits: RenderLimits,
) -> Result<(), RenderError> {
    validate_style_limits(styles, limits)?;
    validate_layout_limits(layout, limits)?;
    validate_display_list_limits(display_list, limits)
}

fn validate_viewport_size(viewport: Size) -> Result<(), RenderError> {
    if viewport.width < 0.0 || viewport.height < 0.0 {
        return Err(RenderError::InvalidViewportSize);
    }
    Ok(())
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

fn vertical_footprint_changed(before: ComputedStyle, after: ComputedStyle) -> bool {
    before.height != after.height
        || before.margin.top != after.margin.top
        || before.margin.bottom != after.margin.bottom
        || before.border_width.top != after.border_width.top
        || before.border_width.bottom != after.border_width.bottom
        || before.padding.top != after.padding.top
        || before.padding.bottom != after.padding.bottom
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

    fn render_ok(source: &str, options: RenderOptions) -> RenderOutput {
        render_html(source, options).expect("valid test viewport")
    }

    fn render_against_ok(
        source: &str,
        options: RenderOptions,
        previous: Option<&DisplayList>,
    ) -> RenderOutput {
        render_html_against(source, options, previous).expect("valid test viewport")
    }

    fn session(source: &str, options: RenderOptions) -> RenderSession {
        RenderSession::new(source, options).expect("valid test viewport")
    }

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
            .expect("document root is valid")
            .iter()
            .find(|node| {
                document
                    .node(**node)
                    .is_some_and(|node| matches!(&node.kind, NodeKind::Element(_)))
            })
            .expect("fixture contains an element")
    }

    fn element_with_id(document: &Document, id: &str) -> NodeId {
        fn find(document: &Document, node: NodeId, id: &str) -> Option<NodeId> {
            if let Some(dom_node) = document.node(node)
                && let NodeKind::Element(element) = &dom_node.kind
                && element.attributes.get("id").map(String::as_str) == Some(id)
            {
                return Some(node);
            }
            document
                .children(node)
                .unwrap_or(&[])
                .iter()
                .find_map(|child| find(document, *child, id))
        }

        find(document, document.root(), id).expect("fixture contains requested id")
    }

    #[test]
    fn full_render_exposes_stage_observability_without_affecting_identity() {
        let first = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());
        let counters = first.observability.counters;

        assert_eq!(counters.dom_nodes, first.document.node_count());
        assert_eq!(counters.layout_nodes, first.layout.tree.node_count());
        assert_eq!(counters.fragments, first.layout.fragments.fragment_count());
        assert_eq!(counters.display_commands, first.display_list.len());
        assert_eq!(counters.damage_rects, first.damage.rects.len());
        assert!(first.observability.timings.total >= first.observability.timings.raster);
        assert_eq!(
            first.deterministic_signature_hash(),
            second.deterministic_signature_hash()
        );
    }

    #[test]
    fn incremental_report_exposes_elapsed_time_and_path_counts() {
        let mut session = session(DETERMINISTIC_FIXTURE, deterministic_options());
        let hero = element_with_id(session.document(), "hero");
        session
            .document_mut()
            .set_attribute(hero, "style", "background:#445566")
            .unwrap();

        let report = session.update().expect("incremental update succeeds");
        assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
        assert!(report.dirty_nodes >= 1);
        assert_eq!(report.patched_nodes, 1);
        let _elapsed = report.elapsed;
    }

    #[test]
    fn bootstrap_pipeline_produces_commands_and_fragments() {
        let output = render_ok(
            "<html><body><div style=\"background:#ffffff;height:32px\">x</div></body></html>",
            RenderOptions::default(),
        );

        assert!(!output.display_list.is_empty());
        assert!(!output.layout.fragments.root.children.is_empty());
        assert_eq!(output.framebuffer.width, 1024);
        assert_eq!(output.framebuffer.height, 768);
    }

    #[test]
    fn box_model_reaches_paint_without_layout_drawing_directly() {
        let output = render_ok(
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
        assert!(output.display_list.len() >= 6);
    }

    #[test]
    fn author_stylesheet_cascade_reaches_rendering() {
        let output = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());
        let fragment = &output.layout.fragments.root.children[0];

        assert_eq!(fragment.boxes.content_box.size.width, 80.0);
        assert_eq!(fragment.style.background, Color::rgb(0x11, 0x22, 0x33));
        assert_eq!(fragment.style.border_width.top, 2.0);
    }

    #[test]
    fn damage_is_empty_when_display_list_is_unchanged() {
        let first = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_against_ok(
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
        let styles = StyleSet::for_document(&document);
        dirty.capture(&document, &styles);

        assert_eq!(dirty.through_generation(), document.generation());
        assert_eq!(
            dirty.entries().get(&node),
            Some(&DirtyFlags::STYLE_LAYOUT_PAINT)
        );
        dirty.clear();
        assert!(dirty.is_clean());
    }

    #[test]
    fn render_session_prunes_consumed_mutation_history() {
        let mut session = session(
            "<div style=\"width:80px;height:20px\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        assert_eq!(session.document().mutation_record_count(), 0);

        session
            .document_mut()
            .set_attribute(node, "title", "metadata")
            .unwrap();
        assert_eq!(session.document().mutation_record_count(), 1);
        session.update();

        assert_eq!(session.document().mutation_record_count(), 0);
        assert_eq!(
            session.document().mutation_history_floor(),
            session.document().generation()
        );
    }

    #[test]
    fn resize_preserves_current_document_state() {
        let source =
            "<div id=\"hero\" style=\"width:80px;height:20px;background:#112233\">Rarog</div>";
        let expected_source =
            "<div id=\"hero\" style=\"width:96px;height:20px;background:#445566\">Rarog</div>";
        let mut session = session(source, deterministic_options());
        let hero = element_with_id(session.document(), "hero");
        session
            .document_mut()
            .set_attribute(hero, "style", "width:96px;height:20px;background:#445566")
            .unwrap();

        let resized_options = RenderOptions {
            viewport: Size {
                width: 220.0,
                height: 120.0,
            },
            background: Color::WHITE,
        };
        session.resize(resized_options.viewport).unwrap();
        let expected = render_ok(expected_source, resized_options);

        assert_eq!(session.framebuffer().width, 220);
        assert_eq!(session.framebuffer().height, 120);
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
        assert_eq!(session.document().mutation_record_count(), 0);
        assert!(session.dirty_state().is_clean());
        assert_eq!(session.observability().timings.parse, Duration::ZERO);
    }

    #[test]
    fn paint_only_update_reuses_layout_and_fragment_geometry() {
        let mut session = session(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        let layout_before = session.layout().tree.snapshot();
        let fragments_before = session.layout().fragments.snapshot();
        let framebuffer_before = session.framebuffer().stable_hash64();

        session
            .document_mut()
            .set_attribute(node, "style", "width:80px;height:20px;background:#445566")
            .unwrap();
        let report = session.update().expect("incremental update succeeds");

        assert_eq!(report.mode, IncrementalMode::PaintOnlyReuse);
        assert_eq!(report.patched_nodes, 1);
        assert_eq!(session.layout().tree.snapshot(), layout_before);
        assert_eq!(session.layout().fragments.snapshot(), fragments_before);
        assert_ne!(session.framebuffer().stable_hash64(), framebuffer_before);
        assert!(!session.damage().rects.is_empty());
        assert!(session.dirty_state().is_clean());
    }

    #[test]
    fn geometry_change_relayouts_without_rebuilding_layout_tree() {
        let mut session = session(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        let layout_before = session.layout().tree.snapshot();

        session
            .document_mut()
            .set_attribute(node, "style", "width:96px;height:20px;background:#445566")
            .unwrap();
        let report = session.update().expect("incremental update succeeds");

        assert_eq!(report.mode, IncrementalMode::SubtreeRelayout);
        assert_eq!(session.layout().tree.snapshot(), layout_before);
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
    fn vertical_geometry_change_reflows_ancestors_and_following_siblings() {
        let source = "<div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"outer\" style=\"padding:2px;background:#112233\"><div id=\"target\" style=\"height:20px\"></div></div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let expected_source = "<div id=\"before\" style=\"height:5px;background:#eeeeee\"></div><div id=\"outer\" style=\"padding:2px;background:#112233\"><div id=\"target\" style=\"height:32px\"></div></div><div id=\"after\" style=\"height:10px;background:#445566\"></div>";
        let mut session = session(source, deterministic_options());
        let target = element_with_id(session.document(), "target");
        let before = element_with_id(session.document(), "before");
        let after = element_with_id(session.document(), "after");
        let layout_before = session.layout().tree.snapshot();
        let before_fragment_id = fragment_for_dom(&session.layout().fragments, before)
            .expect("before fragment exists")
            .id;

        session
            .document_mut()
            .set_attribute(target, "style", "height:32px")
            .unwrap();

        let report = session.update().expect("incremental update succeeds");
        let expected = render_ok(expected_source, deterministic_options());

        assert_eq!(report.mode, IncrementalMode::FlowRelayout);
        assert_eq!(session.layout().tree.snapshot(), layout_before);
        assert_eq!(
            fragment_for_dom(&session.layout().fragments, before)
                .expect("before fragment remains")
                .id,
            before_fragment_id
        );
        assert_eq!(
            fragment_for_dom(&session.layout().fragments, after)
                .expect("after fragment exists")
                .boxes
                .margin_box
                .origin
                .y,
            41.0
        );
        assert_eq!(
            session.framebuffer().stable_hash64(),
            expected.framebuffer.stable_hash64()
        );
        assert!(!session.damage().rects.is_empty());
    }

    #[test]
    fn structural_change_still_falls_back_to_full_rebuild() {
        let mut session = session(
            "<div style=\"width:80px;height:20px\">Rarog</div>",
            deterministic_options(),
        );
        let parent = first_element(session.document());
        session
            .document_mut()
            .append_new(parent, NodeKind::Text("!".into()))
            .unwrap();

        let report = session.update().expect("incremental update succeeds");
        assert_eq!(report.mode, IncrementalMode::FullRebuild);
    }

    #[test]
    fn unrelated_attribute_change_does_not_rebuild_render_state() {
        let mut session = session(
            "<div style=\"width:80px;height:20px;background:#112233\">Rarog</div>",
            deterministic_options(),
        );
        let node = first_element(session.document());
        let framebuffer_before = session.framebuffer().stable_hash64();

        session
            .document_mut()
            .set_attribute(node, "title", "bootstrap metadata")
            .unwrap();
        let report = session.update().expect("incremental update succeeds");

        assert_eq!(report.mode, IncrementalMode::Unchanged);
        assert_eq!(session.framebuffer().stable_hash64(), framebuffer_before);
        assert!(session.damage().rects.is_empty());
    }

    #[test]
    fn deterministic_render_snapshot_and_hash() {
        let first = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());
        let second = render_ok(DETERMINISTIC_FIXTURE, deterministic_options());

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
            16_985_642_107_972_200_629
        );
    }
}

#[cfg(test)]
mod render_boundary_hardening_tests {
    use super::*;

    #[test]
    fn invalid_viewport_is_reported_instead_of_panicking() {
        let error = match render_html(
            "<div>x</div>",
            RenderOptions {
                viewport: Size {
                    width: f32::NAN,
                    height: 100.0,
                },
                background: Color::WHITE,
            },
        ) {
            Ok(_) => panic!("non-finite viewport must fail"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            RenderError::Framebuffer(FramebufferError::NonFiniteSize)
        ));
    }

    #[test]
    fn negative_viewport_is_rejected() {
        let result = render_html(
            "<div>x</div>",
            RenderOptions {
                viewport: Size {
                    width: -1.0,
                    height: 100.0,
                },
                background: Color::WHITE,
            },
        );
        assert!(matches!(result, Err(RenderError::InvalidViewportSize)));
    }
}
