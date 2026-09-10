use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use rarog_url::{Origin, UrlError, WebUrl};

static NEXT_SERVICE_WORKER_REGISTRY_SCOPE: AtomicU64 = AtomicU64::new(1);

pub const DEFAULT_MAX_SERVICE_WORKER_REGISTRATIONS: usize = 1024;
pub const DEFAULT_MAX_SERVICE_WORKER_REGISTRATIONS_PER_ORIGIN: usize = 64;
pub const DEFAULT_MAX_SERVICE_WORKER_VERSIONS: usize = 2048;
pub const DEFAULT_MAX_SERVICE_WORKER_URL_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceWorkerLimits {
    pub max_registrations: usize,
    pub max_registrations_per_origin: usize,
    pub max_versions: usize,
    pub max_url_bytes: usize,
}

impl ServiceWorkerLimits {
    pub fn is_valid(self) -> bool {
        self.max_registrations > 0
            && self.max_registrations_per_origin > 0
            && self.max_registrations_per_origin <= self.max_registrations
            && self.max_versions > 0
            && self.max_url_bytes > 0
    }
}

impl Default for ServiceWorkerLimits {
    fn default() -> Self {
        Self {
            max_registrations: DEFAULT_MAX_SERVICE_WORKER_REGISTRATIONS,
            max_registrations_per_origin: DEFAULT_MAX_SERVICE_WORKER_REGISTRATIONS_PER_ORIGIN,
            max_versions: DEFAULT_MAX_SERVICE_WORKER_VERSIONS,
            max_url_bytes: DEFAULT_MAX_SERVICE_WORKER_URL_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceWorkerRegistrationId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl ServiceWorkerRegistrationId {
    pub fn scope(self) -> u64 {
        self.scope.get()
    }

    pub fn serial(self) -> u64 {
        self.serial.get()
    }
}

impl fmt::Display for ServiceWorkerRegistrationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "service-worker-registration:{}:{}",
            self.scope(),
            self.serial()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceWorkerVersionId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl ServiceWorkerVersionId {
    pub fn scope(self) -> u64 {
        self.scope.get()
    }

    pub fn serial(self) -> u64 {
        self.serial.get()
    }
}

impl fmt::Display for ServiceWorkerVersionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "service-worker-version:{}:{}",
            self.scope(),
            self.serial()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceWorkerVersionState {
    Installing,
    Installed,
    Activating,
    Activated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWorkerScope {
    origin: Origin,
    url: WebUrl,
}

impl ServiceWorkerScope {
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn url(&self) -> &WebUrl {
        &self.url
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWorkerRegistration {
    id: ServiceWorkerRegistrationId,
    scope: ServiceWorkerScope,
    installing: Option<ServiceWorkerVersionId>,
    waiting: Option<ServiceWorkerVersionId>,
    active: Option<ServiceWorkerVersionId>,
}

impl ServiceWorkerRegistration {
    pub fn id(&self) -> ServiceWorkerRegistrationId {
        self.id
    }

    pub fn scope(&self) -> &ServiceWorkerScope {
        &self.scope
    }

    pub fn installing(&self) -> Option<ServiceWorkerVersionId> {
        self.installing
    }

    pub fn waiting(&self) -> Option<ServiceWorkerVersionId> {
        self.waiting
    }

    pub fn active(&self) -> Option<ServiceWorkerVersionId> {
        self.active
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceWorkerVersion {
    id: ServiceWorkerVersionId,
    registration: ServiceWorkerRegistrationId,
    script_url: WebUrl,
    state: ServiceWorkerVersionState,
}

impl ServiceWorkerVersion {
    pub fn id(&self) -> ServiceWorkerVersionId {
        self.id
    }

    pub fn registration(&self) -> ServiceWorkerRegistrationId {
        self.registration
    }

    pub fn script_url(&self) -> &WebUrl {
        &self.script_url
    }

    pub fn state(&self) -> ServiceWorkerVersionState {
        self.state
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceWorkerRegistrationUpdate {
    registration: ServiceWorkerRegistrationId,
    version: ServiceWorkerVersionId,
    created_registration: bool,
    discarded_versions: usize,
}

impl ServiceWorkerRegistrationUpdate {
    pub fn registration(self) -> ServiceWorkerRegistrationId {
        self.registration
    }

    pub fn version(self) -> ServiceWorkerVersionId {
        self.version
    }

    pub fn created_registration(self) -> bool {
        self.created_registration
    }

    pub fn discarded_versions(self) -> usize {
        self.discarded_versions
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceWorkerDiscard {
    state: ServiceWorkerVersionState,
    registration_removed: bool,
}

impl ServiceWorkerDiscard {
    pub fn state(self) -> ServiceWorkerVersionState {
        self.state
    }

    pub fn registration_removed(self) -> bool {
        self.registration_removed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceWorkerError {
    InvalidLimits,
    Url(UrlError),
    OpaqueOrigin,
    UnsupportedScheme,
    OriginMismatch,
    ForbiddenPathEscape,
    UrlTooLong {
        bytes: usize,
        limit: usize,
    },
    RegistrationLimitExceeded,
    OriginRegistrationLimitExceeded,
    VersionLimitExceeded,
    IdentitySpaceExhausted,
    UnknownRegistration,
    UnknownVersion,
    InvalidVersionState {
        version: ServiceWorkerVersionId,
        state: ServiceWorkerVersionState,
    },
    InvalidRegistrationSlot,
}

impl fmt::Display for ServiceWorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("Service Worker limits are invalid"),
            Self::Url(error) => error.fmt(formatter),
            Self::OpaqueOrigin => {
                formatter.write_str("opaque origins cannot own Service Worker registrations")
            }
            Self::UnsupportedScheme => {
                formatter.write_str("Service Worker scope and script URLs must use HTTP(S)")
            }
            Self::OriginMismatch => {
                formatter.write_str("Service Worker scope or script URL is cross-origin")
            }
            Self::ForbiddenPathEscape => formatter
                .write_str("Service Worker URL path contains forbidden escaped slash or backslash"),
            Self::UrlTooLong { bytes, limit } => {
                write!(
                    formatter,
                    "Service Worker URL requires {bytes} bytes; limit is {limit}"
                )
            }
            Self::RegistrationLimitExceeded => {
                formatter.write_str("Service Worker registration limit exceeded")
            }
            Self::OriginRegistrationLimitExceeded => {
                formatter.write_str("Service Worker per-origin registration limit exceeded")
            }
            Self::VersionLimitExceeded => {
                formatter.write_str("Service Worker live-version limit exceeded")
            }
            Self::IdentitySpaceExhausted => {
                formatter.write_str("Service Worker identity space is exhausted")
            }
            Self::UnknownRegistration => {
                formatter.write_str("Service Worker registration is unknown or stale")
            }
            Self::UnknownVersion => {
                formatter.write_str("Service Worker version is unknown or stale")
            }
            Self::InvalidVersionState { version, state } => {
                write!(
                    formatter,
                    "Service Worker version {version} has invalid state {state:?}"
                )
            }
            Self::InvalidRegistrationSlot => {
                formatter.write_str("Service Worker registration/version ownership is stale")
            }
        }
    }
}

impl std::error::Error for ServiceWorkerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Url(error) => Some(error),
            _ => None,
        }
    }
}

impl From<UrlError> for ServiceWorkerError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

#[derive(Debug)]
struct ServiceWorkerIdAllocator {
    scope: NonZeroU64,
    next_registration: u64,
    next_version: u64,
}

impl ServiceWorkerIdAllocator {
    fn new() -> Result<Self, ServiceWorkerError> {
        let scope = NEXT_SERVICE_WORKER_REGISTRY_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| ServiceWorkerError::IdentitySpaceExhausted)?;
        let scope = NonZeroU64::new(scope).ok_or(ServiceWorkerError::IdentitySpaceExhausted)?;
        Ok(Self {
            scope,
            next_registration: 1,
            next_version: 1,
        })
    }

    fn registration(&mut self) -> Result<ServiceWorkerRegistrationId, ServiceWorkerError> {
        let serial = NonZeroU64::new(self.next_registration)
            .ok_or(ServiceWorkerError::IdentitySpaceExhausted)?;
        self.next_registration = self.next_registration.checked_add(1).unwrap_or(0);
        Ok(ServiceWorkerRegistrationId {
            scope: self.scope,
            serial,
        })
    }

    fn version(&mut self) -> Result<ServiceWorkerVersionId, ServiceWorkerError> {
        let serial =
            NonZeroU64::new(self.next_version).ok_or(ServiceWorkerError::IdentitySpaceExhausted)?;
        self.next_version = self.next_version.checked_add(1).unwrap_or(0);
        Ok(ServiceWorkerVersionId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Debug)]
pub struct ServiceWorkerRegistry {
    limits: ServiceWorkerLimits,
    allocator: ServiceWorkerIdAllocator,
    registrations: BTreeMap<ServiceWorkerRegistrationId, ServiceWorkerRegistration>,
    versions: BTreeMap<ServiceWorkerVersionId, ServiceWorkerVersion>,
}

impl ServiceWorkerRegistry {
    pub fn try_new(limits: ServiceWorkerLimits) -> Result<Self, ServiceWorkerError> {
        if !limits.is_valid() {
            return Err(ServiceWorkerError::InvalidLimits);
        }
        Ok(Self {
            limits,
            allocator: ServiceWorkerIdAllocator::new()?,
            registrations: BTreeMap::new(),
            versions: BTreeMap::new(),
        })
    }

    pub fn with_default_limits() -> Result<Self, ServiceWorkerError> {
        Self::try_new(ServiceWorkerLimits::default())
    }

    pub fn limits(&self) -> ServiceWorkerLimits {
        self.limits
    }

    pub fn registration_count(&self) -> usize {
        self.registrations.len()
    }

    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    pub fn registration(
        &self,
        id: ServiceWorkerRegistrationId,
    ) -> Result<&ServiceWorkerRegistration, ServiceWorkerError> {
        self.registrations
            .get(&id)
            .ok_or(ServiceWorkerError::UnknownRegistration)
    }

    pub fn version(
        &self,
        id: ServiceWorkerVersionId,
    ) -> Result<&ServiceWorkerVersion, ServiceWorkerError> {
        self.versions
            .get(&id)
            .ok_or(ServiceWorkerError::UnknownVersion)
    }

    pub fn register(
        &mut self,
        origin: &Origin,
        scope_url: &WebUrl,
        script_url: &WebUrl,
    ) -> Result<ServiceWorkerRegistrationUpdate, ServiceWorkerError> {
        let scope = self.canonical_scope(origin, scope_url)?;
        let script_url = self.canonical_url_for_origin(origin, script_url)?;

        let existing = self
            .registrations
            .values()
            .find(|registration| registration.scope == scope)
            .map(|registration| registration.id);

        match existing {
            Some(registration) => self.update_registration(registration, script_url),
            None => self.create_registration(scope, script_url),
        }
    }

    pub fn finish_install(
        &mut self,
        version: ServiceWorkerVersionId,
    ) -> Result<(), ServiceWorkerError> {
        let registration = self.validate_slot(
            version,
            ServiceWorkerVersionState::Installing,
            RegistrationSlot::Installing,
        )?;
        let previous_waiting = self
            .registrations
            .get(&registration)
            .expect("validated Service Worker registration must remain present")
            .waiting;
        if let Some(waiting) = previous_waiting {
            let waiting_record = self
                .versions
                .get(&waiting)
                .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
            if waiting == version
                || waiting_record.registration != registration
                || waiting_record.state != ServiceWorkerVersionState::Installed
            {
                return Err(ServiceWorkerError::InvalidRegistrationSlot);
            }
        }

        if let Some(waiting) = previous_waiting {
            self.versions
                .remove(&waiting)
                .expect("validated waiting Service Worker version must remain present");
        }
        self.versions
            .get_mut(&version)
            .expect("validated Service Worker version must remain present")
            .state = ServiceWorkerVersionState::Installed;
        let registration = self
            .registrations
            .get_mut(&registration)
            .expect("validated Service Worker registration must remain present");
        registration.installing = None;
        registration.waiting = Some(version);
        Ok(())
    }

    pub fn begin_activate(
        &mut self,
        version: ServiceWorkerVersionId,
    ) -> Result<Option<ServiceWorkerVersionId>, ServiceWorkerError> {
        let registration = self.validate_slot(
            version,
            ServiceWorkerVersionState::Installed,
            RegistrationSlot::Waiting,
        )?;
        let previous_active = self
            .registrations
            .get(&registration)
            .expect("validated Service Worker registration must remain present")
            .active;
        if let Some(previous) = previous_active {
            let previous_record = self
                .versions
                .get(&previous)
                .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
            if previous_record.registration != registration {
                return Err(ServiceWorkerError::InvalidRegistrationSlot);
            }
            match previous_record.state {
                ServiceWorkerVersionState::Activated => {}
                ServiceWorkerVersionState::Activating => {
                    return Err(ServiceWorkerError::InvalidVersionState {
                        version: previous,
                        state: ServiceWorkerVersionState::Activating,
                    });
                }
                ServiceWorkerVersionState::Installing | ServiceWorkerVersionState::Installed => {
                    return Err(ServiceWorkerError::InvalidRegistrationSlot);
                }
            }
        }

        if let Some(previous) = previous_active {
            self.versions.remove(&previous);
        }
        self.versions
            .get_mut(&version)
            .expect("validated Service Worker version must remain present")
            .state = ServiceWorkerVersionState::Activating;
        let registration_record = self
            .registrations
            .get_mut(&registration)
            .expect("validated Service Worker registration must remain present");
        registration_record.waiting = None;
        registration_record.active = Some(version);
        Ok(previous_active)
    }

    pub fn finish_activate(
        &mut self,
        version: ServiceWorkerVersionId,
    ) -> Result<(), ServiceWorkerError> {
        self.validate_slot(
            version,
            ServiceWorkerVersionState::Activating,
            RegistrationSlot::Active,
        )?;
        self.versions
            .get_mut(&version)
            .expect("validated Service Worker version must remain present")
            .state = ServiceWorkerVersionState::Activated;
        Ok(())
    }

    pub fn discard_version(
        &mut self,
        version: ServiceWorkerVersionId,
    ) -> Result<ServiceWorkerDiscard, ServiceWorkerError> {
        let current = self
            .versions
            .get(&version)
            .ok_or(ServiceWorkerError::UnknownVersion)?;
        let registration_id = current.registration;
        let state = current.state;
        let registration = self
            .registrations
            .get(&registration_id)
            .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
        if registration.installing != Some(version)
            && registration.waiting != Some(version)
            && registration.active != Some(version)
        {
            return Err(ServiceWorkerError::InvalidRegistrationSlot);
        }

        self.versions.remove(&version);
        let registration = self
            .registrations
            .get_mut(&registration_id)
            .expect("validated Service Worker registration must remain present");
        if registration.installing == Some(version) {
            registration.installing = None;
        }
        if registration.waiting == Some(version) {
            registration.waiting = None;
        }
        if registration.active == Some(version) {
            registration.active = None;
        }
        let registration_removed = registration.installing.is_none()
            && registration.waiting.is_none()
            && registration.active.is_none();
        if registration_removed {
            self.registrations.remove(&registration_id);
        }
        Ok(ServiceWorkerDiscard {
            state,
            registration_removed,
        })
    }

    pub fn unregister(
        &mut self,
        registration: ServiceWorkerRegistrationId,
    ) -> Result<usize, ServiceWorkerError> {
        if self.registrations.remove(&registration).is_none() {
            return Err(ServiceWorkerError::UnknownRegistration);
        }
        let before = self.versions.len();
        self.versions
            .retain(|_, version| version.registration != registration);
        Ok(before - self.versions.len())
    }

    pub fn match_registration(
        &self,
        client_url: &WebUrl,
    ) -> Result<Option<ServiceWorkerRegistrationId>, ServiceWorkerError> {
        let client = self.canonical_client_url(client_url)?;
        let client_origin = client.origin()?;
        if client_origin.is_opaque() {
            return Ok(None);
        }
        let client_serialized = client.as_str();
        let mut best: Option<(usize, ServiceWorkerRegistrationId)> = None;
        for registration in self.registrations.values() {
            if registration.scope.origin != client_origin {
                continue;
            }
            let scope = registration.scope.url.as_str();
            if !client_serialized.starts_with(scope) {
                continue;
            }
            let length = scope.len();
            if best.is_none_or(|(best_length, _)| length > best_length) {
                best = Some((length, registration.id));
            }
        }
        Ok(best.map(|(_, registration)| registration))
    }

    fn create_registration(
        &mut self,
        scope: ServiceWorkerScope,
        script_url: WebUrl,
    ) -> Result<ServiceWorkerRegistrationUpdate, ServiceWorkerError> {
        if self.registrations.len() >= self.limits.max_registrations {
            return Err(ServiceWorkerError::RegistrationLimitExceeded);
        }
        let origin_registrations = self
            .registrations
            .values()
            .filter(|registration| registration.scope.origin == scope.origin)
            .count();
        if origin_registrations >= self.limits.max_registrations_per_origin {
            return Err(ServiceWorkerError::OriginRegistrationLimitExceeded);
        }
        if self.versions.len() >= self.limits.max_versions {
            return Err(ServiceWorkerError::VersionLimitExceeded);
        }

        let registration = self.allocator.registration()?;
        let version = self.allocator.version()?;
        self.versions.insert(
            version,
            ServiceWorkerVersion {
                id: version,
                registration,
                script_url,
                state: ServiceWorkerVersionState::Installing,
            },
        );
        self.registrations.insert(
            registration,
            ServiceWorkerRegistration {
                id: registration,
                scope,
                installing: Some(version),
                waiting: None,
                active: None,
            },
        );
        Ok(ServiceWorkerRegistrationUpdate {
            registration,
            version,
            created_registration: true,
            discarded_versions: 0,
        })
    }

    fn update_registration(
        &mut self,
        registration: ServiceWorkerRegistrationId,
        script_url: WebUrl,
    ) -> Result<ServiceWorkerRegistrationUpdate, ServiceWorkerError> {
        let current = self
            .registrations
            .get(&registration)
            .ok_or(ServiceWorkerError::UnknownRegistration)?;
        if current.installing.is_some() && current.installing == current.waiting {
            return Err(ServiceWorkerError::InvalidRegistrationSlot);
        }
        if let Some(waiting) = current.waiting {
            let candidate = self
                .versions
                .get(&waiting)
                .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
            if candidate.registration != registration
                || candidate.state != ServiceWorkerVersionState::Installed
            {
                return Err(ServiceWorkerError::InvalidRegistrationSlot);
            }
        }
        if let Some(active) = current.active {
            let candidate = self
                .versions
                .get(&active)
                .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
            if candidate.registration != registration
                || !matches!(
                    candidate.state,
                    ServiceWorkerVersionState::Activating | ServiceWorkerVersionState::Activated
                )
            {
                return Err(ServiceWorkerError::InvalidRegistrationSlot);
            }
        }

        let replaceable = current.installing.into_iter().collect::<Vec<_>>();
        let remaining_versions = self
            .versions
            .len()
            .checked_sub(replaceable.len())
            .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
        let projected_versions = remaining_versions
            .checked_add(1)
            .ok_or(ServiceWorkerError::VersionLimitExceeded)?;
        if projected_versions > self.limits.max_versions {
            return Err(ServiceWorkerError::VersionLimitExceeded);
        }
        for version in &replaceable {
            let candidate = self
                .versions
                .get(version)
                .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
            if candidate.registration != registration
                || candidate.state != ServiceWorkerVersionState::Installing
            {
                return Err(ServiceWorkerError::InvalidRegistrationSlot);
            }
        }

        let version = self.allocator.version()?;
        for stale in &replaceable {
            self.versions.remove(stale);
        }
        self.versions.insert(
            version,
            ServiceWorkerVersion {
                id: version,
                registration,
                script_url,
                state: ServiceWorkerVersionState::Installing,
            },
        );
        let current = self
            .registrations
            .get_mut(&registration)
            .expect("validated Service Worker registration must remain present");
        current.installing = Some(version);
        Ok(ServiceWorkerRegistrationUpdate {
            registration,
            version,
            created_registration: false,
            discarded_versions: replaceable.len(),
        })
    }

    fn validate_slot(
        &self,
        version: ServiceWorkerVersionId,
        expected_state: ServiceWorkerVersionState,
        slot: RegistrationSlot,
    ) -> Result<ServiceWorkerRegistrationId, ServiceWorkerError> {
        let version_record = self
            .versions
            .get(&version)
            .ok_or(ServiceWorkerError::UnknownVersion)?;
        if version_record.state != expected_state {
            return Err(ServiceWorkerError::InvalidVersionState {
                version,
                state: version_record.state,
            });
        }
        let registration = self
            .registrations
            .get(&version_record.registration)
            .ok_or(ServiceWorkerError::InvalidRegistrationSlot)?;
        let current = match slot {
            RegistrationSlot::Installing => registration.installing,
            RegistrationSlot::Waiting => registration.waiting,
            RegistrationSlot::Active => registration.active,
        };
        if current != Some(version) {
            return Err(ServiceWorkerError::InvalidRegistrationSlot);
        }
        Ok(version_record.registration)
    }

    fn canonical_scope(
        &self,
        origin: &Origin,
        url: &WebUrl,
    ) -> Result<ServiceWorkerScope, ServiceWorkerError> {
        let url = self.canonical_url_for_origin(origin, url)?;
        Ok(ServiceWorkerScope {
            origin: origin.clone(),
            url,
        })
    }

    fn canonical_url_for_origin(
        &self,
        origin: &Origin,
        url: &WebUrl,
    ) -> Result<WebUrl, ServiceWorkerError> {
        if origin.is_opaque() {
            return Err(ServiceWorkerError::OpaqueOrigin);
        }
        if !matches!(url.scheme(), "http" | "https") {
            return Err(ServiceWorkerError::UnsupportedScheme);
        }
        let canonical = self.canonical_fragmentless_url(url)?;
        if has_forbidden_path_escape(canonical.path()) {
            return Err(ServiceWorkerError::ForbiddenPathEscape);
        }
        if canonical.origin()? != *origin {
            return Err(ServiceWorkerError::OriginMismatch);
        }
        Ok(canonical)
    }

    fn canonical_client_url(&self, url: &WebUrl) -> Result<WebUrl, ServiceWorkerError> {
        self.canonical_fragmentless_url(url)
    }

    fn canonical_fragmentless_url(&self, url: &WebUrl) -> Result<WebUrl, ServiceWorkerError> {
        let serialized = url.as_str();
        let bytes = serialized.find('#').unwrap_or(serialized.len());
        if bytes > self.limits.max_url_bytes {
            return Err(ServiceWorkerError::UrlTooLong {
                bytes,
                limit: self.limits.max_url_bytes,
            });
        }
        WebUrl::parse(&serialized[..bytes]).map_err(ServiceWorkerError::from)
    }
}

fn has_forbidden_path_escape(path: &str) -> bool {
    path.as_bytes().windows(3).any(|window| {
        window[0] == b'%'
            && ((window[1] == b'2' && (window[2] == b'f' || window[2] == b'F'))
                || (window[1] == b'5' && (window[2] == b'c' || window[2] == b'C')))
    })
}

#[derive(Clone, Copy)]
enum RegistrationSlot {
    Installing,
    Waiting,
    Active,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(
        max_registrations: usize,
        max_registrations_per_origin: usize,
        max_versions: usize,
        max_url_bytes: usize,
    ) -> ServiceWorkerLimits {
        ServiceWorkerLimits {
            max_registrations,
            max_registrations_per_origin,
            max_versions,
            max_url_bytes,
        }
    }

    fn url(input: &str) -> WebUrl {
        WebUrl::parse(input).unwrap()
    }

    fn origin(input: &str) -> Origin {
        url(input).origin().unwrap()
    }

    fn activate(registry: &mut ServiceWorkerRegistry, version: ServiceWorkerVersionId) {
        registry.finish_install(version).unwrap();
        registry.begin_activate(version).unwrap();
        registry.finish_activate(version).unwrap();
    }

    #[test]
    fn default_limits_are_explicit_and_valid() {
        let limits = ServiceWorkerLimits::default();
        assert!(limits.is_valid());
        assert!(limits.max_registrations_per_origin <= limits.max_registrations);
    }

    #[test]
    fn invalid_limits_fail_before_registry_creation() {
        assert_eq!(
            ServiceWorkerRegistry::try_new(limits(1, 0, 1, 1)).unwrap_err(),
            ServiceWorkerError::InvalidLimits
        );
        assert_eq!(
            ServiceWorkerRegistry::try_new(limits(1, 2, 1, 1)).unwrap_err(),
            ServiceWorkerError::InvalidLimits
        );
    }

    #[test]
    fn opaque_cross_origin_and_non_http_urls_fail_without_state() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let tuple = origin("https://example.com/");
        let opaque = origin("data:text/plain,opaque");

        assert_eq!(
            registry
                .register(
                    &opaque,
                    &url("https://example.com/"),
                    &url("https://example.com/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::OpaqueOrigin
        );
        assert_eq!(
            registry
                .register(
                    &tuple,
                    &url("https://other.example/"),
                    &url("https://example.com/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::OriginMismatch
        );
        assert_eq!(
            registry
                .register(
                    &tuple,
                    &url("https://example.com/"),
                    &url("https://other.example/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::OriginMismatch
        );
        assert_eq!(
            registry
                .register(
                    &origin("ftp://example.com/"),
                    &url("ftp://example.com/"),
                    &url("ftp://example.com/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::UnsupportedScheme
        );
        assert_eq!(registry.registration_count(), 0);
        assert_eq!(registry.version_count(), 0);
    }

    #[test]
    fn escaped_slash_or_backslash_in_scope_or_script_path_is_rejected() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        for scope in [
            "https://example.com/a%2Fb/",
            "https://example.com/a%2fb/",
            "https://example.com/a%5Cb/",
            "https://example.com/a%5cb/",
        ] {
            assert_eq!(
                registry
                    .register(&owner, &url(scope), &url("https://example.com/sw.js"))
                    .unwrap_err(),
                ServiceWorkerError::ForbiddenPathEscape
            );
        }
        assert_eq!(
            registry
                .register(
                    &owner,
                    &url("https://example.com/app/"),
                    &url("https://example.com/a%2Fsw.js"),
                )
                .unwrap_err(),
            ServiceWorkerError::ForbiddenPathEscape
        );
        assert_eq!(registry.registration_count(), 0);
        assert_eq!(registry.version_count(), 0);
    }

    #[test]
    fn scope_and_script_fragments_are_removed_before_retention() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        let update = registry
            .register(
                &owner,
                &url("https://example.com/app/#scope-fragment"),
                &url("https://example.com/sw.js?build=1#script-fragment"),
            )
            .unwrap();
        let registration = registry.registration(update.registration()).unwrap();
        assert_eq!(
            registration.scope().url().as_str(),
            "https://example.com/app/"
        );
        assert_eq!(registration.scope().origin(), &owner);
        assert_eq!(
            registry
                .version(update.version())
                .unwrap()
                .script_url()
                .as_str(),
            "https://example.com/sw.js?build=1"
        );
    }

    #[test]
    fn exact_scope_update_preserves_active_until_replacement_commits() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/app/"),
                &url("https://example.com/sw-v1.js"),
            )
            .unwrap();
        activate(&mut registry, first.version());

        let second = registry
            .register(
                &owner,
                &url("https://example.com/app/#ignored"),
                &url("https://example.com/sw-v2.js"),
            )
            .unwrap();
        assert!(!second.created_registration());
        assert_eq!(second.registration(), first.registration());
        assert_eq!(registry.registration_count(), 1);
        assert_eq!(
            registry
                .registration(first.registration())
                .unwrap()
                .active(),
            Some(first.version())
        );
        assert_eq!(
            registry
                .registration(first.registration())
                .unwrap()
                .installing(),
            Some(second.version())
        );

        registry.finish_install(second.version()).unwrap();
        assert_eq!(
            registry.begin_activate(second.version()).unwrap(),
            Some(first.version())
        );
        assert_eq!(
            registry
                .registration(first.registration())
                .unwrap()
                .waiting(),
            None
        );
        assert_eq!(
            registry
                .registration(first.registration())
                .unwrap()
                .active(),
            Some(second.version())
        );
        assert_eq!(
            registry.version(first.version()).unwrap_err(),
            ServiceWorkerError::UnknownVersion
        );
        assert_eq!(
            registry.version(second.version()).unwrap().state(),
            ServiceWorkerVersionState::Activating
        );
        registry.finish_activate(second.version()).unwrap();
        assert_eq!(
            registry.version(second.version()).unwrap().state(),
            ServiceWorkerVersionState::Activated
        );
        assert_eq!(registry.version_count(), 1);
    }

    #[test]
    fn successful_install_retires_previous_waiting_and_recovers_capacity() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 2, 256)).unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v1.js"),
            )
            .unwrap();
        registry.finish_install(first.version()).unwrap();
        let second = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v2.js"),
            )
            .unwrap();
        assert_eq!(second.discarded_versions(), 0);
        assert_eq!(registry.version_count(), 2);
        let registration = registry.registration(first.registration()).unwrap();
        assert_eq!(registration.waiting(), Some(first.version()));
        assert_eq!(registration.installing(), Some(second.version()));

        registry.finish_install(second.version()).unwrap();
        assert_eq!(
            registry.version(first.version()).unwrap_err(),
            ServiceWorkerError::UnknownVersion
        );
        assert_eq!(registry.version_count(), 1);
        let registration = registry.registration(first.registration()).unwrap();
        assert_eq!(registration.waiting(), Some(second.version()));
        assert_eq!(registration.installing(), None);
    }

    #[test]
    fn failed_replacement_install_preserves_previous_waiting_version() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 2, 256)).unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v1.js"),
            )
            .unwrap();
        registry.finish_install(first.version()).unwrap();
        let second = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v2.js"),
            )
            .unwrap();
        let discarded = registry.discard_version(second.version()).unwrap();
        assert_eq!(discarded.state(), ServiceWorkerVersionState::Installing);
        assert!(!discarded.registration_removed());
        let registration = registry.registration(first.registration()).unwrap();
        assert_eq!(registration.waiting(), Some(first.version()));
        assert_eq!(registration.installing(), None);
        assert_eq!(
            registry.version(first.version()).unwrap().state(),
            ServiceWorkerVersionState::Installed
        );
    }

    #[test]
    fn activation_is_blocked_while_current_active_is_still_activating() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 2, 256)).unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v1.js"),
            )
            .unwrap();
        registry.finish_install(first.version()).unwrap();
        registry.begin_activate(first.version()).unwrap();

        let second = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/v2.js"),
            )
            .unwrap();
        registry.finish_install(second.version()).unwrap();
        assert_eq!(
            registry.begin_activate(second.version()).unwrap_err(),
            ServiceWorkerError::InvalidVersionState {
                version: first.version(),
                state: ServiceWorkerVersionState::Activating,
            }
        );
        let registration = registry.registration(first.registration()).unwrap();
        assert_eq!(registration.active(), Some(first.version()));
        assert_eq!(registration.waiting(), Some(second.version()));
        assert_eq!(
            registry.version(second.version()).unwrap().state(),
            ServiceWorkerVersionState::Installed
        );

        registry.finish_activate(first.version()).unwrap();
        assert_eq!(
            registry.begin_activate(second.version()).unwrap(),
            Some(first.version())
        );
        assert_eq!(
            registry.version(first.version()).unwrap_err(),
            ServiceWorkerError::UnknownVersion
        );
    }

    #[test]
    fn invalid_lifecycle_transition_is_rejected_without_slot_mutation() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        let update = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/sw.js"),
            )
            .unwrap();
        assert_eq!(
            registry.begin_activate(update.version()).unwrap_err(),
            ServiceWorkerError::InvalidVersionState {
                version: update.version(),
                state: ServiceWorkerVersionState::Installing,
            }
        );
        assert_eq!(
            registry
                .registration(update.registration())
                .unwrap()
                .installing(),
            Some(update.version())
        );
        assert_eq!(
            registry.version(update.version()).unwrap().state(),
            ServiceWorkerVersionState::Installing
        );
    }

    #[test]
    fn longest_same_origin_scope_match_is_deterministic() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        let root = registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/root.js"),
            )
            .unwrap();
        let app = registry
            .register(
                &owner,
                &url("https://example.com/app/"),
                &url("https://example.com/app.js"),
            )
            .unwrap();
        assert_eq!(
            registry
                .match_registration(&url("https://example.com/app/page#fragment"))
                .unwrap(),
            Some(app.registration())
        );
        assert_eq!(
            registry
                .match_registration(&url("https://example.com/other"))
                .unwrap(),
            Some(root.registration())
        );
        assert_eq!(
            registry
                .match_registration(&url("https://other.example/app/page"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn registration_limits_are_bounded_per_origin_and_recover_on_unregister() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(2, 1, 2, 256)).unwrap();
        let first_origin = origin("https://a.example/");
        let second_origin = origin("https://b.example/");
        let first = registry
            .register(
                &first_origin,
                &url("https://a.example/a/"),
                &url("https://a.example/sw.js"),
            )
            .unwrap();
        assert_eq!(
            registry
                .register(
                    &first_origin,
                    &url("https://a.example/b/"),
                    &url("https://a.example/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::OriginRegistrationLimitExceeded
        );
        registry
            .register(
                &second_origin,
                &url("https://b.example/"),
                &url("https://b.example/sw.js"),
            )
            .unwrap();
        assert_eq!(registry.registration_count(), 2);
        assert_eq!(registry.unregister(first.registration()).unwrap(), 1);
        assert_eq!(registry.registration_count(), 1);
        registry
            .register(
                &first_origin,
                &url("https://a.example/b/"),
                &url("https://a.example/sw.js"),
            )
            .unwrap();
        assert_eq!(registry.registration_count(), 2);
    }

    #[test]
    fn url_limit_is_checked_before_retained_registration_state() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 1, 28)).unwrap();
        let owner = origin("https://example.com/");
        assert!(matches!(
            registry
                .register(
                    &owner,
                    &url("https://example.com/too-long-scope/"),
                    &url("https://example.com/sw.js")
                )
                .unwrap_err(),
            ServiceWorkerError::UrlTooLong { limit: 28, .. }
        ));
        assert_eq!(registry.registration_count(), 0);
        assert_eq!(registry.version_count(), 0);
    }

    #[test]
    fn failed_initial_version_removes_empty_registration_but_failed_update_keeps_active() {
        let mut registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/first/"),
                &url("https://example.com/first.js"),
            )
            .unwrap();
        let discarded = registry.discard_version(first.version()).unwrap();
        assert_eq!(discarded.state(), ServiceWorkerVersionState::Installing);
        assert!(discarded.registration_removed());
        assert_eq!(
            registry.registration(first.registration()).unwrap_err(),
            ServiceWorkerError::UnknownRegistration
        );

        let active = registry
            .register(
                &owner,
                &url("https://example.com/app/"),
                &url("https://example.com/v1.js"),
            )
            .unwrap();
        activate(&mut registry, active.version());
        let replacement = registry
            .register(
                &owner,
                &url("https://example.com/app/"),
                &url("https://example.com/v2.js"),
            )
            .unwrap();
        let discarded = registry.discard_version(replacement.version()).unwrap();
        assert!(!discarded.registration_removed());
        assert_eq!(
            registry
                .registration(active.registration())
                .unwrap()
                .active(),
            Some(active.version())
        );
        assert_eq!(
            registry.version(active.version()).unwrap().state(),
            ServiceWorkerVersionState::Activated
        );
    }

    #[test]
    fn unregister_and_discard_make_ids_stale_and_registry_scopes_do_not_alias() {
        let owner = origin("https://example.com/");
        let mut first_registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let mut second_registry = ServiceWorkerRegistry::with_default_limits().unwrap();
        let first = first_registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/sw.js"),
            )
            .unwrap();
        let second = second_registry
            .register(
                &owner,
                &url("https://example.com/"),
                &url("https://example.com/sw.js"),
            )
            .unwrap();
        assert_ne!(first.registration(), second.registration());
        assert_ne!(first.version(), second.version());
        assert_eq!(
            second_registry
                .registration(first.registration())
                .unwrap_err(),
            ServiceWorkerError::UnknownRegistration
        );
        assert_eq!(
            second_registry.version(first.version()).unwrap_err(),
            ServiceWorkerError::UnknownVersion
        );

        assert_eq!(first_registry.unregister(first.registration()).unwrap(), 1);
        assert_eq!(
            first_registry
                .registration(first.registration())
                .unwrap_err(),
            ServiceWorkerError::UnknownRegistration
        );
        assert_eq!(
            first_registry.version(first.version()).unwrap_err(),
            ServiceWorkerError::UnknownVersion
        );
    }

    #[test]
    fn fragment_is_removed_before_url_budget_is_applied() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 1, 64)).unwrap();
        let owner = origin("https://example.com/");
        let scope = url(&format!(
            "https://example.com/app/#{}",
            "scope-fragment".repeat(32)
        ));
        let script = url(&format!(
            "https://example.com/sw.js#{}",
            "script-fragment".repeat(32)
        ));
        let update = registry.register(&owner, &scope, &script).unwrap();
        assert_eq!(
            registry
                .registration(update.registration())
                .unwrap()
                .scope()
                .url()
                .as_str(),
            "https://example.com/app/"
        );
        assert_eq!(
            registry
                .version(update.version())
                .unwrap()
                .script_url()
                .as_str(),
            "https://example.com/sw.js"
        );
    }

    #[test]
    fn global_registration_limit_fails_atomically_and_recovers_after_unregister() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 2, 256)).unwrap();
        let first_origin = origin("https://a.example/");
        let second_origin = origin("https://b.example/");
        let first = registry
            .register(
                &first_origin,
                &url("https://a.example/"),
                &url("https://a.example/sw.js"),
            )
            .unwrap();
        assert_eq!(
            registry
                .register(
                    &second_origin,
                    &url("https://b.example/"),
                    &url("https://b.example/sw.js"),
                )
                .unwrap_err(),
            ServiceWorkerError::RegistrationLimitExceeded
        );
        assert_eq!(registry.registration_count(), 1);
        assert_eq!(registry.version_count(), 1);
        assert_eq!(registry.unregister(first.registration()).unwrap(), 1);
        registry
            .register(
                &second_origin,
                &url("https://b.example/"),
                &url("https://b.example/sw.js"),
            )
            .unwrap();
        assert_eq!(registry.registration_count(), 1);
        assert_eq!(registry.version_count(), 1);
    }

    #[test]
    fn live_version_limit_preserves_active_registration_without_partial_update() {
        let mut registry = ServiceWorkerRegistry::try_new(limits(1, 1, 1, 256)).unwrap();
        let owner = origin("https://example.com/");
        let first = registry
            .register(
                &owner,
                &url("https://example.com/app/"),
                &url("https://example.com/v1.js"),
            )
            .unwrap();
        activate(&mut registry, first.version());

        assert_eq!(
            registry
                .register(
                    &owner,
                    &url("https://example.com/app/"),
                    &url("https://example.com/v2.js"),
                )
                .unwrap_err(),
            ServiceWorkerError::VersionLimitExceeded
        );
        let registration = registry.registration(first.registration()).unwrap();
        assert_eq!(registration.active(), Some(first.version()));
        assert_eq!(registration.installing(), None);
        assert_eq!(registration.waiting(), None);
        assert_eq!(
            registry.version(first.version()).unwrap().state(),
            ServiceWorkerVersionState::Activated
        );
        assert_eq!(registry.version_count(), 1);
    }
}
