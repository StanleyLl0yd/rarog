use rarog_fetch::{
    FetchError, FetchLimits, FetchMethod, FetchRequest, FetchResponse, HeaderList,
    NetworkCapability, NetworkPoll, NetworkRequest, NetworkTicket,
};
use rarog_host::{
    HostControlErrorKind, HostControlPlane, HostLimits, NavigationTransitionKind,
    ServiceWorkerFetchCompletion, ServiceWorkerFetchResult, ServiceWorkerFetchRoute,
};
use rarog_ipc::IpcLimits;
use rarog_url::{Origin, SiteIdentity, WebUrl};
use rarog_workers::{ServiceWorkerRegistrationUpdate, ServiceWorkerRegistry};
use std::num::NonZeroU64;

fn url(input: &str) -> WebUrl {
    WebUrl::parse(input).unwrap()
}

fn origin(input: &str) -> Origin {
    url(input).origin().unwrap()
}

fn site(input: &str) -> SiteIdentity {
    url(input).site_identity().unwrap()
}

fn limits(max_dispatches: usize, max_url_bytes: usize) -> HostLimits {
    HostLimits {
        max_site_processes: 4,
        ipc: IpcLimits {
            max_message_bytes: 4096,
            max_queued_messages: 16,
            max_queued_bytes: 64 * 1024,
        },
        max_capabilities: 32,
        max_network_operations: 16,
        max_navigation_contexts: 8,
        max_navigation_context_url_bytes: max_url_bytes,
        max_service_worker_fetch_dispatches: max_dispatches,
        max_websocket_connections: 4,
    }
}

fn active_registration(
    registry: &mut ServiceWorkerRegistry,
    owner: &Origin,
    scope: &str,
    script: &str,
) -> ServiceWorkerRegistrationUpdate {
    let update = registry.register(owner, &url(scope), &url(script)).unwrap();
    registry.finish_install(update.version()).unwrap();
    registry.begin_activate(update.version()).unwrap();
    registry.finish_activate(update.version()).unwrap();
    update
}

fn activating_registration(
    registry: &mut ServiceWorkerRegistry,
    owner: &Origin,
    scope: &str,
    script: &str,
) -> ServiceWorkerRegistrationUpdate {
    let update = registry.register(owner, &url(scope), &url(script)).unwrap();
    registry.finish_install(update.version()).unwrap();
    registry.begin_activate(update.version()).unwrap();
    update
}

fn request_for(client_origin: &Origin, target: &str) -> FetchRequest {
    FetchRequest::new(url(target), client_origin.clone())
}

fn post_request_for(client_origin: &Origin, target: &str, body: &[u8]) -> FetchRequest {
    let mut request = request_for(client_origin, target);
    request.set_method(FetchMethod::post()).unwrap();
    request.set_body(Some(body.to_vec())).unwrap();
    request
}

#[derive(Default)]
struct FixtureNetwork {
    next_ticket: u64,
    starts: usize,
    polls: usize,
    cancels: usize,
}

impl NetworkCapability for FixtureNetwork {
    fn start(&mut self, _request: NetworkRequest) -> Result<NetworkTicket, FetchError> {
        self.starts += 1;
        self.next_ticket += 1;
        Ok(NetworkTicket::new(
            NonZeroU64::new(self.next_ticket).unwrap(),
        ))
    }

    fn poll(&mut self, _ticket: NetworkTicket) -> Result<NetworkPoll, FetchError> {
        self.polls += 1;
        Ok(NetworkPoll::Complete(
            FetchResponse::try_new(None, 204, HeaderList::default(), Vec::new(), 1).unwrap(),
        ))
    }

    fn cancel(&mut self, _ticket: NetworkTicket) -> Result<(), FetchError> {
        self.cancels += 1;
        Ok(())
    }
}

#[test]
fn host_owns_bounded_fragmentless_navigation_url() {
    let mut host = HostControlPlane::try_new(limits(4, 40)).unwrap();
    let huge_fragment = "x".repeat(4096);
    let target = url(&format!("https://example.com/app#{}", huge_fragment));
    let context = host.open_navigation_context(&target).unwrap();
    assert_eq!(context.url().unwrap().as_str(), "https://example.com/app");

    let too_long = url("https://example.com/this-path-is-definitely-too-long");
    assert_eq!(
        host.open_navigation_context(&too_long).unwrap_err().kind,
        HostControlErrorKind::NavigationContextUrlLimitExceeded
    );
    assert_eq!(host.active_navigation_contexts(), 1);
}

#[test]
fn url_less_context_is_not_eligible_for_service_worker_control_or_fetch_policy() {
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context_to_site(site("https://example.com/"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    let registry = ServiceWorkerRegistry::with_default_limits().unwrap();

    assert_eq!(
        host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::NavigationContextUrlUnavailable
    );
    assert_eq!(
        host.prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&origin("https://example.com/"), "https://example.com/data"),
            &registry,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::NavigationContextUrlUnavailable
    );
}

#[test]
fn host_current_url_selects_longest_scope_controller() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    let root = active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/root.js",
    );
    let app = active_registration(
        &mut registry,
        &owner,
        "https://example.com/app/",
        "https://example.com/app.js",
    );
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/app/page#ignored"))
        .unwrap();

    let controller = host
        .refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap()
        .unwrap();
    assert_eq!(controller.registration(), app.registration());
    assert_eq!(controller.version(), app.version());
    assert_ne!(controller.registration(), root.registration());
    assert_eq!(
        host.navigation_context(context.context())
            .unwrap()
            .service_worker_controller(),
        Some(controller)
    );
}

#[test]
fn same_site_cross_origin_navigation_clears_controller_and_pending_dispatch() {
    let first_origin = origin("https://a.example.com/");
    let second_origin = origin("https://b.example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &first_origin,
        "https://a.example.com/app/",
        "https://a.example.com/sw.js",
    );
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://a.example.com/app/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();
    let dispatch = match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&first_origin, "https://cdn.example.net/data"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    assert_eq!(host.active_service_worker_fetch_dispatches(), 1);
    assert_eq!(
        host.service_worker_fetch_dispatch(capability, dispatch, &registry)
            .unwrap()
            .context(),
        context.context()
    );

    let transition = host
        .navigate_navigation_context(context.context(), &url("https://b.example.com/next"))
        .unwrap();
    assert_eq!(transition.kind(), NavigationTransitionKind::SameSiteReuse);
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
    let snapshot = host.navigation_context(context.context()).unwrap();
    assert_eq!(snapshot.origin(), Some(&second_origin));
    assert_eq!(snapshot.service_worker_controller(), None);

    match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&second_origin, "https://b.example.com/data"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::NetworkFallback(request) => {
            assert_eq!(request.origin(), &second_origin);
        }
        other => panic!("expected network fallback, got {other:?}"),
    }
}

#[test]
fn controlled_cross_origin_subresource_uses_controller_but_forged_client_origin_is_rejected() {
    let owner = origin("https://app.example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &owner,
        "https://app.example.com/",
        "https://app.example.com/sw.js",
    );
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://app.example.com/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();

    let id = match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://cdn.other.example/resource"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    assert_eq!(
        host.service_worker_fetch_dispatch(capability, id, &registry)
            .unwrap()
            .request()
            .url()
            .as_str(),
        "https://cdn.other.example/resource"
    );

    let forged = request_for(
        &origin("https://attacker.example/"),
        "https://cdn.other.example/forged",
    );
    assert_eq!(
        host.prepare_navigation_context_service_worker_fetch(capability, forged, &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::ServiceWorkerRequestOriginMismatch
    );
    assert_eq!(host.active_service_worker_fetch_dispatches(), 1);
}

#[test]
fn fallback_and_activation_wait_preserve_owned_request_without_backend_access() {
    let owner = origin("https://example.com/");
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/app/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    let mut network = FixtureNetwork::default();

    let request = post_request_for(
        &owner,
        "https://example.com/no-controller",
        b"fallback-body",
    );
    let request = match host
        .prepare_navigation_context_service_worker_fetch(capability, request, &registry)
        .unwrap()
    {
        ServiceWorkerFetchRoute::NetworkFallback(request) => request,
        other => panic!("expected owned fallback, got {other:?}"),
    };
    assert_eq!(request.body(), Some(b"fallback-body".as_slice()));
    assert_eq!(network.starts, 0);

    let operation = host
        .start_navigation_context_network_operation(
            capability,
            request.into_network_request(),
            &mut network,
        )
        .unwrap();
    assert_eq!(network.starts, 1);
    assert!(matches!(
        host.poll_navigation_context_network_operation(capability, operation, &mut network)
            .unwrap(),
        NetworkPoll::Complete(_)
    ));

    let update = activating_registration(
        &mut registry,
        &owner,
        "https://example.com/app/",
        "https://example.com/sw.js",
    );
    let controller = host
        .refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap()
        .unwrap();
    let request = post_request_for(&owner, "https://example.com/wait", b"wait-body");
    let request = match host
        .prepare_navigation_context_service_worker_fetch(capability, request, &registry)
        .unwrap()
    {
        ServiceWorkerFetchRoute::AwaitingActivation {
            controller: waiting,
            request,
        } => {
            assert_eq!(waiting, controller);
            request
        }
        other => panic!("expected activation wait, got {other:?}"),
    };
    assert_eq!(request.body(), Some(b"wait-body".as_slice()));
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
    assert_eq!(network.starts, 1);

    registry.finish_activate(update.version()).unwrap();
    assert!(matches!(
        host.prepare_navigation_context_service_worker_fetch(capability, request, &registry)
            .unwrap(),
        ServiceWorkerFetchRoute::Dispatch(_)
    ));
    assert_eq!(network.starts, 1);
}

#[test]
fn stale_controller_and_cross_context_dispatch_ids_fail_closed() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    let update = active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/sw.js",
    );
    let mut host = HostControlPlane::try_new(limits(8, 512)).unwrap();
    let first = host
        .open_navigation_context(&url("https://example.com/one"))
        .unwrap();
    let second = host
        .open_navigation_context(&url("https://example.com/two"))
        .unwrap();
    let first_capability = host
        .grant_navigation_context_network_capability(first.context())
        .unwrap();
    let second_capability = host
        .grant_navigation_context_network_capability(second.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(first.context(), &registry)
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(second.context(), &registry)
        .unwrap();

    let id = match host
        .prepare_navigation_context_service_worker_fetch(
            first_capability,
            request_for(&owner, "https://example.com/resource"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    assert_eq!(
        host.service_worker_fetch_dispatch(second_capability, id, &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidServiceWorkerFetchDispatchAuthority
    );

    registry.unregister(update.registration()).unwrap();
    assert_eq!(
        host.service_worker_fetch_dispatch(first_capability, id, &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::ServiceWorkerControllerStale
    );
    assert_eq!(
        host.reap_stale_service_worker_fetch_dispatches(&registry),
        1
    );
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
}

#[test]
fn dispatch_bound_and_capability_revocation_recover_capacity() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/sw.js",
    );
    let mut host = HostControlPlane::try_new(limits(1, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();

    let first = match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/one"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    assert_eq!(
        host.prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/two"),
            &registry,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::ServiceWorkerFetchDispatchLimitExceeded
    );
    host.discard_navigation_context_service_worker_fetch(capability, first, &registry)
        .unwrap();
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
    assert!(matches!(
        host.prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/three"),
            &registry,
        )
        .unwrap(),
        ServiceWorkerFetchRoute::Dispatch(_)
    ));
    assert_eq!(host.active_service_worker_fetch_dispatches(), 1);

    host.revoke_navigation_context_capability(capability)
        .unwrap();
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
}

#[test]
fn response_completion_is_bounded_and_fallback_uses_existing_host_network_authority() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/sw.js",
    );
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();
    let mut network = FixtureNetwork::default();

    let request_limits = FetchLimits {
        max_headers: 4,
        max_header_bytes: 64,
        max_request_body_bytes: 16,
        max_response_body_bytes: 1,
    };
    let request = FetchRequest::try_new(
        url("https://example.com/bounded"),
        owner.clone(),
        request_limits,
    )
    .unwrap();
    let bounded_id = match host
        .prepare_navigation_context_service_worker_fetch(capability, request, &registry)
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    let oversized =
        FetchResponse::try_new(None, 200, HeaderList::default(), vec![1, 2], 2).unwrap();
    assert_eq!(
        host.complete_navigation_context_service_worker_fetch(
            capability,
            bounded_id,
            ServiceWorkerFetchCompletion::Response(oversized),
            &registry,
            &mut network,
        )
        .unwrap_err()
        .kind,
        HostControlErrorKind::Fetch(rarog_fetch::FetchErrorKind::ResponseBodyLimitExceeded)
    );
    assert_eq!(host.active_service_worker_fetch_dispatches(), 1);

    let valid = FetchResponse::try_new(None, 200, HeaderList::default(), vec![1], 1).unwrap();
    assert!(matches!(
        host.complete_navigation_context_service_worker_fetch(
            capability,
            bounded_id,
            ServiceWorkerFetchCompletion::Response(valid),
            &registry,
            &mut network,
        )
        .unwrap(),
        ServiceWorkerFetchResult::Response(_)
    ));
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
    assert_eq!(network.starts, 0);

    let fallback_id = match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            post_request_for(&owner, "https://example.com/fallback", b"body"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };
    let operation = match host
        .complete_navigation_context_service_worker_fetch(
            capability,
            fallback_id,
            ServiceWorkerFetchCompletion::Fallback,
            &registry,
            &mut network,
        )
        .unwrap()
    {
        ServiceWorkerFetchResult::Network(operation) => operation,
        ServiceWorkerFetchResult::Response(_) => panic!("expected Host network fallback"),
    };
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
    assert_eq!(network.starts, 1);
    assert_eq!(host.active_network_operations(), 1);
    assert!(matches!(
        host.poll_navigation_context_network_operation(capability, operation, &mut network)
            .unwrap(),
        NetworkPoll::Complete(_)
    ));
    assert_eq!(network.polls, 1);
    assert_eq!(host.active_network_operations(), 0);
}

#[test]
fn process_loss_and_context_close_revoke_pending_dispatches() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/sw.js",
    );

    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/one"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();
    assert!(matches!(
        host.prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/data"),
            &registry,
        )
        .unwrap(),
        ServiceWorkerFetchRoute::Dispatch(_)
    ));
    host.close_navigation_context(context.context()).unwrap();
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);

    let context = host
        .open_navigation_context(&url("https://example.com/two"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    host.refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap();
    assert!(matches!(
        host.prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/data2"),
            &registry,
        )
        .unwrap(),
        ServiceWorkerFetchRoute::Dispatch(_)
    ));
    let process = host
        .process_for_site(&site("https://example.com/"))
        .unwrap();
    host.process_lost(process).unwrap();
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
}

#[test]
fn dispatch_ids_are_scoped_references_not_cross_host_authority() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/sw.js",
    );

    fn make_dispatch(
        owner: &Origin,
        registry: &ServiceWorkerRegistry,
    ) -> (
        HostControlPlane,
        rarog_host::NavigationContextCapability,
        rarog_host::ServiceWorkerFetchDispatchId,
    ) {
        let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
        let context = host
            .open_navigation_context(&url("https://example.com/page"))
            .unwrap();
        let capability = host
            .grant_navigation_context_network_capability(context.context())
            .unwrap();
        host.refresh_navigation_context_service_worker_controller(context.context(), registry)
            .unwrap();
        let id = match host
            .prepare_navigation_context_service_worker_fetch(
                capability,
                request_for(owner, "https://example.com/data"),
                registry,
            )
            .unwrap()
        {
            ServiceWorkerFetchRoute::Dispatch(id) => id,
            other => panic!("expected dispatch, got {other:?}"),
        };
        (host, capability, id)
    }

    let (first_host, first_capability, first_id) = make_dispatch(&owner, &registry);
    let (_second_host, _second_capability, second_id) = make_dispatch(&owner, &registry);
    assert_ne!(first_id.scope(), second_id.scope());
    assert_eq!(
        first_host
            .service_worker_fetch_dispatch(first_capability, second_id, &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::InvalidServiceWorkerFetchDispatchAuthority
    );
    assert_eq!(
        first_host
            .service_worker_fetch_dispatch(first_capability, first_id, &registry)
            .unwrap()
            .id(),
        first_id
    );
}

#[test]
fn stale_version_identity_is_rejected_after_active_replacement() {
    let owner = origin("https://example.com/");
    let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
    let first = active_registration(
        &mut registry,
        &owner,
        "https://example.com/",
        "https://example.com/v1.js",
    );
    let mut host = HostControlPlane::try_new(limits(4, 512)).unwrap();
    let context = host
        .open_navigation_context(&url("https://example.com/page"))
        .unwrap();
    let capability = host
        .grant_navigation_context_network_capability(context.context())
        .unwrap();
    let controller = host
        .refresh_navigation_context_service_worker_controller(context.context(), &registry)
        .unwrap()
        .unwrap();
    assert_eq!(controller.version(), first.version());

    let id = match host
        .prepare_navigation_context_service_worker_fetch(
            capability,
            request_for(&owner, "https://example.com/data"),
            &registry,
        )
        .unwrap()
    {
        ServiceWorkerFetchRoute::Dispatch(id) => id,
        other => panic!("expected dispatch, got {other:?}"),
    };

    let replacement = registry
        .register(
            &owner,
            &url("https://example.com/"),
            &url("https://example.com/v2.js"),
        )
        .unwrap();
    registry.finish_install(replacement.version()).unwrap();
    registry.begin_activate(replacement.version()).unwrap();
    registry.finish_activate(replacement.version()).unwrap();

    assert_eq!(
        host.service_worker_fetch_dispatch(capability, id, &registry)
            .unwrap_err()
            .kind,
        HostControlErrorKind::ServiceWorkerControllerStale
    );
    assert_eq!(
        host.reap_stale_service_worker_fetch_dispatches(&registry),
        1
    );
    assert_eq!(host.active_service_worker_fetch_dispatches(), 0);
}
