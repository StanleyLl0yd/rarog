use std::{fmt, num::NonZeroU64};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealmId(NonZeroU64);

impl RealmId {
    pub fn new(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSource<'a> {
    pub text: &'a str,
    pub name: &'a str,
    pub start_line: u32,
}

impl<'a> ScriptSource<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            name: "<script>",
            start_line: 1,
        }
    }

    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self
    }

    pub fn with_start_line(mut self, start_line: u32) -> Self {
        self.start_line = start_line.max(1);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptErrorKind {
    InvalidRealm,
    Compile,
    Exception,
    Runtime,
    Host,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    pub message: String,
    pub source_name: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl ScriptError {
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source_name: None,
            line: None,
            column: None,
        }
    }

    pub fn with_location(
        mut self,
        source_name: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        self.source_name = Some(source_name.into());
        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

impl fmt::Display for ScriptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScriptError {}

pub trait ScriptRuntime {
    fn create_realm(&mut self) -> Result<RealmId, ScriptError>;

    fn is_realm_live(&self, realm: RealmId) -> bool;

    fn evaluate(&mut self, realm: RealmId, source: ScriptSource<'_>) -> Result<(), ScriptError>;

    fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[derive(Default)]
    struct FixtureRuntime {
        next_realm: u64,
        realms: BTreeSet<RealmId>,
        executed: Vec<(RealmId, String, String, u32)>,
    }

    impl ScriptRuntime for FixtureRuntime {
        fn create_realm(&mut self) -> Result<RealmId, ScriptError> {
            self.next_realm = self.next_realm.saturating_add(1);
            let realm = RealmId::new(self.next_realm).ok_or_else(|| {
                ScriptError::new(ScriptErrorKind::Runtime, "realm identifier space exhausted")
            })?;
            self.realms.insert(realm);
            Ok(realm)
        }

        fn is_realm_live(&self, realm: RealmId) -> bool {
            self.realms.contains(&realm)
        }

        fn evaluate(
            &mut self,
            realm: RealmId,
            source: ScriptSource<'_>,
        ) -> Result<(), ScriptError> {
            if !self.is_realm_live(realm) {
                return Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "script realm is not live",
                ));
            }
            self.executed.push((
                realm,
                source.text.to_owned(),
                source.name.to_owned(),
                source.start_line,
            ));
            Ok(())
        }

        fn destroy_realm(&mut self, realm: RealmId) -> Result<(), ScriptError> {
            if !self.realms.remove(&realm) {
                return Err(ScriptError::new(
                    ScriptErrorKind::InvalidRealm,
                    "script realm is not live",
                ));
            }
            Ok(())
        }
    }

    fn execute_fixture(runtime: &mut dyn ScriptRuntime) -> Result<(), ScriptError> {
        let realm = runtime.create_realm()?;
        runtime.evaluate(
            realm,
            ScriptSource::new("1 + 1")
                .with_name("fixture.js")
                .with_start_line(7),
        )?;
        runtime.destroy_realm(realm)
    }

    #[test]
    fn realm_identifier_rejects_zero() {
        assert_eq!(RealmId::new(0), None);
        assert_eq!(RealmId::new(7).unwrap().get(), 7);
    }

    #[test]
    fn source_defaults_and_location_are_backend_neutral() {
        let source = ScriptSource::new("const value = 1;");
        assert_eq!(source.name, "<script>");
        assert_eq!(source.start_line, 1);

        let adjusted = source.with_name("page.js").with_start_line(0);
        assert_eq!(adjusted.name, "page.js");
        assert_eq!(adjusted.start_line, 1);
    }

    #[test]
    fn runtime_contract_is_object_safe() {
        let mut runtime = FixtureRuntime::default();
        execute_fixture(&mut runtime).unwrap();

        assert_eq!(runtime.executed.len(), 1);
        assert_eq!(runtime.executed[0].1, "1 + 1");
        assert_eq!(runtime.executed[0].2, "fixture.js");
        assert_eq!(runtime.executed[0].3, 7);
    }

    #[test]
    fn invalid_realm_operations_fail_without_backend_types() {
        let mut runtime = FixtureRuntime::default();
        let realm = runtime.create_realm().unwrap();
        runtime.destroy_realm(realm).unwrap();

        let error = runtime
            .evaluate(realm, ScriptSource::new("1 + 1"))
            .unwrap_err();
        assert_eq!(error.kind, ScriptErrorKind::InvalidRealm);
    }

    #[test]
    fn errors_own_their_diagnostic_location() {
        let error = ScriptError::new(ScriptErrorKind::Compile, "unexpected token")
            .with_location(String::from("inline.js"), 4, 9);

        assert_eq!(error.source_name.as_deref(), Some("inline.js"));
        assert_eq!(error.line, Some(4));
        assert_eq!(error.column, Some(9));
    }
}
