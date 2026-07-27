<a id="qpn-sec-8-3-5"></a>
# 8.3.5 应用更新、安装器与可信恢复

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface AppUpdateManifestPayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  releaseEpoch: U64String;
  manifestSequence: U64String;
  versionScheme: 'semver2';
  targetVersion: string;
  binaryFileVersion: string;
  installerBuildSequence: U64String;
  minimumCurrentVersion: string;
  architecture: 'x64' | 'arm64';
  targetBinaryRelativePath: 'Qingpan.exe';
  targetBinarySha256: Sha256;
  targetPublisherSpkiSha256: Sha256;
  authenticodePolicyVersion: U64String;
  installerKind: 'msi';
  msiProductCode: string;
  msiUpgradeCode: string;
  msiProductVersion: string;
  packageSizeBytes: U64String;
  packageSha256: Sha256;
  releaseKeyId: string;
  keyAuthorizationPayloadSha256: Sha256;
  revocationPayloadSha256: Sha256;
  epochMigrationPayloadSha256s: Sha256[];
  issuedAtUtc: string;
  expiresAtUtc: string;
}

type SignedAppUpdateManifest = ReleaseSigned<AppUpdateManifestPayload> & {
  mediaType: 'application/vnd.qingpan.app-update+json';
};

interface ProductAuthenticodePolicy {
  policyVersion: U64String;
  allowedLeafSpkiSha256: NonEmptyArray<Sha256>;
  requiredEnhancedKeyUsageOids: readonly ['1.3.6.1.5.5.7.3.3'];
  requireEmbeddedOrRfc3161Timestamp: true;
  chainTrust: 'windowsTrustedRootAndPinnedLeaf';
  revocation: 'freshOnlineOrSignedOfflineStatus';
  unknownRevocationStatus: 'failClosed';
  allowedTargets: readonly [
    'bootstrapper',
    'msi',
    'installedBinary',
    'updateCoordinator',
    'recoveryExecutor',
  ];
}

type ProductAuthenticodeTarget =
  ProductAuthenticodePolicy['allowedTargets'][number];

interface AuthenticodeOfflineStatusPayload {
  schemaVersion: 1;
  statusSequence: U64String;
  recoveryKeySetId: string;
  productFamilyId: 'qingpan-desktop';
  artifactTarget: ProductAuthenticodeTarget;
  artifactSha256: Sha256;
  authenticodeEvidenceDigestSha256: Sha256;
  leafCertificateSha256: Sha256;
  leafSpkiSha256: Sha256;
  revocationStatus: 'good';
  thisUpdateAtUtc: TimestampUtc;
  nextUpdateAtUtc: TimestampUtc;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type RecoverySignedAuthenticodeOfflineStatus =
  RecoverySigned<AuthenticodeOfflineStatusPayload> & {
    mediaType: 'application/vnd.qingpan.authenticode-offline-status+json';
  };

interface AuthenticodeSignerRotationPayload {
  schemaVersion: 1;
  rotationSequence: U64String;
  recoveryKeySetId: string;
  productFamilyId: 'qingpan-desktop';
  fromPolicyVersion: U64String;
  toPolicyVersion: U64String;
  expectedCurrentSignerSetDigestSha256: Sha256;
  nextAllowedLeafSpkiSha256: NonEmptyArray<Sha256>;
  minimumInstallerBuildSequence: U64String;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type RecoverySignedAuthenticodeSignerRotation =
  RecoverySigned<AuthenticodeSignerRotationPayload> & {
    mediaType: 'application/vnd.qingpan.authenticode-signer-rotation+json';
  };

type SignedPayloadFloor =
  | { sequence: '0'; payloadSha256?: never }
  | { sequence: U64String; payloadSha256: Sha256 };

interface AppUpdateRevocationPayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  revocationSequence: U64String;
  recoveryKeySetId: string;
  revokedReleaseKeyIds: string[];
  revokedManifestPayloadHashes: Sha256[];
  revokedPackageHashes: Sha256[];
  blockedApplicationVersions: string[];
  minimumAcceptedReleaseEpoch: U64String;
  minimumAcceptedManifestSequence: U64String;
  issuedAtUtc: string;
  expiresAtUtc: string;
}

type RecoverySignedAppUpdateRevocation =
  RecoverySigned<AppUpdateRevocationPayload> & {
    mediaType: 'application/vnd.qingpan.app-update-revocations+json';
  };

interface AppEpochMigrationPayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  recoveryKeySetId: string;
  migrationSequence: U64String;
  fromEpoch: U64String;
  toEpoch: U64String;
  firstAcceptedManifestSequence: U64String;
  issuedAtUtc: string;
  expiresAtUtc: string;
}

type RecoverySignedAppEpochMigration =
  RecoverySigned<AppEpochMigrationPayload> & {
    mediaType: 'application/vnd.qingpan.app-epoch-migration+json';
  };

interface AppEpochMigrationResponse {
  schemaVersion: 1;
  migrations: RecoverySignedAppEpochMigration[];
}

interface AppUpdateTrustState {
  scope: 'machine';
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  machinePolicyRevision: U64String;
  channel: UpdateChannel;
  releaseEpoch: U64String;
  highestAuthorizationSequence: U64String;
  highestAuthorizationPayloadSha256: Sha256;
  highestRevocationSequence: U64String;
  highestRevocationPayloadSha256: Sha256;
  highestMigrationSequence: U64String;
  highestMigrationPayloadSha256?: Sha256;
  highestManifestSequence: U64String;
  highestManifestPayloadSha256: Sha256;
  acceptedKeyAuthorizations: RecoverySignedReleaseKeyAuthorization[];
  stickyMinimumAcceptedReleaseEpoch: U64String;
  stickyMinimumAcceptedManifestSequence: U64String;
  stickyRevokedReleaseKeyIds: string[];
  stickyRevokedManifestPayloadHashes: Sha256[];
  stickyRevokedPackageHashes: Sha256[];
  blockedApplicationVersions: string[];
  lastDurableAtUtc: string;
}

interface AvailablePreviousLkg {
  kind: 'available';
  channel: UpdateChannel;
  version: string;
  releaseKeyId: string;
  releaseEpoch: U64String;
  manifestSequence: U64String;
  installerBuildSequence: U64String;
  packageSha256: Sha256;
  manifestPayloadHash: Sha256;
}

type PreviousLkgState =
  | AvailablePreviousLkg
  | {
      kind: 'unavailable';
      reason:
        | 'noPriorCommittedManagedVersion'
        | 'packageMissing'
        | 'evidenceIncomplete'
        | 'artifactRevoked';
      automaticUpdateAllowed: false;
    };

type InstallSubject =
  | { kind: 'appUpdate'; updateId: Uuid; installerKind: 'msi' }
  | {
      kind: 'fullInstaller';
      installationRequestId: Uuid;
      installerKind: 'msi' | 'nsis';
    }
  | {
      kind: 'productUninstall';
      uninstallRequestId: Uuid;
      installerKind: 'msi';
    };

type AppUpdateInstallSubject = Extract<InstallSubject, { kind: 'appUpdate' }>;
type FullInstallerSubject = Extract<InstallSubject, { kind: 'fullInstaller' }>;
type ProductUninstallSubject = Extract<
  InstallSubject,
  { kind: 'productUninstall' }
>;

interface InstallAdmissionBase {
  admissionVersion: 2;
  admissionId: Uuid;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  productAuthenticodePolicyVersion: U64String;
  admissionTicketId: Uuid;
  admissionTicketPayloadSha256: Sha256;
  anchorRevisionAfterAdmission: U64String;
  installerTrustStateRevisionAfterAdmission: U64String;
  installerAdmissionFloorBeforeCall: U64String;
  floorAdvancedAtAdmission: false;
  admittedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type InstallAdmission<
  S extends InstallSubject = AppUpdateInstallSubject,
> = S extends InstallSubject
  ? InstallAdmissionBase &
      { subject: S } &
      (S extends ProductUninstallSubject
        ? {
            currentPackageSha256: Sha256;
            currentInstallerBuildSequence: U64String;
            expectedInstalledIdentityDigestSha256: Sha256;
            currentBinaryProvenanceDigestSha256: Sha256;
            canonicalMsiProductCode: string;
            canonicalMsiUpgradeCode: string;
            removalProperties: readonly ['REMOVE=ALL', 'REBOOT=ReallySuppress'];
            removalPropertiesDigestSha256: Sha256;
          }
        : {
            packageSha256: Sha256;
            targetArchitecture: 'x64' | 'arm64';
            admittedBuildSequence: U64String;
            expectedBaseIdentityDigestSha256: Sha256;
            installerArtifactProvenanceDigestSha256: Sha256;
            callerArtifactProvenanceDigestSha256: Sha256;
            targetBinaryProvenanceDigestSha256: Sha256;
            migrationBackupId?: Uuid;
          })
  : never;

interface ReconciledInstallEvidenceBase {
  kind: 'reconciledExactTarget';
  msiResultCode?: never;
  bootIdBeforeCall: string;
  observedAtUtc: TimestampUtc;
  targetArtifactIdentityDigestSha256: Sha256;
  windowsInstallerTransactionEvidenceDigestSha256: Sha256;
  rebootRequirementEvidenceDigestSha256: Sha256;
}

type MsiFailureResultCode = number & {
  readonly __brand: 'MsiFailureResultCodeExcluding0_3010_1641';
};

type SuccessfulInstallEvidence =
  | {
      kind: 'msiReturnSuccess';
      msiResultCode: 0;
      requiresReboot: false;
      bootIdBeforeCall: string;
      callReturnedAtUtc: TimestampUtc;
    }
  | {
      kind: 'msiReturnSuccess';
      msiResultCode: 3010;
      requiresReboot: true;
      bootIdBeforeCall: string;
      callReturnedAtUtc: TimestampUtc;
    }
  | (ReconciledInstallEvidenceBase & { requiresReboot: false })
  | (ReconciledInstallEvidenceBase & { requiresReboot: true });

type FailedInstallEvidence =
  | {
      kind: 'msiUnexpectedRestart';
      msiResultCode: 1641;
      bootIdBeforeCall: string;
      callReturnedAtUtc: TimestampUtc;
      mappedError: 'UPDATE_OUTCOME_UNKNOWN';
    }
  | {
      kind: 'msiReturnFailure';
      msiResultCode: MsiFailureResultCode;
      bootIdBeforeCall: string;
      callReturnedAtUtc: TimestampUtc;
      mappedError: 'UPDATE_INSTALL_FAILED';
    };

type InstallEvidence = SuccessfulInstallEvidence | FailedInstallEvidence;

type NsisFailureExitCode = number & {
  readonly __brand: 'NsisFailureExitCodeExcluding0';
};

type InstallerCallResult =
  | {
      installerKind: 'msi';
      operation: 'installOrUpdate' | 'productUninstall';
      evidence: InstallEvidence;
      observedFinalStateDigestSha256: Sha256;
    }
  | {
      installerKind: 'nsis';
      operation: 'installOrUpdate';
      kind: 'nsisReturnSuccess';
      exitCode: 0;
      callReturnedAtUtc: TimestampUtc;
      observedFinalStateDigestSha256: Sha256;
    }
  | {
      installerKind: 'nsis';
      operation: 'installOrUpdate';
      kind: 'nsisReturnFailure';
      exitCode: NsisFailureExitCode;
      mappedError: 'UPDATE_INSTALL_FAILED';
      callReturnedAtUtc: TimestampUtc;
      observedFinalStateDigestSha256: Sha256;
    };

type InstallerAdmissionAttempt = {
  attemptVersion: 3;
  installerAttemptId: Uuid;
  admission: InstallAdmission<InstallSubject>;
  lastDurableAtUtc: TimestampUtc;
} &
  (
    | { state: 'admitted'; callAttemptCount: 0; stateData?: never }
    | {
        state: 'callArmed';
        callAttemptCount: 1;
        stateData: {
          callArmedAtUtc: TimestampUtc;
          bootIdBeforeCall: string;
          invocationNonceDigestSha256: Sha256;
          anchorRevisionAfterFloorAdvance: U64String;
          installerAdmissionFloorAfterArm: U64String;
          floorAdvancedForAdmissionId: Uuid;
        };
      }
    | {
        state: 'inFlight';
        callAttemptCount: 1;
        stateData: {
          callArmedAtUtc: TimestampUtc;
          bootIdBeforeCall: string;
          invocationNonceDigestSha256: Sha256;
          anchorRevisionAfterFloorAdvance: U64String;
          installerAdmissionFloorAfterArm: U64String;
          floorAdvancedForAdmissionId: Uuid;
          callerProcessIdentityDigestSha256: Sha256;
          callStartedAtUtc: TimestampUtc;
        };
      }
    | {
        state: 'resolvedWithoutCall';
        callAttemptCount: 0;
        stateData: {
          reason: 'cancelled' | 'expired' | 'superseded' | 'policyChanged';
          floorRemainedAtPreAdmissionValue: true;
          exactSamePackageReadmissionAllowed: true;
          resolvedAtUtc: TimestampUtc;
        };
      }
    | {
        state: 'resolved';
        callAttemptCount: 1;
        stateData: { result: InstallerCallResult; resolvedAtUtc: TimestampUtc };
      }
    | {
        state: 'recoveryRequired';
        callAttemptCount: 0 | 1;
        stateData: {
          failedFrom: 'admitted' | 'callArmed' | 'inFlight';
          cause: AppUpdateErrorCode;
          recoveryEvidenceDigestSha256: Sha256;
        };
      }
  );

interface MachineFullInstallerJournalBase {
  journalVersion: 1;
  scope: 'machine';
  installationRequestId: Uuid;
  journalSequence: U64String;
  installerAttemptId: Uuid;
  admissionId: Uuid;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  initiatingAdminSidDigestSha256: Sha256;
  lastDurableAtUtc: TimestampUtc;
}

type FullInstallerAttemptInState<
  S extends InstallerAdmissionAttempt['state'],
> = InstallerAdmissionAttempt & {
  admission: InstallAdmission<FullInstallerSubject>;
  state: S;
};

type MachineFullInstallerJournal = MachineFullInstallerJournalBase &
  StrictUnion<
    | {
        state: 'admitted';
        attempt: FullInstallerAttemptInState<'admitted'>;
      }
    | {
        state: 'callArmed';
        attempt: FullInstallerAttemptInState<'callArmed'>;
      }
    | {
        state: 'inFlight';
        attempt: FullInstallerAttemptInState<'inFlight'>;
      }
    | {
        state: 'resolvedWithoutCall';
        attempt: FullInstallerAttemptInState<'resolvedWithoutCall'>;
      }
    | {
        state: 'resolved';
        completionKind: 'installerCall';
        attempt: FullInstallerAttemptInState<'resolved'>;
        recoveryResolutionRecordId?: never;
        resolutionEvidenceDigestSha256: Sha256;
      }
    | {
        state: 'resolved';
        completionKind: 'trustedRecovery';
        attempt: FullInstallerAttemptInState<'recoveryRequired'>;
        recoveryResolutionRecordId: Uuid;
        resolutionEvidenceDigestSha256: Sha256;
      }
    | {
        state: 'recoveryRequired';
        attempt: FullInstallerAttemptInState<'recoveryRequired'>;
        recoveryEvidenceDigestSha256: Sha256;
      }
  >;

interface MachineProductUninstallJournalBase {
  journalVersion: 1;
  scope: 'machine';
  uninstallRequestId: Uuid;
  journalSequence: U64String;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  expectedInstalledIdentityDigestSha256: Sha256;
  canonicalMsiProductCode: string;
  canonicalMsiUpgradeCode: string;
  initiatingAdminSidDigestSha256: Sha256;
  lastDurableAtUtc: TimestampUtc;
}

type MachineProductUninstallJournal = MachineProductUninstallJournalBase &
  StrictUnion<
    | {
        state: 'uninstallPrepared';
        attempt: InstallerAdmissionAttempt & {
          admission: InstallAdmission<ProductUninstallSubject>;
          state: 'admitted';
        };
      }
    | {
        state: 'uninstalling';
        attempt: InstallerAdmissionAttempt & {
          admission: InstallAdmission<ProductUninstallSubject>;
          state: 'callArmed' | 'inFlight';
        };
      }
    | {
        state: 'rebootPending';
        attempt: InstallerAdmissionAttempt & {
          admission: InstallAdmission<ProductUninstallSubject>;
          state: 'resolved';
        };
        result: Extract<
          SuccessfulInstallEvidence,
          { msiResultCode: 3010 }
        >;
      }
    | ({
        state: 'uninstalledPreserved';
        absenceEvidence: VerifiedProductAbsenceEvidence;
        anchorLifecycleRevisionAfterCommit: U64String;
      } &
        StrictUnion<
          | {
              completionKind: 'installerCall';
              attempt: InstallerAdmissionAttempt & {
                admission: InstallAdmission<ProductUninstallSubject>;
                state: 'resolved';
              };
              recoveryResolutionRecordId?: never;
            }
          | {
              completionKind: 'trustedRecovery';
              attempt: InstallerAdmissionAttempt & {
                admission: InstallAdmission<ProductUninstallSubject>;
                state: 'recoveryRequired';
              };
              recoveryResolutionRecordId: Uuid;
            }
        >)
    | {
        state: 'recoveryRequired';
        attempt: InstallerAdmissionAttempt & {
          admission: InstallAdmission<ProductUninstallSubject>;
        };
        cause: AppUpdateErrorCode;
        recoveryEvidenceDigestSha256: Sha256;
      }
  >;

type AppUpdateErrorCode = Extract<ErrorCode, `UPDATE_${string}`>;

interface AppUpdateJournalBase {
  journalVersion: 4;
  journalSequence: U64String;
  scope: 'machine';
  installationInstanceId: Uuid;
  updateId: Uuid;
  machinePolicyId: Uuid;
  machinePolicyRevision: U64String;
  channel: UpdateChannel;
  releaseKeyId: string;
  initiatingAdminSidDigest: Sha256;
  lastResumingAdminSidDigest?: Sha256;
  manifestPayloadHash: Sha256;
  packageSha256: Sha256;
  releaseEpoch: U64String;
  manifestSequence: U64String;
  installerBuildSequence: U64String;
  targetVersion: string;
  msiProductCode: string;
  msiUpgradeCode: string;
  msiProductVersion: string;
  binaryFileVersion: string;
  expectedBaseAppVersion: string;
  expectedBaseMsiProductCode: string;
  expectedBaseMsiProductVersion: string;
  expectedBaseInstallerBuildSequence: U64String;
  targetBinaryRelativePath: 'Qingpan.exe';
  targetBinarySha256: Sha256;
  targetPublisherSpkiSha256: Sha256;
  targetBinaryProvenanceDigestSha256: Sha256;
  rollbackEligibleUntilUtc: string;
  lastDurableAtUtc: string;
}

type AppUpdateJournalState =
  | {
      state: 'downloaded' | 'verified' | 'staged';
      previousLkg: PreviousLkgState;
      installAttemptCount: 0;
      rollbackAttemptCount: 0;
      stateData?: never;
    }
  | {
      state: 'installPrepared';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 0;
      rollbackAttemptCount: 0;
      stateData: { admission: InstallAdmission };
    }
  | {
      state: 'installing';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        callArmedAtUtc: TimestampUtc;
        bootIdBeforeCall: string;
      };
    }
  | {
      state: 'rebootPending';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        install: Extract<SuccessfulInstallEvidence, { requiresReboot: true }>;
      };
    }
  | {
      state: 'trialPending';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        install: SuccessfulInstallEvidence;
      };
    }
  | {
      state: 'trialLaunchArmed';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        install: SuccessfulInstallEvidence;
        trialSessionId: Uuid;
        trialSessionRecordSequence: U64String;
      };
    }
  | {
      state: 'trialRunning';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        install: SuccessfulInstallEvidence;
        trialSessionId: Uuid;
        trialSessionRecordSequence: U64String;
      };
    }
  | {
      state: 'committed';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        install: SuccessfulInstallEvidence;
        trialHealthReceiptDigestSha256: Sha256;
        committedAtUtc: string;
      };
    }
  | {
      state: 'rollbackPending';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 0;
      stateData: {
        admission: InstallAdmission;
        cause: AppUpdateErrorCode;
        install: InstallEvidence;
      };
    }
  | {
      state: 'rollingBack';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 1;
      stateData: {
        admission: InstallAdmission;
        cause: AppUpdateErrorCode;
        install: InstallEvidence;
        rollbackCallArmedAtUtc: string;
      };
    }
  | {
      state: 'rolledBack';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 1;
      rollbackAttemptCount: 1;
      stateData: {
        admission: InstallAdmission;
        cause: AppUpdateErrorCode;
        rollbackEvidenceDigestSha256: Sha256;
        rolledBackAtUtc: string;
      };
    }
  | {
      state: 'recoveryRequired';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 0 | 1;
      rollbackAttemptCount: 0 | 1;
      stateData: {
        admission: InstallAdmission;
        failedFrom:
          | 'installPrepared'
          | 'installing'
          | 'rebootPending'
          | 'trialPending'
          | 'trialLaunchArmed'
          | 'trialRunning'
          | 'rollbackPending'
          | 'rollingBack'
          | 'committed'
          | 'rolledBack';
        cause: AppUpdateErrorCode;
        install?: InstallEvidence;
        recoveryEvidenceDigestSha256: Sha256;
      };
    }
  | {
      state: 'recoveredExternally';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 0 | 1;
      rollbackAttemptCount: 0 | 1;
      stateData: {
        admission: InstallAdmission;
        recoveryEvidenceDigestSha256: Sha256;
        recoveryResolutionRecordId: Uuid;
      };
    }
  | {
      state: 'cancelled';
      previousLkg: PreviousLkgState;
      installAttemptCount: 0;
      rollbackAttemptCount: 0;
      stateData: {
        cancelledFrom: 'downloaded' | 'verified' | 'staged';
        admission?: never;
        cancelledAtUtc: string;
        reason:
          | 'user'
          | 'superseded'
          | 'policyChanged'
          | 'artifactMissing'
          | 'artifactRevoked';
      };
    }
  | {
      state: 'cancelled';
      previousLkg: AvailablePreviousLkg;
      installAttemptCount: 0;
      rollbackAttemptCount: 0;
      stateData: {
        cancelledFrom: 'installPrepared';
        admission: InstallAdmission;
        cancelledAtUtc: string;
        reason:
          | 'user'
          | 'superseded'
          | 'policyChanged'
          | 'artifactMissing'
          | 'artifactRevoked';
      };
    };

type AppUpdateJournal = AppUpdateJournalBase & AppUpdateJournalState;

interface AppUpdateJournalViewBase {
  updateId: Uuid;
  journalSequence: U64String;
  channel: UpdateChannel;
  targetVersion: string;
  requiresAdministrator: true;
  updatedAtUtc: TimestampUtc;
}

type AppUpdateJournalView = AppUpdateJournalViewBase &
  (
    | {
        state: 'downloaded';
        progress: 'available';
        canCancel: true;
        lastErrorCode?: never;
      }
    | {
        state: 'verified' | 'staged' | 'installPrepared';
        progress: 'readyToInstall';
        canCancel: true;
        lastErrorCode?: never;
      }
    | {
        state: 'installing';
        progress: 'installing';
        canCancel: false;
        lastErrorCode?: never;
      }
    | {
        state: 'rebootPending';
        progress: 'rebootRequired';
        canCancel: false;
        lastErrorCode?: never;
      }
    | {
        state: 'trialPending' | 'trialLaunchArmed' | 'trialRunning';
        progress: 'trial';
        canCancel: false;
        lastErrorCode?: never;
      }
    | {
        state: 'committed' | 'recoveredExternally';
        progress: 'completed';
        canCancel: false;
        lastErrorCode?: never;
      }
    | {
        state: 'rollbackPending' | 'rollingBack';
        progress: 'rollingBack';
        canCancel: false;
        lastErrorCode: AppUpdateErrorCode;
      }
    | {
        state: 'rolledBack';
        progress: 'rolledBack';
        canCancel: false;
        lastErrorCode: AppUpdateErrorCode;
      }
    | {
        state: 'recoveryRequired';
        progress: 'recoveryRequired';
        canCancel: false;
        lastErrorCode: AppUpdateErrorCode;
      }
    | {
        state: 'cancelled';
        progress: 'cancelled';
        canCancel: false;
        lastErrorCode?: never;
      }
  );

type AppUpdateCheckResult =
  | {
      kind: 'noUpdate';
      currentVersion: string;
      checkedAtUtc: TimestampUtc;
    }
  | {
      kind: 'updateAvailable';
      update: AppUpdateJournalView;
      checkedAtUtc: TimestampUtc;
    };

interface CancelAppUpdateRequest {
  updateId: Uuid;
  expectedJournalSequence: U64String;
}

type AppRecoveryAdmissionFloor =
  | {
      recoveryFloorState: 'none';
      highestRecoverySequence: '0';
      highestRecoveryPayloadSha256?: never;
      activeTrustedRecoveryAttemptId?: never;
      lastCompletedRecoveryResolutionRecordId?: never;
    }
  | {
      recoveryFloorState: 'acceptedPending';
      highestRecoverySequence: U64String;
      highestRecoveryPayloadSha256: Sha256;
      activeTrustedRecoveryAttemptId: Uuid;
      lastCompletedRecoveryResolutionRecordId?: Uuid;
    }
  | {
      recoveryFloorState: 'resolved';
      highestRecoverySequence: U64String;
      highestRecoveryPayloadSha256: Sha256;
      activeTrustedRecoveryAttemptId?: never;
      lastCompletedRecoveryResolutionRecordId: Uuid;
    };

type MachineInstallAnchorKey = {
  productFamilyId: 'qingpan-desktop';
};

interface MsiUpgradeCodeMigrationPayload {
  schemaVersion: 1;
  purpose: 'machineMsiUpgradeCodeMigration';
  recoveryKeySetId: string;
  machineInstallAnchorId: Uuid;
  productFamilyId: MachineInstallAnchorKey['productFamilyId'];
  expectedNativeMachineArchitecture: 'x64' | 'arm64';
  expectedAnchorRevision: U64String;
  expectedCurrentCanonicalMsiUpgradeCode: string;
  nextCanonicalMsiUpgradeCode: string;
  expectedPriorMigrationSequence: U64String;
  migrationSequence: U64String;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type RecoverySignedMsiUpgradeCodeMigration =
  RecoverySigned<MsiUpgradeCodeMigrationPayload> & {
    mediaType: 'application/vnd.qingpan.msi-upgrade-code-migration+json';
  };

type MachineMsiUpgradeCodeBinding =
  | {
      upgradeCodeBindingSource: 'compiledCanonical';
      canonicalMsiUpgradeCode: string;
      highestUpgradeCodeMigrationSequence: '0';
      highestUpgradeCodeMigrationPayloadSha256?: never;
    }
  | {
      upgradeCodeBindingSource: 'recoveryThresholdMigration';
      canonicalMsiUpgradeCode: string;
      highestUpgradeCodeMigrationSequence: U64String;
      highestUpgradeCodeMigrationPayloadSha256: Sha256;
    };

type MachineInstallAnchor = MachineInstallAnchorKey & {
  anchorVersion: 2;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  nativeMachineArchitecture: 'x64' | 'arm64';
  revision: U64String;
  lifecycle: 'active' | 'uninstalledPreserved' | 'recoveryRequired';
  installerAdmissionFloorBuildSequence: U64String;
  stickySecurityStateDigestSha256: Sha256;
  createdAtUtc: TimestampUtc;
  lastDurableAtUtc: TimestampUtc;
} & MachineMsiUpgradeCodeBinding & AppRecoveryAdmissionFloor;

interface InstallerTrustState {
  scope: 'machine';
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  revision: U64String;
  activeAdmission?: InstallerAdmissionAttempt;
  highestCommittedReleaseEpoch: U64String;
  highestCommittedManifestSequence: U64String;
  activeAppVersion: string;
  rollbackBaseline: PreviousLkgState;
  lastDurableAtUtc: string;
}

interface InstallationArtifactSecurityState {
  scope: 'installation';
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  revision: U64String;
  authenticodeOfflineStatusFloor: SignedPayloadFloor;
  authenticodeSignerRotationFloor: SignedPayloadFloor;
  activeAuthenticodePolicyVersion: U64String;
  activeAllowedLeafSpkiSha256: NonEmptyArray<Sha256>;
  stickyRevokedPackageHashesAcrossChannels: Sha256[];
  stickyBlockedVersionsAcrossChannels: string[];
  currentArtifact:
    | {
        kind: 'known';
        appVersion: string;
        packageSha256: Sha256;
        sourceChannel: UpdateChannel;
        releaseKeyId: string;
        manifestPayloadHash: Sha256;
        writeMode: 'normal' | 'readOnlyBlocked';
      }
    | {
        kind: 'identityOnly';
        appVersion: string;
        writeMode: 'readOnlyUntilPackageIdentityEstablished';
      };
  lastDurableAtUtc: string;
}

interface TrialLaunchIntent {
  trialSessionId: Uuid;
  nonceDigest: Sha256;
  jobObjectNameDigestSha256: Sha256;
  armedAtUtc: TimestampUtc;
  deadlineUtc: TimestampUtc;
  bootId: string;
}

interface TrialChildIdentity {
  pid: number;
  createdAtFiletime: FileTimeString;
  processIdentityDigestSha256: Sha256;
  tokenIdentityDigestSha256: Sha256;
}

interface TrialSessionRecordBase {
  recordVersion: 2;
  recordSequence: U64String;
  updateId: Uuid;
  targetVersion: string;
  expectedBinaryRelativePath: 'Qingpan.exe';
  expectedBinarySha256: Sha256;
  expectedBinaryFileVersion: string;
  expectedPublisherSpkiSha256: Sha256;
  expectedBinaryProvenanceDigestSha256: Sha256;
  launchTokenSource: 'initiatingSplitTokenAdminLinkedMediumToken';
  initiatingAdminSidDigest: Sha256;
  initiatingAdminLogonSidDigest: Sha256;
  initiatingAdminSessionId: number;
  initiatingAdminAuthenticationId: U64String;
  expectedChildSessionId: number;
  expectedChildTokenUserSidDigest: Sha256;
  expectedChildLogonSidDigest: Sha256;
  expectedChildAuthenticationId: U64String;
  expectedChildIntegrityLevel: 'medium';
  expectedChildElevationType: 'limited';
  expectedChildAdministratorGroup: 'denyOnly';
  expectedChildEnabledAdministratorPrivileges: readonly [];
  pipeName: string;
  nextMessageSequence: U64String;
  attempt: number;
  intent: TrialLaunchIntent;
}

type TrialSessionRecord = TrialSessionRecordBase &
  (
    | {
        state: 'launchArmed';
        child?: never;
        receiptDigestSha256?: never;
      }
    | {
        state: 'running' | 'connected';
        child: TrialChildIdentity;
        receiptDigestSha256?: never;
      }
    | {
        state: 'receiptAccepted';
        child: TrialChildIdentity;
        receiptDigestSha256: Sha256;
      }
    | {
        state: 'failed' | 'expired';
        child?: TrialChildIdentity;
        receiptDigestSha256?: never;
        failureCode: AppUpdateErrorCode;
      }
  );

interface TrialHealthReceipt {
  schemaVersion: 1;
  trialSessionId: Uuid;
  updateId: Uuid;
  targetVersion: string;
  nonceProofHmacSha256: Sha256;
  messageSequence: U64String;
  checks: {
    processStarted: true;
    machineStateReadable: true;
    quarantineInventoryConsistent: true;
    ruleSubsystem:
      | { state: 'available' }
      | { state: 'failedClosed'; code: 'RULES_UNAVAILABLE' };
    binarySchemaCompatibility: 'compatible';
  };
  checkSummaryDigestSha256: Sha256;
  createdAtUtc: string;
}

interface UserSchemaMigrationJournal {
  journalVersion: 1;
  migrationId: Uuid;
  installationInstanceId: Uuid;
  ownerSidDigest: Sha256;
  targetAppVersion: string;
  fromSchemaVersion: number;
  toSchemaVersion: number;
  backupId: Uuid;
  state:
    | 'pending'
    | 'migrating'
    | 'committed'
    | 'readOnlyRecovery';
  lastDurableAtUtc: string;
  lastErrorCode?: ErrorCode;
}

type InstallRecoverySource = StrictUnion<
  | {
      kind: 'appUpdate';
      updateId: Uuid;
      appUpdateJournalSequence: U64String;
      admissionId: Uuid;
    }
  | {
      kind: 'fullInstaller';
      installationRequestId: Uuid;
      installerAttemptId: Uuid;
      machineFullInstallerJournalSequence: U64String;
      admissionId: Uuid;
    }
  | {
      kind: 'productUninstall';
      uninstallRequestId: Uuid;
      machineUninstallJournalSequence: U64String;
      admissionId: Uuid;
    }
>;

type InstallTargetRecoverySource = Extract<
  InstallRecoverySource,
  { kind: 'appUpdate' | 'fullInstaller' }
>;
type ProductUninstallRecoverySource = Extract<
  InstallRecoverySource,
  { kind: 'productUninstall' }
>;

interface ExpectedProductUninstallIdentity {
  expectedInstalledIdentityDigestSha256: Sha256;
  currentPackageSha256: Sha256;
  currentInstallerBuildSequence: U64String;
  currentBinaryProvenanceDigestSha256: Sha256;
  canonicalMsiProductCode: string;
  canonicalMsiUpgradeCode: string;
  removalPropertiesDigestSha256: Sha256;
}

interface VerifiedProductAbsenceEvidence {
  evidenceVersion: 1;
  expectedInstalledIdentityDigestSha256: Sha256;
  observedAtUtc: TimestampUtc;
  msiRegistrationAbsent: true;
  productServicesAbsent: true;
  targetBinariesAbsent: true;
  windowsInstallerContextSetDigestSha256: Sha256;
  productAbsenceEvidenceDigestSha256: Sha256;
}

interface AppRecoveryPackagePayloadBase<
  S extends InstallRecoverySource,
> {
  schemaVersion: 2;
  recoverySequence: U64String;
  recoveryKeySetId: string;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  source: S;
  expectedPriorRecoverySequence: U64String;
  expectedNativeMachineArchitecture: 'x64' | 'arm64';
  allowedObservedStateDigests: NonEmptyArray<Sha256>;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type AppRecoveryPackagePayload = StrictUnion<
  | (AppRecoveryPackagePayloadBase<InstallTargetRecoverySource> & {
      recoveryAction: 'installRecoveryTarget';
      targetPackageArchitecture: 'x64' | 'arm64';
      targetPackageSha256: Sha256;
      targetInstallerBuildSequence: U64String;
      targetVersion: string;
    })
  | (AppRecoveryPackagePayloadBase<ProductUninstallRecoverySource> & {
      recoveryAction: 'completeProductUninstall';
      expectedProductIdentity: ExpectedProductUninstallIdentity;
      expectedAnchorRevisionBeforeCommit: U64String;
      removalProperties: readonly ['REMOVE=ALL', 'REBOOT=ReallySuppress'];
      requiredAbsenceEvidencePolicyDigestSha256: Sha256;
    })
>;

type RecoverySignedAppRecoveryPackage =
  RecoverySigned<AppRecoveryPackagePayload> & {
    mediaType: 'application/vnd.qingpan.app-recovery+json';
  };

interface TrustedRecoveryAttemptBase {
  attemptVersion: 2;
  attemptId: Uuid;
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  recoverySequence: U64String;
  recoveryPayloadSha256: Sha256;
  expectedPriorRecoverySequence: U64String;
  acceptedAnchorRevision: U64String;
  recoveryExecutorAuthenticodeDigestSha256: Sha256;
  createdAtUtc: TimestampUtc;
}

type TrustedRecoveryActionBinding = StrictUnion<
  | {
      source: InstallTargetRecoverySource;
      recoveryAction: 'installRecoveryTarget';
    }
  | {
      source: ProductUninstallRecoverySource;
      recoveryAction: 'completeProductUninstall';
    }
>;

type TrustedRecoveryAttemptState = StrictUnion<
    | { state: 'prepared'; executorAttemptCount: 0; stateData?: never }
    | {
        state: 'callArmed';
        executorAttemptCount: 1;
        stateData: {
          callArmedAtUtc: TimestampUtc;
          bootIdBeforeCall: string;
        };
      }
    | {
        state: 'inFlight';
        executorAttemptCount: 1;
        stateData: {
          callArmedAtUtc: TimestampUtc;
          executorPid: number;
          executorCreatedAtFiletime: FileTimeString;
          executorProcessIdentityDigestSha256: Sha256;
        };
      }
    | {
        state: 'resolved';
        executorAttemptCount: 0 | 1;
        stateData: {
          recoveryResolutionRecordId: Uuid;
          observedFinalStateDigestSha256: Sha256;
          resolvedAtUtc: TimestampUtc;
        };
      }
    | {
        state: 'recoveryRequired';
        executorAttemptCount: 0 | 1;
        stateData: {
          failedFrom: 'prepared' | 'callArmed' | 'inFlight';
          cause: AppUpdateErrorCode;
          recoveryEvidenceDigestSha256: Sha256;
        };
      }
>;

type TrustedRecoveryAttempt = TrustedRecoveryAttemptBase &
  TrustedRecoveryActionBinding &
  TrustedRecoveryAttemptState;

interface RecoveryResolutionRecordBase<S extends InstallRecoverySource> {
  recoveryResolutionRecordId: Uuid;
  source: S;
  machineInstallAnchorId: Uuid;
  acceptedRecoverySequence: U64String;
  sourceEvidenceDigestSha256: Sha256;
  recoveryPackagePayloadSha256: Sha256;
  recoveryInstallerAuthenticodeDigestSha256: Sha256;
  resolvedByAdminSidDigest: Sha256;
  observedFinalStateDigestSha256: Sha256;
  resolvedAtUtc: TimestampUtc;
}

type RecoveryResolutionRecord = StrictUnion<
  | (RecoveryResolutionRecordBase<InstallTargetRecoverySource> & {
      recoveryAction: 'installRecoveryTarget';
      outcome: 'recoveredToTarget' | 'recoveredToLkg';
      anchorLifecycle: 'active';
      anchorLifecycleRevisionAfterCommit: U64String;
    })
  | (RecoveryResolutionRecordBase<ProductUninstallRecoverySource> & {
      recoveryAction: 'completeProductUninstall';
      expectedProductIdentity: ExpectedProductUninstallIdentity;
      absenceEvidence: VerifiedProductAbsenceEvidence;
      outcome: 'uninstalledPreserved';
      anchorLifecycle: 'uninstalledPreserved';
      anchorLifecycleRevisionAfterCommit: U64String;
    })
>;

```
