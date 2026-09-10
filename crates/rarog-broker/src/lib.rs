use rarog_process::SiteProcessId;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU64;

pub const DEFAULT_MAX_CAPABILITIES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapabilityId(NonZeroU64);

impl CapabilityId {
    pub fn try_new(raw: u64) -> Result<Self, CapabilityError> {
        NonZeroU64::new(raw).map(Self).ok_or_else(|| {
            CapabilityError::new(
                CapabilityErrorKind::InvalidCapabilityId,
                "capability identity must be non-zero",
            )
        })
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "capability:{}", self.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapabilityClass {
    Network,
    Clipboard,
    Storage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityGrant {
    id: CapabilityId,
    owner: SiteProcessId,
    class: CapabilityClass,
}

impl CapabilityGrant {
    pub fn id(self) -> CapabilityId {
        self.id
    }

    pub fn owner(self) -> SiteProcessId {
        self.owner
    }

    pub fn class(self) -> CapabilityClass {
        self.class
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityErrorKind {
    InvalidLimit,
    InvalidCapabilityId,
    CapacityExceeded,
    IdentitySpaceExhausted,
    UnknownCapability,
    WrongOwner,
    WrongClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityError {
    pub kind: CapabilityErrorKind,
    pub message: String,
}

impl CapabilityError {
    fn new(kind: CapabilityErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CapabilityError {}

#[derive(Debug)]
struct CapabilityIdAllocator {
    next: Option<NonZeroU64>,
}

impl CapabilityIdAllocator {
    fn new() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }

    fn allocate(&mut self) -> Result<CapabilityId, CapabilityError> {
        let next = self.next.ok_or_else(|| {
            CapabilityError::new(
                CapabilityErrorKind::IdentitySpaceExhausted,
                "capability identity space is exhausted",
            )
        })?;
        self.next = NonZeroU64::new(next.get().wrapping_add(1));
        Ok(CapabilityId(next))
    }
}

#[derive(Debug)]
pub struct CapabilityBroker {
    max_capabilities: usize,
    allocator: CapabilityIdAllocator,
    grants: HashMap<CapabilityId, CapabilityGrant>,
}

impl CapabilityBroker {
    pub fn try_new(max_capabilities: usize) -> Result<Self, CapabilityError> {
        if max_capabilities == 0 {
            return Err(CapabilityError::new(
                CapabilityErrorKind::InvalidLimit,
                "capability limit must be non-zero",
            ));
        }
        Ok(Self {
            max_capabilities,
            allocator: CapabilityIdAllocator::new(),
            grants: HashMap::new(),
        })
    }

    pub fn with_default_limit() -> Result<Self, CapabilityError> {
        Self::try_new(DEFAULT_MAX_CAPABILITIES)
    }

    pub fn max_capabilities(&self) -> usize {
        self.max_capabilities
    }

    pub fn active_capabilities(&self) -> usize {
        self.grants.len()
    }

    pub fn grant(
        &mut self,
        owner: SiteProcessId,
        class: CapabilityClass,
    ) -> Result<CapabilityGrant, CapabilityError> {
        if self.grants.len() >= self.max_capabilities {
            return Err(CapabilityError::new(
                CapabilityErrorKind::CapacityExceeded,
                format!("capability limit {} reached", self.max_capabilities),
            ));
        }

        let id = self.allocator.allocate()?;
        let grant = CapabilityGrant { id, owner, class };
        self.grants.insert(id, grant);
        Ok(grant)
    }

    pub fn authorize(
        &self,
        id: CapabilityId,
        owner: SiteProcessId,
        required_class: CapabilityClass,
    ) -> Result<(), CapabilityError> {
        let grant = self.grants.get(&id).ok_or_else(|| {
            CapabilityError::new(
                CapabilityErrorKind::UnknownCapability,
                format!("unknown or revoked capability {id}"),
            )
        })?;

        if grant.owner != owner {
            return Err(CapabilityError::new(
                CapabilityErrorKind::WrongOwner,
                format!("{id} is not owned by {owner}"),
            ));
        }
        if grant.class != required_class {
            return Err(CapabilityError::new(
                CapabilityErrorKind::WrongClass,
                format!("{id} does not grant {required_class:?} authority"),
            ));
        }
        Ok(())
    }

    pub fn revoke(&mut self, id: CapabilityId) -> Result<CapabilityGrant, CapabilityError> {
        self.grants.remove(&id).ok_or_else(|| {
            CapabilityError::new(
                CapabilityErrorKind::UnknownCapability,
                format!("unknown or revoked capability {id}"),
            )
        })
    }

    pub fn revoke_all_for_process(&mut self, owner: SiteProcessId) -> usize {
        let before = self.grants.len();
        self.grants.retain(|_, grant| grant.owner != owner);
        before.saturating_sub(self.grants.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_process::ProcessTopology;
    use rarog_url::WebUrl;

    fn processes() -> (SiteProcessId, SiteProcessId) {
        let mut topology = ProcessTopology::try_new(2).unwrap();
        let first = topology
            .assign_site(
                WebUrl::parse("https://example.com/")
                    .unwrap()
                    .site_identity()
                    .unwrap(),
            )
            .unwrap()
            .process();
        let second = topology
            .assign_site(
                WebUrl::parse("https://example.org/")
                    .unwrap()
                    .site_identity()
                    .unwrap(),
            )
            .unwrap()
            .process();
        (first, second)
    }

    #[test]
    fn grant_and_authorize_require_owner_and_class() {
        let (first, second) = processes();
        let mut broker = CapabilityBroker::try_new(4).unwrap();
        let grant = broker.grant(first, CapabilityClass::Network).unwrap();

        assert_eq!(
            broker.authorize(grant.id(), first, CapabilityClass::Network),
            Ok(())
        );
        let wrong_owner = broker
            .authorize(grant.id(), second, CapabilityClass::Network)
            .unwrap_err();
        assert_eq!(wrong_owner.kind, CapabilityErrorKind::WrongOwner);

        let wrong_class = broker
            .authorize(grant.id(), first, CapabilityClass::Clipboard)
            .unwrap_err();
        assert_eq!(wrong_class.kind, CapabilityErrorKind::WrongClass);
    }

    #[test]
    fn untrusted_numeric_reference_is_not_authority() {
        let (owner, _) = processes();
        let broker = CapabilityBroker::try_new(1).unwrap();
        let guessed = CapabilityId::try_new(1).unwrap();

        let error = broker
            .authorize(guessed, owner, CapabilityClass::Network)
            .unwrap_err();

        assert_eq!(error.kind, CapabilityErrorKind::UnknownCapability);
    }

    #[test]
    fn revoked_capability_stays_invalid_and_identity_is_not_reused() {
        let (owner, _) = processes();
        let mut broker = CapabilityBroker::try_new(2).unwrap();
        let first = broker.grant(owner, CapabilityClass::Network).unwrap();
        assert_eq!(broker.revoke(first.id()).unwrap(), first);

        let stale = broker
            .authorize(first.id(), owner, CapabilityClass::Network)
            .unwrap_err();
        assert_eq!(stale.kind, CapabilityErrorKind::UnknownCapability);

        let replacement = broker.grant(owner, CapabilityClass::Network).unwrap();
        assert_ne!(first.id(), replacement.id());
    }

    #[test]
    fn process_loss_can_revoke_all_owned_authority() {
        let (first, second) = processes();
        let mut broker = CapabilityBroker::try_new(4).unwrap();
        let first_network = broker.grant(first, CapabilityClass::Network).unwrap();
        let first_clipboard = broker.grant(first, CapabilityClass::Clipboard).unwrap();
        let second_network = broker.grant(second, CapabilityClass::Network).unwrap();

        assert_eq!(broker.revoke_all_for_process(first), 2);
        assert_eq!(broker.active_capabilities(), 1);
        assert_eq!(
            broker
                .authorize(first_network.id(), first, CapabilityClass::Network)
                .unwrap_err()
                .kind,
            CapabilityErrorKind::UnknownCapability
        );
        assert_eq!(
            broker
                .authorize(first_clipboard.id(), first, CapabilityClass::Clipboard)
                .unwrap_err()
                .kind,
            CapabilityErrorKind::UnknownCapability
        );
        assert_eq!(
            broker.authorize(second_network.id(), second, CapabilityClass::Network),
            Ok(())
        );
    }

    #[test]
    fn capability_capacity_fails_closed() {
        let (owner, _) = processes();
        let mut broker = CapabilityBroker::try_new(1).unwrap();
        broker.grant(owner, CapabilityClass::Network).unwrap();

        let error = broker.grant(owner, CapabilityClass::Clipboard).unwrap_err();

        assert_eq!(error.kind, CapabilityErrorKind::CapacityExceeded);
        assert_eq!(broker.active_capabilities(), 1);
    }

    #[test]
    fn zero_limit_and_zero_reference_are_rejected() {
        assert_eq!(
            CapabilityBroker::try_new(0).unwrap_err().kind,
            CapabilityErrorKind::InvalidLimit
        );
        assert_eq!(
            CapabilityId::try_new(0).unwrap_err().kind,
            CapabilityErrorKind::InvalidCapabilityId
        );
    }

    #[test]
    fn allocator_exhaustion_fails_closed() {
        let mut allocator = CapabilityIdAllocator {
            next: Some(NonZeroU64::new(u64::MAX).unwrap()),
        };
        assert_eq!(allocator.allocate().unwrap().get(), u64::MAX);
        assert_eq!(
            allocator.allocate().unwrap_err().kind,
            CapabilityErrorKind::IdentitySpaceExhausted
        );
    }
}
