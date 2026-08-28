use rarog_engine::{
    BaseUrl, Engine, EngineError, FrameStatus, HostPolicy, NavigationRequest, RequestDisposition,
    ResourceBudget, ViewId, ViewOptions,
};
use rarog_types::Size;

struct BlockNavigation;

impl HostPolicy for BlockNavigation {
    fn allow_navigation(&self, _view: ViewId, _request: &NavigationRequest) -> bool {
        false
    }
}

#[test]
fn p1_engine_view_contract_enforces_budgets_and_exposes_observability() {
    let engine = Engine::builder()
        .resource_budget(ResourceBudget {
            max_document_source_bytes: 32,
            max_viewport_pixels: 20_000,
            ..ResourceBudget::default()
        })
        .build()
        .expect("valid P1 resource budget");
    let first_id = engine
        .create_view(ViewOptions::default())
        .expect("first view")
        .id();
    let mut view = engine
        .create_view(ViewOptions::default())
        .expect("second view");
    assert_ne!(first_id, view.id());

    assert_eq!(
        view.load_html("x".repeat(33), BaseUrl::about_blank()),
        Err(EngineError::DocumentSourceLimitExceeded {
            bytes: 33,
            limit: 32
        })
    );

    view.load_html("<div>Rarog</div>", BaseUrl::about_blank())
        .expect("fixture fits source budget");
    assert!(matches!(
        view.render(Size {
            width: 201.0,
            height: 100.0
        }),
        Err(EngineError::ViewportPixelLimitExceeded {
            pixels: 20_100,
            limit: 20_000
        })
    ));

    let viewport = Size {
        width: 160.0,
        height: 90.0,
    };
    {
        let frame = view.render(viewport).expect("initial P1 frame");
        assert_eq!(frame.status, FrameStatus::Initial);
        let observability = frame
            .full_observability
            .expect("full frame metrics must reach the embedder");
        assert_eq!(
            observability.counters.display_commands,
            frame.display_list.commands.len()
        );
        assert!(observability.counters.dom_nodes > 0);
        assert!(observability.counters.fragments > 0);
    }

    let frame = view.render(viewport).expect("incremental P1 frame");
    assert!(matches!(frame.status, FrameStatus::Incremental(_)));
    assert_eq!(frame.full_observability, None);
}

#[test]
fn p1_navigation_contract_obeys_host_policy() {
    let engine = Engine::builder()
        .host_policy(BlockNavigation)
        .build()
        .expect("engine with host policy");
    let view = engine
        .create_view(ViewOptions::default())
        .expect("policy test view");
    assert_eq!(
        view.navigate(NavigationRequest::new("https://example.test/")),
        RequestDisposition::Blocked
    );
}
