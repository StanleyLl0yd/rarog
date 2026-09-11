use crate::{HostControlError, HostControlErrorKind, HostControlPlane};
use rarog_broker::CapabilityClass;
use rarog_process::StorageProcessId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageLoss {
    process: StorageProcessId,
    revoked_capabilities: usize,
}

impl StorageLoss {
    pub fn process(self) -> StorageProcessId {
        self.process
    }

    pub fn process_generation(self) -> u64 {
        self.process.get()
    }

    pub fn revoked_capabilities(self) -> usize {
        self.revoked_capabilities
    }
}

impl HostControlPlane {
    pub fn storage_process_generation(&self) -> Option<u64> {
        self.topology.storage_process().map(StorageProcessId::get)
    }

    pub fn storage_process_lost_generation(
        &mut self,
        generation: u64,
    ) -> Result<StorageLoss, HostControlError> {
        let process = self.topology.storage_process().ok_or_else(|| {
            HostControlError::new(
                HostControlErrorKind::StorageProcessMismatch,
                format!("unknown, retired or replaced Storage process generation {generation}"),
            )
        })?;
        if process.get() != generation {
            return Err(HostControlError::new(
                HostControlErrorKind::StorageProcessMismatch,
                format!("unknown, retired or replaced Storage process generation {generation}"),
            ));
        }
        self.storage_process_lost(process)
    }

    pub fn storage_process_lost(
        &mut self,
        process: StorageProcessId,
    ) -> Result<StorageLoss, HostControlError> {
        if self.topology.storage_process() != Some(process) {
            return Err(HostControlError::new(
                HostControlErrorKind::StorageProcessMismatch,
                format!("unknown, retired or replaced Storage process {process}"),
            ));
        }

        let context_storage_capabilities = self
            .navigation_context_storage_origins
            .keys()
            .copied()
            .collect::<Vec<_>>();

        for id in &context_storage_capabilities {
            let context = self
                .navigation_context_capabilities
                .get(id)
                .copied()
                .ok_or_else(|| {
                    HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        format!("Storage capability {id} has no navigation-context owner"),
                    )
                })?;
            let owner = self.require_navigation_context(context)?.binding.process;
            self.broker
                .authorize(*id, owner, CapabilityClass::Storage)
                .map_err(|error| {
                    HostControlError::new(
                        HostControlErrorKind::InconsistentState,
                        format!("Storage capability {id} bookkeeping diverged: {error}"),
                    )
                })?;
        }

        let revoked_capabilities = self.broker.revoke_all_for_class(CapabilityClass::Storage);
        for id in &context_storage_capabilities {
            self.navigation_context_capabilities.remove(id);
            self.navigation_context_storage_origins.remove(id);
        }
        self.topology.retire_storage_process(process)?;

        Ok(StorageLoss {
            process,
            revoked_capabilities,
        })
    }

    pub fn recover_storage_process(&mut self) -> Result<StorageProcessId, HostControlError> {
        self.ensure_storage_process()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostLimits;
    use rarog_broker::CapabilityErrorKind;
    use rarog_ipc::IpcLimits;
    use rarog_storage::{StorageLimits, StorageProcessState};
    use rarog_url::WebUrl;

    fn host() -> HostControlPlane {
        HostControlPlane::try_new(HostLimits {
            max_site_processes: 2,
            ipc: IpcLimits {
                max_message_bytes: 32,
                max_queued_messages: 4,
                max_queued_bytes: 64,
            },
            max_capabilities: 8,
            max_network_operations: 4,
            max_navigation_contexts: 4,
            max_navigation_context_url_bytes: 512,
            max_service_worker_fetch_dispatches: 4,
            max_websocket_connections: 4,
            websocket_queues: Default::default(),
        })
        .unwrap()
    }

    #[test]
    fn storage_loss_revokes_all_storage_authority_and_recovery_is_fresh() {
        let mut host = host();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        let storage = host
            .grant_navigation_context_storage_capability(context.context())
            .unwrap();
        let network = host
            .grant_navigation_context_network_capability(context.context())
            .unwrap();
        let first = host.storage_process().unwrap();
        let site_process = host
            .require_navigation_context(context.context())
            .unwrap()
            .binding
            .process;
        let generic_storage = host
            .grant_capability(site_process, CapabilityClass::Storage)
            .unwrap();

        assert_eq!(host.storage_process_generation(), Some(first.get()));
        let loss = host.storage_process_lost_generation(first.get()).unwrap();
        assert_eq!(loss.process(), first);
        assert_eq!(loss.process_generation(), first.get());
        assert_eq!(loss.revoked_capabilities(), 2);
        assert_eq!(host.storage_process(), None);
        assert_eq!(host.active_navigation_contexts(), 1);
        assert_eq!(host.active_capabilities(), 1);
        assert_eq!(
            host.authorize_navigation_context_storage_capability(storage)
                .unwrap_err()
                .kind,
            HostControlErrorKind::InvalidNavigationContextStorageAuthority
        );
        assert_eq!(
            host.broker
                .authorize(generic_storage.id(), site_process, CapabilityClass::Storage)
                .unwrap_err()
                .kind,
            CapabilityErrorKind::UnknownCapability
        );
        assert_eq!(
            host.broker
                .authorize(network.id(), site_process, CapabilityClass::Network),
            Ok(())
        );

        let replacement = host.recover_storage_process().unwrap();
        assert_ne!(replacement, first);
        assert_eq!(host.storage_process_generation(), Some(replacement.get()));
        let replacement_capability = host
            .grant_navigation_context_storage_capability(context.context())
            .unwrap();
        let mut stale_state =
            StorageProcessState::try_new(first, StorageLimits::default()).unwrap();
        assert_eq!(
            host.write_navigation_context_storage(
                replacement_capability,
                "key",
                b"stale",
                &mut stale_state,
            )
            .unwrap_err()
            .kind,
            HostControlErrorKind::StorageProcessMismatch
        );

        assert_eq!(
            host.storage_process_lost_generation(first.get())
                .unwrap_err()
                .kind,
            HostControlErrorKind::StorageProcessMismatch
        );
        assert_eq!(host.storage_process(), Some(replacement));
        host.authorize_navigation_context_storage_capability(replacement_capability)
            .unwrap();
    }

    #[test]
    fn inconsistent_storage_grant_blocks_loss_before_retirement() {
        let mut host = host();
        let context = host
            .open_navigation_context(&WebUrl::parse("https://example.com/").unwrap())
            .unwrap();
        let storage = host
            .grant_navigation_context_storage_capability(context.context())
            .unwrap();
        let process = host.storage_process().unwrap();
        host.broker.revoke(storage.id()).unwrap();

        assert_eq!(
            host.storage_process_lost_generation(process.get())
                .unwrap_err()
                .kind,
            HostControlErrorKind::InconsistentState
        );
        assert_eq!(host.storage_process(), Some(process));
        assert_eq!(
            host.broker
                .authorize(
                    storage.id(),
                    host.require_navigation_context(context.context())
                        .unwrap()
                        .binding
                        .process,
                    CapabilityClass::Storage,
                )
                .unwrap_err()
                .kind,
            CapabilityErrorKind::UnknownCapability
        );
    }
}
