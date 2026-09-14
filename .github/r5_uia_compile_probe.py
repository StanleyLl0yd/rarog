from pathlib import Path

cargo = Path("crates/rarog-platform-windows-native/Cargo.toml")
text = cargo.read_text()
needle = '''[target.'cfg(target_os = "windows")'.dependencies]\n'''
if text.count(needle) != 1:
    raise SystemExit("native Cargo target dependency section not unique")
addition = '''[target.'cfg(target_os = "windows")'.dependencies]\nwindows = { version = "=0.58.0", features = [\n  "implement",\n  "Win32_UI_Accessibility",\n] }\nwindows-core = "=0.58.0"\n'''
cargo.write_text(text.replace(needle, addition, 1))

lib = Path("crates/rarog-platform-windows-native/src/lib.rs")
text = lib.read_text()
needle = "mod accessibility;\n"
if text.count(needle) != 1:
    raise SystemExit("native accessibility module declaration not unique")
lib.write_text(text.replace(needle, needle + '#[cfg(target_os = "windows")]\nmod uia_provider_probe;\n', 1))

Path("crates/rarog-platform-windows-native/src/uia_provider_probe.rs").write_text(r'''use windows::{
    core::{implement, Error, HRESULT, IUnknown, Result, VARIANT},
    Win32::UI::Accessibility::{
        IRawElementProviderSimple, IRawElementProviderSimple_Impl, ProviderOptions,
        ProviderOptions_ServerSideProvider, UIA_PATTERN_ID, UIA_PROPERTY_ID,
    },
};

const E_NOTIMPL: HRESULT = HRESULT(0x80004001_u32 as i32);

#[implement(IRawElementProviderSimple)]
struct ProbeProvider;

impl IRawElementProviderSimple_Impl for ProbeProvider {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    fn GetPatternProvider(&self, _patternid: UIA_PATTERN_ID) -> Result<IUnknown> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetPropertyValue(&self, _propertyid: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(VARIANT::default())
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        Err(Error::from(E_NOTIMPL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_provider_projects_to_com_interface() {
        let provider: IRawElementProviderSimple = ProbeProvider.into();
        let options = unsafe { provider.ProviderOptions() }.unwrap();
        assert_eq!(options, ProviderOptions_ServerSideProvider);
    }
}
''')
