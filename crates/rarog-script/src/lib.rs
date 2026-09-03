use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REALM_SCOPE: AtomicU64 = AtomicU64::new(1);
static NEXT_ROOT_SCOPE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealmId {
    scope: NonZeroU64,
    serial: NonZeroU64,
}

#[derive(Debug)]
pub struct RealmIdAllocator {
    scope: NonZeroU64,
    next: u64,
}

impl RealmIdAllocator {
    pub fn new() -> Result<Self, ScriptError> {
        let scope = NEXT_REALM_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                ScriptError::new(
                    ScriptErrorKind::ResourceLimit,
                    "script runtime identity space is exhausted",
                )
            })?;
        let scope = NonZeroU64::new(scope).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script runtime identity space is exhausted",
            )
        })?;
        Ok(Self { scope, next: 1 })
    }

    pub fn allocate(&mut self) -> Result<RealmId, ScriptError> {
        let serial = NonZeroU64::new(self.next).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script realm identity space is exhausted",
            )
        })?;
        self.next = self.next.checked_add(1).unwrap_or(0);
        Ok(RealmId {
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRealmLimits {
    max_source_bytes: NonZeroUsize,
    max_rooted_values: NonZeroUsize,
}

impl ScriptRealmLimits {
    pub fn try_new(max_source_bytes: usize, max_rooted_values: usize) -> Result<Self, ScriptError> {
        let max_source_bytes = NonZeroUsize::new(max_source_bytes).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::InvalidInput,
                "script source byte limit must be non-zero",
            )
        })?;
        let max_rooted_values = NonZeroUsize::new(max_rooted_values).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::InvalidInput,
                "script rooted-value limit must be non-zero",
            )
        })?;
        Ok(Self {
            max_source_bytes,
            max_rooted_values,
        })
    }

    pub fn max_source_bytes(self) -> usize {
        self.max_source_bytes.get()
    }

    pub fn max_rooted_values(self) -> usize {
        self.max_rooted_values.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalObjectId {
    realm: RealmId,
}

impl GlobalObjectId {
    pub fn realm(self) -> RealmId {
        self.realm
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRealm {
    id: RealmId,
    global: GlobalObjectId,
}

impl ScriptRealm {
    pub fn new(id: RealmId) -> Self {
        Self {
            id,
            global: GlobalObjectId { realm: id },
        }
    }

    pub fn id(self) -> RealmId {
        self.id
    }

    pub fn global(self) -> GlobalObjectId {
        self.global
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootedValueId {
    realm: RealmId,
    scope: NonZeroU64,
    serial: NonZeroU64,
}

impl RootedValueId {
    pub fn realm(self) -> RealmId {
        self.realm
    }
}

#[derive(Debug)]
pub struct RootedValueIdAllocator {
    realm: RealmId,
    scope: NonZeroU64,
    next: u64,
}

impl RootedValueIdAllocator {
    pub fn new(realm: RealmId) -> Result<Self, ScriptError> {
        let scope = NEXT_ROOT_SCOPE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                ScriptError::new(
                    ScriptErrorKind::ResourceLimit,
                    "script rooted-value allocator identity space is exhausted",
                )
            })?;
        let scope = NonZeroU64::new(scope).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script rooted-value allocator identity space is exhausted",
            )
        })?;
        Ok(Self {
            realm,
            scope,
            next: 1,
        })
    }

    pub fn allocate(&mut self) -> Result<RootedValueId, ScriptError> {
        let serial = NonZeroU64::new(self.next).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script rooted-value identity space is exhausted",
            )
        })?;
        self.next = self.next.checked_add(1).unwrap_or(0);
        Ok(RootedValueId {
            realm: self.realm,
            scope: self.scope,
            serial,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSource<'a> {
    text: &'a str,
    name: Option<&'a str>,
}

impl<'a> ScriptSource<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, name: None }
    }

    pub fn named(text: &'a str, name: &'a str) -> Self {
        Self {
            text,
            name: Some(name),
        }
    }

    pub fn text(&self) -> &'a str {
        self.text
    }

    pub fn name(&self) -> Option<&'a str> {
        self.name
    }

    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    pub fn ensure_byte_limit(self, max_bytes: usize) -> Result<Self, ScriptError> {
        if self.byte_len() > max_bytes {
            return Err(ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                format!(
                    "script source is {} bytes, exceeding the configured {max_bytes}-byte limit",
                    self.byte_len()
                ),
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptDiagnosticLevel {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSourceSpan {
    pub start: usize,
    pub end: usize,
}

impl ScriptSourceSpan {
    pub fn try_new(start: usize, end: usize) -> Result<Self, ScriptError> {
        if start > end {
            return Err(ScriptError::new(
                ScriptErrorKind::InvalidInput,
                "script diagnostic source span must have start <= end",
            ));
        }
        Ok(Self { start, end })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDiagnostic {
    pub level: ScriptDiagnosticLevel,
    pub message: String,
    pub source_name: Option<String>,
    pub span: Option<ScriptSourceSpan>,
}

impl ScriptDiagnostic {
    pub fn new(level: ScriptDiagnosticLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            source_name: None,
            span: None,
        }
    }

    pub fn with_source_name(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }

    pub fn with_span(mut self, span: ScriptSourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptErrorKind {
    InvalidInput,
    InvalidRealm,
    InvalidRoot,
    ResourceLimit,
    Backend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    pub message: String,
    pub diagnostic: Option<ScriptDiagnostic>,
}

impl ScriptError {
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            diagnostic: None,
        }
    }

    pub fn with_diagnostic(mut self, diagnostic: ScriptDiagnostic) -> Self {
        self.diagnostic = Some(diagnostic);
        self
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScriptError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptException {
    pub value: RootedValueId,
    pub message: Option<String>,
    pub stack: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptCompletion {
    Normal(RootedValueId),
    Throw(ScriptException),
}

impl ScriptCompletion {
    pub fn rooted_value(&self) -> RootedValueId {
        match self {
            Self::Normal(value) => *value,
            Self::Throw(exception) => exception.value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutcome {
    pub completion: ScriptCompletion,
    pub diagnostics: Vec<ScriptDiagnostic>,
}

pub trait ScriptRuntime {
    fn create_realm(&mut self, limits: ScriptRealmLimits) -> Result<ScriptRealm, ScriptError>;

    fn evaluate(
        &mut self,
        realm: RealmId,
        source: ScriptSource<'_>,
    ) -> Result<EvaluationOutcome, ScriptError>;

    fn duplicate_root(&mut self, value: RootedValueId) -> Result<RootedValueId, ScriptError>;

    fn release_root(&mut self, value: RootedValueId) -> Result<(), ScriptError>;

    fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError>;
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    struct FixtureRealmState {
        limits: ScriptRealmLimits,
        root_ids: RootedValueIdAllocator,
        roots: BTreeSet<RootedValueId>,
    }

    struct FixtureRuntime {
        realm_ids: RealmIdAllocator,
        realms: BTreeMap<RealmId, FixtureRealmState>,
    }

    impl FixtureRuntime {
        fn new() -> Self {
            Self {
                realm_ids: RealmIdAllocator::new().unwrap(),
                realms: BTreeMap::new(),
            }
        }

        fn state(&self, realm: RealmId) -> Result<&FixtureRealmState, ScriptError> {
            self.realms.get(&realm).ok_or_else(|| {
                ScriptError::new(ScriptErrorKind::InvalidRealm, "script realm is not live")
            })
        }

        fn state_mut(&mut self, realm: RealmId) -> Result<&mut FixtureRealmState, ScriptError> {
            self.realms.get_mut(&realm).ok_or_else(|| {
                ScriptError::new(ScriptErrorKind::InvalidRealm, "script realm is not live")
            })
        }

        fn allocate_root(&mut self, realm: RealmId) -> Result<RootedValueId, ScriptError> {
            let state = self.state_mut(realm)?;
            if state.roots.len() >= state.limits.max_rooted_values() {
                return Err(ScriptError::new(
                    ScriptErrorKind::ResourceLimit,
                    "script rooted-value limit exceeded",
                ));
            }
            let value = state.root_ids.allocate()?;
            state.roots.insert(value);
            Ok(value)
        }

        fn require_root(&self, value: RootedValueId) -> Result<(), ScriptError> {
            let state = self.state(value.realm())?;
            if state.roots.contains(&value) {
                Ok(())
            } else {
                Err(ScriptError::new(
                    ScriptErrorKind::InvalidRoot,
                    "script rooted value is not live",
                ))
            }
        }
    }

    impl ScriptRuntime for FixtureRuntime {
        fn create_realm(&mut self, limits: ScriptRealmLimits) -> Result<ScriptRealm, ScriptError> {
            let realm = self.realm_ids.allocate()?;
            let root_ids = RootedValueIdAllocator::new(realm)?;
            self.realms.insert(
                realm,
                FixtureRealmState {
                    limits,
                    root_ids,
                    roots: BTreeSet::new(),
                },
            );
            Ok(ScriptRealm::new(realm))
        }

        fn evaluate(
            &mut self,
            realm: RealmId,
            source: ScriptSource<'_>,
        ) -> Result<EvaluationOutcome, ScriptError> {
            let limit = self.state(realm)?.limits.max_source_bytes();
            source.ensure_byte_limit(limit)?;
            let value = self.allocate_root(realm)?;
            let completion = if source.text() == "throw fixture" {
                ScriptCompletion::Throw(ScriptException {
                    value,
                    message: Some(String::from("fixture exception")),
                    stack: Some(String::from("fixture.js:1")),
                })
            } else {
                ScriptCompletion::Normal(value)
            };
            let mut diagnostics = Vec::new();
            if let Some(name) = source.name() {
                diagnostics.push(
                    ScriptDiagnostic::new(ScriptDiagnosticLevel::Warning, "fixture diagnostic")
                        .with_source_name(name),
                );
            }
            Ok(EvaluationOutcome {
                completion,
                diagnostics,
            })
        }

        fn duplicate_root(&mut self, value: RootedValueId) -> Result<RootedValueId, ScriptError> {
            self.require_root(value)?;
            self.allocate_root(value.realm())
        }

        fn release_root(&mut self, value: RootedValueId) -> Result<(), ScriptError> {
            let state = self.state_mut(value.realm())?;
            if state.roots.remove(&value) {
                Ok(())
            } else {
                Err(ScriptError::new(
                    ScriptErrorKind::InvalidRoot,
                    "script rooted value is not live",
                ))
            }
        }

        fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError> {
            if self.realms.remove(&realm).is_some() {
                Ok(())
            } else {
                Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "script realm is not live",
                ))
            }
        }
    }

    fn limits() -> ScriptRealmLimits {
        ScriptRealmLimits::try_new(1024, 8).unwrap()
    }

    #[test]
    fn realm_ids_are_nonzero_and_monotonic_within_one_runtime() {
        let mut ids = RealmIdAllocator::new().unwrap();
        let first = ids.allocate().unwrap();
        let second = ids.allocate().unwrap();
        assert_eq!(first.scope, second.scope);
        assert_eq!(first.serial.get(), 1);
        assert_eq!(second.serial.get(), 2);
    }

    #[test]
    fn realm_ids_from_different_runtimes_do_not_alias() {
        let first = RealmIdAllocator::new().unwrap().allocate().unwrap();
        let second = RealmIdAllocator::new().unwrap().allocate().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn realm_id_allocator_fails_after_exhaustion() {
        let mut ids = RealmIdAllocator::new().unwrap();
        ids.next = u64::MAX;
        assert_eq!(ids.allocate().unwrap().serial.get(), u64::MAX);
        let error = ids.allocate().unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
    }

    #[test]
    fn rooted_value_ids_from_independent_allocators_do_not_alias() {
        let realm = RealmIdAllocator::new().unwrap().allocate().unwrap();
        let first = RootedValueIdAllocator::new(realm)
            .unwrap()
            .allocate()
            .unwrap();
        let second = RootedValueIdAllocator::new(realm)
            .unwrap()
            .allocate()
            .unwrap();
        assert_eq!(first.realm(), realm);
        assert_eq!(second.realm(), realm);
        assert_ne!(first.scope, second.scope);
        assert_ne!(first, second);
    }

    #[test]
    fn rooted_value_id_allocator_fails_after_exhaustion() {
        let realm = RealmIdAllocator::new().unwrap().allocate().unwrap();
        let mut ids = RootedValueIdAllocator::new(realm).unwrap();
        ids.next = u64::MAX;
        assert_eq!(ids.allocate().unwrap().serial.get(), u64::MAX);
        let error = ids.allocate().unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
    }

    #[test]
    fn realm_limits_reject_zero_values() {
        let source_error = ScriptRealmLimits::try_new(0, 1).unwrap_err();
        assert_eq!(source_error.kind, ScriptErrorKind::InvalidInput);
        let roots_error = ScriptRealmLimits::try_new(1, 0).unwrap_err();
        assert_eq!(roots_error.kind, ScriptErrorKind::InvalidInput);
    }

    #[test]
    fn runtime_contract_is_object_safe() {
        let mut runtime = FixtureRuntime::new();
        let runtime: &mut dyn ScriptRuntime = &mut runtime;
        let realm = runtime.create_realm(limits()).unwrap();
        let outcome = runtime
            .evaluate(realm.id(), ScriptSource::new("1 + 1"))
            .unwrap();
        assert!(matches!(outcome.completion, ScriptCompletion::Normal(_)));
        runtime
            .release_root(outcome.completion.rooted_value())
            .unwrap();
        runtime.destroy_realm(realm.id()).unwrap();
    }

    #[test]
    fn realm_global_identity_is_bound_to_its_realm() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        assert_eq!(realm.global().realm(), realm.id());
    }

    #[test]
    fn evaluation_diagnostics_are_owned() {
        let diagnostic = {
            let name = String::from("fixture.js");
            let source_text = String::from("let value = 1;");
            let mut runtime = FixtureRuntime::new();
            let realm = runtime.create_realm(limits()).unwrap();
            runtime
                .evaluate(realm.id(), ScriptSource::named(&source_text, &name))
                .unwrap()
                .diagnostics
                .pop()
                .unwrap()
        };

        assert_eq!(diagnostic.source_name.as_deref(), Some("fixture.js"));
        assert_eq!(diagnostic.message, "fixture diagnostic");
    }

    #[test]
    fn normal_completion_returns_a_rooted_value() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        let outcome = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap();
        let value = outcome.completion.rooted_value();
        assert_eq!(value.realm(), realm.id());
        runtime.require_root(value).unwrap();
    }

    #[test]
    fn script_throw_is_a_completion_not_a_backend_error() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        let outcome = runtime
            .evaluate(realm.id(), ScriptSource::new("throw fixture"))
            .unwrap();
        let ScriptCompletion::Throw(exception) = outcome.completion else {
            panic!("expected fixture throw completion");
        };
        assert_eq!(exception.value.realm(), realm.id());
        assert_eq!(exception.message.as_deref(), Some("fixture exception"));
        assert_eq!(exception.stack.as_deref(), Some("fixture.js:1"));
        runtime.require_root(exception.value).unwrap();
    }

    #[test]
    fn duplicate_root_has_independent_liveness() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        let original = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();
        let duplicate = runtime.duplicate_root(original).unwrap();
        runtime.release_root(original).unwrap();
        runtime.require_root(duplicate).unwrap();
        let error = runtime.require_root(original).unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidRoot);
    }

    #[test]
    fn release_root_rejects_stale_handles() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        let value = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();
        runtime.release_root(value).unwrap();
        let error = runtime.release_root(value).unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidRoot);
    }

    #[test]
    fn rooted_value_limit_is_enforced() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime
            .create_realm(ScriptRealmLimits::try_new(1024, 1).unwrap())
            .unwrap();
        let first = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();
        let error = runtime
            .evaluate(realm.id(), ScriptSource::new("2"))
            .unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
        runtime.release_root(first).unwrap();
        runtime
            .evaluate(realm.id(), ScriptSource::new("3"))
            .unwrap();
    }

    #[test]
    fn runtime_enforces_source_limit() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime
            .create_realm(ScriptRealmLimits::try_new(4, 8).unwrap())
            .unwrap();
        let error = runtime
            .evaluate(realm.id(), ScriptSource::new("12345"))
            .unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
    }

    #[test]
    fn destroyed_realms_reject_evaluation_and_roots() {
        let mut runtime = FixtureRuntime::new();
        let realm = runtime.create_realm(limits()).unwrap();
        let value = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();
        runtime.destroy_realm(realm.id()).unwrap();
        let evaluation_error = runtime
            .evaluate(realm.id(), ScriptSource::new("1"))
            .unwrap_err();
        assert_eq!(evaluation_error.kind, ScriptErrorKind::InvalidRealm);
        let root_error = runtime.release_root(value).unwrap_err();
        assert_eq!(root_error.kind, ScriptErrorKind::InvalidRealm);
    }

    #[test]
    fn foreign_rooted_values_are_rejected() {
        let mut first = FixtureRuntime::new();
        let first_realm = first.create_realm(limits()).unwrap();
        let foreign = first
            .evaluate(first_realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();

        let mut second = FixtureRuntime::new();
        let second_realm = second.create_realm(limits()).unwrap();
        let local = second
            .evaluate(second_realm.id(), ScriptSource::new("1"))
            .unwrap()
            .completion
            .rooted_value();
        assert_ne!(foreign, local);
        let error = second.duplicate_root(foreign).unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidRealm);
    }

    #[test]
    fn source_span_rejects_reversed_offsets() {
        let error = ScriptSourceSpan::try_new(4, 3).unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidInput);
    }
}
