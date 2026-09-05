use super::{
    DEFAULT_MAX_CSS_RULES, DEFAULT_MAX_DISPLAY_COMMANDS, DEFAULT_MAX_DOM_DEPTH,
    DEFAULT_MAX_DOM_NODES, DEFAULT_MAX_FRAGMENTS, DEFAULT_MAX_TEXT_SCALARS, IncrementalReport,
    RenderError, RenderLimits, RenderObservability, RenderOptions, RenderSession,
    validate_viewport_size,
};
use rarog_compositor::{
    DisplayListRevision, FrameCause, FrameDecision, FramePlanner, FramePlannerError, SurfaceSize,
};
use rarog_paint::{
    DamageRegion, DisplayList, Framebuffer, FramebufferError, MAX_FRAMEBUFFER_PIXELS,
};
use rarog_platform::{NullPlatformHost, PlatformCapabilities, PlatformHost};
use rarog_types::{Color, Size};
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

pub const DEFAULT_MAX_DOCUMENT_SOURCE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BaseUrl(String);

impl BaseUrl {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn about_blank() -> Self {
        Self("about:blank".into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for BaseUrl {
    fn default() -> Self {
        Self::about_blank()
    }
}

impl From<&str> for BaseUrl {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for BaseUrl {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestDestination {
    Document,
    Style,
    Image,
    Script,
    Font,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequest {
    pub url: BaseUrl,
    pub destination: RequestDestination,
}

impl ResourceRequest {
    pub fn new(url: impl Into<BaseUrl>, destination: RequestDestination) -> Self {
        Self {
            url: url.into(),
            destination,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationRequest {
    pub url: BaseUrl,
    pub user_initiated: bool,
}

impl NavigationRequest {
    pub fn new(url: impl Into<BaseUrl>) -> Self {
        Self {
            url: url.into(),
            user_initiated: false,
        }
    }

    pub fn with_user_initiated(mut self, user_initiated: bool) -> Self {
        self.user_initiated = user_initiated;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestDisposition {
    ForwardToEmbedder,
    Blocked,
}

pub trait HostPolicy: Send + Sync {
    fn allow_navigation(&self, _view: ViewId, _request: &NavigationRequest) -> bool {
        true
    }

    fn allow_resource_request(&self, _view: ViewId, _request: &ResourceRequest) -> bool {
        true
    }
}

#[derive(Default)]
pub struct AllowAllHostPolicy;

impl HostPolicy for AllowAllHostPolicy {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStatus {
    Initial,
    ViewportRebuild,
    Incremental(IncrementalReport),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    DocumentLoaded {
        view: ViewId,
        base_url: BaseUrl,
        source_bytes: usize,
    },
    NavigationRequested {
        view: ViewId,
        request: NavigationRequest,
    },
    NavigationBlocked {
        view: ViewId,
        request: NavigationRequest,
    },
    ResourceRequested {
        view: ViewId,
        request: ResourceRequest,
    },
    ResourceBlocked {
        view: ViewId,
        request: ResourceRequest,
    },
    FrameRendered {
        view: ViewId,
        viewport: Size,
        status: FrameStatus,
    },
}

pub trait EventSink: Send + Sync {
    fn on_event(&self, event: &ViewEvent);
}

#[derive(Default)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn on_event(&self, _event: &ViewEvent) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_document_source_bytes: usize,
    pub max_viewport_pixels: u64,
    pub max_dom_nodes: usize,
    pub max_dom_depth: usize,
    pub max_text_scalars: usize,
    pub max_css_rules: usize,
    pub max_fragments: usize,
    pub max_display_commands: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_document_source_bytes: DEFAULT_MAX_DOCUMENT_SOURCE_BYTES,
            max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS,
            max_dom_nodes: DEFAULT_MAX_DOM_NODES,
            max_dom_depth: DEFAULT_MAX_DOM_DEPTH,
            max_text_scalars: DEFAULT_MAX_TEXT_SCALARS,
            max_css_rules: DEFAULT_MAX_CSS_RULES,
            max_fragments: DEFAULT_MAX_FRAGMENTS,
            max_display_commands: DEFAULT_MAX_DISPLAY_COMMANDS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineError {
    InvalidResourceBudget,
    DocumentSourceLimitExceeded { bytes: usize, limit: usize },
    ViewportPixelLimitExceeded { pixels: u64, limit: u64 },
    NoDocumentLoaded,
    ViewIdExhausted,
    Render(RenderError),
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResourceBudget => formatter.write_str("engine resource budget is invalid"),
            Self::DocumentSourceLimitExceeded { bytes, limit } => {
                write!(
                    formatter,
                    "document source requires {bytes} bytes; limit is {limit}"
                )
            }
            Self::ViewportPixelLimitExceeded { pixels, limit } => {
                write!(
                    formatter,
                    "viewport requires {pixels} pixels; limit is {limit}"
                )
            }
            Self::NoDocumentLoaded => formatter.write_str("view has no loaded document"),
            Self::ViewIdExhausted => {
                formatter.write_str("engine view identifier space is exhausted")
            }
            Self::Render(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<RenderError> for EngineError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

pub struct EngineBuilder {
    budget: ResourceBudget,
    host_policy: Arc<dyn HostPolicy>,
    event_sink: Arc<dyn EventSink>,
    platform_host: Arc<dyn PlatformHost>,
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self {
            budget: ResourceBudget::default(),
            host_policy: Arc::new(AllowAllHostPolicy),
            event_sink: Arc::new(NullEventSink),
            platform_host: Arc::new(NullPlatformHost),
        }
    }
}

impl EngineBuilder {
    pub fn resource_budget(mut self, budget: ResourceBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn host_policy<P>(mut self, policy: P) -> Self
    where
        P: HostPolicy + 'static,
    {
        self.host_policy = Arc::new(policy);
        self
    }

    pub fn event_sink<E>(mut self, sink: E) -> Self
    where
        E: EventSink + 'static,
    {
        self.event_sink = Arc::new(sink);
        self
    }

    pub fn platform_host<P>(mut self, host: P) -> Self
    where
        P: PlatformHost + 'static,
    {
        self.platform_host = Arc::new(host);
        self
    }

    pub fn build(self) -> Result<Engine, EngineError> {
        if self.budget.max_document_source_bytes == 0
            || self.budget.max_viewport_pixels == 0
            || self.budget.max_dom_nodes == 0
            || self.budget.max_dom_depth == 0
            || self.budget.max_text_scalars == 0
            || self.budget.max_css_rules == 0
            || self.budget.max_fragments == 0
            || self.budget.max_display_commands == 0
            || self.budget.max_viewport_pixels > MAX_FRAMEBUFFER_PIXELS
        {
            return Err(EngineError::InvalidResourceBudget);
        }

        Ok(Engine {
            shared: Arc::new(EngineShared {
                budget: self.budget,
                host_policy: self.host_policy,
                event_sink: self.event_sink,
                platform_host: self.platform_host,
                next_view_id: AtomicU64::new(1),
            }),
        })
    }
}

struct EngineShared {
    budget: ResourceBudget,
    host_policy: Arc<dyn HostPolicy>,
    event_sink: Arc<dyn EventSink>,
    platform_host: Arc<dyn PlatformHost>,
    next_view_id: AtomicU64,
}

#[derive(Clone)]
pub struct Engine {
    shared: Arc<EngineShared>,
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder::default()
    }

    pub fn resource_budget(&self) -> ResourceBudget {
        self.shared.budget
    }

    pub fn platform_name(&self) -> &'static str {
        self.shared.platform_host.name()
    }

    pub fn platform_capabilities(&self) -> PlatformCapabilities {
        self.shared.platform_host.capabilities()
    }

    pub fn create_view(&self, options: ViewOptions) -> Result<View, EngineError> {
        let raw_id = self
            .shared
            .next_view_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| EngineError::ViewIdExhausted)?;

        Ok(View {
            id: ViewId(raw_id),
            shared: Arc::clone(&self.shared),
            options,
            loaded: None,
            viewport: None,
            session: None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewOptions {
    pub background: Color,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            background: Color::WHITE,
        }
    }
}

struct LoadedDocument {
    source: String,
    base_url: BaseUrl,
}

pub struct View {
    id: ViewId,
    shared: Arc<EngineShared>,
    options: ViewOptions,
    loaded: Option<LoadedDocument>,
    viewport: Option<Size>,
    session: Option<RenderSession>,
}

impl View {
    pub fn id(&self) -> ViewId {
        self.id
    }

    pub fn base_url(&self) -> Option<&BaseUrl> {
        self.loaded.as_ref().map(|loaded| &loaded.base_url)
    }

    pub fn load_html(
        &mut self,
        source: impl Into<String>,
        base_url: impl Into<BaseUrl>,
    ) -> Result<(), EngineError> {
        let source = source.into();
        let limit = self.shared.budget.max_document_source_bytes;
        if source.len() > limit {
            return Err(EngineError::DocumentSourceLimitExceeded {
                bytes: source.len(),
                limit,
            });
        }

        let base_url = base_url.into();
        self.shared.event_sink.on_event(&ViewEvent::DocumentLoaded {
            view: self.id,
            base_url: base_url.clone(),
            source_bytes: source.len(),
        });
        self.loaded = Some(LoadedDocument { source, base_url });
        self.viewport = None;
        self.session = None;
        Ok(())
    }

    pub fn navigate(&self, request: NavigationRequest) -> RequestDisposition {
        if self.shared.host_policy.allow_navigation(self.id, &request) {
            self.shared
                .event_sink
                .on_event(&ViewEvent::NavigationRequested {
                    view: self.id,
                    request,
                });
            RequestDisposition::ForwardToEmbedder
        } else {
            self.shared
                .event_sink
                .on_event(&ViewEvent::NavigationBlocked {
                    view: self.id,
                    request,
                });
            RequestDisposition::Blocked
        }
    }

    pub fn request_resource(&self, request: ResourceRequest) -> RequestDisposition {
        if self
            .shared
            .host_policy
            .allow_resource_request(self.id, &request)
        {
            self.shared
                .event_sink
                .on_event(&ViewEvent::ResourceRequested {
                    view: self.id,
                    request,
                });
            RequestDisposition::ForwardToEmbedder
        } else {
            self.shared
                .event_sink
                .on_event(&ViewEvent::ResourceBlocked {
                    view: self.id,
                    request,
                });
            RequestDisposition::Blocked
        }
    }

    pub fn render(&mut self, viewport: Size) -> Result<ViewFrame<'_>, EngineError> {
        self.validate_viewport(viewport)?;
        if self.loaded.is_none() {
            return Err(EngineError::NoDocumentLoaded);
        }

        let status = match self.session.as_mut() {
            Some(session) if self.viewport != Some(viewport) => {
                session.resize(viewport)?;
                FrameStatus::ViewportRebuild
            }
            Some(session) => FrameStatus::Incremental(session.update()?),
            None => {
                let loaded = self.loaded.as_ref().ok_or(EngineError::NoDocumentLoaded)?;
                self.session = Some(RenderSession::new_with_limits(
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
                )?);
                FrameStatus::Initial
            }
        };
        self.viewport = Some(viewport);

        self.shared.event_sink.on_event(&ViewEvent::FrameRendered {
            view: self.id,
            viewport,
            status,
        });

        let session = self
            .session
            .as_ref()
            .expect("successful render establishes an active session");
        let full_observability = match status {
            FrameStatus::Initial | FrameStatus::ViewportRebuild => Some(session.observability()),
            FrameStatus::Incremental(_) => None,
        };
        Ok(ViewFrame {
            framebuffer: session.framebuffer(),
            display_list: session.display_list(),
            display_list_revision: session.display_list_revision(),
            damage: session.damage(),
            clear_color: self.options.background,
            status,
            full_observability,
        })
    }

    fn validate_viewport(&self, viewport: Size) -> Result<(), EngineError> {
        let pixels = viewport_pixel_count(viewport)?;
        let limit = self.shared.budget.max_viewport_pixels;
        if pixels > limit {
            return Err(EngineError::ViewportPixelLimitExceeded { pixels, limit });
        }
        Ok(())
    }
}

pub struct ViewFrame<'a> {
    pub framebuffer: &'a Framebuffer,
    pub display_list: &'a DisplayList,
    pub display_list_revision: DisplayListRevision,
    pub damage: &'a DamageRegion,
    pub clear_color: Color,
    pub status: FrameStatus,
    pub full_observability: Option<RenderObservability>,
}

impl ViewFrame<'_> {
    pub fn compositor_cause(&self) -> FrameCause {
        match self.status {
            FrameStatus::Initial => FrameCause::Initial,
            FrameStatus::ViewportRebuild => FrameCause::Resize,
            FrameStatus::Incremental(_) => FrameCause::SceneChange,
        }
    }

    pub fn plan_compositor_frame(
        &self,
        planner: &mut FramePlanner,
        surface_size: SurfaceSize,
    ) -> Result<FrameDecision, FramePlannerError> {
        self.plan_compositor_frame_with_cause(planner, surface_size, self.compositor_cause())
    }

    pub fn plan_compositor_frame_with_cause(
        &self,
        planner: &mut FramePlanner,
        surface_size: SurfaceSize,
        cause: FrameCause,
    ) -> Result<FrameDecision, FramePlannerError> {
        planner.plan(surface_size, self.display_list_revision, self.damage, cause)
    }
}

fn viewport_pixel_count(size: Size) -> Result<u64, RenderError> {
    validate_viewport_size(size)?;
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(RenderError::Framebuffer(FramebufferError::NonFiniteSize));
    }

    let width = size.width.max(1.0).round();
    let height = size.height.max(1.0).round();
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return Err(RenderError::Framebuffer(
            FramebufferError::DimensionsTooLarge,
        ));
    }

    u64::from(width as u32)
        .checked_mul(u64::from(height as u32))
        .ok_or(RenderError::Framebuffer(
            FramebufferError::PixelCountOverflow,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IncrementalMode;
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct RecordingEvents(Arc<Mutex<Vec<ViewEvent>>>);

    impl RecordingEvents {
        fn snapshot(&self) -> Vec<ViewEvent> {
            self.0.lock().expect("event recorder lock").clone()
        }
    }

    impl EventSink for RecordingEvents {
        fn on_event(&self, event: &ViewEvent) {
            self.0
                .lock()
                .expect("event recorder lock")
                .push(event.clone());
        }
    }

    struct BlockNavigation;

    impl HostPolicy for BlockNavigation {
        fn allow_navigation(&self, _view: ViewId, _request: &NavigationRequest) -> bool {
            false
        }
    }

    struct TestPlatformHost;

    impl PlatformHost for TestPlatformHost {
        fn name(&self) -> &'static str {
            "test-platform"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities {
                window_events: true,
                ..PlatformCapabilities::NONE
            }
        }
    }

    #[test]
    fn engine_exposes_platform_host_without_platform_specific_types() {
        let engine = Engine::builder()
            .platform_host(TestPlatformHost)
            .build()
            .unwrap();

        assert_eq!(engine.platform_name(), "test-platform");
        assert!(
            engine
                .platform_capabilities()
                .supports(rarog_platform::PlatformService::WindowEvents)
        );
        assert!(
            !engine
                .platform_capabilities()
                .supports(rarog_platform::PlatformService::GpuCompositor)
        );
    }

    #[test]
    fn engine_view_loads_renders_and_reuses_session() {
        let engine = Engine::builder().build().unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();
        view.load_html("<div>Rarog</div>", BaseUrl::about_blank())
            .unwrap();
        let viewport = Size {
            width: 160.0,
            height: 90.0,
        };

        {
            let frame = view.render(viewport).unwrap();
            assert_eq!(frame.status, FrameStatus::Initial);
            assert!(!frame.display_list.is_empty());
            let observability = frame
                .full_observability
                .expect("initial frame exposes full render observability");
            assert_eq!(
                observability.counters.display_commands,
                frame.display_list.len()
            );
        }

        let frame = view.render(viewport).unwrap();
        assert!(matches!(
            frame.status,
            FrameStatus::Incremental(IncrementalReport {
                mode: IncrementalMode::Unchanged,
                ..
            })
        ));
        assert_eq!(frame.full_observability, None);
    }

    #[test]
    fn view_frames_drive_compositor_planning_from_retained_revisions() {
        let engine = Engine::builder().build().unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();
        view.load_html("<div>Rarog</div>", BaseUrl::about_blank())
            .unwrap();
        let viewport = Size {
            width: 160.0,
            height: 90.0,
        };
        let surface = rarog_compositor::SurfaceId::new(9).unwrap();
        let surface_size = SurfaceSize::new(160, 90);
        let mut planner = FramePlanner::new(surface);

        let initial_revision = {
            let initial = view.render(viewport).unwrap();
            assert_eq!(initial.display_list_revision, DisplayListRevision::new(1));
            let FrameDecision::Submit(initial_plan) = initial
                .plan_compositor_frame(&mut planner, surface_size)
                .unwrap()
            else {
                panic!("initial view frame must submit");
            };
            assert_eq!(initial_plan.cause(), FrameCause::Initial);
            assert_eq!(
                initial_plan.update_kind(),
                rarog_compositor::FrameUpdateKind::Full
            );
            planner.complete(initial_plan.id()).unwrap();
            initial.display_list_revision
        };

        {
            let unchanged = view.render(viewport).unwrap();
            assert_eq!(unchanged.display_list_revision, initial_revision);
            assert_eq!(
                unchanged
                    .plan_compositor_frame(&mut planner, surface_size)
                    .unwrap(),
                FrameDecision::Noop
            );
        }

        let resized = view
            .render(Size {
                width: 220.0,
                height: 120.0,
            })
            .unwrap();
        let FrameDecision::Submit(resize_plan) = resized
            .plan_compositor_frame(&mut planner, SurfaceSize::new(220, 120))
            .unwrap()
        else {
            panic!("resized view frame must submit");
        };
        assert_eq!(resize_plan.cause(), FrameCause::Resize);
        assert_eq!(
            resize_plan.update_kind(),
            rarog_compositor::FrameUpdateKind::Full
        );
    }

    #[test]
    fn viewport_change_reuses_session_and_reports_rebuild_observability() {
        let engine = Engine::builder().build().unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();
        view.load_html("<div>Rarog</div>", BaseUrl::about_blank())
            .unwrap();

        view.render(Size {
            width: 160.0,
            height: 90.0,
        })
        .unwrap();
        let frame = view
            .render(Size {
                width: 220.0,
                height: 120.0,
            })
            .unwrap();

        assert_eq!(frame.status, FrameStatus::ViewportRebuild);
        assert_eq!(frame.framebuffer.width, 220);
        assert_eq!(frame.framebuffer.height, 120);
        assert_eq!(
            frame
                .full_observability
                .expect("viewport rebuild exposes observability")
                .timings
                .parse,
            std::time::Duration::ZERO
        );
    }

    #[test]
    fn load_html_enforces_source_budget_before_parsing() {
        let engine = Engine::builder()
            .resource_budget(ResourceBudget {
                max_document_source_bytes: 4,
                max_viewport_pixels: 100,
                ..ResourceBudget::default()
            })
            .build()
            .unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();

        assert_eq!(
            view.load_html("12345", BaseUrl::about_blank()),
            Err(EngineError::DocumentSourceLimitExceeded { bytes: 5, limit: 4 })
        );
    }

    #[test]
    fn viewport_budget_is_checked_before_framebuffer_allocation() {
        let engine = Engine::builder()
            .resource_budget(ResourceBudget {
                max_document_source_bytes: 1024,
                max_viewport_pixels: 100,
                ..ResourceBudget::default()
            })
            .build()
            .unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();
        view.load_html("<div>x</div>", BaseUrl::about_blank())
            .unwrap();

        assert!(matches!(
            view.render(Size {
                width: 11.0,
                height: 10.0,
            }),
            Err(EngineError::ViewportPixelLimitExceeded {
                pixels: 110,
                limit: 100
            })
        ));
    }

    #[test]
    fn negative_viewport_is_rejected_before_rendering() {
        let engine = Engine::builder().build().unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();
        view.load_html("<div>x</div>", BaseUrl::about_blank())
            .unwrap();

        assert!(matches!(
            view.render(Size {
                width: -1.0,
                height: 10.0,
            }),
            Err(EngineError::Render(RenderError::InvalidViewportSize))
        ));
    }

    #[test]
    fn navigation_policy_blocks_without_networking() {
        let events = RecordingEvents::default();
        let engine = Engine::builder()
            .host_policy(BlockNavigation)
            .event_sink(events.clone())
            .build()
            .unwrap();
        let view = engine.create_view(ViewOptions::default()).unwrap();
        let request = NavigationRequest::new("https://example.test/").with_user_initiated(true);

        assert_eq!(view.navigate(request.clone()), RequestDisposition::Blocked);
        assert!(events.snapshot().contains(&ViewEvent::NavigationBlocked {
            view: view.id(),
            request,
        }));
    }

    #[test]
    fn resource_requests_are_forwarded_to_embedder() {
        let events = RecordingEvents::default();
        let engine = Engine::builder()
            .event_sink(events.clone())
            .build()
            .unwrap();
        let view = engine.create_view(ViewOptions::default()).unwrap();
        let request =
            ResourceRequest::new("https://example.test/app.css", RequestDestination::Style);

        assert_eq!(
            view.request_resource(request.clone()),
            RequestDisposition::ForwardToEmbedder
        );
        assert!(events.snapshot().contains(&ViewEvent::ResourceRequested {
            view: view.id(),
            request,
        }));
    }

    #[test]
    fn rendering_requires_a_loaded_document() {
        let engine = Engine::builder().build().unwrap();
        let mut view = engine.create_view(ViewOptions::default()).unwrap();

        assert!(matches!(
            view.render(Size {
                width: 20.0,
                height: 20.0,
            }),
            Err(EngineError::NoDocumentLoaded)
        ));
    }

    #[test]
    fn builder_rejects_budget_above_framebuffer_safety_limit() {
        assert!(matches!(
            Engine::builder()
                .resource_budget(ResourceBudget {
                    max_document_source_bytes: 1,
                    max_viewport_pixels: MAX_FRAMEBUFFER_PIXELS + 1,
                    ..ResourceBudget::default()
                })
                .build(),
            Err(EngineError::InvalidResourceBudget)
        ));
    }
}
