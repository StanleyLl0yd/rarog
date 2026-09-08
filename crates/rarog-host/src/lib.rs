use rarog_broker::{
    CapabilityBroker, CapabilityClass, CapabilityError, CapabilityErrorKind, CapabilityGrant,
    CapabilityId, DEFAULT_MAX_CAPABILITIES,
};
use rarog_ipc::{EndpointRole, IpcChannel, IpcEnvelope, IpcError, IpcErrorKind, IpcLimits};
use rarog_process::{
    DEFAULT_MAX_SITE_PROCESSES, ProcessTopology, ProcessTopologyError, ProcessTopologyErrorKind,
    SiteAssignmentKind, SiteProcessId,
};
use rarog_url::SiteIdentity;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostLimits {
    pub max_site_processes: usize,
    pub ipc: IpcLimits,
    pub max_capabilities: usize,
}

impl HostLimits {
    pub fn is_valid(self) -> bool {
        self.max_site_processes > 0 && self.ipc.is_valid() && self.max_capabilities > 0
    }
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            max_site_processes: DEFAULT_MAX_SITE_PROCESSES,
            ipc: IpcLimits::default(),
            max_capabilities: DEFAULT_MAX_CAPABILITIES,
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
}


pub const DEFAULT_MAX_NAVIGATION_CONTEXTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationLimits {
    pub max_contexts: usize,
}

impl NavigationLimits {
    pub const fn is_valid(self) -> bool {
        self.max_contexts > 0
    }
}

impl Default for NavigationLimits {
    fn default() -> Self {
        Self {
            max_contexts: DEFAULT_MAX_NAVIGATION_CONTEXTS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NavigationContextId(NonZeroU64);

impl NavigationContextId {
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for NavigationContextId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "navigation:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationErrorKind {
    InvalidLimit,
    ContextLimitExceeded,
    IdentitySpaceExhausted,
    UnknownContext,
    Host(HostControlErrorKind),
    TransitionInvalidated(HostControlErrorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationError {
    pub kind: NavigationErrorKind,
    pub message: String,
}

impl NavigationError {
    fn new(kind: NavigationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn unknown_context(context: NavigationContextId) -> Self {
        Self::new(
            NavigationErrorKind::UnknownContext,
            format!("unknown or closed navigation context {context}"),
        )
    }
}

impl fmt::Display for NavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NavigationError {}

impl From<HostControlError> for NavigationError {
    fn from(error: HostControlError) -> Self {
        Self::new(NavigationErrorKind::Host(error.kind), error.message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationLease {
    context: NavigationContextId,
    process: SiteProcessId,
}

impl NavigationLease {
    pub const fn context(self) -> NavigationContextId {
        self.context
    }

    pub const fn process(self) -> SiteProcessId {
        self.process
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationTransitionKind {
    SameSite,
    ReusedExistingSite,
    CreatedSite,
    CapacityReplacement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationTransition {
    context: NavigationContextId,
    previous_process: SiteProcessId,
    process: SiteProcessId,
    kind: NavigationTransitionKind,
    source_loss: Option<SiteLoss>,
}

impl NavigationTransition {
    pub const fn context(&self) -> NavigationContextId {
        self.context
    }

    pub const fn previous_process(&self) -> SiteProcessId {
        self.previous_process
    }

    pub const fn process(&self) -> SiteProcessId {
        self.process
    }

    pub const fn kind(&self) -> NavigationTransitionKind {
        self.kind
    }

    pub fn source_loss(&self) -> Option<&SiteLoss> {
        self.source_loss.as_ref()
    }
}

#[derive(Debug)]
pub struct NavigationControlPlane {
    host: HostControlPlane,
    max_contexts: usize,
    next_context: Option<NonZeroU64>,
    contexts: HashMap<NavigationContextId, SiteProcessId>,
}

impl NavigationControlPlane {
    pub fn try_new(
        host_limits: HostLimits,
        navigation_limits: NavigationLimits,
    ) -> Result<Self, NavigationError> {
        if !navigation_limits.is_valid() {
            return Err(NavigationError::new(
                NavigationErrorKind::InvalidLimit,
                "navigation context limit must be non-zero",
            ));
        }

        Ok(Self {
            host: HostControlPlane::try_new(host_limits)?,
            max_contexts: navigation_limits.max_contexts,
            next_context: NonZeroU64::new(1),
            contexts: HashMap::new(),
        })
    }

    pub fn with_default_limits() -> Result<Self, NavigationError> {
        Self::try_new(HostLimits::default(), NavigationLimits::default())
    }

    pub fn host_process(&self) -> rarog_process::HostProcessId {
        self.host.host_process()
    }

    pub fn active_contexts(&self) -> usize {
        self.contexts.len()
    }

    pub fn active_site_processes(&self) -> usize {
        self.host.active_site_processes()
    }

    pub fn active_capabilities(&self) -> usize {
        self.host.active_capabilities()
    }

    pub fn process_for_context(&self, context: NavigationContextId) -> Option<SiteProcessId> {
        self.contexts.get(&context).copied()
    }

    pub fn site_for_context(&self, context: NavigationContextId) -> Option<&SiteIdentity> {
        let process = self.process_for_context(context)?;
        self.host.site_for_process(process)
    }

    pub fn open_context(
        &mut self,
        site: SiteIdentity,
    ) -> Result<NavigationLease, NavigationError> {
        if self.contexts.len() >= self.max_contexts {
            return Err(NavigationError::new(
                NavigationErrorKind::ContextLimitExceeded,
                format!("navigation context limit {} reached", self.max_contexts),
            ));
        }

        let context = self.allocate_context()?;
        let process = self.host.ensure_site(site)?.process();
        self.contexts.insert(context, process);
        Ok(NavigationLease { context, process })
    }

    pub fn transition_context(
        &mut self,
        context: NavigationContextId,
        target_site: SiteIdentity,
    ) -> Result<NavigationTransition, NavigationError> {
        let previous_process = self.require_context(context)?;

        if self.host.site_for_process(previous_process) == Some(&target_site) {
            return Ok(NavigationTransition {
                context,
                previous_process,
                process: previous_process,
                kind: NavigationTransitionKind::SameSite,
                source_loss: None,
            });
        }

        if let Some(process) = self.host.process_for_site(&target_site) {
            self.contexts.insert(context, process);
            let source_loss = self.retire_if_unreferenced(previous_process)?;
            return Ok(NavigationTransition {
                context,
                previous_process,
                process,
                kind: NavigationTransitionKind::ReusedExistingSite,
                source_loss,
            });
        }

        match self.host.ensure_site(target_site.clone()) {
            Ok(lease) => {
                let process = lease.process();
                self.contexts.insert(context, process);
                let source_loss = self.retire_if_unreferenced(previous_process)?;
                Ok(NavigationTransition {
                    context,
                    previous_process,
                    process,
                    kind: NavigationTransitionKind::CreatedSite,
                    source_loss,
                })
            }
            Err(error)
                if error.kind
                    == HostControlErrorKind::Process(
                        ProcessTopologyErrorKind::ProcessLimitExceeded,
                    )
                    && self.context_ref_count(previous_process) == 1 =>
            {
                let source_loss = self.host.process_lost(previous_process)?;
                let replacement = match self.host.ensure_site(target_site) {
                    Ok(lease) => lease,
                    Err(error) => {
                        self.contexts.remove(&context);
                        return Err(NavigationError::new(
                            NavigationErrorKind::TransitionInvalidated(error.kind),
                            format!(
                                "navigation context {context} lost source authority before replacement failed: {}",
                                error.message
                            ),
                        ));
                    }
                };
                let process = replacement.process();
                self.contexts.insert(context, process);
                Ok(NavigationTransition {
                    context,
                    previous_process,
                    process,
                    kind: NavigationTransitionKind::CapacityReplacement,
                    source_loss: Some(source_loss),
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn close_context(
        &mut self,
        context: NavigationContextId,
    ) -> Result<Option<SiteLoss>, NavigationError> {
        let process = self
            .contexts
            .remove(&context)
            .ok_or_else(|| NavigationError::unknown_context(context))?;
        self.retire_if_unreferenced(process)
    }

    pub fn grant_capability(
        &mut self,
        context: NavigationContextId,
        class: CapabilityClass,
    ) -> Result<CapabilityGrant, NavigationError> {
        let process = self.require_context(context)?;
        Ok(self.host.grant_capability(process, class)?)
    }

    pub fn authorize_capability(
        &self,
        context: NavigationContextId,
        id: CapabilityId,
        class: CapabilityClass,
    ) -> Result<(), NavigationError> {
        let process = self.require_context(context)?;
        Ok(self.host.authorize_capability(process, id, class)?)
    }

    fn allocate_context(&mut self) -> Result<NavigationContextId, NavigationError> {
        let next = self.next_context.ok_or_else(|| {
            NavigationError::new(
                NavigationErrorKind::IdentitySpaceExhausted,
                "navigation context identity space is exhausted",
            )
        })?;
        self.next_context = NonZeroU64::new(next.get().wrapping_add(1));
        Ok(NavigationContextId(next))
    }

    fn require_context(
        &self,
        context: NavigationContextId,
    ) -> Result<SiteProcessId, NavigationError> {
        self.process_for_context(context)
            .ok_or_else(|| NavigationError::unknown_context(context))
    }

    fn context_ref_count(&self, process: SiteProcessId) -> usize {
        self.contexts
            .values()
            .filter(|candidate| **candidate == process)
            .count()
    }

    fn retire_if_unreferenced(
        &mut self,
        process: SiteProcessId,
    ) -> Result<Option<SiteLoss>, NavigationError> {
        if self.context_ref_count(process) != 0 {
            return Ok(None);
        }
        Ok(Some(self.host.process_lost(process)?))
    }
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
    sites: HashMap<SiteProcessId, SiteInstance>,
}

impl HostControlPlane {
    pub fn try_new(limits: HostLimits) -> Result<Self, HostControlError> {
        if !limits.is_valid() {
            return Err(HostControlError::new(
                HostControlErrorKind::InvalidLimits,
                "Host limits must contain non-zero process/capability limits and valid IPC limits",
            ));
        }

        Ok(Self {
            topology: ProcessTopology::try_new(limits.max_site_processes)?,
            broker: CapabilityBroker::try_new(limits.max_capabilities)?,
            ipc_limits: limits.ipc,
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
        Ok(self.broker.revoke(id)?)
    }

    pub fn process_lost(&mut self, process: SiteProcessId) -> Result<SiteLoss, HostControlError> {
        let mut instance = self
            .sites
            .remove(&process)
            .ok_or_else(|| HostControlError::unknown_process(process))?;

        instance.channel.disconnect();
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
        })
    }

    pub fn recover_site(&mut self, site: SiteIdentity) -> Result<SiteLease, HostControlError> {
        self.ensure_site(site)
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
    use rarog_ipc::{IpcEnvelope, RequestId};
    use rarog_url::WebUrl;

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


    fn navigation_limits(max_contexts: usize) -> NavigationLimits {
        NavigationLimits { max_contexts }
    }

    #[test]
    fn navigation_contexts_reuse_same_site_and_replace_cross_site_authority() {
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(2, 4), navigation_limits(4)).unwrap();
        let opened = navigation
            .open_context(site("https://a.example.com/"))
            .unwrap();

        let same = navigation
            .transition_context(opened.context(), site("https://b.example.com/path"))
            .unwrap();
        assert_eq!(same.kind(), NavigationTransitionKind::SameSite);
        assert_eq!(same.process(), opened.process());
        assert!(same.source_loss().is_none());

        let cross_site = navigation
            .transition_context(opened.context(), site("https://example.org/"))
            .unwrap();
        assert_eq!(cross_site.kind(), NavigationTransitionKind::CreatedSite);
        assert_ne!(cross_site.process(), opened.process());
        assert_eq!(
            cross_site.source_loss().unwrap().process(),
            opened.process()
        );
        assert_eq!(navigation.active_contexts(), 1);
        assert_eq!(navigation.active_site_processes(), 1);
    }

    #[test]
    fn shared_source_site_stays_live_until_its_last_context_leaves() {
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(2, 4), navigation_limits(4)).unwrap();
        let first = navigation
            .open_context(site("https://a.example.com/"))
            .unwrap();
        let second = navigation
            .open_context(site("https://b.example.com/"))
            .unwrap();
        assert_eq!(first.process(), second.process());

        let capability = navigation
            .grant_capability(first.context(), CapabilityClass::Network)
            .unwrap();
        let moved = navigation
            .transition_context(first.context(), site("https://example.org/"))
            .unwrap();

        assert!(moved.source_loss().is_none());
        assert_eq!(navigation.active_site_processes(), 2);
        assert_eq!(
            navigation
                .authorize_capability(
                    first.context(),
                    capability.id(),
                    CapabilityClass::Network,
                )
                .unwrap_err()
                .kind,
            NavigationErrorKind::Host(HostControlErrorKind::Capability(
                CapabilityErrorKind::WrongOwner,
            ))
        );
        assert_eq!(
            navigation.authorize_capability(
                second.context(),
                capability.id(),
                CapabilityClass::Network,
            ),
            Ok(())
        );

        let loss = navigation.close_context(second.context()).unwrap().unwrap();
        assert_eq!(loss.process(), second.process());
        assert_eq!(loss.revoked_capabilities(), 1);
        assert_eq!(navigation.active_site_processes(), 1);
    }

    #[test]
    fn one_slot_cross_site_transition_revokes_before_fresh_replacement() {
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(1, 4), navigation_limits(2)).unwrap();
        let opened = navigation
            .open_context(site("https://example.com/"))
            .unwrap();
        let capability = navigation
            .grant_capability(opened.context(), CapabilityClass::Clipboard)
            .unwrap();

        let moved = navigation
            .transition_context(opened.context(), site("https://example.org/"))
            .unwrap();

        assert_eq!(
            moved.kind(),
            NavigationTransitionKind::CapacityReplacement
        );
        assert_ne!(moved.process(), opened.process());
        assert_eq!(moved.source_loss().unwrap().revoked_capabilities(), 1);
        assert_eq!(navigation.active_site_processes(), 1);
        assert_eq!(navigation.active_capabilities(), 0);
        assert_eq!(
            navigation
                .authorize_capability(
                    opened.context(),
                    capability.id(),
                    CapabilityClass::Clipboard,
                )
                .unwrap_err()
                .kind,
            NavigationErrorKind::Host(HostControlErrorKind::Capability(
                CapabilityErrorKind::UnknownCapability,
            ))
        );
    }

    #[test]
    fn shared_source_never_weakens_isolation_to_fit_one_process_slot() {
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(1, 2), navigation_limits(3)).unwrap();
        let first = navigation
            .open_context(site("https://a.example.com/"))
            .unwrap();
        let second = navigation
            .open_context(site("https://b.example.com/"))
            .unwrap();

        let error = navigation
            .transition_context(first.context(), site("https://example.org/"))
            .unwrap_err();

        assert_eq!(
            error.kind,
            NavigationErrorKind::Host(HostControlErrorKind::Process(
                ProcessTopologyErrorKind::ProcessLimitExceeded,
            ))
        );
        assert_eq!(
            navigation.process_for_context(first.context()),
            Some(first.process())
        );
        assert_eq!(
            navigation.process_for_context(second.context()),
            Some(second.process())
        );
        assert_eq!(navigation.active_site_processes(), 1);
    }

    #[test]
    fn opaque_site_identity_is_propagated_exactly_across_navigation_contexts() {
        let url = WebUrl::parse("data:text/plain,hello").unwrap();
        let first_site = url.origin().unwrap().site();
        let exact_same_site = first_site.clone();
        let distinct_opaque_site = url.origin().unwrap().site();
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(2, 2), navigation_limits(2)).unwrap();
        let opened = navigation.open_context(first_site).unwrap();

        let same = navigation
            .transition_context(opened.context(), exact_same_site)
            .unwrap();
        assert_eq!(same.kind(), NavigationTransitionKind::SameSite);
        assert_eq!(same.process(), opened.process());

        let distinct = navigation
            .transition_context(opened.context(), distinct_opaque_site)
            .unwrap();
        assert_ne!(distinct.process(), opened.process());
        assert_eq!(distinct.kind(), NavigationTransitionKind::CreatedSite);
    }

    #[test]
    fn navigation_contexts_are_bounded_and_identities_are_not_reused() {
        let mut navigation =
            NavigationControlPlane::try_new(test_limits(1, 2), navigation_limits(1)).unwrap();
        let first = navigation
            .open_context(site("https://example.com/"))
            .unwrap();

        assert_eq!(
            navigation
                .open_context(site("https://example.com/"))
                .unwrap_err()
                .kind,
            NavigationErrorKind::ContextLimitExceeded
        );

        navigation.close_context(first.context()).unwrap();
        let replacement = navigation
            .open_context(site("https://example.com/"))
            .unwrap();
        assert_ne!(first.context(), replacement.context());
        assert_ne!(first.process(), replacement.process());
    }

    #[test]
    fn invalid_composed_limits_are_rejected() {
        let mut limits = test_limits(1, 1);
        limits.ipc.max_message_bytes = limits.ipc.max_queued_bytes + 1;
        assert_eq!(
            HostControlPlane::try_new(limits).unwrap_err().kind,
            HostControlErrorKind::InvalidLimits
        );
    }
}
