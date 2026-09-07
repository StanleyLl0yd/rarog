use rarog_url::SiteIdentity;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_SITE_PROCESSES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ProcessId(NonZeroU64);

impl ProcessId {
    fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HostProcessId(ProcessId);

impl HostProcessId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for HostProcessId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "host:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SiteProcessId(ProcessId);

impl SiteProcessId {
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for SiteProcessId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "site:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteAssignmentKind {
    Existing,
    Created,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteAssignment {
    process: SiteProcessId,
    kind: SiteAssignmentKind,
}

impl SiteAssignment {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessTopologyErrorKind {
    InvalidProcessLimit,
    ProcessLimitExceeded,
    IdentitySpaceExhausted,
    UnknownSiteProcess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessTopologyError {
    pub kind: ProcessTopologyErrorKind,
    pub message: String,
}

impl ProcessTopologyError {
    fn new(kind: ProcessTopologyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for ProcessTopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProcessTopologyError {}

#[derive(Debug)]
struct ProcessIdAllocator {
    next: Option<NonZeroU64>,
}

impl ProcessIdAllocator {
    fn new() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }

    fn allocate(&mut self) -> Result<ProcessId, ProcessTopologyError> {
        let next = self.next.ok_or_else(|| {
            ProcessTopologyError::new(
                ProcessTopologyErrorKind::IdentitySpaceExhausted,
                "process identity space is exhausted",
            )
        })?;
        self.next = NonZeroU64::new(next.get().wrapping_add(1));
        Ok(ProcessId(next))
    }
}

#[derive(Debug)]
pub struct ProcessTopology {
    host: HostProcessId,
    max_site_processes: usize,
    allocator: ProcessIdAllocator,
    process_by_site: HashMap<SiteIdentity, SiteProcessId>,
    site_by_process: HashMap<SiteProcessId, SiteIdentity>,
}

impl ProcessTopology {
    pub fn try_new(max_site_processes: usize) -> Result<Self, ProcessTopologyError> {
        if max_site_processes == 0 {
            return Err(ProcessTopologyError::new(
                ProcessTopologyErrorKind::InvalidProcessLimit,
                "site process limit must be non-zero",
            ));
        }

        let mut allocator = ProcessIdAllocator::new();
        let host = HostProcessId(allocator.allocate()?);
        Ok(Self {
            host,
            max_site_processes,
            allocator,
            process_by_site: HashMap::new(),
            site_by_process: HashMap::new(),
        })
    }

    pub fn with_default_limit() -> Result<Self, ProcessTopologyError> {
        Self::try_new(DEFAULT_MAX_SITE_PROCESSES)
    }

    pub fn host_process(&self) -> HostProcessId {
        self.host
    }

    pub fn max_site_processes(&self) -> usize {
        self.max_site_processes
    }

    pub fn active_site_processes(&self) -> usize {
        self.process_by_site.len()
    }

    pub fn assign_site(
        &mut self,
        site: SiteIdentity,
    ) -> Result<SiteAssignment, ProcessTopologyError> {
        if let Some(process) = self.process_by_site.get(&site).copied() {
            return Ok(SiteAssignment {
                process,
                kind: SiteAssignmentKind::Existing,
            });
        }

        if self.process_by_site.len() >= self.max_site_processes {
            return Err(ProcessTopologyError::new(
                ProcessTopologyErrorKind::ProcessLimitExceeded,
                format!(
                    "site process limit {} reached; cross-site process sharing is forbidden",
                    self.max_site_processes
                ),
            ));
        }

        let process = SiteProcessId(self.allocator.allocate()?);
        self.site_by_process.insert(process, site.clone());
        self.process_by_site.insert(site, process);
        Ok(SiteAssignment {
            process,
            kind: SiteAssignmentKind::Created,
        })
    }

    pub fn process_for_site(&self, site: &SiteIdentity) -> Option<SiteProcessId> {
        self.process_by_site.get(site).copied()
    }

    pub fn site_for_process(&self, process: SiteProcessId) -> Option<&SiteIdentity> {
        self.site_by_process.get(&process)
    }

    pub fn retire_site_process(
        &mut self,
        process: SiteProcessId,
    ) -> Result<SiteIdentity, ProcessTopologyError> {
        let site = self.site_by_process.remove(&process).ok_or_else(|| {
            ProcessTopologyError::new(
                ProcessTopologyErrorKind::UnknownSiteProcess,
                format!("unknown site process {process}"),
            )
        })?;
        self.process_by_site.remove(&site);
        Ok(site)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_url::{Origin, WebUrl};

    fn site(url: &str) -> SiteIdentity {
        WebUrl::parse(url).unwrap().site_identity().unwrap()
    }

    #[test]
    fn host_and_site_process_identities_use_one_nonzero_space() {
        let mut topology = ProcessTopology::try_new(2).unwrap();
        assert_eq!(topology.host_process().get(), 1);
        let assignment = topology.assign_site(site("https://example.com/")).unwrap();
        assert_eq!(assignment.process().get(), 2);
        assert!(assignment.is_new());
    }

    #[test]
    fn same_schemeful_site_reuses_one_assignment() {
        let mut topology = ProcessTopology::try_new(2).unwrap();
        let first_site = site("https://a.example.com/");
        let second_site = site("https://b.example.com/path");

        let first = topology.assign_site(first_site.clone()).unwrap();
        let second = topology.assign_site(second_site.clone()).unwrap();

        assert_eq!(first.kind(), SiteAssignmentKind::Created);
        assert_eq!(second.kind(), SiteAssignmentKind::Existing);
        assert_eq!(first.process(), second.process());
        assert_eq!(topology.active_site_processes(), 1);
        assert_eq!(
            topology.process_for_site(&first_site),
            topology.process_for_site(&second_site)
        );
    }

    #[test]
    fn distinct_sites_never_share_to_satisfy_a_process_budget() {
        let mut topology = ProcessTopology::try_new(1).unwrap();
        let first_site = site("https://example.com/");
        let cross_scheme = site("http://example.com/");

        let first = topology.assign_site(first_site.clone()).unwrap();
        let error = topology.assign_site(cross_scheme.clone()).unwrap_err();

        assert_eq!(error.kind, ProcessTopologyErrorKind::ProcessLimitExceeded);
        assert_eq!(topology.active_site_processes(), 1);
        assert_eq!(topology.process_for_site(&first_site), Some(first.process()));
        assert_eq!(topology.process_for_site(&cross_scheme), None);
    }

    #[test]
    fn opaque_site_identity_isolated_unless_exact_identity_is_reused() {
        let url = WebUrl::parse("data:text/plain,hello").unwrap();
        let first_origin = url.origin().unwrap();
        let first_site = first_origin.site();
        let same_site = first_site.clone();
        let second_site = match url.origin().unwrap() {
            Origin::Opaque(_) as origin => origin.site(),
            Origin::Tuple { .. } => unreachable!(),
        };

        let mut topology = ProcessTopology::try_new(3).unwrap();
        let first = topology.assign_site(first_site).unwrap();
        let same = topology.assign_site(same_site).unwrap();
        let second = topology.assign_site(second_site).unwrap();

        assert_eq!(first.process(), same.process());
        assert_ne!(first.process(), second.process());
        assert_eq!(topology.active_site_processes(), 2);
    }

    #[test]
    fn retired_process_identity_is_not_reused() {
        let mut topology = ProcessTopology::try_new(1).unwrap();
        let site_identity = site("https://example.com/");
        let first = topology.assign_site(site_identity.clone()).unwrap();
        assert_eq!(
            topology.retire_site_process(first.process()).unwrap(),
            site_identity
        );
        assert_eq!(topology.active_site_processes(), 0);

        let replacement = topology.assign_site(site("https://example.com/")).unwrap();
        assert_ne!(first.process(), replacement.process());
        assert_eq!(replacement.kind(), SiteAssignmentKind::Created);
    }

    #[test]
    fn unknown_process_retirement_fails_without_mutating_assignments() {
        let mut topology = ProcessTopology::try_new(2).unwrap();
        let first = topology.assign_site(site("https://example.com/")).unwrap();
        let unknown = SiteProcessId(ProcessId(NonZeroU64::new(999).unwrap()));

        let error = topology.retire_site_process(unknown).unwrap_err();

        assert_eq!(error.kind, ProcessTopologyErrorKind::UnknownSiteProcess);
        assert_eq!(topology.active_site_processes(), 1);
        assert!(topology.site_for_process(first.process()).is_some());
    }

    #[test]
    fn zero_process_limit_is_rejected() {
        let error = ProcessTopology::try_new(0).unwrap_err();
        assert_eq!(error.kind, ProcessTopologyErrorKind::InvalidProcessLimit);
    }

    #[test]
    fn allocator_exhaustion_fails_closed_after_last_identity() {
        let mut allocator = ProcessIdAllocator {
            next: Some(NonZeroU64::new(u64::MAX).unwrap()),
        };
        assert_eq!(allocator.allocate().unwrap().get(), u64::MAX);
        let error = allocator.allocate().unwrap_err();
        assert_eq!(error.kind, ProcessTopologyErrorKind::IdentitySpaceExhausted);
    }
}
