#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{OsStr, OsString};
use std::fmt;

pub const MAX_WINDOWS_COMMAND_LINE_UNITS: usize = 32_767;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SandboxErrorKind {
    UnsupportedTarget,
    InvalidCommand,
    CommandLineTooLong,
    JobConfiguration,
    AttributeConfiguration,
    Launch,
    Wait,
    Terminate,
    Evidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxError {
    pub kind: SandboxErrorKind,
    pub message: String,
}

impl SandboxError {
    fn new(kind: SandboxErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn unsupported() -> Self {
        Self::new(
            SandboxErrorKind::UnsupportedTarget,
            "Windows Site-process sandbox is unavailable on this target",
        )
    }
}

impl fmt::Display for SandboxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SandboxError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SandboxEvidence {
    pub dep_enabled: bool,
    pub force_relocate_images: bool,
    pub bottom_up_aslr: bool,
    pub high_entropy_aslr: bool,
    pub strict_handle_checks: bool,
    pub extension_points_disabled: bool,
    pub sehop_enabled: bool,
    pub child_process_restricted: bool,
    pub job_active_process_limit: u32,
    pub job_kill_on_close: bool,
}

impl SandboxEvidence {
    pub const fn satisfies_r4_policy(self) -> bool {
        self.dep_enabled
            && self.force_relocate_images
            && self.bottom_up_aslr
            && self.high_entropy_aslr
            && self.strict_handle_checks
            && self.extension_points_disabled
            && self.sehop_enabled
            && self.child_process_restricted
            && self.job_active_process_limit == 1
            && self.job_kill_on_close
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::ffi::c_void;
    use std::mem::{size_of, size_of_val};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        QueryInformationJobObject, SetInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::{
        CREATE_NO_WINDOW, CreateProcessW, DeleteProcThreadAttributeList,
        EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, GetProcessMitigationPolicy, INFINITE,
        InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY,
        PROC_THREAD_ATTRIBUTE_JOB_LIST, PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY,
        PROCESS_INFORMATION, ProcessASLRPolicy, ProcessChildProcessPolicy, ProcessDEPPolicy,
        ProcessExtensionPointDisablePolicy, ProcessSEHOPPolicy, ProcessStrictHandleCheckPolicy,
        STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
    };

    const WAIT_OBJECT_0_VALUE: u32 = 0;
    const WAIT_TIMEOUT_VALUE: u32 = 258;
    const PROCESS_CREATION_CHILD_PROCESS_RESTRICTED: u32 = 0x1;

    const MITIGATION_DEP_ENABLE: u64 = 0x0000_0001;
    const MITIGATION_SEHOP_ENABLE: u64 = 0x0000_0004;
    const MITIGATION_FORCE_RELOCATE_IMAGES_ALWAYS_ON: u64 = 0x0000_0001 << 8;
    const MITIGATION_HEAP_TERMINATE_ALWAYS_ON: u64 = 0x0000_0001 << 12;
    const MITIGATION_BOTTOM_UP_ASLR_ALWAYS_ON: u64 = 0x0000_0001 << 16;
    const MITIGATION_HIGH_ENTROPY_ASLR_ALWAYS_ON: u64 = 0x0000_0001 << 20;
    const MITIGATION_STRICT_HANDLE_CHECKS_ALWAYS_ON: u64 = 0x0000_0001 << 24;
    const MITIGATION_EXTENSION_POINT_DISABLE_ALWAYS_ON: u64 = 0x0000_0001 << 32;
    const MITIGATION_POLICY: u64 = MITIGATION_DEP_ENABLE
        | MITIGATION_SEHOP_ENABLE
        | MITIGATION_FORCE_RELOCATE_IMAGES_ALWAYS_ON
        | MITIGATION_HEAP_TERMINATE_ALWAYS_ON
        | MITIGATION_BOTTOM_UP_ASLR_ALWAYS_ON
        | MITIGATION_HIGH_ENTROPY_ASLR_ALWAYS_ON
        | MITIGATION_STRICT_HANDLE_CHECKS_ALWAYS_ON
        | MITIGATION_EXTENSION_POINT_DISABLE_ALWAYS_ON;

    #[derive(Debug)]
    struct OwnedHandle(HANDLE);

    impl OwnedHandle {
        fn try_new(
            handle: HANDLE,
            kind: SandboxErrorKind,
            operation: &str,
        ) -> Result<Self, SandboxError> {
            if handle.is_null() {
                Err(os_error(kind, operation))
            } else {
                Ok(Self(handle))
            }
        }

        fn raw(&self) -> HANDLE {
            self.0
        }
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    struct AttributeList {
        storage: Vec<usize>,
        list: windows_sys::Win32::System::Threading::LPPROC_THREAD_ATTRIBUTE_LIST,
    }

    impl AttributeList {
        fn new(job: HANDLE) -> Result<Self, SandboxError> {
            let mut bytes = 0usize;
            unsafe {
                InitializeProcThreadAttributeList(null_mut(), 3, 0, &mut bytes);
            }
            if bytes == 0 {
                return Err(os_error(
                    SandboxErrorKind::AttributeConfiguration,
                    "measure process attribute list",
                ));
            }

            let words = bytes.div_ceil(size_of::<usize>());
            let mut storage = vec![0usize; words];
            let list = storage.as_mut_ptr().cast();
            if unsafe { InitializeProcThreadAttributeList(list, 3, 0, &mut bytes) } == 0 {
                return Err(os_error(
                    SandboxErrorKind::AttributeConfiguration,
                    "initialize process attribute list",
                ));
            }

            let mitigation = MITIGATION_POLICY;
            if unsafe {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY as usize,
                    (&mitigation as *const u64).cast(),
                    size_of_val(&mitigation),
                    null_mut(),
                    null(),
                )
            } == 0
            {
                unsafe {
                    DeleteProcThreadAttributeList(list);
                }
                return Err(os_error(
                    SandboxErrorKind::AttributeConfiguration,
                    "apply process mitigation policy",
                ));
            }

            let child_policy = PROCESS_CREATION_CHILD_PROCESS_RESTRICTED;
            if unsafe {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY as usize,
                    (&child_policy as *const u32).cast(),
                    size_of_val(&child_policy),
                    null_mut(),
                    null(),
                )
            } == 0
            {
                unsafe {
                    DeleteProcThreadAttributeList(list);
                }
                return Err(os_error(
                    SandboxErrorKind::AttributeConfiguration,
                    "apply child-process restriction",
                ));
            }

            let jobs = [job];
            if unsafe {
                UpdateProcThreadAttribute(
                    list,
                    0,
                    PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                    jobs.as_ptr().cast(),
                    size_of_val(&jobs),
                    null_mut(),
                    null(),
                )
            } == 0
            {
                unsafe {
                    DeleteProcThreadAttributeList(list);
                }
                return Err(os_error(
                    SandboxErrorKind::AttributeConfiguration,
                    "bind process job list",
                ));
            }

            Ok(Self { storage, list })
        }
    }

    impl Drop for AttributeList {
        fn drop(&mut self) {
            unsafe {
                DeleteProcThreadAttributeList(self.list);
            }
            let _ = self.storage.len();
        }
    }

    #[derive(Debug)]
    pub struct SandboxedChild {
        process: OwnedHandle,
        job: OwnedHandle,
    }

    impl SandboxedChild {
        pub fn spawn(program: &OsStr, args: &[OsString]) -> Result<Self, SandboxError> {
            let mut program_wide = encode_os(program)?;
            let mut command_line = build_command_line(program, args)?;
            let job = create_job()?;
            let attributes = AttributeList::new(job.raw())?;

            let mut startup = STARTUPINFOEXW::default();
            startup.StartupInfo.cb = u32::try_from(size_of::<STARTUPINFOEXW>()).map_err(|_| {
                SandboxError::new(
                    SandboxErrorKind::Launch,
                    "Windows STARTUPINFOEXW size is not representable",
                )
            })?;
            startup.lpAttributeList = attributes.list;
            let mut info = PROCESS_INFORMATION::default();
            let flags = CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT;

            let created = unsafe {
                CreateProcessW(
                    program_wide.as_mut_ptr(),
                    command_line.as_mut_ptr(),
                    null(),
                    null(),
                    0,
                    flags,
                    null(),
                    null(),
                    (&startup as *const STARTUPINFOEXW).cast(),
                    &mut info,
                )
            };
            if created == 0 {
                return Err(os_error(
                    SandboxErrorKind::Launch,
                    "create sandboxed Site process",
                ));
            }

            let process = OwnedHandle::try_new(
                info.hProcess,
                SandboxErrorKind::Launch,
                "receive Site process handle",
            )?;
            let thread = OwnedHandle::try_new(
                info.hThread,
                SandboxErrorKind::Launch,
                "receive Site thread handle",
            )?;
            drop(thread);
            drop(attributes);

            Ok(Self { process, job })
        }

        pub fn try_wait(&mut self) -> Result<Option<u32>, SandboxError> {
            match unsafe { WaitForSingleObject(self.process.raw(), 0) } {
                WAIT_OBJECT_0_VALUE => self.exit_code().map(Some),
                WAIT_TIMEOUT_VALUE => Ok(None),
                _ => Err(os_error(
                    SandboxErrorKind::Wait,
                    "observe Site process status",
                )),
            }
        }

        pub fn wait(&mut self) -> Result<u32, SandboxError> {
            if unsafe { WaitForSingleObject(self.process.raw(), INFINITE) } != WAIT_OBJECT_0_VALUE {
                return Err(os_error(
                    SandboxErrorKind::Wait,
                    "wait for Site process exit",
                ));
            }
            self.exit_code()
        }

        pub fn terminate(&mut self) -> Result<u32, SandboxError> {
            if let Some(code) = self.try_wait()? {
                return Ok(code);
            }
            if unsafe { TerminateProcess(self.process.raw(), 1) } == 0 {
                return Err(os_error(
                    SandboxErrorKind::Terminate,
                    "terminate Site process",
                ));
            }
            self.wait()
        }

        pub fn evidence(&self) -> Result<SandboxEvidence, SandboxError> {
            let dep = query_mitigation(self.process.raw(), ProcessDEPPolicy)?;
            let aslr = query_mitigation(self.process.raw(), ProcessASLRPolicy)?;
            let strict = query_mitigation(self.process.raw(), ProcessStrictHandleCheckPolicy)?;
            let extension =
                query_mitigation(self.process.raw(), ProcessExtensionPointDisablePolicy)?;
            let sehop = query_mitigation(self.process.raw(), ProcessSEHOPPolicy)?;
            let child = query_mitigation(self.process.raw(), ProcessChildProcessPolicy)?;

            let mut job_info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            if unsafe {
                QueryInformationJobObject(
                    self.job.raw(),
                    JobObjectExtendedLimitInformation,
                    (&mut job_info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    u32::try_from(size_of_val(&job_info)).map_err(|_| {
                        SandboxError::new(
                            SandboxErrorKind::Evidence,
                            "job information size is not representable",
                        )
                    })?,
                    null_mut(),
                )
            } == 0
            {
                return Err(os_error(
                    SandboxErrorKind::Evidence,
                    "query Site process job policy",
                ));
            }

            let limit_flags = job_info.BasicLimitInformation.LimitFlags;
            Ok(SandboxEvidence {
                dep_enabled: dep & 0x1 != 0,
                bottom_up_aslr: aslr & 0x1 != 0,
                force_relocate_images: aslr & 0x2 != 0,
                high_entropy_aslr: aslr & 0x4 != 0,
                strict_handle_checks: strict & 0x1 != 0,
                extension_points_disabled: extension & 0x1 != 0,
                sehop_enabled: sehop & 0x1 != 0,
                child_process_restricted: child & 0x1 != 0,
                job_active_process_limit: job_info.BasicLimitInformation.ActiveProcessLimit,
                job_kill_on_close: limit_flags & JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE != 0
                    && limit_flags & JOB_OBJECT_LIMIT_ACTIVE_PROCESS != 0,
            })
        }

        fn exit_code(&self) -> Result<u32, SandboxError> {
            let mut code = 0u32;
            if unsafe { GetExitCodeProcess(self.process.raw(), &mut code) } == 0 {
                return Err(os_error(
                    SandboxErrorKind::Wait,
                    "read Site process exit code",
                ));
            }
            Ok(code)
        }
    }

    impl Drop for SandboxedChild {
        fn drop(&mut self) {
            let _ = self.terminate();
        }
    }

    fn create_job() -> Result<OwnedHandle, SandboxError> {
        let job = OwnedHandle::try_new(
            unsafe { CreateJobObjectW(null(), null()) },
            SandboxErrorKind::JobConfiguration,
            "create Site process job",
        )?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        info.BasicLimitInformation.ActiveProcessLimit = 1;
        if unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
                    .cast_mut()
                    .cast(),
                u32::try_from(size_of_val(&info)).map_err(|_| {
                    SandboxError::new(
                        SandboxErrorKind::JobConfiguration,
                        "job information size is not representable",
                    )
                })?,
            )
        } == 0
        {
            return Err(os_error(
                SandboxErrorKind::JobConfiguration,
                "configure Site process job",
            ));
        }
        Ok(job)
    }

    fn query_mitigation(
        process: HANDLE,
        policy: windows_sys::Win32::System::Threading::PROCESS_MITIGATION_POLICY,
    ) -> Result<u32, SandboxError> {
        let mut value = 0u32;
        if unsafe {
            GetProcessMitigationPolicy(
                process,
                policy,
                (&mut value as *mut u32).cast::<c_void>(),
                size_of_val(&value),
            )
        } == 0
        {
            return Err(os_error(
                SandboxErrorKind::Evidence,
                "query Site process mitigation policy",
            ));
        }
        Ok(value)
    }

    fn encode_os(value: &OsStr) -> Result<Vec<u16>, SandboxError> {
        let mut units: Vec<u16> = value.encode_wide().collect();
        if units.is_empty() || units.contains(&0) {
            return Err(SandboxError::new(
                SandboxErrorKind::InvalidCommand,
                "Windows Site-process command values must be non-empty and contain no NUL",
            ));
        }
        units.push(0);
        Ok(units)
    }

    fn build_command_line(program: &OsStr, args: &[OsString]) -> Result<Vec<u16>, SandboxError> {
        let mut line = Vec::new();
        append_quoted(&mut line, program)?;
        for arg in args {
            line.push(b' ' as u16);
            append_quoted(&mut line, arg)?;
        }
        if line.len() + 1 > MAX_WINDOWS_COMMAND_LINE_UNITS {
            return Err(SandboxError::new(
                SandboxErrorKind::CommandLineTooLong,
                format!(
                    "Windows Site-process command line exceeds {} UTF-16 units",
                    MAX_WINDOWS_COMMAND_LINE_UNITS
                ),
            ));
        }
        line.push(0);
        Ok(line)
    }

    fn append_quoted(target: &mut Vec<u16>, value: &OsStr) -> Result<(), SandboxError> {
        let units: Vec<u16> = value.encode_wide().collect();
        if units.contains(&0) {
            return Err(SandboxError::new(
                SandboxErrorKind::InvalidCommand,
                "Windows Site-process arguments cannot contain NUL",
            ));
        }

        target.push(b'"' as u16);
        let mut backslashes = 0usize;
        for unit in units {
            if unit == b'\\' as u16 {
                backslashes += 1;
                continue;
            }
            if unit == b'"' as u16 {
                target.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2 + 1));
                target.push(unit);
                backslashes = 0;
                continue;
            }
            target.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            backslashes = 0;
            target.push(unit);
        }
        target.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
        target.push(b'"' as u16);
        Ok(())
    }

    fn os_error(kind: SandboxErrorKind, operation: &str) -> SandboxError {
        let error = std::io::Error::last_os_error();
        SandboxError::new(kind, format!("{operation} failed: {error}"))
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct SandboxedChild;

    impl SandboxedChild {
        pub fn spawn(_program: &OsStr, _args: &[OsString]) -> Result<Self, SandboxError> {
            Err(SandboxError::unsupported())
        }

        pub fn try_wait(&mut self) -> Result<Option<u32>, SandboxError> {
            Err(SandboxError::unsupported())
        }

        pub fn wait(&mut self) -> Result<u32, SandboxError> {
            Err(SandboxError::unsupported())
        }

        pub fn terminate(&mut self) -> Result<u32, SandboxError> {
            Err(SandboxError::unsupported())
        }

        pub fn evidence(&self) -> Result<SandboxEvidence, SandboxError> {
            Err(SandboxError::unsupported())
        }
    }
}

pub use imp::SandboxedChild;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_site_sandbox_is_explicitly_unsupported_off_windows() {
        assert_eq!(
            SandboxedChild::spawn(OsStr::new("rarog-site.exe"), &[])
                .unwrap_err()
                .kind,
            SandboxErrorKind::UnsupportedTarget
        );
    }

    #[cfg(target_os = "windows")]
    fn shell() -> OsString {
        std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"))
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_site_sandbox_reports_required_policy() {
        let args = [
            OsString::from("/D"),
            OsString::from("/V:OFF"),
            OsString::from("/C"),
            OsString::from("for /L %i in (1,1,1000000) do @set /A x=1+1 >nul"),
        ];
        let mut child = SandboxedChild::spawn(&shell(), &args).unwrap();
        let evidence = child.evidence().unwrap();
        assert!(evidence.satisfies_r4_policy(), "{evidence:?}");
        child.terminate().unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_site_sandbox_blocks_nested_process_creation() {
        let args = [
            OsString::from("/D"),
            OsString::from("/C"),
            OsString::from("cmd.exe /D /C exit 0"),
        ];
        let mut child = SandboxedChild::spawn(&shell(), &args).unwrap();
        let code = child.wait().unwrap();
        assert_ne!(code, 0);
    }
}
