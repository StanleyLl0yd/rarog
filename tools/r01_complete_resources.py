from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


engine = "crates/rarog-engine/src/lib.rs"
replace(engine, """#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub viewport: Size,
    pub background: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {""", """#[derive(Clone, Copy, Debug)]
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
pub enum RenderError {""")
replace(engine, """pub enum RenderError {
    InvalidViewportSize,
    Framebuffer(FramebufferError),
}""", """pub enum RenderError {
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
}""")
replace(engine, """            Self::InvalidViewportSize => {
                formatter.write_str("viewport dimensions must be non-negative")
            }
            Self::Framebuffer(error) => write!(formatter, "{error}"),""", """            Self::InvalidViewportSize => {
                formatter.write_str("viewport dimensions must be non-negative")
            }
            Self::InvalidRenderLimits => formatter.write_str("render limits must be non-zero"),
            Self::DocumentSourceLimitExceeded { bytes, limit } => write!(formatter, "document source requires {bytes} bytes; limit is {limit}"),
            Self::DomNodeLimitExceeded { nodes, limit } => write!(formatter, "document contains {nodes} nodes; limit is {limit}"),
            Self::DomDepthLimitExceeded { depth, limit } => write!(formatter, "document depth is {depth}; limit is {limit}"),
            Self::TextScalarLimitExceeded { scalars, limit } => write!(formatter, "document contains {scalars} text scalars; limit is {limit}"),
            Self::CssRuleLimitExceeded { rules, limit } => write!(formatter, "document contains {rules} CSS rules; limit is {limit}"),
            Self::FragmentLimitExceeded { fragments, limit } => write!(formatter, "layout produced {fragments} fragments; limit is {limit}"),
            Self::DisplayCommandLimitExceeded { commands, limit } => write!(formatter, "paint produced {commands} display commands; limit is {limit}"),
            Self::Framebuffer(error) => write!(formatter, "{error}"),""")
replace(engine, """pub struct RenderSession {
    options: RenderOptions,""", """pub struct RenderSession {
    options: RenderOptions,
    limits: RenderLimits,""")
replace(engine, """impl RenderSession {
    pub fn new(source: &str, options: RenderOptions) -> Result<Self, RenderError> {
        let mut output = render_html(source, options)?;""", """impl RenderSession {
    pub fn new(source: &str, options: RenderOptions) -> Result<Self, RenderError> {
        Self::new_with_limits(source, options, RenderLimits::default())
    }

    pub fn new_with_limits(source: &str, options: RenderOptions, limits: RenderLimits) -> Result<Self, RenderError> {
        let mut output = render_html_with_limits(source, options, limits)?;""")
replace(engine, """        Ok(Self {
            options,
            document: output.document,""", """        Ok(Self {
            options,
            limits,
            document: output.document,""")
replace(engine, """    pub fn update(&mut self) -> IncrementalReport {
        let update_started = Instant::now();""", """    pub fn update(&mut self) -> Result<IncrementalReport, RenderError> {
        validate_document_limits(&self.document, self.limits)?;
        let update_started = Instant::now();""")
replace(engine, """            return IncrementalReport {
                mode: IncrementalMode::Unchanged,
                from_generation,
                through_generation,
                dirty_nodes,
                patched_nodes: 0,
                elapsed: update_started.elapsed(),
            };""", """            return Ok(IncrementalReport {
                mode: IncrementalMode::Unchanged,
                from_generation,
                through_generation,
                dirty_nodes,
                patched_nodes: 0,
                elapsed: update_started.elapsed(),
            });""")
replace(engine, """        IncrementalReport {
            mode,
            from_generation,
            through_generation,
            dirty_nodes,
            patched_nodes,
            elapsed: update_started.elapsed(),
        }
    }""", """        validate_render_state_limits(&self.styles, &self.layout, &self.display_list, self.limits)?;
        Ok(IncrementalReport {
            mode,
            from_generation,
            through_generation,
            dirty_nodes,
            patched_nodes,
            elapsed: update_started.elapsed(),
        })
    }""")
replace(engine, """pub fn render_html(source: &str, options: RenderOptions) -> Result<RenderOutput, RenderError> {
    render_html_against(source, options, None)
}

pub fn render_html_against(
    source: &str,
    options: RenderOptions,
    previous_display_list: Option<&DisplayList>,
) -> Result<RenderOutput, RenderError> {
    validate_viewport_size(options.viewport)?;
    let total_started = Instant::now();""", """pub fn render_html(source: &str, options: RenderOptions) -> Result<RenderOutput, RenderError> {
    render_html_with_limits(source, options, RenderLimits::default())
}

pub fn render_html_with_limits(source: &str, options: RenderOptions, limits: RenderLimits) -> Result<RenderOutput, RenderError> {
    render_html_against_with_limits(source, options, None, limits)
}

pub fn render_html_against(source: &str, options: RenderOptions, previous_display_list: Option<&DisplayList>) -> Result<RenderOutput, RenderError> {
    render_html_against_with_limits(source, options, previous_display_list, RenderLimits::default())
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
        return Err(RenderError::DocumentSourceLimitExceeded { bytes: source.len(), limit: limits.max_document_source_bytes });
    }
    let total_started = Instant::now();""")
replace(engine, """    let document = rarog_html::parse(source);
    let parse = stage_started.elapsed();

    let stage_started = Instant::now();
    let styles = StyleSet::for_document(&document);""", """    let document = rarog_html::parse(source);
    let parse = stage_started.elapsed();
    validate_document_limits(&document, limits)?;

    let stage_started = Instant::now();
    let styles = StyleSet::for_document(&document);
    validate_style_limits(&styles, limits)?;""")
replace(engine, """    let fragments = relayout_tree(&tree, options.viewport);
    let fragment = stage_started.elapsed();
    let layout = LayoutOutput { tree, fragments };

    let stage_started = Instant::now();
    let display_list = build_display_list(&layout.fragments);""", """    let fragments = relayout_tree(&tree, options.viewport);
    let fragment = stage_started.elapsed();
    let layout = LayoutOutput { tree, fragments };
    validate_layout_limits(&layout, limits)?;

    let stage_started = Instant::now();
    let display_list = build_display_list(&layout.fragments);
    validate_display_list_limits(&display_list, limits)?;""")
replace(engine, """fn validate_viewport_size(viewport: Size) -> Result<(), RenderError> {""", """fn validate_document_limits(document: &Document, limits: RenderLimits) -> Result<(), RenderError> {
    if !limits.is_valid() { return Err(RenderError::InvalidRenderLimits); }
    let nodes = document.node_count();
    if nodes > limits.max_dom_nodes { return Err(RenderError::DomNodeLimitExceeded { nodes, limit: limits.max_dom_nodes }); }
    let depth = document.max_depth();
    if depth > limits.max_dom_depth { return Err(RenderError::DomDepthLimitExceeded { depth, limit: limits.max_dom_depth }); }
    let scalars = document.text_scalar_count();
    if scalars > limits.max_text_scalars { return Err(RenderError::TextScalarLimitExceeded { scalars, limit: limits.max_text_scalars }); }
    Ok(())
}

fn validate_style_limits(styles: &StyleSet, limits: RenderLimits) -> Result<(), RenderError> {
    let rules = styles.rule_count();
    if rules > limits.max_css_rules { return Err(RenderError::CssRuleLimitExceeded { rules, limit: limits.max_css_rules }); }
    Ok(())
}

fn validate_layout_limits(layout: &LayoutOutput, limits: RenderLimits) -> Result<(), RenderError> {
    let fragments = layout.fragments.fragment_count();
    if fragments > limits.max_fragments { return Err(RenderError::FragmentLimitExceeded { fragments, limit: limits.max_fragments }); }
    Ok(())
}

fn validate_display_list_limits(display_list: &DisplayList, limits: RenderLimits) -> Result<(), RenderError> {
    let commands = display_list.commands.len();
    if commands > limits.max_display_commands { return Err(RenderError::DisplayCommandLimitExceeded { commands, limit: limits.max_display_commands }); }
    Ok(())
}

fn validate_render_state_limits(styles: &StyleSet, layout: &LayoutOutput, display_list: &DisplayList, limits: RenderLimits) -> Result<(), RenderError> {
    validate_style_limits(styles, limits)?;
    validate_layout_limits(layout, limits)?;
    validate_display_list_limits(display_list, limits)
}

fn validate_viewport_size(viewport: Size) -> Result<(), RenderError> {""")
replace(engine, """    pub fn resize(&mut self, viewport: Size) -> Result<(), RenderError> {
        validate_viewport_size(viewport)?;
        let total_started = Instant::now();""", """    pub fn resize(&mut self, viewport: Size) -> Result<(), RenderError> {
        validate_viewport_size(viewport)?;
        validate_document_limits(&self.document, self.limits)?;
        let total_started = Instant::now();""")
replace(engine, """        let styles = StyleSet::for_document(&self.document);
        let style = stage_started.elapsed();""", """        let styles = StyleSet::for_document(&self.document);
        validate_style_limits(&styles, self.limits)?;
        let style = stage_started.elapsed();""")
replace(engine, """        let fragments = relayout_tree(&tree, viewport);
        let fragment = stage_started.elapsed();
        let layout = LayoutOutput { tree, fragments };

        let stage_started = Instant::now();
        let display_list = build_display_list(&layout.fragments);""", """        let fragments = relayout_tree(&tree, viewport);
        let fragment = stage_started.elapsed();
        let layout = LayoutOutput { tree, fragments };
        validate_layout_limits(&layout, self.limits)?;

        let stage_started = Instant::now();
        let display_list = build_display_list(&layout.fragments);
        validate_display_list_limits(&display_list, self.limits)?;""")

css = "crates/rarog-css/src/lib.rs"
replace(css, """    pub fn invalidation_dependencies(&self) -> SelectorInvalidationDependencies {""", """    pub fn rule_count(&self) -> usize {
        self.stylesheets.iter().map(|stylesheet| stylesheet.rules.len()).fold(0usize, usize::saturating_add)
    }

    pub fn invalidation_dependencies(&self) -> SelectorInvalidationDependencies {""")

embedder = "crates/rarog-engine/src/embedder.rs"
replace(embedder, """use super::{
    IncrementalReport, RenderError, RenderObservability, RenderOptions, RenderSession,
    validate_viewport_size,
};""", """use super::{
    DEFAULT_MAX_CSS_RULES, DEFAULT_MAX_DISPLAY_COMMANDS, DEFAULT_MAX_DOM_DEPTH,
    DEFAULT_MAX_DOM_NODES, DEFAULT_MAX_FRAGMENTS, DEFAULT_MAX_TEXT_SCALARS, IncrementalReport,
    RenderError, RenderLimits, RenderObservability, RenderOptions, RenderSession,
    validate_viewport_size,
};""")
replace(embedder, """pub struct ResourceBudget {
    pub max_document_source_bytes: usize,
    pub max_viewport_pixels: u64,
}""", """pub struct ResourceBudget {
    pub max_document_source_bytes: usize,
    pub max_viewport_pixels: u64,
    pub max_dom_nodes: usize,
    pub max_dom_depth: usize,
    pub max_text_scalars: usize,
    pub max_css_rules: usize,
    pub max_fragments: usize,
    pub max_display_commands: usize,
}""")
replace(embedder, """        Self {
            max_document_source_bytes: DEFAULT_MAX_DOCUMENT_SOURCE_BYTES,
            max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS,
        }""", """        Self {
            max_document_source_bytes: DEFAULT_MAX_DOCUMENT_SOURCE_BYTES,
            max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS,
            max_dom_nodes: DEFAULT_MAX_DOM_NODES,
            max_dom_depth: DEFAULT_MAX_DOM_DEPTH,
            max_text_scalars: DEFAULT_MAX_TEXT_SCALARS,
            max_css_rules: DEFAULT_MAX_CSS_RULES,
            max_fragments: DEFAULT_MAX_FRAGMENTS,
            max_display_commands: DEFAULT_MAX_DISPLAY_COMMANDS,
        }""")
replace(embedder, """        if self.budget.max_document_source_bytes == 0
            || self.budget.max_viewport_pixels == 0
            || self.budget.max_viewport_pixels > MAX_FRAMEBUFFER_PIXELS
        {""", """        if self.budget.max_document_source_bytes == 0
            || self.budget.max_viewport_pixels == 0
            || self.budget.max_dom_nodes == 0
            || self.budget.max_dom_depth == 0
            || self.budget.max_text_scalars == 0
            || self.budget.max_css_rules == 0
            || self.budget.max_fragments == 0
            || self.budget.max_display_commands == 0
            || self.budget.max_viewport_pixels > MAX_FRAMEBUFFER_PIXELS
        {""")
replace(embedder, """            Some(session) => FrameStatus::Incremental(session.update()),""", """            Some(session) => FrameStatus::Incremental(session.update()?),""")
replace(embedder, """                self.session = Some(RenderSession::new(
                    &loaded.source,
                    RenderOptions {
                        viewport,
                        background: self.options.background,
                    },
                )?);""", """                self.session = Some(RenderSession::new_with_limits(
                    &loaded.source,
                    RenderOptions {
                        viewport,
                        background: self.options.background,
                    },
                    RenderLimits {
                        max_document_source_bytes: self.shared.budget.max_document_source_bytes,
                        max_dom_nodes: self.shared.budget.max_dom_nodes,
                        max_dom_depth: self.shared.budget.max_dom_depth,
                        max_text_scalars: self.shared.budget.max_text_scalars,
                        max_css_rules: self.shared.budget.max_css_rules,
                        max_fragments: self.shared.budget.max_fragments,
                        max_display_commands: self.shared.budget.max_display_commands,
                    },
                )?);""")
text = Path(embedder).read_text(encoding="utf-8")
text = text.replace("""                max_document_source_bytes: 4,
                max_viewport_pixels: 100,
            })""", """                max_document_source_bytes: 4,
                max_viewport_pixels: 100,
                ..ResourceBudget::default()
            })""")
text = text.replace("""                max_document_source_bytes: 1024,
                max_viewport_pixels: 100,
            })""", """                max_document_source_bytes: 1024,
                max_viewport_pixels: 100,
                ..ResourceBudget::default()
            })""")
Path(embedder).write_text(text, encoding="utf-8")
