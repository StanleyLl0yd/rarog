use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealmId(NonZeroU64);

#[derive(Debug)]
pub struct RealmIdAllocator {
    next: u64,
}

impl Default for RealmIdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl RealmIdAllocator {
    pub fn allocate(&mut self) -> Result<RealmId, ScriptError> {
        let serial = NonZeroU64::new(self.next).ok_or_else(|| {
            ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script realm identity space is exhausted",
            )
        })?;
        self.next = self.next.checked_add(1).unwrap_or(0);
        Ok(RealmId(serial))
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvaluationOutcome {
    pub diagnostics: Vec<ScriptDiagnostic>,
}

pub trait ScriptRuntime {
    fn create_realm(&mut self) -> Result<RealmId, ScriptError>;

    fn evaluate(
        &mut self,
        realm: RealmId,
        source: ScriptSource<'_>,
    ) -> Result<EvaluationOutcome, ScriptError>;

    fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[derive(Default)]
    struct FixtureRuntime {
        ids: RealmIdAllocator,
        live: BTreeSet<RealmId>,
    }

    impl FixtureRuntime {
        fn require_live(&self, realm: RealmId) -> Result<(), ScriptError> {
            if self.live.contains(&realm) {
                Ok(())
            } else {
                Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "script realm is not live",
                ))
            }
        }
    }

    impl ScriptRuntime for FixtureRuntime {
        fn create_realm(&mut self) -> Result<RealmId, ScriptError> {
            let realm = self.ids.allocate()?;
            self.live.insert(realm);
            Ok(realm)
        }

        fn evaluate(
            &mut self,
            realm: RealmId,
            source: ScriptSource<'_>,
        ) -> Result<EvaluationOutcome, ScriptError> {
            self.require_live(realm)?;
            let mut outcome = EvaluationOutcome::default();
            if let Some(name) = source.name() {
                outcome.diagnostics.push(
                    ScriptDiagnostic::new(ScriptDiagnosticLevel::Warning, "fixture diagnostic")
                        .with_source_name(name),
                );
            }
            Ok(outcome)
        }

        fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError> {
            if self.live.remove(&realm) {
                Ok(())
            } else {
                Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "script realm is not live",
                ))
            }
        }
    }

    #[test]
    fn realm_ids_are_nonzero_and_monotonic() {
        let mut ids = RealmIdAllocator::default();
        let first = ids.allocate().unwrap();
        let second = ids.allocate().unwrap();
        assert_eq!(first.0.get(), 1);
        assert_eq!(second.0.get(), 2);
    }

    #[test]
    fn realm_id_allocator_fails_after_exhaustion() {
        let mut ids = RealmIdAllocator { next: u64::MAX };
        assert_eq!(ids.allocate().unwrap().0.get(), u64::MAX);
        let error = ids.allocate().unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
    }

    #[test]
    fn runtime_contract_is_object_safe() {
        let mut runtime = FixtureRuntime::default();
        let runtime: &mut dyn ScriptRuntime = &mut runtime;
        let realm = runtime.create_realm().unwrap();
        let outcome = runtime
            .evaluate(realm, ScriptSource::new("1 + 1"))
            .unwrap();
        assert!(outcome.diagnostics.is_empty());
        runtime.destroy_realm(realm).unwrap();
    }

    #[test]
    fn evaluation_diagnostics_are_owned() {
        let diagnostic = {
            let name = String::from("fixture.js");
            let source_text = String::from("let value = 1;");
            let mut runtime = FixtureRuntime::default();
            let realm = runtime.create_realm().unwrap();
            runtime
                .evaluate(realm, ScriptSource::named(&source_text, &name))
                .unwrap()
                .diagnostics
                .pop()
                .unwrap()
        };

        assert_eq!(diagnostic.source_name.as_deref(), Some("fixture.js"));
        assert_eq!(diagnostic.message, "fixture diagnostic");
    }

    #[test]
    fn destroyed_realms_reject_evaluation() {
        let mut runtime = FixtureRuntime::default();
        let realm = runtime.create_realm().unwrap();
        runtime.destroy_realm(realm).unwrap();
        let error = runtime
            .evaluate(realm, ScriptSource::new("1"))
            .unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidRealm);
    }

    #[test]
    fn source_limit_is_checked_before_backend_work() {
        let error = ScriptSource::new("12345")
            .ensure_byte_limit(4)
            .unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::ResourceLimit);
    }

    #[test]
    fn source_span_rejects_reversed_offsets() {
        let error = ScriptSourceSpan::try_new(4, 3).unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidInput);
    }
}
