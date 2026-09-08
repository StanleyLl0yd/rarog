use rarog_broker::{
    CapabilityBroker, CapabilityClass, CapabilityError, CapabilityErrorKind, CapabilityGrant,
    CapabilityId, DEFAULT_MAX_CAPABILITIES,
};
use rarog_fetch::{
    FetchError, FetchErrorKind, NetworkCapability, NetworkPoll, NetworkRequest, NetworkTicket,
};
use rarog_ipc::{EndpointRole, IpcChannel, IpcEnvelope, IpcError, IpcErrorKind, IpcLimits};
use rarog_platform::{ClipboardError, ClipboardText, PlatformClipboardService};
use rarog_process::{
    DEFAULT_MAX_SITE_PROCESSES, ProcessTopology, ProcessTopologyError, ProcessTopologyErrorKind,
    SiteAssignmentKind, SiteProcessId,
};
use rarog_url::{SiteIdentity, UrlError, UrlErrorKind};
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_NETWORK_OPERATIONS: usize = 4096;
pub const DEFAULT_MAX_NAVIGATION_CONTEXTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostLimits {
    pub max_site_processes: usize,
    pub ipc: IpcLimits,
    pub max_capabilities: usize,
    pub max_network_operations: usize,
    pub max_navigation_contexts: usize,
}

impl HostLimits {
    pub fn is_valid(self) -> bool {
        self.max_site_processes > 0
            && self.ipc.is_valid()
            && self.max_capabilities > 0
            && self.max_network_operations > 0
            && self.max_navigation_contexts > 0
    }
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            max_site_processes: DEFAULT_MAX_SITE_PROCESSES,
            ipc: IpcLimits::default(),
            max_capabilities: DEFAULT_MAX_CAPABILITIES,
            max_network_operations: DEFAULT_MAX_NETWORK_OPERATIONS,
            max_navigation_contexts: DEFAULT_MAX_NAVIGATION_CONTEXTS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostControlErrorKind {
    InvalidLimits,
    UnknownSiteProcess,
    InvalidEnvelopeDirection,
    InconsistentState,
    Process(ProcessTopologyErrorKind),
    Ipc(IpcErrorKind),
    Capability(CapabilityErrorKind),
    Url(UrlErrorKind),
    Fetch(FetchErrorKind),
    Clipboard(ClipboardError),
    InvalidDocumentBinding,
    InvalidNavigationContextId,
    NavigationContextLimitExceeded,
    NavigationContextIdentitySpaceExhausted,
    UnknownNavigationContext,
    NavigationContextInvalidated,
    InvalidNavigationContextCapabilityAuthority,
    InvalidNetworkOperationId,
    NetworkOperationLimitExceeded,
    NetworkOperationIdentitySpaceExhausted,
    InvalidNetworkOperationAuthority,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostControlError {
    pub kind: HostControlErrorKind,
    pub message: String,
}

impl HostControlError {
    fn new(kind: HostControlErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn unknown_process(process: SiteProcessId) -> Self {
        Self::new(
            HostControlErrorKind::UnknownSiteProcess,
            format!("unknown or retired site process {process}"),
        )
    }
}

impl fmt::Display for HostControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HostControlError {}

impl From<ProcessTopologyError> for HostControlError {
    fn from(error: ProcessTopologyError) -> Self {
        Self::new(HostControlErrorKind::Process(error.kind), error.message)
    }
}

impl From<IpcError> for HostControlError {
    fn from(error: IpcError) -> Self {
        Self::new(HostControlErrorKind::Ipc(error.kind), error.message)
    }
}

impl From<CapabilityError> for HostControlError {
    fn from(error: CapabilityError) -> Self {
        Self::new(HostControlErrorKind::Capability(error.kind), error.message)
    }
}

impl From<UrlError> for HostControlError {
    fn from(error: UrlError) -> Self {
        Self::new(HostControlErrorKind::Url(error.kind), error.message)
    }
}

impl From<FetchError> for HostControlError {
    fn from(error: FetchError) -> Self {
        Self::new(HostControlErrorKind::Fetch(error.kind), error.message)
    }
}

impl From<ClipboardError> for HostControlError {
    fn from(error: ClipboardError) -> Self {
        Self::new(HostControlErrorKind::Clipboard(error), error.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteLease {
    process: SiteProcessId,
    kind: SiteAssignmentKind,
}

impl SiteLease {
    pub fn process(self) -> SiteProcessId {
        self.process
    }

    pub fn kind(self) -> SiteAssignmentKind {
        self.kind
    }

    pub fn is_new(self) -> bool {
        self.kind == SiteAssignmentKind::Created
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteLoss {
    process: SiteProcessId,
    site: SiteIdentity,
    revoked_capabilities: usize,
    revoked_network_operations: usize,
    invalidated_navigation_contexts: usize,
}

impl SiteLoss {
    pub fn process(&self) -> SiteProcessId {
        self.process
    }

    pub fn site(&self) -> &SiteIdentity {
        &self.site
    }

    pub fn revoked_capabilities(&self) -> usize {
        self.revoked_capabilities
    }

    pub fn revoked_network_operations(&self) -> usize {
        self.revoked_network_operations
    }

    pub fn invalidated_navigation_contexts(&self) -> usize {
        self.invalidated_navigation_contexts
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSiteBinding {
    site: SiteIdentity,
    process: SiteProcessId,
}

impl DocumentSiteBinding {
    pub fn site(&self) -> &SiteIdentity {
        &self.site
    }

    pub fn process(&self) -> SiteProcessId {
        self.process
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTransitionKind {
    Initial,
    SameSiteReuse,
    CrossSiteReplacement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationTransition {
    previous: Option<DocumentSiteBinding>,
    current: DocumentSiteBinding,
    kind: NavigationTransitionKind,
}

impl NavigationTransition {
    pub fn previous(&self) -> Option<&DocumentSiteBinding> {
        self.previous.as_ref()
    }

    pub fn current(&self) -> &DocumentSiteBinding {
        &self.current
    }

    pub fn kind(&self) -> NavigationTransitionKind {
        self.kind
    }

    pub fn into_current(self) -> DocumentSiteBinding {
        self.current
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NavigationContextId(NonZeroU64);

impl NavigationContextId {
    pub fn try_new(raw: u64) -> Result<Self, HostControlError> {
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            HostControlError::new(
                HostControlErrorKind::InvalidNavigationContextId,
                "navigation context identity must be non-zero",
            )
        })
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for NavigationContextId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "navigation-context:{}", self.get())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationContextSnapshot {
    context: NavigationContextId,
    site: SiteIdentity,
}

impl NavigationContextSnapshot {
    pub fn context(&self) -> NavigationContextId {
        self.context
    }

    pub fn site(&self) -> &SiteIdentity {
        &self.site
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationContextTransition {
    context: NavigationContextId,
    previous_site: SiteIdentity,
    current_site: SiteIdentity,
    kind: NavigationTransitionKind,
    source_retired: bool,
}

impl NavigationContextTransition {
    pub fn context(&self) -> NavigationContextId {
        self.context
    }

    pub fn previous_site(&self) -> &SiteIdentity {
        &self.previous_site
    }

    pub fn current_site(&self) -> &SiteIdentity {
        &self.current_site
    }

    pub fn kind(&self) -> NavigationTransitionKind {
        self.kind
    }

    pub fn source_retired(&self) -> bool {
        self.source_retired
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationContextClose {
    context: NavigationContextId,
    site: SiteIdentity,
    source_retired: bool,
    revoked_capabilities: usize,
}

impl NavigationContextClose {
    pub fn context(&self) -> NavigationContextId {
        self.context
    }

    pub fn site(&self) -> &SiteIdentity {
        &self.site
    }

    pub fn source_retired(&self) -> bool {
        self.source_retired
    }

    pub fn revoked_capabilities(&self) -> usize {
        self.revoked_capabilities
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationContextCapability {
    context: NavigationContextId,
    id: CapabilityId,
    class: CapabilityClass,
}

impl NavigationContextCapability {
    pub fn context(self) -> NavigationContextId {
        self.context
    }

    pub fn id(self) -> CapabilityId {
        self.id
    }

    pub fn class(self) -> CapabilityClass {
        self.class
    }
}

#[derive(Debug)]
struct NavigationContextIdAllocator {
    next: Option<NonZeroU64>,
}

impl NavigationContextIdAllocator {
    fn new() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }

    fn allocate(&mut self) -> Result<NavigationContextId, HostControlError> {
        let next = self.next.ok_or_else(|| {
            HostControlError::new(
                HostControlErrorKind::NavigationContextIdentitySpaceExhausted,
                "navigation context identity space is exhausted",
            )
        })?;
        self.next = NonZeroU64::new(next.get().wrapping_add(1));
        Ok(NavigationContextId(next))
    }
}

#[derive(Clone, Debug)]
struct NavigationContext {
    binding: DocumentSiteBinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkOperationId(NonZeroU64);

impl NetworkOperationId {
    pub fn try_new(raw: u64) -> Result<Self, HostControlError> {
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            HostControlError::new(
                HostControlErrorKind::InvalidNetworkOperationId,
                "network operation identity must be non-zero",
            )
        })
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug)]
struct NetworkOperationIdAllocator {
    next: Option<NonZeroU64>,
}

impl NetworkOperationIdAllocator {
    fn new() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }

    fn allocate(&mut self) -> Result<NetworkOperationId, HostControlError> {
        let next = self.next.ok_or_else(|| {
            HostControlError::new(
                HostControlErrorKind::NetworkOperationIdentitySpaceExhausted,
                "network operation identity space is exhausted",
            )
        })?;
        self.next = NonZeroU64::new(next.get().wrapping_add(1));
        Ok(NetworkOperationId(next))
    }
}

#[derive(Clone, Copy, Debug)]
struct NetworkOperation {
    owner: SiteProcessId,
    capability: CapabilityId,
    ticket: NetworkTicket,
}

#[derive(Debug)]
struct SiteInstance {
    site: SiteIdentity,
    channel: IpcChannel,
}

#[derive(Debug)]
pub struct HostControlPlane {
    topology: ProcessTopology,
    broker: CapabilityBroker,
    ipc_limits: IpcLimits,
    max_network_operations: usize,
    max_navigation_contexts: usize,
    navigation_context_allocator: NavigationContextIdAllocator,
    navigation_contexts: HashMap<NavigationContextId, NavigationContext>,
    navigation_context_capabilities: HashMap<CapabilityId, NavigationContextId>,
    network_operation_allocator: NetworkOperationIdAllocator,
    network_operations: HashMap<NetworkOperationId, NetworkOperation>,
    sites: HashMap<SiteProcessId, SiteInstance>,
}

impl HostControlPlane {
    pub fn try_new(limits: HostLimits) -> Result<Self, HostControlError> {
        if !limits.is_valid() {
            return Err(HostControlError::new(
                HostControlErrorKind::InvalidLimits,
                "Host limits must contain non-zero process/capability/network-operation/navigation-context limits and valid IPC limits",
            ));
        }

        Ok(Self {
            topology: ProcessTopology::try_new(limits.max_site_processes)?,
            broker: CapabilityBroker::try_new(limits.max_capabilities)?,
            ipc_limits: limits.ipc,
            max_network_operations: limits.max_network_operations,
            max_navigation_contexts: limits.max_navigation_contexts,
            navigation_context_allocator: NavigationContextIdAllocator::new(),
            navigation_contexts: HashMap::new(),
            navigation_context_capabilities: HashMap::new(),
            network_operation_allocator: NetworkOperationIdAllocator::new(),
            network_operations: HashMap::new(),
            sites: HashMap::new(),
        })
    }

    pub fn with_default_limits() -> Result<Self, HostControlError> {
        Self::try_new(HostLimits::default())
    }

    pub fn host_process(&self) -> rarog_process::HostProcessId {
        self.topology.host_process()
    }

    pub fn active_site_processes(&self) -> usize {
        self.sites.len()
    }

    pub fn active_capabilities(&self) -> usize {
        self.broker.active_capabilities()
    }

    pub fn active_network_operations(&self) -> usize {
        self.network_operations.len()
    }

    pub fn active_navigation_contexts(&self) -> usize {
        self.navigation_contexts.len()
    }

    pub fn ensure_site(&mut self, site: SiteIdentity) -> Result<SiteLease, HostControlError> {
        let assignment = self.topology.assign_site(site.clone())?;
        let process = assignment.process();

        match assignment.kind() {
            SiteAssignmentKind::Existing => {
                let instance = self.sites.get(&process).ok_or_else(|| {
                    HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        format!("topology returned {process} without a live Host site instance"),
                    )
                })?;
                if instance.site != site {
                    return Err(HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        format!("{process} is bound to a different site identity"),
                    ));
                }
            }
            SiteAssignmentKind::Created => {
                let channel = match IpcChannel::try_new(self.ipc_limits) {
                    Ok(channel) => channel,
                    Err(error) => {
                        let _ = self.topology.retire_site_process(process);
                        return Err(error.into());
                    }
                };
                if self
                    .sites
                    .insert(process, SiteInstance { site, channel })
                    .is_some()
                {
                    let _ = self.topology.retire_site_process(process);
                    return Err(HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        format!("new process identity {process} already had a live site instance"),
                    ));
                }
            }
        }

        Ok(SiteLease {
            process,
            kind: assignment.kind(),
        })
    }

    pub fn process_for_site(&self, site: &SiteIdentity) -> Option<SiteProcessId> {
        self.topology.process_for_site(site)
    }

    pub fn site_for_process(&self, process: SiteProcessId) -> Option<&SiteIdentity> {
        self.sites.get(&process).map(|instance| &instance.site)
    }

    pub fn enqueue_from_host(
        &mut self,
        process: SiteProcessId,
        envelope: IpcEnvelope,
    ) -> Result<(), HostControlError> {
        validate_direction(&envelope, EndpointRole::Host)?;
        self.site_mut(process)?.channel.send(envelope)?;
        Ok(())
    }

    pub fn enqueue_from_site(
        &mut self,
        process: SiteProcessId,
        envelope: IpcEnvelope,
    ) -> Result<(), HostControlError> {
        validate_direction(&envelope, EndpointRole::Site)?;
        self.site_mut(process)?.channel.send(envelope)?;
        Ok(())
    }

    pub fn take_for_site(
        &mut self,
        process: SiteProcessId,
    ) -> Result<Option<IpcEnvelope>, HostControlError> {
        Ok(self
            .site_mut(process)?
            .channel
            .receive(EndpointRole::Site)?)
    }

    pub fn take_for_host(
        &mut self,
        process: SiteProcessId,
    ) -> Result<Option<IpcEnvelope>, HostControlError> {
        Ok(self
            .site_mut(process)?
            .channel
            .receive(EndpointRole::Host)?)
    }

    pub fn queued_for_site(&self, process: SiteProcessId) -> Result<usize, HostControlError> {
        Ok(self
            .site(process)?
            .channel
            .queued_messages_for(EndpointRole::Site))
    }

    pub fn queued_for_host(&self, process: SiteProcessId) -> Result<usize, HostControlError> {
        Ok(self
            .site(process)?
            .channel
            .queued_messages_for(EndpointRole::Host))
    }

    pub fn grant_capability(
        &mut self,
        process: SiteProcessId,
        class: CapabilityClass,
    ) -> Result<CapabilityGrant, HostControlError> {
        self.site(process)?;
        Ok(self.broker.grant(process, class)?)
    }

    pub fn authorize_capability(
        &self,
        process: SiteProcessId,
        id: CapabilityId,
        class: CapabilityClass,
    ) -> Result<(), HostControlError> {
        self.site(process)?;
        Ok(self.broker.authorize(id, process, class)?)
    }

    pub fn revoke_capability(
        &mut self,
        id: CapabilityId,
    ) -> Result<CapabilityGrant, HostControlError> {
        let grant = self.broker.revoke(id)?;
        self.navigation_context_capabilities.remove(&id);
        self.network_operations
            .retain(|_, operation| operation.capability != id);
        Ok(grant)
    }

    pub fn process_lost(&mut self, process: SiteProcessId) -> Result<SiteLoss, HostControlError> {
        let mut instance = self
            .sites
            .remove(&process)
            .ok_or_else(|| HostControlError::unknown_process(process))?;

        instance.channel.disconnect();
        let invalidated_contexts = self
            .navigation_contexts
            .iter()
            .filter_map(|(context, state)| (state.binding.process == process).then_some(*context))
            .collect::<Vec<_>>();
        for context in &invalidated_contexts {
            self.navigation_contexts.remove(context);
        }
        self.navigation_context_capabilities
            .retain(|_, context| !invalidated_contexts.contains(context));

        let network_operations_before = self.network_operations.len();
        self.network_operations
            .retain(|_, operation| operation.owner != process);
        let revoked_network_operations =
            network_operations_before.saturating_sub(self.network_operations.len());
        let revoked_capabilities = self.broker.revoke_all_for_process(process);
        let topology_site = self.topology.retire_site_process(process)?;
        if topology_site != instance.site {
            return Err(HostControlError::new(
                HostControlErrorKind::InconsistentState,
                format!("{process} topology/site instance identities diverged"),
            ));
        }

        Ok(SiteLoss {
            process,
            site: instance.site,
            revoked_capabilities,
            revoked_network_operations,
            invalidated_navigation_contexts: invalidated_contexts.len(),
        })
    }

    pub fn recover_site(&mut self, site: SiteIdentity) -> Result<SiteLease, HostControlError> {
        self.ensure_site(site)
    }

    pub fn start_network_operation(
        &mut self,
        process: SiteProcessId,
        capability: CapabilityId,
        request: NetworkRequest,
        network: &mut dyn NetworkCapability,
    ) -> Result<NetworkOperationId, HostControlError> {
        self.authorize_capability(process, capability, CapabilityClass::Network)?;
        if self.network_operations.len() >= self.max_network_operations {
            return Err(HostControlError::new(
                HostControlErrorKind::NetworkOperationLimitExceeded,
                format!(
                    "network operation limit {} reached",
                    self.max_network_operations
                ),
            ));
        }

        let operation = self.network_operation_allocator.allocate()?;
        let ticket = network.start(request)?;

        if let Some(existing) = self
            .network_operations
            .iter()
            .find_map(|(id, active)| (active.ticket == ticket).then_some(*id))
        {
            self.network_operations.remove(&existing);
            return Err(HostControlError::new(
                HostControlErrorKind::InconsistentState,
                "network backend reused a live ticket; affected Host operation authority was revoked",
            ));
        }

        if self
            .network_operations
            .insert(
                operation,
                NetworkOperation {
                    owner: process,
                    capability,
                    ticket,
                },
            )
            .is_some()
        {
            return Err(HostControlError::new(
                HostControlErrorKind::InconsistentState,
                "network operation allocator reused a live identity",
            ));
        }

        Ok(operation)
    }

    pub fn poll_network_operation(
        &mut self,
        process: SiteProcessId,
        capability: CapabilityId,
        operation: NetworkOperationId,
        network: &mut dyn NetworkCapability,
    ) -> Result<NetworkPoll, HostControlError> {
        self.authorize_capability(process, capability, CapabilityClass::Network)?;
        let ticket = self.network_ticket(process, capability, operation)?;

        let poll = match network.poll(ticket) {
            Ok(poll) => poll,
            Err(error) => {
                self.network_operations.remove(&operation);
                return Err(error.into());
            }
        };
        if matches!(&poll, NetworkPoll::Complete(_)) {
            self.network_operations.remove(&operation);
        }
        Ok(poll)
    }

    pub fn cancel_network_operation(
        &mut self,
        process: SiteProcessId,
        capability: CapabilityId,
        operation: NetworkOperationId,
        network: &mut dyn NetworkCapability,
    ) -> Result<(), HostControlError> {
        self.authorize_capability(process, capability, CapabilityClass::Network)?;
        let ticket = self.network_ticket(process, capability, operation)?;
        self.network_operations.remove(&operation);
        network.cancel(ticket)?;
        Ok(())
    }

    pub fn read_clipboard_text(
        &self,
        process: SiteProcessId,
        capability: CapabilityId,
        clipboard: &dyn PlatformClipboardService,
    ) -> Result<Option<ClipboardText>, HostControlError> {
        self.authorize_capability(process, capability, CapabilityClass::Clipboard)?;
        let limits = clipboard.limits();
        if !limits.is_valid() {
            return Err(ClipboardError::InvalidLimits.into());
        }
        clipboard
            .read_text()?
            .map(|text| {
                ClipboardText::try_new(text.as_str(), limits).map_err(HostControlError::from)
            })
            .transpose()
    }

    pub fn write_clipboard_text(
        &self,
        process: SiteProcessId,
        capability: CapabilityId,
        text: &ClipboardText,
        clipboard: &dyn PlatformClipboardService,
    ) -> Result<(), HostControlError> {
        self.authorize_capability(process, capability, CapabilityClass::Clipboard)?;
        let bounded = ClipboardText::try_new(text.as_str(), clipboard.limits())?;
        clipboard.write_text(&bounded)?;
        Ok(())
    }

    pub fn open_navigation_context(
        &mut self,
        target: &rarog_url::WebUrl,
    ) -> Result<NavigationContextSnapshot, HostControlError> {
        let site = target.site_identity()?;
        self.open_navigation_context_to_site(site)
    }

    pub fn open_navigation_context_to_site(
        &mut self,
        site: SiteIdentity,
    ) -> Result<NavigationContextSnapshot, HostControlError> {
        if self.navigation_contexts.len() >= self.max_navigation_contexts {
            return Err(HostControlError::new(
                HostControlErrorKind::NavigationContextLimitExceeded,
                format!(
                    "navigation context limit {} reached",
                    self.max_navigation_contexts
                ),
            ));
        }

        let context = self.navigation_context_allocator.allocate()?;
        let binding = self
            .begin_document_navigation_to_site(None, site.clone())?
            .into_current();
        self.navigation_contexts
            .insert(context, NavigationContext { binding });
        Ok(NavigationContextSnapshot { context, site })
    }

    pub fn navigation_context(
        &self,
        context: NavigationContextId,
    ) -> Result<NavigationContextSnapshot, HostControlError> {
        let state = self.require_navigation_context(context)?;
        Ok(NavigationContextSnapshot {
            context,
            site: state.binding.site.clone(),
        })
    }

    pub fn navigate_navigation_context(
        &mut self,
        context: NavigationContextId,
        target: &rarog_url::WebUrl,
    ) -> Result<NavigationContextTransition, HostControlError> {
        let target_site = target.site_identity()?;
        self.navigate_navigation_context_to_site(context, target_site)
    }

    pub fn navigate_navigation_context_to_site(
        &mut self,
        context: NavigationContextId,
        target_site: SiteIdentity,
    ) -> Result<NavigationContextTransition, HostControlError> {
        let current = self.require_navigation_context(context)?.binding.clone();
        let previous_site = current.site.clone();
        if previous_site == target_site {
            let transition =
                self.begin_document_navigation_to_site(Some(&current), target_site.clone())?;
            self.require_navigation_context_mut(context)?.binding = transition.current().clone();
            return Ok(NavigationContextTransition {
                context,
                previous_site,
                current_site: target_site,
                kind: transition.kind(),
                source_retired: false,
            });
        }

        match self.begin_document_navigation_to_site(Some(&current), target_site.clone()) {
            Ok(transition) => {
                let previous_process = current.process;
                self.revoke_navigation_context_capabilities(context)?;
                self.require_navigation_context_mut(context)?.binding =
                    transition.current().clone();
                let source_retired = if self.navigation_context_ref_count(previous_process) == 0 {
                    self.process_lost(previous_process)?;
                    true
                } else {
                    false
                };
                Ok(NavigationContextTransition {
                    context,
                    previous_site,
                    current_site: target_site,
                    kind: transition.kind(),
                    source_retired,
                })
            }
            Err(error)
                if matches!(
                    error.kind,
                    HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
                ) && self.navigation_context_ref_count(current.process) == 1 =>
            {
                self.revoke_navigation_context_capabilities(context)?;
                self.navigation_contexts.remove(&context);
                self.process_lost(current.process)?;

                match self.begin_document_navigation_to_site(None, target_site.clone()) {
                    Ok(transition) => {
                        self.navigation_contexts.insert(
                            context,
                            NavigationContext {
                                binding: transition.current().clone(),
                            },
                        );
                        Ok(NavigationContextTransition {
                            context,
                            previous_site,
                            current_site: target_site,
                            kind: NavigationTransitionKind::CrossSiteReplacement,
                            source_retired: true,
                        })
                    }
                    Err(target_error) => Err(HostControlError::new(
                        HostControlErrorKind::NavigationContextInvalidated,
                        format!(
                            "{context} source authority was retired for bounded cross-site replacement, but target assignment failed: {target_error}"
                        ),
                    )),
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn close_navigation_context(
        &mut self,
        context: NavigationContextId,
    ) -> Result<NavigationContextClose, HostControlError> {
        let state = self
            .navigation_contexts
            .remove(&context)
            .ok_or_else(|| Self::unknown_navigation_context(context))?;
        let revoked_capabilities = self.revoke_navigation_context_capabilities(context)?;
        let source_retired = if self.navigation_context_ref_count(state.binding.process) == 0 {
            self.process_lost(state.binding.process)?;
            true
        } else {
            false
        };
        Ok(NavigationContextClose {
            context,
            site: state.binding.site,
            source_retired,
            revoked_capabilities,
        })
    }

    pub fn grant_navigation_context_network_capability(
        &mut self,
        context: NavigationContextId,
    ) -> Result<NavigationContextCapability, HostControlError> {
        self.grant_navigation_context_capability(context, CapabilityClass::Network)
    }

    pub fn grant_navigation_context_capability(
        &mut self,
        context: NavigationContextId,
        class: CapabilityClass,
    ) -> Result<NavigationContextCapability, HostControlError> {
        let process = self.require_navigation_context(context)?.binding.process;
        let grant = self.grant_capability(process, class)?;
        let id = grant.id();
        if self
            .navigation_context_capabilities
            .insert(id, context)
            .is_some()
        {
            let _ = self.revoke_capability(id);
            return Err(HostControlError::new(
                HostControlErrorKind::InconsistentState,
                "new capability identity already had navigation-context ownership",
            ));
        }
        Ok(NavigationContextCapability { context, id, class })
    }

    pub fn revoke_navigation_context_capability(
        &mut self,
        capability: NavigationContextCapability,
    ) -> Result<(), HostControlError> {
        self.authorize_navigation_context_capability(capability)?;
        self.revoke_capability(capability.id)?;
        Ok(())
    }

    pub fn start_navigation_context_network_operation(
        &mut self,
        capability: NavigationContextCapability,
        request: NetworkRequest,
        network: &mut dyn NetworkCapability,
    ) -> Result<NetworkOperationId, HostControlError> {
        let process = self
            .authorize_navigation_context_capability_class(capability, CapabilityClass::Network)?;
        self.start_network_operation(process, capability.id, request, network)
    }

    pub fn poll_navigation_context_network_operation(
        &mut self,
        capability: NavigationContextCapability,
        operation: NetworkOperationId,
        network: &mut dyn NetworkCapability,
    ) -> Result<NetworkPoll, HostControlError> {
        let process = self
            .authorize_navigation_context_capability_class(capability, CapabilityClass::Network)?;
        self.poll_network_operation(process, capability.id, operation, network)
    }

    pub fn cancel_navigation_context_network_operation(
        &mut self,
        capability: NavigationContextCapability,
        operation: NetworkOperationId,
        network: &mut dyn NetworkCapability,
    ) -> Result<(), HostControlError> {
        let process = self
            .authorize_navigation_context_capability_class(capability, CapabilityClass::Network)?;
        self.cancel_network_operation(process, capability.id, operation, network)
    }

    pub fn read_navigation_context_clipboard_text(
        &self,
        capability: NavigationContextCapability,
        clipboard: &dyn PlatformClipboardService,
    ) -> Result<Option<ClipboardText>, HostControlError> {
        let process = self.authorize_navigation_context_capability_class(
            capability,
            CapabilityClass::Clipboard,
        )?;
        self.read_clipboard_text(process, capability.id, clipboard)
    }

    pub fn write_navigation_context_clipboard_text(
        &self,
        capability: NavigationContextCapability,
        text: &ClipboardText,
        clipboard: &dyn PlatformClipboardService,
    ) -> Result<(), HostControlError> {
        let process = self.authorize_navigation_context_capability_class(
            capability,
            CapabilityClass::Clipboard,
        )?;
        self.write_clipboard_text(process, capability.id, text, clipboard)
    }

    pub fn begin_document_navigation(
        &mut self,
        current: Option<&DocumentSiteBinding>,
        target: &rarog_url::WebUrl,
    ) -> Result<NavigationTransition, HostControlError> {
        self.validate_current_document(current)?;
        let target_site = target.site_identity()?;
        self.begin_document_navigation_validated(current, target_site)
    }

    pub fn begin_document_navigation_to_site(
        &mut self,
        current: Option<&DocumentSiteBinding>,
        target_site: SiteIdentity,
    ) -> Result<NavigationTransition, HostControlError> {
        self.validate_current_document(current)?;
        self.begin_document_navigation_validated(current, target_site)
    }

    fn begin_document_navigation_validated(
        &mut self,
        current: Option<&DocumentSiteBinding>,
        target_site: SiteIdentity,
    ) -> Result<NavigationTransition, HostControlError> {
        let lease = self.ensure_site(target_site.clone())?;
        let next = DocumentSiteBinding {
            site: target_site,
            process: lease.process(),
        };

        let kind = match current {
            None => NavigationTransitionKind::Initial,
            Some(previous) if previous.site == next.site => {
                if previous.process != next.process {
                    return Err(HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        "same-site navigation resolved to a different Site-process identity",
                    ));
                }
                NavigationTransitionKind::SameSiteReuse
            }
            Some(previous) => {
                if previous.process == next.process {
                    return Err(HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        "cross-site navigation resolved to the previous Site-process identity",
                    ));
                }
                NavigationTransitionKind::CrossSiteReplacement
            }
        };

        Ok(NavigationTransition {
            previous: current.cloned(),
            current: next,
            kind,
        })
    }

    fn validate_current_document(
        &self,
        current: Option<&DocumentSiteBinding>,
    ) -> Result<(), HostControlError> {
        let Some(binding) = current else {
            return Ok(());
        };
        let instance = self.site(binding.process)?;
        if instance.site != binding.site {
            return Err(HostControlError::new(
                HostControlErrorKind::InvalidDocumentBinding,
                format!(
                    "document binding for {} does not match Site-process {}",
                    binding.site, binding.process
                ),
            ));
        }
        Ok(())
    }

    fn unknown_navigation_context(context: NavigationContextId) -> HostControlError {
        HostControlError::new(
            HostControlErrorKind::UnknownNavigationContext,
            format!("unknown or invalidated navigation context {context}"),
        )
    }

    fn require_navigation_context(
        &self,
        context: NavigationContextId,
    ) -> Result<&NavigationContext, HostControlError> {
        self.navigation_contexts
            .get(&context)
            .ok_or_else(|| Self::unknown_navigation_context(context))
    }

    fn require_navigation_context_mut(
        &mut self,
        context: NavigationContextId,
    ) -> Result<&mut NavigationContext, HostControlError> {
        self.navigation_contexts
            .get_mut(&context)
            .ok_or_else(|| Self::unknown_navigation_context(context))
    }

    fn navigation_context_ref_count(&self, process: SiteProcessId) -> usize {
        self.navigation_contexts
            .values()
            .filter(|context| context.binding.process == process)
            .count()
    }

    fn revoke_navigation_context_capabilities(
        &mut self,
        context: NavigationContextId,
    ) -> Result<usize, HostControlError> {
        let ids = self
            .navigation_context_capabilities
            .iter()
            .filter_map(|(id, owner)| (*owner == context).then_some(*id))
            .collect::<Vec<_>>();
        for id in &ids {
            self.revoke_capability(*id)?;
        }
        Ok(ids.len())
    }

    fn authorize_navigation_context_capability(
        &self,
        capability: NavigationContextCapability,
    ) -> Result<SiteProcessId, HostControlError> {
        self.authorize_navigation_context_capability_class(capability, capability.class)
    }

    fn authorize_navigation_context_capability_class(
        &self,
        capability: NavigationContextCapability,
        class: CapabilityClass,
    ) -> Result<SiteProcessId, HostControlError> {
        if capability.class != class
            || self
                .navigation_context_capabilities
                .get(&capability.id)
                .copied()
                != Some(capability.context)
        {
            return Err(HostControlError::new(
                HostControlErrorKind::InvalidNavigationContextCapabilityAuthority,
                "navigation-context capability class or context ownership does not match",
            ));
        }
        let process = self
            .require_navigation_context(capability.context)?
            .binding
            .process;
        self.authorize_capability(process, capability.id, class)?;
        Ok(process)
    }

    fn network_ticket(
        &self,
        process: SiteProcessId,
        capability: CapabilityId,
        operation: NetworkOperationId,
    ) -> Result<NetworkTicket, HostControlError> {
        self.network_operations
            .get(&operation)
            .filter(|active| active.owner == process && active.capability == capability)
            .map(|active| active.ticket)
            .ok_or_else(|| {
                HostControlError::new(
                    HostControlErrorKind::InvalidNetworkOperationAuthority,
                    "unknown network operation or operation authority does not match",
                )
            })
    }

    fn site(&self, process: SiteProcessId) -> Result<&SiteInstance, HostControlError> {
        self.sites
            .get(&process)
            .ok_or_else(|| HostControlError::unknown_process(process))
    }

    fn site_mut(&mut self, process: SiteProcessId) -> Result<&mut SiteInstance, HostControlError> {
        self.sites
            .get_mut(&process)
            .ok_or_else(|| HostControlError::unknown_process(process))
    }
}

fn validate_direction(
    envelope: &IpcEnvelope,
    expected_source: EndpointRole,
) -> Result<(), HostControlError> {
    if envelope.source() == expected_source && envelope.destination() == expected_source.peer() {
        Ok(())
    } else {
        Err(HostControlError::new(
            HostControlErrorKind::InvalidEnvelopeDirection,
            format!(
                "Host control plane expected {expected_source:?} -> {:?} IPC envelope",
                expected_source.peer()
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_fetch::{FetchRequest, FetchResponse, HeaderList};
    use rarog_ipc::{IpcEnvelope, RequestId};
    use rarog_platform::ClipboardLimits;
    use rarog_url::WebUrl;
    use std::sync::Mutex;

    fn site(url: &str) -> SiteIdentity {
        WebUrl::parse(url).unwrap().site_identity().unwrap()
    }

    fn test_limits(max_sites: usize, max_capabilities: usize) -> HostLimits {
        HostLimits {
            max_site_processes: max_sites,
            ipc: IpcLimits {
                max_message_bytes: 32,
                max_queued_messages: 4,
                max_queued_bytes: 64,
            },
            max_capabilities,
            max_network_operations: 4,
            max_navigation_contexts: 4,
        }
    }

    fn network_request(url: &str) -> NetworkRequest {
        let url = WebUrl::parse(url).unwrap();
        let origin = WebUrl::parse("https://app.example.com/")
            .unwrap()
            .origin()
            .unwrap();
        FetchRequest::new(url, origin).network_request()
    }

    struct FixtureNetwork {
        next_ticket: u64,
        starts: usize,
        polls: usize,
        cancels: usize,
    }

    impl Default for FixtureNetwork {
        fn default() -> Self {
            Self {
                next_ticket: 1,
                starts: 0,
                polls: 0,
                cancels: 0,
            }
        }
    }

    impl NetworkCapability for FixtureNetwork {
        fn start(&mut self, _request: NetworkRequest) -> Result<NetworkTicket, FetchError> {
            self.starts += 1;
            let ticket = NetworkTicket::new(NonZeroU64::new(self.next_ticket).unwrap());
            self.next_ticket += 1;
            Ok(ticket)
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

    #[derive(Default)]
    struct ClipboardState {
        reads: usize,
        writes: usize,
        text: Option<ClipboardText>,
    }

    struct FixtureClipboard {
        limits: ClipboardLimits,
        state: Mutex<ClipboardState>,
    }

    impl FixtureClipboard {
        fn new(max_text_bytes: usize) -> Self {
            Self {
                limits: ClipboardLimits { max_text_bytes },
                state: Mutex::new(ClipboardState::default()),
            }
        }

        fn reads(&self) -> usize {
            self.state.lock().unwrap().reads
        }

        fn writes(&self) -> usize {
            self.state.lock().unwrap().writes
        }
    }

    impl PlatformClipboardService for FixtureClipboard {
        fn limits(&self) -> ClipboardLimits {
            self.limits
        }

        fn read_text(&self) -> Result<Option<ClipboardText>, ClipboardError> {
            let mut state = self.state.lock().unwrap();
            state.reads += 1;
            Ok(state.text.clone())
        }

        fn write_text(&self, text: &ClipboardText) -> Result<(), ClipboardError> {
            let mut state = self.state.lock().unwrap();
            state.writes += 1;
            state.text = Some(text.clone());
            Ok(())
        }
    }

    #[test]
    fn same_site_reuses_one_instance_and_cross_site_isolates() {
        let mut host = HostControlPlane::try_new(test_limits(2, 4)).unwrap();
        let first = host.ensure_site(site("https://a.example.com/")).unwrap();
        let same = host
            .ensure_site(site("https://b.example.com/path"))
            .unwrap();
        let other = host.ensure_site(site("https://example.org/")).unwrap();

        assert!(first.is_new());
        assert_eq!(same.kind(), SiteAssignmentKind::Existing);
        assert_eq!(first.process(), same.process());
        assert_ne!(first.process(), other.process());
        assert_eq!(host.active_site_processes(), 2);
    }

    #[test]
    fn process_budget_never_falls_back_to_cross_site_sharing() {
        let mut host = HostControlPlane::try_new(test_limits(1, 2)).unwrap();
        let first = host.ensure_site(site("https://example.com/")).unwrap();

        let error = host.ensure_site(site("https://example.org/")).unwrap_err();

        assert_eq!(
            error.kind,
            HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
        );
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(
            host.process_for_site(&site("https://example.com/")),
            Some(first.process())
        );
        assert_eq!(host.process_for_site(&site("https://example.org/")), None);
    }

    #[test]
    fn channel_binding_rejects_self_asserted_direction() {
        let limits = test_limits(1, 2);
        let mut host = HostControlPlane::try_new(limits).unwrap();
        let process = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let request = RequestId::try_new(1).unwrap();

        let site_envelope =
            IpcEnvelope::request(EndpointRole::Site, request, Vec::new(), limits.ipc).unwrap();
        let error = host.enqueue_from_host(process, site_envelope).unwrap_err();

        assert_eq!(error.kind, HostControlErrorKind::InvalidEnvelopeDirection);
        assert_eq!(host.queued_for_site(process).unwrap(), 0);
    }

    #[test]
    fn bound_channel_routes_host_and_site_messages() {
        let limits = test_limits(1, 2);
        let mut host = HostControlPlane::try_new(limits).unwrap();
        let process = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let request = RequestId::try_new(7).unwrap();

        host.enqueue_from_host(
            process,
            IpcEnvelope::request(EndpointRole::Host, request, b"request".to_vec(), limits.ipc)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(host.queued_for_site(process).unwrap(), 1);
        assert_eq!(
            host.take_for_site(process).unwrap().unwrap().payload(),
            b"request"
        );

        host.enqueue_from_site(
            process,
            IpcEnvelope::response(
                EndpointRole::Site,
                request,
                b"response".to_vec(),
                limits.ipc,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(host.queued_for_host(process).unwrap(), 1);
        assert_eq!(
            host.take_for_host(process).unwrap().unwrap().payload(),
            b"response"
        );
    }

    #[test]
    fn process_loss_revokes_capabilities_and_replacement_is_fresh() {
        let limits = test_limits(1, 4);
        let mut host = HostControlPlane::try_new(limits).unwrap();
        let site_identity = site("https://example.com/");
        let first = host.ensure_site(site_identity.clone()).unwrap().process();
        let capability = host
            .grant_capability(first, CapabilityClass::Network)
            .unwrap();

        host.enqueue_from_host(
            first,
            IpcEnvelope::event(EndpointRole::Host, b"stale".to_vec(), limits.ipc).unwrap(),
        )
        .unwrap();
        assert_eq!(host.queued_for_site(first).unwrap(), 1);

        let loss = host.process_lost(first).unwrap();

        assert_eq!(loss.process(), first);
        assert_eq!(loss.site(), &site_identity);
        assert_eq!(loss.revoked_capabilities(), 1);
        assert_eq!(host.active_site_processes(), 0);
        assert_eq!(host.active_capabilities(), 0);
        assert_eq!(
            host.authorize_capability(first, capability.id(), CapabilityClass::Network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::UnknownSiteProcess
        );

        let replacement = host.recover_site(site_identity).unwrap().process();
        assert_ne!(first, replacement);
        assert_eq!(host.queued_for_site(replacement).unwrap(), 0);
        assert_eq!(
            host.authorize_capability(replacement, capability.id(), CapabilityClass::Network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::Capability(CapabilityErrorKind::UnknownCapability)
        );
    }

    #[test]
    fn stale_process_cannot_receive_new_authority() {
        let mut host = HostControlPlane::try_new(test_limits(1, 2)).unwrap();
        let process = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        host.process_lost(process).unwrap();

        let error = host
            .grant_capability(process, CapabilityClass::Clipboard)
            .unwrap_err();

        assert_eq!(error.kind, HostControlErrorKind::UnknownSiteProcess);
    }

    #[test]
    fn network_operations_are_host_owned_and_backend_tickets_are_not_authority() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let first = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let second = host
            .ensure_site(site("https://example.org/"))
            .unwrap()
            .process();
        let first_capability = host
            .grant_capability(first, CapabilityClass::Network)
            .unwrap();
        let second_capability = host
            .grant_capability(second, CapabilityClass::Network)
            .unwrap();
        let mut network = FixtureNetwork::default();

        let operation = host
            .start_network_operation(
                first,
                first_capability.id(),
                network_request("https://api.example.com/data"),
                &mut network,
            )
            .unwrap();

        assert_eq!(network.starts, 1);
        assert_eq!(host.active_network_operations(), 1);

        let guessed = NetworkOperationId::try_new(operation.get() + 1).unwrap();
        assert_eq!(
            host.poll_network_operation(first, first_capability.id(), guessed, &mut network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNetworkOperationAuthority
        );
        assert_eq!(network.polls, 0);

        assert_eq!(
            host.poll_network_operation(second, second_capability.id(), operation, &mut network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNetworkOperationAuthority
        );
        assert_eq!(network.polls, 0);

        assert!(matches!(
            host.poll_network_operation(first, first_capability.id(), operation, &mut network)
                .unwrap(),
            NetworkPoll::Complete(_)
        ));
        assert_eq!(network.polls, 1);
        assert_eq!(host.active_network_operations(), 0);

        assert_eq!(
            host.poll_network_operation(first, first_capability.id(), operation, &mut network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNetworkOperationAuthority
        );
        assert_eq!(network.polls, 1);
    }

    #[test]
    fn network_operation_capacity_revocation_and_process_loss_fail_closed() {
        let mut limits = test_limits(1, 8);
        limits.max_network_operations = 1;
        let mut host = HostControlPlane::try_new(limits).unwrap();
        let process = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let first_capability = host
            .grant_capability(process, CapabilityClass::Network)
            .unwrap();
        let mut network = FixtureNetwork::default();

        let first_operation = host
            .start_network_operation(
                process,
                first_capability.id(),
                network_request("https://api.example.com/one"),
                &mut network,
            )
            .unwrap();

        assert_eq!(
            host.start_network_operation(
                process,
                first_capability.id(),
                network_request("https://api.example.com/two"),
                &mut network,
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::NetworkOperationLimitExceeded
        );
        assert_eq!(network.starts, 1);

        host.revoke_capability(first_capability.id()).unwrap();
        assert_eq!(host.active_network_operations(), 0);
        assert_eq!(
            host.poll_network_operation(
                process,
                first_capability.id(),
                first_operation,
                &mut network
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::Capability(CapabilityErrorKind::UnknownCapability)
        );
        assert_eq!(network.polls, 0);

        let replacement_capability = host
            .grant_capability(process, CapabilityClass::Network)
            .unwrap();
        let replacement_operation = host
            .start_network_operation(
                process,
                replacement_capability.id(),
                network_request("https://api.example.com/three"),
                &mut network,
            )
            .unwrap();

        let loss = host.process_lost(process).unwrap();
        assert_eq!(loss.revoked_network_operations(), 1);
        assert_eq!(host.active_network_operations(), 0);
        assert_eq!(
            host.poll_network_operation(
                process,
                replacement_capability.id(),
                replacement_operation,
                &mut network
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::UnknownSiteProcess
        );
        assert_eq!(network.polls, 0);
    }

    #[test]
    fn network_cancel_revokes_host_operation_before_backend_result() {
        let mut host = HostControlPlane::try_new(test_limits(1, 4)).unwrap();
        let process = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let capability = host
            .grant_capability(process, CapabilityClass::Network)
            .unwrap();
        let mut network = FixtureNetwork::default();
        let operation = host
            .start_network_operation(
                process,
                capability.id(),
                network_request("https://api.example.com/"),
                &mut network,
            )
            .unwrap();

        host.cancel_network_operation(process, capability.id(), operation, &mut network)
            .unwrap();

        assert_eq!(network.cancels, 1);
        assert_eq!(host.active_network_operations(), 0);
        assert_eq!(
            host.cancel_network_operation(process, capability.id(), operation, &mut network)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNetworkOperationAuthority
        );
        assert_eq!(network.cancels, 1);
    }

    #[test]
    fn clipboard_route_authorizes_before_backend_and_revalidates_limits() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let first = host
            .ensure_site(site("https://example.com/"))
            .unwrap()
            .process();
        let second = host
            .ensure_site(site("https://example.org/"))
            .unwrap()
            .process();
        let clipboard_capability = host
            .grant_capability(first, CapabilityClass::Clipboard)
            .unwrap();
        let network_capability = host
            .grant_capability(first, CapabilityClass::Network)
            .unwrap();
        let second_clipboard = host
            .grant_capability(second, CapabilityClass::Clipboard)
            .unwrap();
        let clipboard = FixtureClipboard::new(8);
        let text = ClipboardText::try_new("Rarog", ClipboardLimits { max_text_bytes: 32 }).unwrap();

        host.write_clipboard_text(first, clipboard_capability.id(), &text, &clipboard)
            .unwrap();
        assert_eq!(clipboard.writes(), 1);

        assert_eq!(
            host.write_clipboard_text(first, network_capability.id(), &text, &clipboard)
                .unwrap_err()
                .kind,
            HostControlErrorKind::Capability(CapabilityErrorKind::WrongClass)
        );
        assert_eq!(clipboard.writes(), 1);

        assert_eq!(
            host.read_clipboard_text(second, clipboard_capability.id(), &clipboard)
                .unwrap_err()
                .kind,
            HostControlErrorKind::Capability(CapabilityErrorKind::WrongOwner)
        );
        assert_eq!(clipboard.reads(), 0);

        let oversized =
            ClipboardText::try_new("0123456789", ClipboardLimits { max_text_bytes: 32 }).unwrap();
        assert_eq!(
            host.write_clipboard_text(first, clipboard_capability.id(), &oversized, &clipboard)
                .unwrap_err()
                .kind,
            HostControlErrorKind::Clipboard(ClipboardError::TextLimitExceeded {
                bytes: 10,
                limit: 8
            })
        );
        assert_eq!(clipboard.writes(), 1);

        assert_eq!(
            host.read_clipboard_text(second, second_clipboard.id(), &clipboard)
                .unwrap()
                .unwrap()
                .as_str(),
            "Rarog"
        );
        assert_eq!(clipboard.reads(), 1);

        host.revoke_capability(second_clipboard.id()).unwrap();
        assert_eq!(
            host.read_clipboard_text(second, second_clipboard.id(), &clipboard)
                .unwrap_err()
                .kind,
            HostControlErrorKind::Capability(CapabilityErrorKind::UnknownCapability)
        );
        assert_eq!(clipboard.reads(), 1);
    }

    #[test]
    fn initial_and_same_site_navigation_reuse_host_assignment() {
        let mut host = HostControlPlane::try_new(test_limits(2, 2)).unwrap();
        let first_url = WebUrl::parse("https://a.example.com/start").unwrap();
        let first = host.begin_document_navigation(None, &first_url).unwrap();

        assert_eq!(first.kind(), NavigationTransitionKind::Initial);
        assert!(first.previous().is_none());

        let next_url = WebUrl::parse("https://b.example.com/next").unwrap();
        let next = host
            .begin_document_navigation(Some(first.current()), &next_url)
            .unwrap();

        assert_eq!(next.kind(), NavigationTransitionKind::SameSiteReuse);
        assert_eq!(next.current().process(), first.current().process());
        assert_eq!(next.current().site(), first.current().site());
        assert_eq!(host.active_site_processes(), 1);
    }

    #[test]
    fn cross_site_and_cross_scheme_navigation_replace_document_process() {
        let mut host = HostControlPlane::try_new(test_limits(3, 2)).unwrap();
        let first_url = WebUrl::parse("https://example.com/").unwrap();
        let first = host
            .begin_document_navigation(None, &first_url)
            .unwrap()
            .into_current();

        let cross_site_url = WebUrl::parse("https://example.org/").unwrap();
        let cross_site = host
            .begin_document_navigation(Some(&first), &cross_site_url)
            .unwrap()
            .into_current();

        assert_ne!(cross_site.process(), first.process());

        let cross_scheme_url = WebUrl::parse("http://example.org/").unwrap();
        let cross_scheme = host
            .begin_document_navigation(Some(&cross_site), &cross_scheme_url)
            .unwrap();

        assert_eq!(
            cross_scheme.kind(),
            NavigationTransitionKind::CrossSiteReplacement
        );
        assert_ne!(cross_scheme.current().process(), cross_site.process());
        assert_eq!(host.active_site_processes(), 3);
    }

    #[test]
    fn opaque_site_identity_is_explicitly_propagated_or_fresh_per_navigation() {
        let mut host = HostControlPlane::try_new(test_limits(2, 2)).unwrap();
        let opaque_url = WebUrl::parse("data:text/html,rarog").unwrap();
        let first = host
            .begin_document_navigation(None, &opaque_url)
            .unwrap()
            .into_current();

        assert!(first.site().is_opaque());

        let inherited = host
            .begin_document_navigation_to_site(Some(&first), first.site().clone())
            .unwrap();

        assert_eq!(inherited.kind(), NavigationTransitionKind::SameSiteReuse);
        assert_eq!(inherited.current(), &first);

        let fresh = host
            .begin_document_navigation(Some(&first), &opaque_url)
            .unwrap();

        assert_eq!(fresh.kind(), NavigationTransitionKind::CrossSiteReplacement);
        assert!(fresh.current().site().is_opaque());
        assert_ne!(fresh.current().site(), first.site());
        assert_ne!(fresh.current().process(), first.process());
    }

    #[test]
    fn stale_document_binding_is_rejected_before_target_assignment() {
        let mut host = HostControlPlane::try_new(test_limits(1, 2)).unwrap();
        let first_url = WebUrl::parse("https://example.com/").unwrap();
        let first = host
            .begin_document_navigation(None, &first_url)
            .unwrap()
            .into_current();
        host.process_lost(first.process()).unwrap();

        let target = WebUrl::parse("https://example.org/").unwrap();
        let error = host
            .begin_document_navigation(Some(&first), &target)
            .unwrap_err();

        assert_eq!(error.kind, HostControlErrorKind::UnknownSiteProcess);
        assert_eq!(host.active_site_processes(), 0);
        assert!(
            host.process_for_site(&target.site_identity().unwrap())
                .is_none()
        );
    }

    #[test]
    fn navigation_budget_failure_preserves_current_document_binding() {
        let mut host = HostControlPlane::try_new(test_limits(1, 2)).unwrap();
        let first_url = WebUrl::parse("https://example.com/").unwrap();
        let first = host
            .begin_document_navigation(None, &first_url)
            .unwrap()
            .into_current();

        let target = WebUrl::parse("https://example.org/").unwrap();
        let error = host
            .begin_document_navigation(Some(&first), &target)
            .unwrap_err();

        assert_eq!(
            error.kind,
            HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
        );
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(host.site_for_process(first.process()), Some(first.site()));
        assert!(
            host.process_for_site(&target.site_identity().unwrap())
                .is_none()
        );
    }

    #[test]
    fn navigation_contexts_share_same_site_process_until_last_close() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let first = host
            .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
            .unwrap();
        let second = host
            .open_navigation_context(&WebUrl::parse("https://b.example.com/path").unwrap())
            .unwrap();

        let first_process = host
            .require_navigation_context(first.context())
            .unwrap()
            .binding
            .process;
        let second_process = host
            .require_navigation_context(second.context())
            .unwrap()
            .binding
            .process;
        assert_eq!(first_process, second_process);
        assert_eq!(host.active_navigation_contexts(), 2);
        assert_eq!(host.active_site_processes(), 1);

        let first_capability = host
            .grant_navigation_context_capability(first.context(), CapabilityClass::Network)
            .unwrap();
        let second_capability = host
            .grant_navigation_context_capability(second.context(), CapabilityClass::Network)
            .unwrap();

        let closed = host.close_navigation_context(first.context()).unwrap();
        assert!(!closed.source_retired());
        assert_eq!(closed.revoked_capabilities(), 1);
        assert_eq!(host.active_navigation_contexts(), 1);
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(
            host.authorize_navigation_context_capability(first_capability)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
        );
        host.authorize_navigation_context_capability(second_capability)
            .unwrap();

        let closed = host.close_navigation_context(second.context()).unwrap();
        assert!(closed.source_retired());
        assert_eq!(host.active_navigation_contexts(), 0);
        assert_eq!(host.active_site_processes(), 0);
    }

    #[test]
    fn network_grant_helper_preserves_context_scoped_authority() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();

        let capability = host
            .grant_navigation_context_network_capability(context.context())
            .unwrap();

        assert_eq!(capability.context(), context.context());
        assert_eq!(capability.class(), CapabilityClass::Network);
        assert_eq!(host.active_capabilities(), 1);

        let closed = host.close_navigation_context(context.context()).unwrap();
        assert_eq!(closed.revoked_capabilities(), 1);
        assert_eq!(host.active_capabilities(), 0);
    }

    #[test]
    fn cross_site_context_transition_revokes_only_moving_context_authority() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let moving = host
            .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
            .unwrap();
        let staying = host
            .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
            .unwrap();
        let moving_capability = host
            .grant_navigation_context_capability(moving.context(), CapabilityClass::Network)
            .unwrap();
        let staying_capability = host
            .grant_navigation_context_capability(staying.context(), CapabilityClass::Network)
            .unwrap();
        let mut network = FixtureNetwork::default();
        let operation = host
            .start_navigation_context_network_operation(
                moving_capability,
                network_request("https://a.example.com/data"),
                &mut network,
            )
            .unwrap();
        assert_eq!(network.starts, 1);

        let transition = host
            .navigate_navigation_context(
                moving.context(),
                &WebUrl::parse("https://example.org/").unwrap(),
            )
            .unwrap();

        assert_eq!(
            transition.kind(),
            NavigationTransitionKind::CrossSiteReplacement
        );
        assert!(!transition.source_retired());
        assert_eq!(host.active_site_processes(), 2);
        assert_eq!(
            host.poll_navigation_context_network_operation(
                moving_capability,
                operation,
                &mut network,
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
        );
        assert_eq!(network.polls, 0);
        host.authorize_navigation_context_capability(staying_capability)
            .unwrap();

        let staying_close = host.close_navigation_context(staying.context()).unwrap();
        assert!(staying_close.source_retired());
        assert_eq!(host.active_site_processes(), 1);
        let moving_close = host.close_navigation_context(moving.context()).unwrap();
        assert!(moving_close.source_retired());
        assert_eq!(host.active_site_processes(), 0);
    }

    #[test]
    fn one_slot_cross_site_context_retires_unshared_source_before_replacement() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        let old_process = host
            .require_navigation_context(context.context())
            .unwrap()
            .binding
            .process;
        let capability = host
            .grant_navigation_context_capability(context.context(), CapabilityClass::Network)
            .unwrap();

        let transition = host
            .navigate_navigation_context(
                context.context(),
                &WebUrl::parse("https://example.org/").unwrap(),
            )
            .unwrap();

        let new_process = host
            .require_navigation_context(context.context())
            .unwrap()
            .binding
            .process;
        assert_eq!(
            transition.kind(),
            NavigationTransitionKind::CrossSiteReplacement
        );
        assert!(transition.source_retired());
        assert_ne!(new_process, old_process);
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(host.active_navigation_contexts(), 1);
        assert_eq!(
            host.authorize_navigation_context_capability(capability)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
        );
    }

    #[test]
    fn failed_one_slot_target_creation_invalidates_context_after_source_retirement() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        host.ipc_limits.max_message_bytes = host.ipc_limits.max_queued_bytes + 1;

        let error = host
            .navigate_navigation_context(
                context.context(),
                &WebUrl::parse("https://example.org/").unwrap(),
            )
            .unwrap_err();

        assert_eq!(
            error.kind,
            HostControlErrorKind::NavigationContextInvalidated
        );
        assert_eq!(host.active_navigation_contexts(), 0);
        assert_eq!(host.active_site_processes(), 0);
        assert_eq!(
            host.navigation_context(context.context()).unwrap_err().kind,
            HostControlErrorKind::UnknownNavigationContext
        );
    }

    #[test]
    fn shared_source_blocks_one_slot_cross_site_transition_without_mutation() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let first = host
            .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
            .unwrap();
        let second = host
            .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
            .unwrap();
        let capability = host
            .grant_navigation_context_capability(first.context(), CapabilityClass::Network)
            .unwrap();

        let error = host
            .navigate_navigation_context(
                first.context(),
                &WebUrl::parse("https://example.org/").unwrap(),
            )
            .unwrap_err();

        assert_eq!(
            error.kind,
            HostControlErrorKind::Process(ProcessTopologyErrorKind::ProcessLimitExceeded)
        );
        assert_eq!(host.active_navigation_contexts(), 2);
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(
            host.navigation_context(first.context()).unwrap().site(),
            first.site()
        );
        assert_eq!(
            host.navigation_context(second.context()).unwrap().site(),
            second.site()
        );
        host.authorize_navigation_context_capability(capability)
            .unwrap();
    }

    #[test]
    fn process_loss_invalidates_bound_contexts_and_context_capabilities() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let first = host
            .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
            .unwrap();
        let second = host
            .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
            .unwrap();
        let process = host
            .require_navigation_context(first.context())
            .unwrap()
            .binding
            .process;
        let capability = host
            .grant_navigation_context_capability(first.context(), CapabilityClass::Network)
            .unwrap();

        let loss = host.process_lost(process).unwrap();

        assert_eq!(loss.invalidated_navigation_contexts(), 2);
        assert_eq!(host.active_navigation_contexts(), 0);
        assert_eq!(host.active_capabilities(), 0);
        assert_eq!(
            host.navigation_context(first.context()).unwrap_err().kind,
            HostControlErrorKind::UnknownNavigationContext
        );
        assert_eq!(
            host.navigation_context(second.context()).unwrap_err().kind,
            HostControlErrorKind::UnknownNavigationContext
        );
        assert_eq!(
            host.authorize_navigation_context_capability(capability)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
        );
    }

    #[test]
    fn same_process_does_not_make_context_capabilities_interchangeable() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let first = host
            .open_navigation_context(&WebUrl::parse("https://a.example.com/").unwrap())
            .unwrap();
        let second = host
            .open_navigation_context(&WebUrl::parse("https://b.example.com/").unwrap())
            .unwrap();
        let capability = host
            .grant_navigation_context_capability(first.context(), CapabilityClass::Network)
            .unwrap();
        let forged = NavigationContextCapability {
            context: second.context(),
            id: capability.id(),
            class: CapabilityClass::Network,
        };
        let mut network = FixtureNetwork::default();

        assert_eq!(
            host.start_navigation_context_network_operation(
                forged,
                network_request("https://a.example.com/data"),
                &mut network,
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::InvalidNavigationContextCapabilityAuthority
        );
        assert_eq!(network.starts, 0);
    }

    #[test]
    fn opaque_context_identity_is_reused_only_when_explicitly_propagated() {
        let mut host = HostControlPlane::try_new(test_limits(3, 8)).unwrap();
        let opaque_url = WebUrl::parse("data:text/html,rarog").unwrap();
        let inherited_site = opaque_url.site_identity().unwrap();
        let first = host
            .open_navigation_context_to_site(inherited_site.clone())
            .unwrap();
        let second = host
            .open_navigation_context_to_site(inherited_site)
            .unwrap();

        let first_process = host
            .require_navigation_context(first.context())
            .unwrap()
            .binding
            .process;
        let second_process = host
            .require_navigation_context(second.context())
            .unwrap()
            .binding
            .process;
        assert_eq!(first.site(), second.site());
        assert_eq!(first_process, second_process);

        let fresh = host.open_navigation_context(&opaque_url).unwrap();
        let fresh_process = host
            .require_navigation_context(fresh.context())
            .unwrap()
            .binding
            .process;
        assert!(fresh.site().is_opaque());
        assert_ne!(fresh.site(), first.site());
        assert_ne!(fresh_process, first_process);
    }

    #[test]
    fn navigation_context_limit_is_enforced_before_site_allocation() {
        let mut limits = test_limits(2, 8);
        limits.max_navigation_contexts = 1;
        let mut host = HostControlPlane::try_new(limits).unwrap();
        host.open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();

        let error = host
            .open_navigation_context(&WebUrl::parse("https://example.org/").unwrap())
            .unwrap_err();
        assert_eq!(
            error.kind,
            HostControlErrorKind::NavigationContextLimitExceeded
        );
        assert_eq!(host.active_navigation_contexts(), 1);
        assert_eq!(host.active_site_processes(), 1);
        assert!(
            host.process_for_site(&site("https://example.org/"))
                .is_none()
        );
    }

    #[test]
    fn navigation_context_identity_is_monotonic_and_not_reused_after_close() {
        let mut host = HostControlPlane::try_new(test_limits(1, 8)).unwrap();
        let first = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        let first_id = first.context();
        host.close_navigation_context(first_id).unwrap();

        let second = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();

        assert!(second.context().get() > first_id.get());
        assert_ne!(second.context(), first_id);
        assert_eq!(
            host.navigation_context(first_id).unwrap_err().kind,
            HostControlErrorKind::UnknownNavigationContext
        );
    }

    #[test]
    fn transition_to_existing_target_site_retires_last_source_reference() {
        let mut host = HostControlPlane::try_new(test_limits(2, 8)).unwrap();
        let source = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        let target = host
            .open_navigation_context(&WebUrl::parse("https://example.org/").unwrap())
            .unwrap();
        let source_process = host
            .require_navigation_context(source.context())
            .unwrap()
            .binding
            .process;
        let target_process = host
            .require_navigation_context(target.context())
            .unwrap()
            .binding
            .process;

        let transition = host
            .navigate_navigation_context(
                source.context(),
                &WebUrl::parse("https://example.org/next").unwrap(),
            )
            .unwrap();

        assert!(transition.source_retired());
        assert_eq!(host.active_site_processes(), 1);
        assert_eq!(
            host.require_navigation_context(source.context())
                .unwrap()
                .binding
                .process,
            target_process
        );
        assert_eq!(
            host.require_navigation_context(target.context())
                .unwrap()
                .binding
                .process,
            target_process
        );
        assert_ne!(source_process, target_process);
        assert_eq!(host.site_for_process(source_process), None);
    }

    #[test]
    fn invalid_composed_limits_are_rejected() {
        let mut limits = test_limits(1, 1);
        limits.ipc.max_message_bytes = limits.ipc.max_queued_bytes + 1;
        assert_eq!(
            HostControlPlane::try_new(limits).unwrap_err().kind,
            HostControlErrorKind::InvalidLimits
        );

        let mut limits = test_limits(1, 1);
        limits.max_network_operations = 0;
        assert_eq!(
            HostControlPlane::try_new(limits).unwrap_err().kind,
            HostControlErrorKind::InvalidLimits
        );

        let mut limits = test_limits(1, 1);
        limits.max_navigation_contexts = 0;
        assert_eq!(
            HostControlPlane::try_new(limits).unwrap_err().kind,
            HostControlErrorKind::InvalidLimits
        );
    }
}
