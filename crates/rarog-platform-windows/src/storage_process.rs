use rarog_host::{HostControlPlane, StorageLoss};
use rarog_platform_windows_native::{SandboxEvidence, SandboxedChild};
use rarog_process::StorageProcessId;
use std::ffi::{OsStr, OsString};
use std::fmt;

pub const DEFAULT_MAX_STORAGE_PROCESS_ARGS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsStorageProcessErrorKind {
    UnsupportedTarget,
    EmptyProgram,
    ArgumentLimitExceeded,
    UnknownStorageProcess,
    LaunchFailed,
    WaitFailed,
    HostControl,
    SandboxEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsStorageProcessError {
    pub kind: WindowsStorageProcessErrorKind,
    pub message: String,
}

impl WindowsStorageProcessError {
    fn new(kind: WindowsStorageProcessErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn unsupported_target() -> Self {
        Self::new(
            WindowsStorageProcessErrorKind::UnsupportedTarget,
            "Windows Storage-process launch is unavailable on this target",
        )
    }
}

impl fmt::Display for WindowsStorageProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WindowsStorageProcessError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsStorageProcessCommand {
    program: OsString,
    args: Vec<OsString>,
}

impl WindowsStorageProcessCommand {
    pub fn try_new(program: impl Into<OsString>) -> Result<Self, WindowsStorageProcessError> {
        let program = program.into();
        if program.is_empty() {
            return Err(WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::EmptyProgram,
                "Storage-process program must not be empty",
            ));
        }
        Ok(Self {
            program,
            args: Vec::new(),
        })
    }

    pub fn try_arg(
        mut self,
        argument: impl Into<OsString>,
    ) -> Result<Self, WindowsStorageProcessError> {
        if self.args.len() >= DEFAULT_MAX_STORAGE_PROCESS_ARGS {
            return Err(WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::ArgumentLimitExceeded,
                format!(
                    "Storage-process argument limit {} reached",
                    DEFAULT_MAX_STORAGE_PROCESS_ARGS
                ),
            ));
        }
        self.args.push(argument.into());
        Ok(self)
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn args(&self) -> impl Iterator<Item = &OsStr> {
        self.args.iter().map(OsString::as_os_str)
    }
}

#[derive(Debug)]
pub struct WindowsStorageProcess {
    process: StorageProcessId,
    child: Option<SandboxedChild>,
    loss_reported: bool,
}

impl WindowsStorageProcess {
    pub fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn launch(
        host: &HostControlPlane,
        process: StorageProcessId,
        command: &WindowsStorageProcessCommand,
    ) -> Result<Self, WindowsStorageProcessError> {
        if !Self::target_available() {
            return Err(WindowsStorageProcessError::unsupported_target());
        }
        if host.storage_process() != Some(process) {
            return Err(WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::UnknownStorageProcess,
                "Storage-process identity is no longer live in the Host control plane",
            ));
        }

        let child = SandboxedChild::spawn(command.program(), &command.args).map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::LaunchFailed,
                format!("Windows Storage-process sandbox launch failed: {error}"),
            )
        })?;
        Ok(Self {
            process,
            child: Some(child),
            loss_reported: false,
        })
    }

    pub fn process(&self) -> StorageProcessId {
        self.process
    }

    pub fn try_observe_loss(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<Option<StorageLoss>, WindowsStorageProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        let status = child.try_wait().map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::WaitFailed,
                format!("Windows Storage-process status check failed: {error}"),
            )
        })?;
        if status.is_none() {
            return Ok(None);
        }

        self.child.take();
        self.report_loss(host).map(Some)
    }

    pub fn wait_for_loss(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<Option<StorageLoss>, WindowsStorageProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        child.wait().map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::WaitFailed,
                format!("Windows Storage-process wait failed: {error}"),
            )
        })?;
        self.child.take();
        self.report_loss(host).map(Some)
    }

    pub fn terminate_and_report(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<Option<StorageLoss>, WindowsStorageProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };

        child.terminate().map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::WaitFailed,
                format!("Windows Storage-process termination failed: {error}"),
            )
        })?;
        self.child.take();
        self.report_loss(host).map(Some)
    }

    pub fn sandbox_evidence(&self) -> Result<SandboxEvidence, WindowsStorageProcessError> {
        let child = self.child.as_ref().ok_or_else(|| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::SandboxEvidence,
                "Windows Storage-process sandbox evidence is unavailable after process loss",
            )
        })?;
        child.evidence().map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::SandboxEvidence,
                format!("Windows Storage-process sandbox evidence query failed: {error}"),
            )
        })
    }

    fn report_loss(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<StorageLoss, WindowsStorageProcessError> {
        let loss = host.storage_process_lost(self.process).map_err(|error| {
            WindowsStorageProcessError::new(
                WindowsStorageProcessErrorKind::HostControl,
                format!("Host Storage-process loss handling failed: {error}"),
            )
        })?;
        self.loss_reported = true;
        Ok(loss)
    }
}

impl Drop for WindowsStorageProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.terminate();
        }
        self.child.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    use rarog_broker::CapabilityClass;
    #[cfg(target_os = "windows")]
    use rarog_url::WebUrl;

    #[test]
    fn windows_storage_process_command_is_bounded() {
        assert_eq!(
            WindowsStorageProcessCommand::try_new("").unwrap_err().kind,
            WindowsStorageProcessErrorKind::EmptyProgram
        );

        let mut command = WindowsStorageProcessCommand::try_new("rarog-storage.exe").unwrap();
        for index in 0..DEFAULT_MAX_STORAGE_PROCESS_ARGS {
            command = command.try_arg(index.to_string()).unwrap();
        }
        assert_eq!(
            command.try_arg("overflow").unwrap_err().kind,
            WindowsStorageProcessErrorKind::ArgumentLimitExceeded
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_storage_process_is_explicitly_unsupported_off_windows() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let process = host.ensure_storage_process().unwrap();
        let command = WindowsStorageProcessCommand::try_new("rarog-storage.exe").unwrap();

        assert_eq!(
            WindowsStorageProcess::launch(&host, process, &command)
                .unwrap_err()
                .kind,
            WindowsStorageProcessErrorKind::UnsupportedTarget
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_storage_process_exit_revokes_storage_authority_and_recovers_fresh() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        host.grant_navigation_context_storage_capability(context.context())
            .unwrap();
        host.grant_navigation_context_capability(context.context(), CapabilityClass::Network)
            .unwrap();
        let storage = host.storage_process().unwrap();

        let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"));
        let command = WindowsStorageProcessCommand::try_new(shell)
            .unwrap()
            .try_arg("/D")
            .unwrap()
            .try_arg("/C")
            .unwrap()
            .try_arg("exit 0")
            .unwrap();
        let mut process = WindowsStorageProcess::launch(&host, storage, &command).unwrap();
        assert_eq!(process.process(), storage);
        assert!(process.sandbox_evidence().unwrap().satisfies_r4_policy());

        let loss = process.wait_for_loss(&mut host).unwrap().unwrap();
        assert_eq!(loss.process(), storage);
        assert_eq!(loss.revoked_capabilities(), 1);
        assert_eq!(host.storage_process(), None);
        assert_eq!(host.active_navigation_contexts(), 1);
        assert_eq!(host.active_capabilities(), 1);
        assert!(process.try_observe_loss(&mut host).unwrap().is_none());

        let replacement = host.recover_storage_process().unwrap();
        assert_ne!(replacement, storage);
        assert_eq!(
            WindowsStorageProcess::launch(&host, storage, &command)
                .unwrap_err()
                .kind,
            WindowsStorageProcessErrorKind::UnknownStorageProcess
        );
    }
}
