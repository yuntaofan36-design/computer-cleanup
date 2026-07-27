<a id="qpn-sec-8-3-8"></a>
# 8.3.8 执行、逐项结果与空间核算

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface ElevationSessionRecord {
  elevationSessionId: Uuid;
  delegatingAppInstanceId: Uuid;
  purpose: 'executeClaimedPlan';
  planId: Uuid;
  operationId: Uuid;
  expectedClientPid: number;
  expectedClientCreatedAtFiletime: FileTimeString;
  userSidDigest: Sha256;
  logonSidDigest: Sha256;
  sessionId: number;
  pipeName: string;
  nonceDigest: Sha256;
  expectedHelperSha256: Sha256;
  expectedHelperProtocolVersion: number;
  expiresAtUtc: string;
  state: 'created' | 'consumed' | 'completed' | 'failed' | 'expired';
}

interface ElevationExecutionBundle {
  schemaVersion: 1;
  planId: Uuid;
  operationId: Uuid;
  planHash: Sha256;
  items: ReadonlyArray<PlanItem>;
  userSidDigest: Sha256;
  logonSidDigest: Sha256;
  sessionId: number;
  expiresAtUtc: string;
  executionBundleDigestSha256: Sha256;
}

type ActiveOperationStatus =
  | 'created'
  | 'preflight'
  | 'elevationPending'
  | 'executing'
  | 'awaitingExternalResult'
  | 'rebootPending'
  | 'verifying';

type TerminalOperationStatus =
  | 'succeeded'
  | 'partiallySucceeded'
  | 'failed'
  | 'cancelled'
  | 'recoveryRequired';

type OperationStatus = ActiveOperationStatus | TerminalOperationStatus;

type OperationRef =
  | {
      kind: 'planExecution';
      operationId: Uuid;
      planId: Uuid;
      acceptedAtUtc: string;
    }
  | {
      kind: 'restore';
      operationId: Uuid;
      acceptedAtUtc: string;
    }
  | {
      kind: 'quarantineSalvage';
      operationId: Uuid;
      acceptedAtUtc: string;
    };

interface OperationViewBase<R extends OperationRef = OperationRef> {
  ref: R;
  createdAtUtc: string;
  updatedAtUtc: string;
}

interface ActiveOperationProgress {
  itemCounts: Record<ItemOutcome, number>;
  accounting: SpaceAccounting;
  measuredAtSequence: U64String;
}

type OperationView =
  | (OperationViewBase & {
      status: ActiveOperationStatus;
      progress: ActiveOperationProgress;
      completedAtUtc?: never;
      terminalCode?: never;
      terminalResult?: never;
    })
  | (OperationViewBase<Extract<OperationRef, { kind: 'planExecution' }>> & {
      status?: never;
      progress?: never;
      completedAtUtc: string;
      terminalResult: Extract<ExecuteResult, { kind: 'planExecution' }>;
    })
  | (OperationViewBase<Extract<OperationRef, { kind: 'restore' }>> & {
      status?: never;
      progress?: never;
      completedAtUtc: string;
      terminalResult: Extract<ExecuteResult, { kind: 'restore' }>;
    })
  | (OperationViewBase<Extract<OperationRef, { kind: 'quarantineSalvage' }>> & {
      status?: never;
      progress?: never;
      completedAtUtc: string;
      terminalResult: Extract<ExecuteResult, { kind: 'quarantineSalvage' }>;
    });

type ItemOutcome = 'succeeded' | 'skipped' | 'failed' | 'unprocessed';

type Disposition =
  | 'originalPreserved'
  | 'stagedRecoverable'
  | 'containerRecoverableSourcePreserved'
  | 'exported'
  | 'salvageVerifiedCopy'
  | 'salvageUnverifiedCopy'
  | 'salvageSourcePreserved'
  | 'permanentlyRemoved'
  | 'applicationRemoved'
  | 'applicationStillPresent'
  | 'externalOutcomeUnknown'
  | 'unknownNeedsAttention'
  | 'notApplicable'
  | 'notAttempted';

type FileOperationItemRef =
  | {
      kind: 'fileCandidate';
      action: 'deleteRebuildableCache' | 'permanentDeleteOriginal';
      planItemId: Uuid;
      candidateId: Uuid;
    }
  | {
      kind: 'fileCandidate';
      action: 'quarantine';
      planItemId: Uuid;
      candidateId: Uuid;
    };

type AppUninstallOperationItemRef = {
  [A in UninstallAdapter]: {
    kind: 'appUninstall';
    planItemId: Uuid;
    appSnapshotId: Uuid;
    adapter: A;
  };
}[UninstallAdapter];

type OperationItemRef =
  | FileOperationItemRef
  | {
      kind: 'quarantinePurge';
      planItemId: Uuid;
      quarantineRecordId: Uuid;
    }
  | AppUninstallOperationItemRef
  | { kind: 'quarantineRestore'; quarantineRecordId: Uuid }
  | { kind: 'quarantineSalvage'; quarantineRecordId: Uuid };

type NoProcessEvidence = {
  processState: 'unavailable';
  processExitCode?: never;
};

type Win32ProcessEvidence =
  | { processState: 'exited'; processExitCode: number }
  | {
      processState: 'running' | 'detached' | 'unavailable';
      processExitCode?: never;
    };

type UninstallProcessEvidence<A extends UninstallAdapter> =
  A extends 'win32Exe' ? Win32ProcessEvidence : NoProcessEvidence;

type KnownNotRemovedProcessEvidence<A extends UninstallAdapter> =
  A extends 'win32Exe'
    ? { processState: 'exited'; processExitCode: number }
    : NoProcessEvidence;

type FileMutationTarget =
  | {
      kind: 'planFile';
      action: 'deleteRebuildableCache';
      risk: 'R1';
      planId: Uuid;
      planItemId: Uuid;
      candidateId: Uuid;
      snapshotDigestSha256: Sha256;
      targetBindingDigestSha256: Sha256;
    }
  | {
      kind: 'planFile';
      action: 'permanentDeleteOriginal';
      risk: 'R4';
      planId: Uuid;
      planItemId: Uuid;
      candidateId: Uuid;
      snapshotDigestSha256: Sha256;
      targetBindingDigestSha256: Sha256;
    }
  | {
      kind: 'quarantineSource';
      action: 'removeSourceAfterContainerCommit';
      risk: 'R2';
      planId: Uuid;
      planItemId: Uuid;
      candidateId: Uuid;
      quarantineRecordId: Uuid;
      containerIdentity: QuarantineObjectIdentity;
      sourceSnapshot: QuarantineSourceSnapshot;
      sourceBindingDigestSha256: Sha256;
    }
  | {
      kind: 'quarantineObject';
      action: 'purgeQuarantine';
      risk: 'R4';
      purgePlanId: Uuid;
      quarantineRecordId: Uuid;
      recordJournalSequence: U64String;
      expectedObjectIdentity: QuarantineObjectIdentity;
      fixedRepositoryEntryDigestSha256: Sha256;
    };

interface FileMutationAttemptBase {
  attemptVersion: 1;
  attemptId: Uuid;
  operationId: Uuid;
  operationItemId: Uuid;
  executorInstanceId: Uuid;
  target: FileMutationTarget;
  preparedAtUtc: string;
  lastDurableAtUtc: string;
}

type FileMutationAttempt = FileMutationAttemptBase &
  (
    | { phase: 'prepared' }
    | { phase: 'callPrepared'; callPreparedAtUtc: string }
    | {
        phase: 'callRejected';
        code: ErrorCode;
        callReturnedAtUtc: string;
      }
    | { phase: 'callAccepted'; callReturnedAtUtc: string }
    | {
        phase: 'removedVerified';
        verifiedByExecutorInstanceId: Uuid;
        verifiedAtUtc: string;
        absenceEvidenceDigestSha256: Sha256;
      }
    | {
        phase: 'resolvedNoMutation';
        observed: 'expectedPresent' | 'targetMissing' | 'targetChanged';
        code: ErrorCode;
        reconciledAtUtc: string;
        evidenceDigestSha256: Sha256;
      }
    | {
        phase: 'resolvedPreservedAfterPossibleCall';
        code: 'FILE_MUTATION_INTERRUPTED' | 'PURGE_INTERRUPTED';
        deletePendingObserved: false;
        reconciledAtUtc: string;
        evidenceDigestSha256: Sha256;
      }
    | {
        phase: 'outcomeUnknown';
        code: 'FILE_MUTATION_OUTCOME_UNKNOWN' | 'PURGE_OUTCOME_UNKNOWN';
        reconciledAtUtc: string;
        evidenceDigestSha256: Sha256;
      }
  );

type FileItemSkippedCode =
  | 'VOLUME_CHANGED'
  | 'PATH_OUTSIDE_ROOT'
  | 'PARENT_CHANGED'
  | 'IDENTITY_CHANGED'
  | 'REPARSE_POINT'
  | 'MULTIPLE_HARD_LINKS'
  | 'UNEXPECTED_STREAM'
  | 'PROCESS_RUNNING'
  | 'PROCESS_STATE_UNKNOWN'
  | 'FILE_NOT_FOUND'
  | 'FILE_LOCKED'
  | 'CLOUD_PLACEHOLDER'
  | 'EFS_UNSUPPORTED';

type AppUninstallItemSkippedCode =
  | 'PROCESS_RUNNING'
  | 'PROCESS_STATE_UNKNOWN';

type ItemSkippedCode =
  | FileItemSkippedCode
  | AppUninstallItemSkippedCode;

type ItemCancellationCode =
  | 'USER_CANCELLED'
  | 'UAC_CANCELLED'
  | 'APPROVAL_GRANT_NOT_FOUND'
  | 'APPROVAL_GRANT_INVALID'
  | 'APPROVAL_GRANT_EXPIRED'
  | 'APPROVAL_GRANT_REVOKED'
  | 'APPROVAL_GRANT_BINDING_MISMATCH'
  | 'APPROVAL_GRANT_RUN_LIMIT_REACHED'
  | 'AUTOMATION_POLICY_NOT_FOUND'
  | 'AUTOMATION_POLICY_INVALID'
  | 'JOB_NOT_FOUND'
  | 'JOB_INVALID'
  | 'JOB_CONFLICT'
  | 'JOB_DEFINITION_MISMATCH'
  | 'JOB_TRIGGER_ATTESTATION_INVALID'
  | 'JOB_RECONCILIATION_REQUIRED'
  | 'RULES_UNAVAILABLE'
  | 'RULE_KEY_REVOKED'
  | 'RULE_PACKAGE_REVOKED';

type FileDeleteFailedCode =
  | 'UNSUPPORTED_FILESYSTEM'
  | 'ACCESS_DENIED'
  | 'FILE_MUTATION_INTERRUPTED';

type FileQuarantinePreservedFailedCode =
  | 'UNSUPPORTED_FILESYSTEM'
  | 'ACCESS_DENIED'
  | 'DISK_FULL'
  | 'QUARANTINE_QUOTA'
  | 'QUARANTINE_ACCOUNTING_UNKNOWN'
  | 'QUARANTINE_LEDGER_INVALID'
  | 'QUARANTINE_UNAVAILABLE'
  | 'QUARANTINE_PREPARE_FAILED'
  | 'QUARANTINE_CRYPTO_UNSUPPORTED'
  | 'QUARANTINE_CONTENT_GUARD_UNAVAILABLE'
  | 'QUARANTINE_KEY_UNAVAILABLE'
  | 'QUARANTINE_CONTAINER_CREATE_FAILED'
  | 'QUARANTINE_COPY_FAILED'
  | 'QUARANTINE_COPY_INTERRUPTED'
  | 'QUARANTINE_SOURCE_CHANGED_DURING_COPY'
  | 'QUARANTINE_CONTAINER_INTEGRITY_FAILED';

type FileQuarantineStagedFailedCode =
  | 'FILE_MUTATION_INTERRUPTED';

type FileCurrentUnknownCode =
  | 'FILE_MUTATION_OUTCOME_UNKNOWN'
  | 'QUARANTINE_CONTAINER_COMMIT_FAILED'
  | 'QUARANTINE_RECOVERY_REQUIRED'
  | 'QUARANTINE_DAMAGED';

type PurgeKnownFailedCode =
  | 'UNSUPPORTED_FILESYSTEM'
  | 'ACCESS_DENIED'
  | 'QUARANTINE_ACCOUNTING_UNKNOWN'
  | 'QUARANTINE_LEDGER_INVALID'
  | 'QUARANTINE_UNAVAILABLE'
  | 'QUARANTINE_RECORD_NOT_FOUND'
  | 'QUARANTINE_STATE_INVALID'
  | 'QUARANTINE_IDENTITY_CHANGED'
  | 'PURGE_FAILED'
  | 'PURGE_INTERRUPTED';

type PurgeCurrentUnknownCode =
  | 'PURGE_OUTCOME_UNKNOWN'
  | 'QUARANTINE_RECOVERY_REQUIRED'
  | 'QUARANTINE_DAMAGED';

type RestoreKnownFailedCode =
  | 'UNSUPPORTED_FILESYSTEM'
  | 'ACCESS_DENIED'
  | 'DISK_FULL'
  | 'QUARANTINE_UNAVAILABLE'
  | 'QUARANTINE_RECORD_NOT_FOUND'
  | 'QUARANTINE_STATE_INVALID'
  | 'QUARANTINE_IDENTITY_CHANGED'
  | 'RESTORE_TARGET_INVALID'
  | 'RESTORE_TARGET_CONFLICT'
  | 'RESTORE_INTEGRITY_FAILED';

type RestoreCurrentUnknownCode =
  | 'RESTORE_INTERRUPTED'
  | 'QUARANTINE_RECOVERY_REQUIRED'
  | 'QUARANTINE_DAMAGED';

type SalvageKnownFailedCode =
  | RestoreKnownFailedCode
  | 'QUARANTINE_SALVAGE_FAILED';

type SalvageCurrentUnknownCode = RestoreCurrentUnknownCode;

type AppUninstallKnownFailedCode =
  | 'APP_SNAPSHOT_NOT_FOUND'
  | 'APP_SNAPSHOT_STALE'
  | 'UNINSTALL_TARGET_INVALID'
  | 'UNINSTALL_TARGET_AMBIGUOUS'
  | 'ACCESS_DENIED';

type AppUninstallCurrentUnknownCode =
  | 'UNINSTALL_OUTCOME_UNKNOWN'
  | 'UNINSTALL_RECOVERY_REQUIRED';

type FileItemUnprocessedCode =
  | Exclude<ItemCancellationCode, 'UAC_CANCELLED'>
  | 'STATE_STORE_UNAVAILABLE'
  | 'STATE_STORE_CORRUPT'
  | 'OPERATION_STATE_INVALID'
  | 'OPERATION_OUTCOME_UNKNOWN';

type PurgeItemUnprocessedCode =
  | 'USER_CANCELLED'
  | 'STATE_STORE_UNAVAILABLE'
  | 'STATE_STORE_CORRUPT'
  | 'OPERATION_STATE_INVALID'
  | 'OPERATION_OUTCOME_UNKNOWN';

type RestoreItemUnprocessedCode =
  | 'USER_CANCELLED'
  | 'RESTORE_AUTH_SNAPSHOT_INVALID'
  | 'STATE_STORE_UNAVAILABLE'
  | 'STATE_STORE_CORRUPT'
  | 'OPERATION_STATE_INVALID'
  | 'OPERATION_OUTCOME_UNKNOWN';

type AppUninstallItemUnprocessedCode =
  | 'USER_CANCELLED'
  | 'UAC_CANCELLED'
  | 'ELEVATION_SAME_USER_REQUIRED'
  | 'UAC_TIMEOUT'
  | 'IPC_PEER_INVALID'
  | 'IPC_PROTOCOL_INVALID'
  | 'ELEVATED_ACTION_NOT_ALLOWED'
  | 'STATE_STORE_UNAVAILABLE'
  | 'STATE_STORE_CORRUPT'
  | 'OPERATION_STATE_INVALID';

type ItemUnprocessedCode =
  | FileItemUnprocessedCode
  | PurgeItemUnprocessedCode
  | RestoreItemUnprocessedCode
  | AppUninstallItemUnprocessedCode;

type ItemFailedCode =
  | FileDeleteFailedCode
  | FileQuarantinePreservedFailedCode
  | FileQuarantineStagedFailedCode
  | FileCurrentUnknownCode
  | PurgeKnownFailedCode
  | PurgeCurrentUnknownCode
  | RestoreKnownFailedCode
  | RestoreCurrentUnknownCode
  | SalvageKnownFailedCode
  | AppUninstallKnownFailedCode
  | 'UNINSTALL_NOT_REMOVED'
  | AppUninstallCurrentUnknownCode;

type UnknownOutcomeItemCode =
  | FileCurrentUnknownCode
  | PurgeCurrentUnknownCode
  | RestoreCurrentUnknownCode
  | SalvageCurrentUnknownCode
  | AppUninstallCurrentUnknownCode;

type AssertNever<T extends never> = T;
type ItemResultStableCode =
  | ItemSkippedCode
  | ItemFailedCode
  | ItemUnprocessedCode;
type _ItemCodesAreStable = AssertNever<
  Exclude<ItemResultStableCode, ErrorCode>
>;
type _ItemOutcomeCodeSetsAreDisjoint = AssertNever<
  | Extract<ItemSkippedCode, ItemFailedCode | ItemUnprocessedCode>
  | Extract<ItemFailedCode, ItemUnprocessedCode>
>;

interface OperationItemResultBase<R extends OperationItemRef> {
  operationItemId: Uuid;
  ref: R;
}

type DeleteFileItemRef = Extract<
  FileOperationItemRef,
  { action: 'deleteRebuildableCache' | 'permanentDeleteOriginal' }
>;
type QuarantineFileItemRef = Extract<
  FileOperationItemRef,
  { action: 'quarantine' }
>;

type OperationItemResultContractTarget =
  | 'fileDelete'
  | 'fileQuarantine'
  | 'quarantinePurge'
  | 'quarantineRestore'
  | 'quarantineSalvage'
  | 'appMsi'
  | 'appAppx'
  | 'appWin32Exe';

type OperationItemResultPhase =
  | 'notStarted'
  | 'preflight'
  | 'prepare'
  | 'apply'
  | 'launch'
  | 'observe'
  | 'verify'
  | 'reconcile';

type OperationItemProcessEvidenceContract =
  | 'none'
  | 'uninstallUnavailable'
  | 'uninstallObserved'
  | 'uninstallExited';

interface OperationItemResultContractRow {
  targets: readonly OperationItemResultContractTarget[];
  codes: readonly ItemResultStableCode[];
  outcome: Exclude<ItemOutcome, 'succeeded'>;
  phases: readonly OperationItemResultPhase[];
  disposition: Disposition;
  retryable: boolean;
  unknownEvidence: 'required' | 'forbidden';
  processEvidence: OperationItemProcessEvidenceContract;
}

// This table, not independently maintained unions, generates TS/Rust/schema/tests.
const operationItemResultContract = [
  {
    targets: ['fileDelete', 'fileQuarantine'],
    codes: [
      'VOLUME_CHANGED', 'PATH_OUTSIDE_ROOT', 'PARENT_CHANGED',
      'IDENTITY_CHANGED', 'REPARSE_POINT', 'MULTIPLE_HARD_LINKS',
      'UNEXPECTED_STREAM', 'PROCESS_RUNNING', 'PROCESS_STATE_UNKNOWN',
      'FILE_LOCKED', 'CLOUD_PLACEHOLDER', 'EFS_UNSUPPORTED',
    ],
    outcome: 'skipped', phases: ['preflight'],
    disposition: 'originalPreserved', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete', 'fileQuarantine'], codes: ['FILE_NOT_FOUND'],
    outcome: 'skipped', phases: ['preflight'],
    disposition: 'notApplicable', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'],
    codes: ['PROCESS_RUNNING', 'PROCESS_STATE_UNKNOWN'],
    outcome: 'skipped', phases: ['preflight'],
    disposition: 'applicationStillPresent', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'uninstallUnavailable',
  },

  {
    targets: ['fileDelete'], codes: ['UNSUPPORTED_FILESYSTEM'],
    outcome: 'failed', phases: ['prepare'], disposition: 'originalPreserved',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete'], codes: ['ACCESS_DENIED'],
    outcome: 'failed', phases: ['apply'], disposition: 'originalPreserved',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete'], codes: ['FILE_MUTATION_INTERRUPTED'],
    outcome: 'failed', phases: ['reconcile'], disposition: 'originalPreserved',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete'], codes: ['FILE_MUTATION_OUTCOME_UNKNOWN'],
    outcome: 'failed', phases: ['apply', 'verify', 'reconcile'],
    disposition: 'unknownNeedsAttention', retryable: false,
    unknownEvidence: 'required', processEvidence: 'none',
  },

  {
    targets: ['fileQuarantine'], codes: ['UNSUPPORTED_FILESYSTEM'],
    outcome: 'failed', phases: ['prepare'], disposition: 'originalPreserved',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'], codes: ['ACCESS_DENIED'],
    outcome: 'failed', phases: ['apply'], disposition: 'originalPreserved',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'],
    codes: [
      'DISK_FULL', 'QUARANTINE_QUOTA', 'QUARANTINE_UNAVAILABLE',
      'QUARANTINE_PREPARE_FAILED', 'QUARANTINE_CRYPTO_UNSUPPORTED',
      'QUARANTINE_CONTENT_GUARD_UNAVAILABLE',
    ],
    outcome: 'failed', phases: ['prepare'], disposition: 'originalPreserved',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'],
    codes: ['QUARANTINE_ACCOUNTING_UNKNOWN', 'QUARANTINE_LEDGER_INVALID'],
    outcome: 'failed', phases: ['prepare'], disposition: 'originalPreserved',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'],
    codes: [
      'QUARANTINE_KEY_UNAVAILABLE', 'QUARANTINE_CONTAINER_CREATE_FAILED',
      'QUARANTINE_COPY_FAILED', 'QUARANTINE_COPY_INTERRUPTED',
      'QUARANTINE_SOURCE_CHANGED_DURING_COPY',
      'QUARANTINE_CONTAINER_INTEGRITY_FAILED',
    ],
    outcome: 'failed', phases: ['apply', 'verify'],
    disposition: 'originalPreserved', retryable: false,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'], codes: ['FILE_MUTATION_INTERRUPTED'],
    outcome: 'failed', phases: ['reconcile'],
    disposition: 'containerRecoverableSourcePreserved', retryable: false,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'], codes: ['QUARANTINE_CONTAINER_COMMIT_FAILED'],
    outcome: 'failed', phases: ['verify', 'reconcile'],
    disposition: 'unknownNeedsAttention', retryable: false,
    unknownEvidence: 'required', processEvidence: 'none',
  },
  {
    targets: ['fileQuarantine'],
    codes: ['QUARANTINE_RECOVERY_REQUIRED', 'QUARANTINE_DAMAGED'],
    outcome: 'failed', phases: ['reconcile'], disposition: 'unknownNeedsAttention',
    retryable: false, unknownEvidence: 'required', processEvidence: 'none',
  },

  {
    targets: ['quarantinePurge'],
    codes: [
      'UNSUPPORTED_FILESYSTEM', 'QUARANTINE_ACCOUNTING_UNKNOWN',
      'QUARANTINE_LEDGER_INVALID', 'QUARANTINE_UNAVAILABLE',
      'QUARANTINE_RECORD_NOT_FOUND', 'QUARANTINE_STATE_INVALID',
      'QUARANTINE_IDENTITY_CHANGED',
    ],
    outcome: 'failed', phases: ['prepare'], disposition: 'stagedRecoverable',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'], codes: ['ACCESS_DENIED', 'PURGE_FAILED'],
    outcome: 'failed', phases: ['apply'], disposition: 'stagedRecoverable',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'], codes: ['PURGE_INTERRUPTED'],
    outcome: 'failed', phases: ['reconcile'], disposition: 'stagedRecoverable',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'], codes: ['PURGE_OUTCOME_UNKNOWN'],
    outcome: 'failed', phases: ['apply', 'verify', 'reconcile'],
    disposition: 'unknownNeedsAttention', retryable: false,
    unknownEvidence: 'required', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'],
    codes: ['QUARANTINE_RECOVERY_REQUIRED', 'QUARANTINE_DAMAGED'],
    outcome: 'failed', phases: ['reconcile'], disposition: 'unknownNeedsAttention',
    retryable: false, unknownEvidence: 'required', processEvidence: 'none',
  },

  {
    targets: ['quarantineSalvage'],
    codes: [
      'UNSUPPORTED_FILESYSTEM', 'QUARANTINE_UNAVAILABLE',
      'QUARANTINE_RECORD_NOT_FOUND', 'QUARANTINE_STATE_INVALID',
      'QUARANTINE_IDENTITY_CHANGED', 'RESTORE_TARGET_INVALID',
    ],
    outcome: 'failed', phases: ['prepare'], disposition: 'salvageSourcePreserved',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore'],
    codes: [
      'UNSUPPORTED_FILESYSTEM', 'QUARANTINE_UNAVAILABLE',
      'QUARANTINE_RECORD_NOT_FOUND', 'QUARANTINE_STATE_INVALID',
      'QUARANTINE_IDENTITY_CHANGED', 'RESTORE_TARGET_INVALID',
    ],
    outcome: 'failed', phases: ['prepare'], disposition: 'stagedRecoverable',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore'], codes: ['ACCESS_DENIED', 'DISK_FULL'],
    outcome: 'failed', phases: ['apply'], disposition: 'stagedRecoverable',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineSalvage'], codes: ['ACCESS_DENIED', 'DISK_FULL'],
    outcome: 'failed', phases: ['apply'], disposition: 'salvageSourcePreserved',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore'], codes: ['RESTORE_TARGET_CONFLICT'],
    outcome: 'failed', phases: ['prepare', 'apply', 'reconcile'],
    disposition: 'stagedRecoverable', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineSalvage'], codes: ['RESTORE_TARGET_CONFLICT'],
    outcome: 'failed', phases: ['prepare', 'apply', 'reconcile'],
    disposition: 'salvageSourcePreserved', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore'], codes: ['RESTORE_INTEGRITY_FAILED'],
    outcome: 'failed', phases: ['verify'], disposition: 'stagedRecoverable',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineSalvage'],
    codes: ['RESTORE_INTEGRITY_FAILED', 'QUARANTINE_SALVAGE_FAILED'],
    outcome: 'failed', phases: ['verify'], disposition: 'salvageSourcePreserved',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore', 'quarantineSalvage'],
    codes: ['RESTORE_INTERRUPTED', 'QUARANTINE_RECOVERY_REQUIRED', 'QUARANTINE_DAMAGED'],
    outcome: 'failed', phases: ['apply', 'verify', 'reconcile'],
    disposition: 'unknownNeedsAttention', retryable: false,
    unknownEvidence: 'required', processEvidence: 'none',
  },

  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'],
    codes: [
      'APP_SNAPSHOT_NOT_FOUND', 'APP_SNAPSHOT_STALE',
      'UNINSTALL_TARGET_INVALID', 'UNINSTALL_TARGET_AMBIGUOUS',
    ],
    outcome: 'failed', phases: ['prepare'], disposition: 'applicationStillPresent',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'uninstallUnavailable',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'], codes: ['ACCESS_DENIED'],
    outcome: 'failed', phases: ['launch'], disposition: 'applicationStillPresent',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'uninstallUnavailable',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'], codes: ['UNINSTALL_NOT_REMOVED'],
    outcome: 'failed', phases: ['observe', 'verify', 'reconcile'],
    disposition: 'applicationStillPresent', retryable: true,
    unknownEvidence: 'forbidden', processEvidence: 'uninstallExited',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'],
    codes: ['UNINSTALL_OUTCOME_UNKNOWN', 'UNINSTALL_RECOVERY_REQUIRED'],
    outcome: 'failed', phases: ['launch', 'observe', 'verify', 'reconcile'],
    disposition: 'externalOutcomeUnknown', retryable: false,
    unknownEvidence: 'required', processEvidence: 'uninstallObserved',
  },

  {
    targets: ['fileDelete', 'fileQuarantine'],
    codes: [
      'USER_CANCELLED', 'APPROVAL_GRANT_NOT_FOUND', 'APPROVAL_GRANT_INVALID',
      'APPROVAL_GRANT_EXPIRED', 'APPROVAL_GRANT_REVOKED',
      'APPROVAL_GRANT_BINDING_MISMATCH', 'APPROVAL_GRANT_RUN_LIMIT_REACHED',
      'AUTOMATION_POLICY_NOT_FOUND', 'AUTOMATION_POLICY_INVALID',
      'JOB_NOT_FOUND', 'JOB_INVALID', 'JOB_CONFLICT',
      'JOB_DEFINITION_MISMATCH', 'JOB_TRIGGER_ATTESTATION_INVALID',
      'JOB_RECONCILIATION_REQUIRED', 'RULES_UNAVAILABLE',
      'RULE_KEY_REVOKED', 'RULE_PACKAGE_REVOKED', 'STATE_STORE_UNAVAILABLE',
    ],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete', 'fileQuarantine'],
    codes: ['STATE_STORE_CORRUPT', 'OPERATION_STATE_INVALID'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['fileDelete', 'fileQuarantine'], codes: ['OPERATION_OUTCOME_UNKNOWN'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'], codes: ['USER_CANCELLED', 'STATE_STORE_UNAVAILABLE'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantinePurge'],
    codes: ['STATE_STORE_CORRUPT', 'OPERATION_STATE_INVALID', 'OPERATION_OUTCOME_UNKNOWN'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore', 'quarantineSalvage'],
    codes: ['USER_CANCELLED', 'RESTORE_AUTH_SNAPSHOT_INVALID', 'STATE_STORE_UNAVAILABLE'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['quarantineRestore', 'quarantineSalvage'],
    codes: ['STATE_STORE_CORRUPT', 'OPERATION_STATE_INVALID', 'OPERATION_OUTCOME_UNKNOWN'],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'none',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'],
    codes: [
      'USER_CANCELLED', 'UAC_CANCELLED', 'ELEVATION_SAME_USER_REQUIRED',
      'UAC_TIMEOUT', 'STATE_STORE_UNAVAILABLE',
    ],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: true, unknownEvidence: 'forbidden', processEvidence: 'uninstallUnavailable',
  },
  {
    targets: ['appMsi', 'appAppx', 'appWin32Exe'],
    codes: [
      'IPC_PEER_INVALID', 'IPC_PROTOCOL_INVALID', 'ELEVATED_ACTION_NOT_ALLOWED',
      'STATE_STORE_CORRUPT', 'OPERATION_STATE_INVALID',
    ],
    outcome: 'unprocessed', phases: ['notStarted'], disposition: 'notAttempted',
    retryable: false, unknownEvidence: 'forbidden', processEvidence: 'uninstallUnavailable',
  },
] as const satisfies readonly OperationItemResultContractRow[];

type OperationItemResultContractEntry =
  (typeof operationItemResultContract)[number];
type OperationItemResultContractCode =
  OperationItemResultContractEntry['codes'][number];
type _OperationItemResultContractCoversAllCodes = AssertNever<
  Exclude<ItemResultStableCode, OperationItemResultContractCode>
>;
type _OperationItemResultContractHasNoExtraCodes = AssertNever<
  Exclude<OperationItemResultContractCode, ItemResultStableCode>
>;

type OperationItemRefForContractTarget<T extends OperationItemResultContractTarget> =
  T extends 'fileDelete' ? DeleteFileItemRef
  : T extends 'fileQuarantine' ? QuarantineFileItemRef
  : T extends 'quarantinePurge'
    ? Extract<OperationItemRef, { kind: 'quarantinePurge' }>
  : T extends 'quarantineRestore'
    ? Extract<OperationItemRef, { kind: 'quarantineRestore' }>
  : T extends 'quarantineSalvage'
    ? Extract<OperationItemRef, { kind: 'quarantineSalvage' }>
  : T extends 'appMsi'
    ? Extract<AppUninstallOperationItemRef, { adapter: 'msi' }>
  : T extends 'appAppx'
    ? Extract<AppUninstallOperationItemRef, { adapter: 'appx' }>
  : Extract<AppUninstallOperationItemRef, { adapter: 'win32Exe' }>;

type UninstallAdapterForContractTarget<T extends OperationItemResultContractTarget> =
  T extends 'appMsi' ? 'msi'
  : T extends 'appAppx' ? 'appx'
  : T extends 'appWin32Exe' ? 'win32Exe'
  : never;

type ProcessEvidenceFromContract<
  T extends OperationItemResultContractTarget,
  E extends OperationItemProcessEvidenceContract,
> = E extends 'none' ? unknown
  : E extends 'uninstallUnavailable' ? NoProcessEvidence
  : E extends 'uninstallExited'
    ? KnownNotRemovedProcessEvidence<UninstallAdapterForContractTarget<T>>
  : UninstallProcessEvidence<UninstallAdapterForContractTarget<T>>;

type KnownUninstallFailureEvidenceFromContract<
  R extends OperationItemResultContractEntry,
  T extends OperationItemResultContractTarget,
> = T extends 'appMsi' | 'appAppx' | 'appWin32Exe'
  ? 'UNINSTALL_NOT_REMOVED' extends R['codes'][number]
    ? KnownNotRemovedEvidence<UninstallAdapterForContractTarget<T>>
    : unknown
  : unknown;

type OperationItemErrorResultForTarget<
  R extends OperationItemResultContractEntry,
  T extends OperationItemResultContractTarget = R['targets'][number],
> = T extends OperationItemResultContractTarget
  ? OperationItemResultBase<OperationItemRefForContractTarget<T>> & {
      outcome: R['outcome'];
      phase: R['phases'][number];
      disposition: R['disposition'];
      code: R['codes'][number];
      retryable: R['retryable'];
    } & (R['unknownEvidence'] extends 'required'
      ? { unknownEvidence: true }
      : { unknownEvidence?: never }) &
      ProcessEvidenceFromContract<T, R['processEvidence']> &
      KnownUninstallFailureEvidenceFromContract<R, T>
  : never;

type OperationItemErrorResult =
  OperationItemResultContractEntry extends infer R
    ? R extends OperationItemResultContractEntry
      ? OperationItemErrorResultForTarget<R>
      : never
    : never;

type SuccessfulItem<D extends Disposition, P extends OperationItemResultPhase> = {
  outcome: 'succeeded';
  phase: P;
  disposition: D;
  code?: never;
  retryable: false;
  unknownEvidence?: never;
};

type AppUninstallSucceededItemResult = {
  [A in UninstallAdapter]: OperationItemResultBase<
    Extract<AppUninstallOperationItemRef, { adapter: A }>
  > &
    SuccessfulItem<'applicationRemoved', 'observe' | 'verify' | 'reconcile'> &
    KnownRemovedEvidence<A>;
}[UninstallAdapter];

type OperationItemSucceededResult =
  | (OperationItemResultBase<DeleteFileItemRef> &
      SuccessfulItem<'permanentlyRemoved', 'verify' | 'reconcile'>)
  | (OperationItemResultBase<QuarantineFileItemRef> &
      SuccessfulItem<'stagedRecoverable', 'verify' | 'reconcile'>)
  | (OperationItemResultBase<
        Extract<OperationItemRef, { kind: 'quarantinePurge' }>
      > & SuccessfulItem<'permanentlyRemoved', 'verify' | 'reconcile'>)
  | (OperationItemResultBase<
        Extract<OperationItemRef, { kind: 'quarantineRestore' }>
      > & SuccessfulItem<'exported', 'verify' | 'reconcile'>)
  | (OperationItemResultBase<
        Extract<OperationItemRef, { kind: 'quarantineSalvage' }>
      > & SuccessfulItem<
        'salvageVerifiedCopy' | 'salvageUnverifiedCopy',
        'verify' | 'reconcile'
      >)
  | AppUninstallSucceededItemResult;

type OperationItemResult =
  | OperationItemSucceededResult
  | OperationItemErrorResult;

interface SpaceTotals {
  originalLocationRemovedBytes: U64String;
  stagedBytes: U64String;
  verifiedPurgeBytes: U64String;
  reclaimedBytes: U64String;
  outcomeUnknownBytes: U64String;
  restoreTargetBytesWritten: U64String;
}

type AvailableSpaceObservationUnavailableReason =
  | 'preObservationFailed'
  | 'postObservationFailed'
  | 'volumeUnavailable'
  | 'counterNotSupported';

type VolumeAvailableSpaceObservation =
  | {
      status: 'measured';
      availableBytesBefore: U64String;
      availableBytesAfter: U64String;
      observedBeforeAtUtc: TimestampUtc;
      observedAfterAtUtc: TimestampUtc;
      deltaBytes: I64String;
      attribution: 'wholeVolumeWindow';
    }
  | {
      status: 'unavailable';
      reason: AvailableSpaceObservationUnavailableReason;
    }
  | { status: 'notApplicable'; reason?: never };

type VolumeSpaceAccounting = Omit<SpaceTotals, 'reclaimedBytes'> & {
  volumeGuid: string;
  availableSpaceObservation: VolumeAvailableSpaceObservation;
} &
  (
    | {
        reclaimedBytes: '0';
        reclaimedBasis: 'notApplicable';
      }
    | {
        reclaimedBytes: U64String;
        reclaimedBasis:
          | 'observedVolumeFreeSpace'
          | 'allocatedSizeEstimate'
          | 'logicalFallback'
          | 'mixed';
      }
  );

interface SpaceAccounting {
  totals: SpaceTotals;
  byVolume: VolumeSpaceAccounting[];
  availableSpaceObservation:
    | { status: 'notApplicable' }
    | { status: 'complete'; deltaBytes: I64String }
    | {
        status: 'partial';
        measuredVolumeCount: number;
        unavailableVolumeCount: number;
      }
    | {
        status: 'unavailable';
        reason: AvailableSpaceObservationUnavailableReason;
      };
}

interface ExecuteResultBase {
  schemaVersion: 1;
  operationId: Uuid;
  itemCounts: Record<ItemOutcome, number>;
  accounting: SpaceAccounting;
}

type ExecuteTerminal<C extends ErrorCode> =
  | { status: 'succeeded'; terminalCode?: never }
  | { status: 'partiallySucceeded'; terminalCode?: ErrorCode }
  | { status: 'failed'; terminalCode: ErrorCode }
  | { status: 'cancelled'; terminalCode: C }
  | { status: 'recoveryRequired'; terminalCode: ErrorCode };

type ExecuteResult =
  | (ExecuteResultBase &
      ExecuteTerminal<ItemCancellationCode> & {
      kind: 'planExecution';
      planId: Uuid;
    })
  | (ExecuteResultBase &
      ExecuteTerminal<'USER_CANCELLED'> & {
      kind: 'restore';
      planId?: never;
    })
  | (ExecuteResultBase &
      ExecuteTerminal<'USER_CANCELLED'> & {
      kind: 'quarantineSalvage';
      planId?: never;
    });

```
