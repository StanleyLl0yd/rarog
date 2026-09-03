use std::collections::BTreeMap;
use std::ffi::CString;
use std::marker::PhantomData;
use std::ptr;

use mozjs::gc::RootedTraceableBox;
use mozjs::jsapi::{Heap, JSObject, OnNewGlobalHookOption};
use mozjs::jsval::{JSVal, UndefinedValue};
use mozjs::rooted;
use mozjs::rust::wrappers2::JS_NewGlobalObject;
use mozjs::rust::{
    CompileOptionsWrapper, JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS,
    error_info_from_exception_stack_safe, evaluate_script,
};
use rarog_script::{
    EvaluationOutcome, RealmId, RealmIdAllocator, RootedValueId, RootedValueIdAllocator,
    ScriptCompletion, ScriptError, ScriptErrorKind, ScriptException, ScriptRealm,
    ScriptRealmLimits, ScriptRuntime, ScriptSource,
};

type PersistentObject = RootedTraceableBox<Heap<*mut JSObject>>;
type PersistentValue = RootedTraceableBox<Heap<JSVal>>;

pub struct SpiderMonkeyEngine {
    engine: JSEngine,
}

impl SpiderMonkeyEngine {
    pub fn initialize() -> Result<Self, ScriptError> {
        JSEngine::init()
            .map(|engine| Self { engine })
            .map_err(|error| {
                ScriptError::new(
                    ScriptErrorKind::Backend,
                    format!("failed to initialize SpiderMonkey: {error:?}"),
                )
            })
    }

    pub fn create_runtime(&self) -> Result<SpiderMonkeyRuntime<'_>, ScriptError> {
        let realm_ids = RealmIdAllocator::new()?;
        Ok(SpiderMonkeyRuntime {
            realms: BTreeMap::new(),
            runtime: Runtime::new(self.engine.handle()),
            realm_ids,
            _engine: PhantomData,
        })
    }
}

struct RealmState {
    limits: ScriptRealmLimits,
    global: PersistentObject,
    root_ids: RootedValueIdAllocator,
    roots: BTreeMap<RootedValueId, PersistentValue>,
}

impl RealmState {
    fn ensure_root_capacity(&self) -> Result<(), ScriptError> {
        if self.roots.len() >= self.limits.max_rooted_values() {
            Err(ScriptError::new(
                ScriptErrorKind::ResourceLimit,
                "script rooted-value limit exceeded",
            ))
        } else {
            Ok(())
        }
    }

    fn store_root(&mut self, value: JSVal) -> Result<RootedValueId, ScriptError> {
        self.ensure_root_capacity()?;
        let id = self.root_ids.allocate()?;
        let root = RootedTraceableBox::from_box(Heap::boxed(value));
        self.roots.insert(id, root);
        Ok(id)
    }

    fn require_root(&self, value: RootedValueId) -> Result<JSVal, ScriptError> {
        self.roots
            .get(&value)
            .map(|root| root.get())
            .ok_or_else(|| {
                ScriptError::new(
                    ScriptErrorKind::InvalidRoot,
                    "script rooted value is not live",
                )
            })
    }
}

pub struct SpiderMonkeyRuntime<'engine> {
    realms: BTreeMap<RealmId, RealmState>,
    runtime: Runtime,
    realm_ids: RealmIdAllocator,
    _engine: PhantomData<&'engine SpiderMonkeyEngine>,
}

impl SpiderMonkeyRuntime<'_> {
    fn state(&self, realm: RealmId) -> Result<&RealmState, ScriptError> {
        self.realms.get(&realm).ok_or_else(|| {
            ScriptError::new(ScriptErrorKind::InvalidRealm, "script realm is not live")
        })
    }

    fn state_mut(&mut self, realm: RealmId) -> Result<&mut RealmState, ScriptError> {
        self.realms.get_mut(&realm).ok_or_else(|| {
            ScriptError::new(ScriptErrorKind::InvalidRealm, "script realm is not live")
        })
    }

    fn source_filename(source: ScriptSource<'_>) -> Result<CString, ScriptError> {
        CString::new(source.name().unwrap_or("inline.js")).map_err(|_| {
            ScriptError::new(
                ScriptErrorKind::InvalidInput,
                "script source name must not contain NUL bytes",
            )
        })
    }
}

impl ScriptRuntime for SpiderMonkeyRuntime<'_> {
    fn create_realm(&mut self, limits: ScriptRealmLimits) -> Result<ScriptRealm, ScriptError> {
        let realm = self.realm_ids.allocate()?;
        let options = RealmOptions::default();
        let runtime = &mut self.runtime;
        rooted!(&in(runtime.cx()) let global = unsafe {
            // SAFETY: the runtime owns a live JSContext, the global class is static, and
            // RealmOptions stays alive for the duration of JS_NewGlobalObject.
            JS_NewGlobalObject(
                runtime.cx(),
                &SIMPLE_GLOBAL_CLASS,
                ptr::null_mut(),
                OnNewGlobalHookOption::FireOnNewGlobalHook,
                &*options,
            )
        });
        let global_ptr = global.get();
        if global_ptr.is_null() {
            return Err(ScriptError::new(
                ScriptErrorKind::Backend,
                "SpiderMonkey failed to create a global object",
            ));
        }
        let global = RootedTraceableBox::from_box(Heap::boxed(global_ptr));
        self.realms.insert(
            realm,
            RealmState {
                limits,
                global,
                root_ids: RootedValueIdAllocator::new(realm),
                roots: BTreeMap::new(),
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
        let filename = Self::source_filename(source)?;

        let (runtime, realms) = (&mut self.runtime, &mut self.realms);
        let state = realms.get_mut(&realm).ok_or_else(|| {
            ScriptError::new(ScriptErrorKind::InvalidRealm, "script realm is not live")
        })?;
        state.ensure_root_capacity()?;

        rooted!(&in(runtime.cx()) let mut result = UndefinedValue());
        let options = CompileOptionsWrapper::new(runtime.cx_no_gc(), filename, 1);
        let evaluation = evaluate_script(
            runtime.cx(),
            state.global.handle(),
            source.text(),
            result.handle_mut(),
            options,
        );

        let completion = match evaluation {
            Ok(()) => ScriptCompletion::Normal(state.store_root(result.get())?),
            Err(()) => {
                let Some(error) =
                    error_info_from_exception_stack_safe(runtime.cx(), result.handle_mut())
                else {
                    return Err(ScriptError::new(
                        ScriptErrorKind::Backend,
                        "SpiderMonkey evaluation failed without a pending exception",
                    ));
                };
                let value = state.store_root(result.get())?;
                ScriptCompletion::Throw(ScriptException {
                    value,
                    message: (!error.message.is_empty()).then_some(error.message),
                    stack: None,
                })
            }
        };

        Ok(EvaluationOutcome {
            completion,
            diagnostics: Vec::new(),
        })
    }

    fn duplicate_root(&mut self, value: RootedValueId) -> Result<RootedValueId, ScriptError> {
        let state = self.state_mut(value.realm())?;
        let value = state.require_root(value)?;
        state.store_root(value)
    }

    fn release_root(&mut self, value: RootedValueId) -> Result<(), ScriptError> {
        let state = self.state_mut(value.realm())?;
        if state.roots.remove(&value).is_some() {
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

impl Drop for SpiderMonkeyRuntime<'_> {
    fn drop(&mut self) {
        self.realms.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ScriptRealmLimits {
        ScriptRealmLimits::try_new(1024, 8).unwrap()
    }

    #[test]
    fn real_spidermonkey_runtime_obeys_the_script_contract() {
        eprintln!("checkpoint: initialize engine");
        let engine = SpiderMonkeyEngine::initialize().unwrap();
        eprintln!("checkpoint: create runtime");
        let mut runtime = engine.create_runtime().unwrap();
        eprintln!("checkpoint: create realm");
        let realm = runtime.create_realm(limits()).unwrap();

        eprintln!("checkpoint: evaluate normal");
        let normal = runtime
            .evaluate(realm.id(), ScriptSource::named("40 + 2", "normal.js"))
            .unwrap();
        eprintln!("checkpoint: normal evaluated");
        let normal_value = match normal.completion {
            ScriptCompletion::Normal(value) => value,
            ScriptCompletion::Throw(_) => panic!("normal evaluation unexpectedly threw"),
        };

        eprintln!("checkpoint: duplicate root");
        let duplicate = runtime.duplicate_root(normal_value).unwrap();
        assert_ne!(normal_value, duplicate);
        runtime.release_root(normal_value).unwrap();
        assert_eq!(
            runtime.release_root(normal_value).unwrap_err().kind,
            ScriptErrorKind::InvalidRoot
        );
        runtime.release_root(duplicate).unwrap();

        eprintln!("checkpoint: evaluate throw");
        let thrown = runtime
            .evaluate(
                realm.id(),
                ScriptSource::named("throw new Error('boom')", "throw.js"),
            )
            .unwrap();
        eprintln!("checkpoint: throw evaluated");
        let exception = match thrown.completion {
            ScriptCompletion::Throw(exception) => exception,
            ScriptCompletion::Normal(_) => panic!("throwing evaluation unexpectedly completed"),
        };
        assert!(
            exception
                .message
                .as_deref()
                .is_some_and(|message| message.contains("boom"))
        );
        runtime.release_root(exception.value).unwrap();

        eprintln!("checkpoint: source limit");
        let oversized = "x".repeat(1025);
        assert_eq!(
            runtime
                .evaluate(realm.id(), ScriptSource::new(&oversized))
                .unwrap_err()
                .kind,
            ScriptErrorKind::ResourceLimit
        );

        eprintln!("checkpoint: destroy realm");
        let stale_realm = realm.id();
        runtime.destroy_realm(stale_realm).unwrap();
        assert_eq!(
            runtime
                .evaluate(stale_realm, ScriptSource::new("1"))
                .unwrap_err()
                .kind,
            ScriptErrorKind::InvalidRealm
        );
        eprintln!("checkpoint: test complete");
    }
}
