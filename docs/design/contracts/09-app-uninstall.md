<a id="qpn-sec-8-3-10"></a>
# 8.3.10 应用枚举与卸载事务

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface Win32UninstallRegistrationIdentity {
  hive: 'HKCU' | 'HKLM';
  registryView: '32' | '64';
  keyIdentityDigestSha256: Sha256;
  keyLastWriteTimeFiletime: FileTimeString;
  keySecurityDescriptorDigestSha256: Sha256;
  keyProtection: 'userWritable' | 'systemAdministratorsOnly';
  uninstallValuesDigestSha256: Sha256;
}

interface SealedElevatedProcessContext {
  contextVersion: 1;
  currentDirectory:
    | {
        kind: 'verifiedExecutableDirectory';
        directoryIdentityDigestSha256: Sha256;
        protection: 'systemAdministratorsOnly';
      }
    | {
        kind: 'verifiedSystem32';
        directoryIdentityDigestSha256: Sha256;
        protection: 'windowsResourceProtected';
      };
  environmentPolicy: 'minimalSystemV1';
  allowedEnvironmentNames: readonly [
    'SystemRoot',
    'SystemDrive',
    'TEMP',
    'TMP',
    'PATH',
  ];
  pathPolicy: 'verifiedSystem32ThenWindowsOnly';
  temporaryDirectoryIdentityDigestSha256: Sha256;
  childImageLoadMitigations: readonly [
    'preferSystem32',
    'noRemoteImages',
    'noLowMandatoryLabelImages',
  ];
  protectedDependencyIdentityDigests: Sha256[];
  adapterPlantingTestId: string;
}

type UninstallAdapter = 'msi' | 'appx' | 'win32Exe';

type CanonicalMsiInstallContext =
  | { kind: 'machine'; targetUserSidDigest?: never }
  | {
      kind: 'userManaged' | 'userUnmanaged';
      targetUserSidDigest: Sha256;
    };

interface MsiInstalledContextIdentity {
  canonicalProductCode: string;
  installContext: CanonicalMsiInstallContext;
  productState: 'installedDefault';
  productVersion: string;
  localPackageIdentityDigestSha256: Sha256;
  registrationIdentityDigestSha256: Sha256;
}

interface MsiSingletonContextEvidence {
  enumeratedBy: readonly ['MsiEnumProductsExW', 'MsiGetProductInfoExW'];
  enumerationScope: 'allContextsAllUsersVisibleToBroker';
  matchingContexts: readonly [MsiInstalledContextIdentity];
  target: MsiInstalledContextIdentity;
  contextSetDigestSha256: Sha256;
  capturedAtUtc: TimestampUtc;
}

type CanonicalUninstallResourceIdentity =
  | {
      adapter: 'msi';
      canonicalProductCode: string;
      installContext: CanonicalMsiInstallContext;
    }
  | {
      adapter: 'appx';
      packageFamilyName: string;
      targetUserSidDigest: Sha256;
    }
  | {
      adapter: 'win32Exe';
      hive: Win32UninstallRegistrationIdentity['hive'];
      registryView: Win32UninstallRegistrationIdentity['registryView'];
      canonicalRegistryKeyAddressDigestSha256: Sha256;
    };

type MsiUninstallInvocation<E extends boolean> = {
  adapter: 'msi';
  productCode: string;
  installContext: CanonicalMsiInstallContext;
  resourceIdentity: Extract<
    CanonicalUninstallResourceIdentity,
    { adapter: 'msi' }
  >;
  resourceIdentityDigestSha256: Sha256;
  singletonContextEvidence: MsiSingletonContextEvidence;
  requiresElevation: E;
};

type AppxUninstallInvocation = {
  adapter: 'appx';
  packageFullName: string;
  resourceIdentity: Extract<
    CanonicalUninstallResourceIdentity,
    { adapter: 'appx' }
  >;
  resourceIdentityDigestSha256: Sha256;
  requiresElevation: false;
};

interface Win32UninstallInvocationCommon {
  adapter: 'win32Exe';
  registrationIdentity: Win32UninstallRegistrationIdentity;
  resourceIdentity: Extract<
    CanonicalUninstallResourceIdentity,
    { adapter: 'win32Exe' }
  >;
  resourceIdentityDigestSha256: Sha256;
  encryptedAbsoluteExePath: string;
  volumeGuid: string;
  fileId128: string;
  sizeBytes: U64String;
  lastWriteTimeFiletime: FileTimeString;
  sha256: Sha256;
  fixedArguments: string[];
  locationProtection: 'userWritable' | 'administratorProtected';
  authenticode:
    | { status: 'valid'; leafSpkiSha256: Sha256 }
    | { status: 'unsigned' | 'invalid' | 'unknown' };
}

type SealedUninstallInvocation =
  | MsiUninstallInvocation<false>
  | MsiUninstallInvocation<true>
  | AppxUninstallInvocation
  | (Win32UninstallInvocationCommon & {
      requiresElevation: false;
      elevationPolicy: { kind: 'currentUserOnly' };
    })
  | (Win32UninstallInvocationCommon & {
      requiresElevation: true;
      elevationPolicy: {
        kind: 'compiledElevatedAdapter';
        argumentTemplateId: string;
        protectedProductMetadataDigestSha256: Sha256;
        trustedDisplayName: string;
        trustedPublisherName: string;
        sealedProcessContext: SealedElevatedProcessContext;
      };
    });

interface AppSnapshotBase {
  appSnapshotId: Uuid;
  capturedAtUtc: string;
  displayName: string;
  displayPublisher?: string;
  installScope: 'currentUser' | 'perMachine';
  snapshotDigest: Sha256;
}

type AppSnapshot = AppSnapshotBase &
  (
    | {
        uninstall: Extract<
          SealedUninstallInvocation,
          { requiresElevation: false }
        >;
        requiresElevation: false;
      }
    | {
        uninstall: Extract<
          SealedUninstallInvocation,
          { requiresElevation: true }
        >;
        requiresElevation: true;
      }
  );

interface AppView {
  appSnapshotId: Uuid;
  capturedAtUtc: string;
  displayName: string;
  displayPublisher?: string;
  installScope: 'currentUser' | 'perMachine';
  adapter: AppSnapshot['uninstall']['adapter'];
  requiresElevation: boolean;
  availability:
    | 'supported'
    | 'hiddenSystemComponent'
    | 'unsafeTarget'
    | 'unresolvedPriorAttempt';
}

interface UninstallObservationBase {
  observedAtUtc: string;
  targetPresence: 'present' | 'absent' | 'unknown';
  targetSnapshotDigestSha256?: Sha256;
  evidenceDigestSha256: Sha256;
}

type UninstallAdapterObservationEvidence = {
  msi: {
    postCallContextSetDigestSha256: Sha256;
    observedTargetContext: CanonicalMsiInstallContext;
    otherMatchingContexts: readonly [];
  };
  appx: {
    observedPackageFamilyName: string;
    observedTargetUserSidDigest: Sha256;
  };
  win32Exe: {
    observedRegistrationIdentityDigestSha256: Sha256;
  };
};

type UninstallObservationFor<A extends UninstallAdapter> =
  UninstallObservationBase &
  UninstallProcessEvidence<A> &
  UninstallAdapterObservationEvidence[A];

type UninstallObservation = {
  [A in UninstallAdapter]: UninstallObservationFor<A>;
}[UninstallAdapter];

type UninstallObservationWithTargetPresence<
  A extends UninstallAdapter,
  P extends UninstallObservationBase['targetPresence'],
> = A extends UninstallAdapter
  ? Omit<UninstallObservationFor<A>, 'targetPresence'> & {
      targetPresence: P;
    }
  : never;

interface UninstallAttemptBase<A extends UninstallAdapter> {
  recordVersion: 1;
  journalSequence: U64String;
  operationId: Uuid;
  planId: Uuid;
  planItemId: Uuid;
  appSnapshotId: Uuid;
  planHash: Sha256;
  invocationDigestSha256: Sha256;
  adapter: A;
  resourceIdentityDigestSha256: Sha256;
  coordination:
    | {
        scope: 'currentUser';
        ownerSidDigestSha256: Sha256;
        userJournalId: Uuid;
      }
    | {
        scope: 'machine';
        machineUninstallAttemptId: Uuid;
        machineJournalSequence: U64String;
        protectedGlobalMutexNameDigestSha256: Sha256;
        initiatingAdminSidDigestSha256: Sha256;
      };
  preparedAtUtc: string;
  observationDeadlineUtc: string;
}

interface Win32ControlledJobIdentityEvidence {
  jobIdentityDigestSha256: Sha256;
  killOnJobClose: true;
  breakawayProhibited: true;
}

interface Win32ControlledJobTreeDrainedEvidence {
  controlledJobTreeState: 'drained';
  controlledJobTreeDrainedAtUtc: TimestampUtc;
  controlledJobTreeEvidenceDigestSha256: Sha256;
}

type MsiUninstallFailureResultCode = number & {
  readonly __brand: 'MsiUninstallFailureExcluding0_3010_1641';
};

type MsiUninstallCallEvidence = {
  kind: 'msiCallCompleted';
  invokedAtUtc: TimestampUtc;
  callReturnedAtUtc: TimestampUtc;
  bootIdBeforeCall: string;
  apiReturnEvidenceDigestSha256: Sha256;
} &
  (
    | {
        result: 'successNoRestart';
        msiResultCode: 0;
        rebootEvidence?: never;
      }
    | {
        result: 'rebootRequired';
        msiResultCode: 3010;
        rebootEvidence?: never;
      }
    | {
        result: 'rebootSatisfied';
        msiResultCode: 3010;
        rebootEvidence: {
          bootIdAfterRestart: string;
          bootChangedAtUtc: TimestampUtc;
          evidenceDigestSha256: Sha256;
        };
      }
    | {
        result: 'unexpectedRestartOutcomeUnknown';
        msiResultCode: 1641;
        rebootEvidence?: never;
      }
    | {
        result: 'failed';
        msiResultCode: MsiUninstallFailureResultCode;
        rebootEvidence?: never;
      }
  );

type AppxDeploymentEvidence =
  | {
      kind: 'appxDeployment';
      phase: 'started';
      deploymentOperationId: Uuid;
      startedAtUtc: TimestampUtc;
    }
  | ({
      kind: 'appxDeployment';
      phase: 'completed';
      deploymentOperationId: Uuid;
      startedAtUtc: TimestampUtc;
      completedAtUtc: TimestampUtc;
      hresultHex: string;
      extendedHresultHex: string;
      activityId: Uuid;
      bootIdBeforeCall: string;
      deploymentResultEvidenceDigestSha256: Sha256;
    } &
      (
        | { result: 'successNoRestart'; rebootEvidence?: never }
        | { result: 'rebootRequired'; rebootEvidence?: never }
        | {
            result: 'rebootSatisfied';
            rebootEvidence: {
              bootIdAfterRestart: string;
              bootChangedAtUtc: TimestampUtc;
              evidenceDigestSha256: Sha256;
            };
          }
        | {
            result: 'failed' | 'outcomeUnknown';
            rebootEvidence?: never;
          }
      ));

type MsiKnownTerminalCallEvidence = Extract<
  MsiUninstallCallEvidence,
  { result: 'successNoRestart' | 'rebootSatisfied' }
>;

type AppxKnownTerminalDeploymentEvidence = Extract<
  AppxDeploymentEvidence,
  { phase: 'completed'; result: 'successNoRestart' | 'rebootSatisfied' }
>;

type UninstallLaunchEvidenceByAdapter = {
  msi: MsiUninstallCallEvidence;
  appx: AppxDeploymentEvidence;
  win32Exe: {
      kind: 'win32Process';
      createdSuspendedAtUtc: TimestampUtc;
      childIdentityVerifiedAtUtc: TimestampUtc;
      assignedToJobAtUtc: TimestampUtc;
      launchEvidencePersistedAtUtc: TimestampUtc;
      resumedAtUtc: TimestampUtc;
      launchEvidenceDigestSha256: Sha256;
      job: Win32ControlledJobIdentityEvidence;
      process: {
        pid: number;
        createdAtFiletime: FileTimeString;
        imageIdentityDigestSha256: Sha256;
      };
  };
};

type KnownRemovedResult<A extends UninstallAdapter> =
  A extends 'msi'
    ? {
        result: 'removed';
        launch: MsiKnownTerminalCallEvidence;
        observation: UninstallObservationWithTargetPresence<'msi', 'absent'>;
      }
    : A extends 'appx'
    ? {
        result: 'removed';
        launch: AppxKnownTerminalDeploymentEvidence;
        observation: UninstallObservationWithTargetPresence<
          'appx',
          'absent'
        >;
      }
    : {
        result: 'removed';
        launch: UninstallLaunchEvidenceByAdapter['win32Exe'];
        observation: UninstallObservationWithTargetPresence<
          'win32Exe',
          'absent'
        > &
          { processState: 'exited'; processExitCode: number } &
          Win32ControlledJobTreeDrainedEvidence;
      };

type KnownNotRemovedResult<A extends UninstallAdapter> =
  A extends 'msi'
    ? {
        result: 'notRemoved';
        launch: MsiKnownTerminalCallEvidence;
        observation: UninstallObservationWithTargetPresence<
          'msi',
          'present'
        >;
      }
    : A extends 'appx'
      ? {
          result: 'notRemoved';
          launch: AppxKnownTerminalDeploymentEvidence;
          observation: UninstallObservationWithTargetPresence<
            'appx',
            'present'
          >;
        }
      : {
          result: 'notRemoved';
          launch: UninstallLaunchEvidenceByAdapter['win32Exe'];
          observation: UninstallObservationWithTargetPresence<
            'win32Exe',
            'present'
          > &
            { processState: 'exited'; processExitCode: number } &
            Win32ControlledJobTreeDrainedEvidence;
        };

interface UninstallResolvedAttemptEvidenceRef {
  sourceAttemptJournalSequence: U64String;
  sourceAttemptRecordDigestSha256: Sha256;
}

type KnownRemovedEvidence<A extends UninstallAdapter> =
  Omit<KnownRemovedResult<A>, 'result'> &
  UninstallResolvedAttemptEvidenceRef &
  (A extends 'win32Exe'
    ? { processState: 'exited'; processExitCode: number }
    : NoProcessEvidence);

type KnownNotRemovedEvidence<A extends UninstallAdapter> =
  Omit<KnownNotRemovedResult<A>, 'result'> &
  UninstallResolvedAttemptEvidenceRef &
  KnownNotRemovedProcessEvidence<A>;

type UninstallRebootPendingEvidence<A extends UninstallAdapter> =
  A extends 'msi'
    ? {
        launch: Extract<
          MsiUninstallCallEvidence,
          { result: 'rebootRequired' }
        >;
      }
    : A extends 'appx'
      ? {
          launch: Extract<
            AppxDeploymentEvidence,
            { phase: 'completed'; result: 'rebootRequired' }
          >;
        }
      : never;

type UninstallAttemptState<A extends UninstallAdapter> =
  | { state: 'launchPrepared'; launch?: never }
  | {
      state: 'launched' | 'observing';
      launch: UninstallLaunchEvidenceByAdapter[A];
    }
  | (A extends 'msi' | 'appx'
      ? {
          state: 'rebootPending';
          launch: UninstallRebootPendingEvidence<A>['launch'];
        }
      : never)
  | ({
      state: 'resolved';
      resolvedAtUtc: string;
    } & (
      | KnownRemovedResult<A>
      | KnownNotRemovedResult<A>
      | {
          result: 'unknown';
          launch?: UninstallLaunchEvidenceByAdapter[A];
          observation: UninstallObservationFor<A>;
        }
      | {
          result: 'notStarted';
          launch?: never;
          sideEffectStarted: false;
          launchFailureCode: ErrorCode;
          observation: Omit<UninstallObservationBase, 'targetPresence'> &
            { targetPresence: 'present' | 'unknown' } &
            NoProcessEvidence;
        }
    ));

type UninstallAttempt = {
  [A in UninstallAdapter]: UninstallAttemptBase<A> & UninstallAttemptState<A>;
}[UninstallAdapter];

type UninstallTargetLock = {
  resourceIdentityDigestSha256: Sha256;
  sourceAttemptOperationId: Uuid;
  state:
    | 'active'
    | 'rebootPending'
    | 'recoveryRequired'
    | 'releasedAfterKnownNotStarted'
    | 'releasedAfterVerifiedNotRemoved'
    | 'releasedAfterVerifiedRemoval';
  lastDurableAtUtc: string;
} &
  (
    | {
        scope: 'currentUser';
        ownerSidDigestSha256: Sha256;
        userJournalId: Uuid;
      }
    | {
        scope: 'machine';
        machineUninstallAttemptId: Uuid;
        machineJournalSequence: U64String;
        protectedGlobalMutexNameDigestSha256: Sha256;
        machineStoreSecurityDescriptorDigestSha256: Sha256;
      }
  );

interface MachineUninstallAttachmentBase {
  attachmentVersion: 1;
  attachmentId: Uuid;
  machineUninstallAttemptId: Uuid;
  resourceIdentityDigestSha256: Sha256;
  ownerSidDigestSha256: Sha256;
  localOperationId: Uuid;
  localPlanId: Uuid;
  localPlanItemId: Uuid;
  localAppSnapshotId: Uuid;
  localPlanHash: Sha256;
  attachedAtMachineJournalSequence: U64String;
  attachedAttemptRecordDigestSha256: Sha256;
  attachedAtUtc: TimestampUtc;
}

type MachineUninstallAttachment = MachineUninstallAttachmentBase &
  (
    | {
        state: 'attached' | 'rebootPending' | 'recoveryRequired';
        sourceTerminalResult?: never;
        sourceResolvedAttemptJournalSequence?: never;
        sourceResolvedAttemptRecordDigestSha256?: never;
        localProjectionDigestSha256?: never;
        projectedAtUtc?: never;
      }
    | {
        state: 'projectedTerminal';
        sourceTerminalResult: 'removed' | 'notRemoved' | 'notStarted' | 'unknown';
        sourceResolvedAttemptJournalSequence: U64String;
        sourceResolvedAttemptRecordDigestSha256: Sha256;
        localProjectionDigestSha256: Sha256;
        projectedAtUtc: TimestampUtc;
      }
  );

interface MachineUninstallJournal {
  journalVersion: 1;
  scope: 'machine';
  machineUninstallAttemptId: Uuid;
  journalSequence: U64String;
  resourceIdentityDigestSha256: Sha256;
  owner: 'SYSTEMAndAdministratorsProtectedBroker';
  attempt: Extract<
    UninstallAttempt,
    { coordination: { scope: 'machine' } }
  >;
  targetLock: Extract<UninstallTargetLock, { scope: 'machine' }>;
  attachments: NonEmptyArray<MachineUninstallAttachment>;
  lastDurableAtUtc: TimestampUtc;
}

interface UninstallResultBase<A extends UninstallAdapter> {
  operationId: Uuid;
  planId: Uuid;
  planItemId: Uuid;
  appSnapshotId: Uuid;
  adapter: A;
  observedAtUtc: TimestampUtc;
}

type UninstallRejectedCode =
  | 'APP_SNAPSHOT_NOT_FOUND'
  | 'APP_SNAPSHOT_STALE'
  | 'UNINSTALL_TARGET_INVALID'
  | 'UNINSTALL_TARGET_AMBIGUOUS'
  | 'UNINSTALL_REBOOT_REQUIRED'
  | 'ACCESS_DENIED'
  | 'ELEVATION_SAME_USER_REQUIRED'
  | 'UAC_TIMEOUT'
  | 'IPC_PEER_INVALID'
  | 'IPC_PROTOCOL_INVALID'
  | 'ELEVATED_ACTION_NOT_ALLOWED'
  | 'STATE_STORE_UNAVAILABLE'
  | 'STATE_STORE_CORRUPT'
  | 'OPERATION_STATE_INVALID';

type UninstallResultState<A extends UninstallAdapter> =
  | ({
      state: 'pending';
      operationStatus:
        | 'created'
        | 'preflight'
        | 'elevationPending'
        | 'executing';
    } & NoProcessEvidence)
  | ({
      state: 'launched';
      operationStatus: 'awaitingExternalResult';
    } & UninstallProcessEvidence<A>)
  | (A extends 'msi' | 'appx'
      ? ({
          state: 'rebootPending';
          operationStatus: 'rebootPending';
        } & UninstallRebootPendingEvidence<A> & NoProcessEvidence)
      : never)
  | ({
      state: 'completed';
      operationStatus: 'succeeded';
    } & KnownRemovedEvidence<A>)
  | ({
      state: 'notRemoved';
      operationStatus: 'failed';
      errorCode: 'UNINSTALL_NOT_REMOVED';
    } & KnownNotRemovedEvidence<A>)
  | ({
      state: 'unknown';
      operationStatus: 'recoveryRequired';
      errorCode:
        | 'UNINSTALL_OUTCOME_UNKNOWN'
        | 'UNINSTALL_RECOVERY_REQUIRED';
    } & UninstallProcessEvidence<A>)
  | ({
      state: 'rejected';
      operationStatus: 'failed';
      errorCode: UninstallRejectedCode;
    } & NoProcessEvidence)
  | ({
      state: 'cancelled';
      operationStatus: 'cancelled';
      errorCode: 'USER_CANCELLED' | 'UAC_CANCELLED';
    } & NoProcessEvidence);

type UninstallResult = {
  [A in UninstallAdapter]:
    UninstallResultBase<A> & UninstallResultState<A>;
}[UninstallAdapter];

```
