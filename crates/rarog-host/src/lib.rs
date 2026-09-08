use rarog_broker::{
    CapabilityBroker, CapabilityClass, CapabilityError, CapabilityErrorKind, CapabilityGrant,
    CapabilityId, DEFAULT_MAX_CAPABILITIES,
};
use rarog_ipc::{EndpointRole, IpcChannel, IpcEnvelope, IpcError, IpcErrorKind, IpcLimits};
use rarog_process::{
    DEFAULT_MAX_SITE_PROCESSES, ProcessTopology, ProcessTopologyError, ProcessTopologyErrorKind,
    SiteAssignmentKind, SiteProcessId,
};
use rarog_url::{SiteIdentity, UrlError, UrlErrorKind};
use std::collections::HashMap;
use std::fmt;

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
    Url(UrlErrorKind),
    InvalidDocumentBinding,
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

    #[test]
    fn initial_and_same_site_navigation_reuse_host_assignment() {
        let mut host = HostControlPlane::try_new(test_limits(2, 2)).unwrap();
        let first_url = WebUrl::parse("https://a.example.com/start").unwrap();
        let first = host
            .begin_document_navigation(None, &first_url)
            .unwrap();

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

        assert_eq!(
            inherited.kind(),
            NavigationTransitionKind::SameSiteReuse
        );
        assert_eq!(inherited.current(), &first);

        let fresh = host
            .begin_document_navigation(Some(&first), &opaque_url)
            .unwrap();

        assert_eq!(
            fresh.kind(),
            NavigationTransitionKind::CrossSiteReplacement
        );
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
        assert!(host
            .process_for_site(&target.site_identity().unwrap())
            .is_none());
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
        assert!(host
            .process_for_site(&target.site_identity().unwrap())
            .is_none());
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
