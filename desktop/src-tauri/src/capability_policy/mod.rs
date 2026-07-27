mod build_channel;

use build_channel::{compiled_build_channel, BuildChannel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DangerousWriteCapability {
    ExperimentalQuarantineSourceRemoval,
    PermanentOriginalFileDelete,
    LegacyWin32UninstallLaunch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CapabilityPolicy {
    build_channel: BuildChannel,
}

impl CapabilityPolicy {
    pub(crate) const fn compiled() -> Self {
        Self {
            build_channel: compiled_build_channel(),
        }
    }

    pub(crate) fn require(self, capability: DangerousWriteCapability) -> Result<(), String> {
        if !matches!(self.build_channel, BuildChannel::ProductionRelease) {
            return Ok(());
        }

        Err(match capability {
            DangerousWriteCapability::ExperimentalQuarantineSourceRemoval => {
                "当前发布构建未启用实验性隔离源文件移除，源文件已安全保留".into()
            }
            DangerousWriteCapability::PermanentOriginalFileDelete => {
                "当前发布策略未启用原文件永久删除，文件已安全保留".into()
            }
            DangerousWriteCapability::LegacyWin32UninstallLaunch => {
                "当前发布策略未启用旧式 Win32 卸载器，未启动外部进程".into()
            }
        })
    }

    #[cfg(test)]
    pub(crate) const fn production_release_for_test() -> Self {
        Self {
            build_channel: BuildChannel::ProductionRelease,
        }
    }
}

pub(crate) fn require(capability: DangerousWriteCapability) -> Result<(), String> {
    CapabilityPolicy::compiled().require(capability)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DANGEROUS_CAPABILITIES: [DangerousWriteCapability; 3] = [
        DangerousWriteCapability::ExperimentalQuarantineSourceRemoval,
        DangerousWriteCapability::PermanentOriginalFileDelete,
        DangerousWriteCapability::LegacyWin32UninstallLaunch,
    ];

    #[test]
    fn production_release_denies_every_dangerous_write_capability() {
        let policy = CapabilityPolicy::production_release_for_test();

        for capability in DANGEROUS_CAPABILITIES {
            assert!(policy.require(capability).is_err());
        }
    }

    #[test]
    fn debug_and_internal_channels_allow_explicit_preview_capabilities() {
        for build_channel in [BuildChannel::Debug, BuildChannel::Internal] {
            let policy = CapabilityPolicy { build_channel };
            for capability in DANGEROUS_CAPABILITIES {
                assert_eq!(policy.require(capability), Ok(()));
            }
        }
    }

    #[test]
    fn compiled_policy_matches_the_compile_time_channel() {
        assert_eq!(
            CapabilityPolicy::compiled().build_channel,
            compiled_build_channel()
        );
    }
}
