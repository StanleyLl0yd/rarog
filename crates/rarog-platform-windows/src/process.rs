use rarog_host::{HostControlPlane, SiteLease, SiteLoss};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::process::{Child, Command, Stdio};

pub const DEFAULT_MAX_SITE_PROCESS_ARGS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsSiteProcessErrorKind {
    UnsupportedTarget,
    EmptyProgram,
    ArgumentLimitExceeded,
    UnknownSiteProcess,
    LaunchFailed,
    WaitFailed,
    HostControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsSiteProcessError {
    pub kind: WindowsSiteProcessErrorKind,
    pub message: String,
}

impl WindowsSiteProcessError {
    fn new(kind: WindowsSiteProcessErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn unsupported_target() -> Self {
        Self::new(
            WindowsSiteProcessErrorKind::UnsupportedTarget,
            "Windows Site-process launch is unavailable on this target",
        )
    }
}

impl fmt::Display for WindowsSiteProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WindowsSiteProcessError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsSiteProcessCommand {
    program: OsString,
    args: Vec<OsString>,
}

impl WindowsSiteProcessCommand {
    pub fn try_new(program: impl Into<OsString>) -> Result<Self, WindowsSiteProcessError> {
        let program = program.into();
        if program.is_empty() {
            return Err(WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::EmptyProgram,
                "Site-process program must not be empty",
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
    ) -> Result<Self, WindowsSiteProcessError> {
        if self.args.len() >= DEFAULT_MAX_SITE_PROCESS_ARGS {
            return Err(WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::ArgumentLimitExceeded,
                format!(
                    "Site-process argument limit {} reached",
                    DEFAULT_MAX_SITE_PROCESS_ARGS
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
pub struct WindowsSiteProcess {
    lease: SiteLease,
    child: Option<Child>,
    loss_reported: bool,
}

impl WindowsSiteProcess {
    pub fn target_available() -> bool {
        cfg!(target_os = "windows")
    }

    pub fn launch(
        host: &HostControlPlane,
        lease: SiteLease,
        command: &WindowsSiteProcessCommand,
    ) -> Result<Self, WindowsSiteProcessError> {
        if !Self::target_available() {
            return Err(WindowsSiteProcessError::unsupported_target());
        }
        if host.site_for_process(lease.process()).is_none() {
            return Err(WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::UnknownSiteProcess,
                "Site-process lease is no longer live in the Host control plane",
            ));
        }

        let child = spawn_windows_child(command).map_err(|error| {
            WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::LaunchFailed,
                format!("Windows Site-process launch failed: {error}"),
            )
        })?;
        Ok(Self {
            lease,
            child: Some(child),
            loss_reported: false,
        })
    }

    pub fn site_lease(&self) -> SiteLease {
        self.lease
    }

    pub fn try_observe_loss(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<Option<SiteLoss>, WindowsSiteProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        let status = child.try_wait().map_err(|error| {
            WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::WaitFailed,
                format!("Windows Site-process status check failed: {error}"),
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
    ) -> Result<Option<SiteLoss>, WindowsSiteProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        child.wait().map_err(|error| {
            WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::WaitFailed,
                format!("Windows Site-process wait failed: {error}"),
            )
        })?;
        self.child.take();
        self.report_loss(host).map(Some)
    }

    pub fn terminate_and_report(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<Option<SiteLoss>, WindowsSiteProcessError> {
        if self.loss_reported {
            return Ok(None);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };

        match child.try_wait().map_err(|error| {
            WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::WaitFailed,
                format!("Windows Site-process status check failed: {error}"),
            )
        })? {
            Some(_) => {}
            None => {
                child.kill().map_err(|error| {
                    WindowsSiteProcessError::new(
                        WindowsSiteProcessErrorKind::WaitFailed,
                        format!("Windows Site-process termination failed: {error}"),
                    )
                })?;
                child.wait().map_err(|error| {
                    WindowsSiteProcessError::new(
                        WindowsSiteProcessErrorKind::WaitFailed,
                        format!("Windows Site-process reap failed: {error}"),
                    )
                })?;
            }
        }

        self.child.take();
        self.report_loss(host).map(Some)
    }

    fn report_loss(
        &mut self,
        host: &mut HostControlPlane,
    ) -> Result<SiteLoss, WindowsSiteProcessError> {
        self.loss_reported = true;
        host.process_lost(self.lease.process()).map_err(|error| {
            WindowsSiteProcessError::new(
                WindowsSiteProcessErrorKind::HostControl,
                format!("Host Site-process loss handling failed: {error}"),
            )
        })
    }
}

impl Drop for WindowsSiteProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child.take();
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_child(command: &WindowsSiteProcessCommand) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut child = Command::new(command.program());
    child
        .args(command.args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    child.spawn()
}

#[cfg(not(target_os = "windows"))]
fn spawn_windows_child(_command: &WindowsSiteProcessCommand) -> std::io::Result<Child> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Windows Site-process launch is unavailable on this target",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rarog_broker::CapabilityClass;
    #[cfg(target_os = "windows")]
    use rarog_fetch::{
        FetchError, FetchRequest, NetworkCapability, NetworkPoll, NetworkRequest, NetworkTicket,
    };
    use rarog_url::WebUrl;
    #[cfg(target_os = "windows")]
    use std::num::NonZeroU64;

    fn site() -> rarog_url::SiteIdentity {
        WebUrl::parse("https://example.com/")
            .unwrap()
            .site_identity()
            .unwrap()
    }

    #[test]
    fn windows_site_process_command_is_bounded() {
        assert_eq!(
            WindowsSiteProcessCommand::try_new("").unwrap_err().kind,
            WindowsSiteProcessErrorKind::EmptyProgram
        );

        let mut command = WindowsSiteProcessCommand::try_new("rarog-site.exe").unwrap();
        for index in 0..DEFAULT_MAX_SITE_PROCESS_ARGS {
            command = command.try_arg(index.to_string()).unwrap();
        }
        assert_eq!(
            command.try_arg("overflow").unwrap_err().kind,
            WindowsSiteProcessErrorKind::ArgumentLimitExceeded
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_site_process_is_explicitly_unsupported_off_windows() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let lease = host.ensure_site(site()).unwrap();
        let command = WindowsSiteProcessCommand::try_new("rarog-site.exe").unwrap();

        assert_eq!(
            WindowsSiteProcess::launch(&host, lease, &command)
                .unwrap_err()
                .kind,
            WindowsSiteProcessErrorKind::UnsupportedTarget
        );
    }

    #[cfg(target_os = "windows")]
    struct PendingNetwork;

    #[cfg(target_os = "windows")]
    impl NetworkCapability for PendingNetwork {
        fn start(&mut self, _request: NetworkRequest) -> Result<NetworkTicket, FetchError> {
            Ok(NetworkTicket::new(NonZeroU64::new(1).unwrap()))
        }

        fn poll(&mut self, _ticket: NetworkTicket) -> Result<NetworkPoll, FetchError> {
            Ok(NetworkPoll::Pending)
        }

        fn cancel(&mut self, _ticket: NetworkTicket) -> Result<(), FetchError> {
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_site_process_exit_invalidates_host_authority_and_recovers_fresh() {
        let mut host = HostControlPlane::with_default_limits().unwrap();
        let site = site();
        let lease = host.ensure_site(site.clone()).unwrap();
        let capability = host
            .grant_capability(lease.process(), CapabilityClass::Network)
            .unwrap();
        let origin = WebUrl::parse("https://example.com/")
            .unwrap()
            .origin()
            .unwrap();
        let request = FetchRequest::new(
            WebUrl::parse("https://api.example.com/data").unwrap(),
            origin,
        )
        .network_request();
        let mut network = PendingNetwork;
        host.start_network_operation(
            lease.process(),
            capability.id(),
            request,
            &mut network,
        )
        .unwrap();

        let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| OsString::from("cmd.exe"));
        let command = WindowsSiteProcessCommand::try_new(shell)
            .unwrap()
            .try_arg("/D")
            .unwrap()
            .try_arg("/C")
            .unwrap()
            .try_arg("exit 0")
            .unwrap();
        let mut process = WindowsSiteProcess::launch(&host, lease, &command).unwrap();
        let loss = process.wait_for_loss(&mut host).unwrap().unwrap();

        assert_eq!(loss.process(), lease.process());
        assert_eq!(loss.revoked_capabilities(), 1);
        assert_eq!(loss.revoked_network_operations(), 1);
        assert_eq!(host.active_site_processes(), 0);
        assert_eq!(host.active_capabilities(), 0);
        assert_eq!(host.active_network_operations(), 0);
        assert!(process.try_observe_loss(&mut host).unwrap().is_none());

        let replacement = host.recover_site(site).unwrap();
        assert_ne!(replacement.process(), lease.process());
    }
}
