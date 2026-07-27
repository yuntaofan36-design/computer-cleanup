<a id="qpn-sec-8-3-9"></a>
# 8.3.9 隔离、恢复、导出与清除

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface QuarantineRecordMeta {
  recordVersion: 5;
  journalSequence: U64String;
  recordId: Uuid;
  lastDurableAtUtc: string;
  lastErrorCode?: ErrorCode;
}

interface KnownQuarantineOrigin {
  quarantineOperationId: Uuid;
  planId: Uuid;
  planItemId: Uuid;
  candidateId: Uuid;
  ruleId?: string;
  rulePackageHash?: Sha256;
  action: 'quarantine';
  logicalBytes: U64String;
  allocatedBytes?: U64String;
}

interface QuarantineSourceSnapshot {
  volumeGuid: string;
  volumeSerialNumber: U64String;
  fileId128: string;
  parentChainDigestSha256: Sha256;
  sizeBytes: U64String;
  allocatedBytes?: U64String;
  creationTimeFiletime: FileTimeString;
  lastWriteTimeFiletime: FileTimeString;
  changeTimeFiletime: FileTimeString;
  usn?: U64String;
  attributes: number;
  hardLinkCount: 1;
  streamCount: 1;
  streamSetDigestSha256: Sha256;
  securityDescriptorDigestSha256: Sha256;
  snapshotDigestSha256: Sha256;
}

interface QuarantineObjectIdentity {
  containerFormat: 'QPC1';
  containerFormatVersion: 1;
  volumeGuid: string;
  fileId128: string;
  ciphertextBytes: U64String;
  allocatedBytes: U64String;
  headerDigestSha256: Sha256;
  manifestDigestSha256: Sha256;
  ciphertextSha256: Sha256;
  encryptedPlaintextSha256: string;
  wrappedDekDigestSha256: Sha256;
  ownerSidDigestSha256: Sha256;
  groupSidDigestSha256: Sha256;
  daclDigestSha256: Sha256;
  mandatoryLabelDigestSha256: Sha256;
  hardLinkCount: 1;
  streamCount: 1;
  identityDigestSha256: Sha256;
}

interface QuarantineContentGuardEvidence {
  api: 'FSCTL_REQUEST_OPLOCK';
  requestedLevels: readonly [
    'OPLOCK_LEVEL_CACHE_READ',
    'OPLOCK_LEVEL_CACHE_WRITE',
    'OPLOCK_LEVEL_CACHE_HANDLE',
  ];
  requestInputDigestSha256: Sha256;
  grantEvidenceDigestSha256: Sha256;
  breakChannelId: Uuid;
  breakSequence: U64String;
  grantedAtUtc: TimestampUtc;
  platformMappedViewEvidenceId: string;
  state: 'granted';
}

interface QuarantineContainerSecurityIdentity {
  ownerSidDigestSha256: Sha256;
  groupSidDigestSha256: Sha256;
  daclDigestSha256: Sha256;
  mandatoryLabelDigestSha256: Sha256;
  inheritance: 'protected';
  integrity: 'mediumOrHigherNoWriteUp';
  securityIdentityDigestSha256: Sha256;
}

interface TemporaryContainerIdentity
  extends QuarantineContainerSecurityIdentity {
  volumeGuid: string;
  fileId128: string;
  creationMarkerSha256: Sha256;
}

type QuotaCharge =
  | { kind: 'known'; chargedAllocatedBytes: U64String }
  | { kind: 'accountingUnknown'; blocksNewQuarantine: true };

type QuotaReservation = {
  reservationVersion: 1;
  reservationId: Uuid;
  volumeGuid: string;
  ownerSidDigest: Sha256;
  recordId: Uuid;
  operationItemId: Uuid;
  reservedUpperBoundBytes: U64String;
  createdAtUtc: string;
} &
  (
    | { state: 'active' }
    | {
        state: 'converted';
        ledgerChargeId: Uuid;
        convertedAtUtc: string;
      }
    | {
        state: 'released';
        reason:
          | 'sourcePreserved'
          | 'verifiedRollback'
          | 'verifiedNoRepositoryObject';
        releasedAtUtc: string;
      }
  );

type QuotaLedgerCharge = {
  ledgerChargeId: Uuid;
  volumeGuid: string;
  ownerSidDigest: Sha256;
  owner:
    | { kind: 'quarantineRecord'; recordId: Uuid }
    | { kind: 'tombstone'; recordId: Uuid }
    | { kind: 'repositoryInfrastructure' };
  charge: QuotaCharge;
  createdAtUtc: string;
} &
  (
    | { state: 'active' }
    | {
        state: 'released';
        releasedAtUtc: string;
        releaseEvidenceDigestSha256: Sha256;
      }
  );

interface QuarantineQuotaLedger {
  ledgerVersion: 1;
  volumeGuid: string;
  ownerSidDigest: Sha256;
  revision: U64String;
  quotaLimitBytes: U64String;
  safetyReserveBytes: U64String;
  activeReservationBytes: U64String;
  knownChargeBytes: U64String;
  admission:
    | { kind: 'open'; unknownChargeCount: 0 }
    | {
        kind: 'blocked';
        reason: 'accountingUnknown' | 'overQuota' | 'ledgerInvalid';
        unknownChargeCount: number;
      };
  lastReconciledAtUtc: string;
}

interface QuarantinePreparedData {
  planHash: Sha256;
  quotaReservationId: Uuid;
  reservedUpperBoundBytes: U64String;
  quotaLedgerRevisionAtReserve: U64String;
  sourceSnapshot: QuarantineSourceSnapshot;
  encryptedOriginalPath: string;
  encryptedOriginalSecurityDescriptor: string;
  temporaryContainerRelativeName: string;
  finalContainerRelativeName: string;
  preparedAtUtc: string;
}

interface QuarantineSourceDigestPreparedData extends QuarantinePreparedData {
  sourceSnapshotAtDigest: QuarantineSourceSnapshot;
  encryptedPreCopyPlaintextSha256: string;
  contentGuard: QuarantineContentGuardEvidence;
  sourceDigestPreparedAtUtc: TimestampUtc;
}

interface QuarantineContainerPreparedData
  extends QuarantineSourceDigestPreparedData {
  copyAttemptId: Uuid;
  cryptoSuite: 'AES-256-GCM';
  chunkSizeBytes: 4194304;
  nonceConstruction: 'random32be-prefix+uint64be-index';
  noncePrefixBase64: string;
  wrappedDekDpapiCurrentUser: string;
  wrappedDekDigestSha256: Sha256;
  qpc1HeaderDigestSha256: Sha256;
  temporaryContainerIdentity: TemporaryContainerIdentity;
  containerPreparedAtUtc: TimestampUtc;
}

interface QuarantineCopyingData extends QuarantineContainerPreparedData {
  nextChunkIndex: U64String;
  plaintextBytesProcessed: U64String;
  containerBytesWritten: U64String;
  progressPersistedAtUtc: TimestampUtc;
}

interface QuarantineCopiedData extends QuarantineContainerPreparedData {
  chunkCount: U64String;
  plaintextBytes: U64String;
  encryptedCopyPlaintextSha256: string;
  temporaryCiphertextSha256: Sha256;
  qpc1ManifestDigestSha256: Sha256;
  copiedAtUtc: TimestampUtc;
}

interface QuarantineContainerVerifiedData extends QuarantineCopiedData {
  encryptedVerifiedPlaintextSha256: string;
  verifiedSourceSnapshot: QuarantineSourceSnapshot;
  verifiedContentGuardBreakSequence: U64String;
  containerVerifiedAtUtc: TimestampUtc;
}

interface QuarantineContainerCommittedData
  extends QuarantineContainerVerifiedData {
  objectIdentity: QuarantineObjectIdentity;
  quotaLedgerChargeId: Uuid;
  containerCommittedAtUtc: TimestampUtc;
}

type QuarantineRetention =
  | { kind: 'active' }
  | { kind: 'expired'; expiredAtUtc: string };

type QuarantineExport =
  | { kind: 'none' }
  | {
      kind: 'exported';
      successfulExportCount: U64String;
      latest: {
        restoreOperationId: Uuid;
        targetGrantIdForAudit: Uuid;
        targetVolumeGuid: string;
        exportedFileId128: string;
        exportedSha256: Sha256;
        exportedAtUtc: string;
      };
    };

interface QuarantineSourceDeletePreparedData
  extends QuarantineContainerCommittedData {
  sourceMutation: FileMutationAttempt;
  sourceDeletePreparedAtUtc: TimestampUtc;
}

interface QuarantineSourceRemovedVerifiedData
  extends QuarantineSourceDeletePreparedData {
  sourceMutation: Extract<
    FileMutationAttempt,
    { phase: 'removedVerified' }
  >;
  sourceRemovedVerifiedAtUtc: TimestampUtc;
}

interface QuarantineLifecycleData {
  quarantinedAtUtc: string;
  expiresAtUtc: string;
  retention: QuarantineRetention;
  export: QuarantineExport;
}

type QuarantineStoredData = QuarantineSourceRemovedVerifiedData &
  QuarantineLifecycleData;

type QuarantineSourceRetainedData = QuarantineContainerCommittedData &
  QuarantineLifecycleData &
  (
    | {
        sourceRetainedReason:
          | 'cancelledBeforeDelete'
          | 'policyRevokedBeforeDelete'
          | 'contentGuardBrokenBeforeDelete';
        sourceMutation?: never;
      }
    | {
        sourceRetainedReason: 'deleteRejected' | 'preservedAfterPossibleCall';
        sourceMutation: Extract<
          FileMutationAttempt,
          { phase: 'callRejected' | 'resolvedPreservedAfterPossibleCall' }
        >;
      }
  );

type QuarantineExportableData =
  | QuarantineStoredData
  | QuarantineSourceRetainedData;

interface RestoreAuthorizationBase {
  bindingVersion: 2;
  sourceTargetGrantIdForAudit: Uuid;
  copyOperation:
    | { kind: 'restore'; operationId: Uuid }
    | { kind: 'quarantineSalvage'; operationId: Uuid };
  recordId: Uuid;
  recordJournalSequence: U64String;
  recordSetDigestSha256: Sha256;
  ownerSidDigest: Sha256;
  encryptedTargetParentPath: string;
  targetVolumeGuid: string;
  targetParentFileId128: string;
  targetParentChainDigestSha256: Sha256;
  targetDirectoryRelativeName: string;
  targetDirectoryCreationMarkerSha256: Sha256;
  capturedAtUtc: string;
  authorizationDigestSha256: Sha256;
}

type RestoreTargetAuthorizationSnapshot =
  | (RestoreAuthorizationBase & {
      phase: 'directoryCreateAuthorized';
      allowedAction: 'createTargetAndResumeThisPreparedBatchOnly';
    })
  | (RestoreAuthorizationBase & {
      phase: 'directoryReady';
      allowedAction: 'resumeThisPreparedCopyOnly';
      targetDirectoryFileId128: string;
      targetDirectoryDaclDigestSha256: Sha256;
    });

interface RestoreTemporaryIdentity {
  volumeGuid: string;
  fileId128: string;
  daclDigestSha256: Sha256;
  creationMarkerSha256: Sha256;
}

type RestoreAttempt = {
  temporaryRelativeName: string;
  finalRelativeName: string;
  expectedSourceIdentity: QuarantineObjectIdentity;
  previousMainState: 'committed' | 'sourceRetained';
  preparedAtUtc: string;
} & (
  | {
      phase: 'targetDirectoryPending';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryCreateAuthorized' }
      >;
    }
  | {
      phase: 'prepared';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
    }
  | {
      phase: 'temporaryCreated' | 'decrypting';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      temporaryIdentity: RestoreTemporaryIdentity;
    }
  | {
      phase: 'verified';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      temporaryIdentity: RestoreTemporaryIdentity;
      verifiedSha256: Sha256;
    }
  | {
      phase: 'published';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      publishedFileId128: string;
      publishedSha256: Sha256;
    }
);

interface PurgeAttemptBase {
  purgePlanId: Uuid;
  purgeOperationId: Uuid;
  previousRetention: QuarantineRetention;
  previousExport: QuarantineExport;
  confirmationSummaryDigest: Sha256;
  expectedObjectIdentity: QuarantineObjectIdentity;
  previousMainState: 'committed' | 'sourceRetained';
  preparedAtUtc: string;
}

type PurgeAttempt = PurgeAttemptBase &
  (
    | { phase: 'batchPrepared' }
    | { phase: 'mutationStarted'; mutation: FileMutationAttempt }
  );

interface PreparedBatchStop {
  operationId: Uuid;
  kind: 'restore' | 'quarantineSalvage' | 'purge';
  reason:
    | 'userCancelled'
    | 'executorFailed'
    | 'ownerChanged'
    | 'targetChanged';
  code: ErrorCode;
  requestedAtUtc: string;
  stopSequence: U64String;
}

type KnownQuarantineState =
  | { state: 'prepared'; data: QuarantinePreparedData }
  | {
      state: 'sourceDigestPrepared';
      data: QuarantineSourceDigestPreparedData;
    }
  | {
      state: 'containerPrepared';
      data: QuarantineContainerPreparedData;
    }
  | { state: 'copying'; data: QuarantineCopyingData }
  | { state: 'copied'; data: QuarantineCopiedData }
  | {
      state: 'containerVerified';
      data: QuarantineContainerVerifiedData;
    }
  | {
      state: 'containerCommitted';
      data: QuarantineContainerCommittedData;
    }
  | {
      state: 'sourceDeletePrepared';
      data: QuarantineSourceDeletePreparedData;
    }
  | {
      state: 'sourceRemovedVerified';
      data: QuarantineSourceRemovedVerifiedData;
    }
  | { state: 'committed'; data: QuarantineStoredData }
  | { state: 'sourceRetained'; data: QuarantineSourceRetainedData }
  | {
      state: 'restorePrepared';
      data: QuarantineExportableData;
      attempt: RestoreAttempt;
    }
  | {
      state: 'purgePrepared';
      data: QuarantineExportableData;
      attempt: PurgeAttempt;
    };

type ReconciliationEvidence =
  | {
      context: 'quarantineObject';
      observedAtUtc: string;
      sourcePresence: 'present' | 'absent' | 'unknown';
      containerPresence: 'present' | 'absent' | 'unknown';
      observedObjectIdentity?: QuarantineObjectIdentity;
      observedSourceSnapshot?: QuarantineSourceSnapshot;
      contentGuardState?: 'granted' | 'broken' | 'unknown';
      evidenceDigestSha256: Sha256;
    }
  | {
      context: 'restoreTarget';
      observedAtUtc: string;
      temporaryPresence: 'present' | 'absent' | 'unknown';
      finalPresence: 'present' | 'absent' | 'unknown';
      observedTemporaryIdentity?: RestoreTemporaryIdentity;
      observedFinalFileId128?: string;
      observedFinalSha256?: Sha256;
      evidenceDigestSha256: Sha256;
    };

interface ContainerObjectEvidence {
  containerRelativeName: string;
  observedAtUtc: string;
  observedObjectIdentity?: QuarantineObjectIdentity;
  quotaCharge: QuotaCharge;
  evidenceDigestSha256: Sha256;
}

interface PurgeReconciliationEvidenceBase {
  evidenceVersion: 1;
  purgeReconciliationEvidenceId: Uuid;
  recordId: Uuid;
  sourceQuarantineOperationId: Uuid;
  purgeOperationId: Uuid;
  originalQuotaLedgerChargeId: Uuid;
  encryptedRepositoryRelativeName: string;
  fixedRepositoryEntryDigestSha256: Sha256;
  expectedVolumeGuid: string;
  expectedVolumeSerial: U64String;
  expectedFileId128: string;
  sourceRecordJournalSequence: U64String;
  createdAtUtc: TimestampUtc;
  lastObservedAtUtc: TimestampUtc;
  observationSequence: U64String;
}

type PurgeReconciliationEvidence = PurgeReconciliationEvidenceBase &
  (
    | {
        state: 'unresolved';
        observedPresence: 'absent' | 'unknown';
        chargeState: 'retained' | 'accountingUnknown';
        absenceEvidenceDigestSha256?: never;
        resolvedAtUtc?: never;
        retentionStartedAtUtc?: never;
      }
    | {
        state: 'resolved';
        observedPresence: 'absent';
        chargeState: 'released' | 'replacedByTombstoneCharge';
        absenceEvidenceDigestSha256: Sha256;
        resolvedAtUtc: TimestampUtc;
        retentionStartedAtUtc: TimestampUtc;
      }
  );

interface PurgeTombstoneData {
  sourceQuarantineOperationId: Uuid;
  purgeOperationId: Uuid;
  ruleId?: string;
  action: 'purgeQuarantine';
  logicalBytes: U64String;
  allocatedBytes?: U64String;
  quotaLedgerChargeId: Uuid;
  attemptCompletedAtUtc: TimestampUtc;
}

type PurgeTombstone = QuarantineRecordMeta &
  PurgeTombstoneData &
  (
    | {
        state: 'purged';
        resultCode: 'OK';
        purgedAtUtc: TimestampUtc;
        tombstoneExpiresAtUtc: TimestampUtc;
        purgeReconciliationEvidenceId?: never;
        retention?: never;
      }
    | {
        state: 'purgedUnverified';
        resultCode: 'PURGE_OUTCOME_UNKNOWN';
        purgeReconciliationEvidenceId: Uuid;
        purgedAtUtc?: never;
        tombstoneExpiresAtUtc?: never;
        retention:
          | { state: 'blockedOnReconciliation' }
          | {
              state: 'running';
              startedAtUtc: TimestampUtc;
              expiresAtUtc: TimestampUtc;
            };
      }
  );

type QuarantineRecord = StrictUnion<
  | (QuarantineRecordMeta & KnownQuarantineOrigin & KnownQuarantineState)
  | (QuarantineRecordMeta &
      KnownQuarantineOrigin & {
        state: 'aborted';
        abortedAtUtc: string;
        disposition: 'originalPreserved';
      })
  | (QuarantineRecordMeta &
      KnownQuarantineOrigin & {
        state: 'recoveryRequired' | 'damaged' | 'conflicted';
        lastKnown: KnownQuarantineState;
        observed: ReconciliationEvidence;
        automaticActions: 'blocked';
      })
  | (QuarantineRecordMeta & {
      state: 'orphaned';
      discoveredAtUtc: string;
      observedContainerObject: ContainerObjectEvidence;
      automaticActions: 'blocked';
    })
  | PurgeTombstone
>;

interface QuarantineRecordView {
  recordId: Uuid;
  journalSequence: U64String;
  state: QuarantineRecord['state'];
  displayLabel: string;
  logicalBytes?: U64String;
  allocatedBytes?: U64String;
  retention?: 'active' | 'expired';
  exportCount: U64String;
  verification: 'verified' | 'unverified' | 'notApplicable';
  canRestore: boolean;
  canSalvageExport: boolean;
  canPurge: boolean;
  lastErrorCode?: ErrorCode;
}

type SalvageSourceEvidence =
  | {
      kind: 'knownVerified';
      expectedObjectIdentity: QuarantineObjectIdentity;
      committedManifestSha256: Sha256;
    }
  | {
      kind: 'incompleteOrOrphan';
      fixedRepositoryEntryDigestSha256: Sha256;
      observedObjectIdentity?: QuarantineObjectIdentity;
    };

type SalvageInProgressPhase =
  | {
      phase: 'targetDirectoryPending';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryCreateAuthorized' }
      >;
    }
  | {
      phase: 'prepared';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
    }
  | {
      phase: 'temporaryCreated' | 'copying';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      temporaryIdentity: RestoreTemporaryIdentity;
    }
  | {
      phase: 'verified';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      temporaryIdentity: RestoreTemporaryIdentity;
      verifiedSha256: Sha256;
    };

type SalvageCopyPhase<R extends 'verifiedCopy' | 'unverifiedCopy'> =
  | SalvageInProgressPhase
  | {
      phase: 'published';
      authorization: Extract<
        RestoreTargetAuthorizationSnapshot,
        { phase: 'directoryReady' }
      >;
      result: R;
      exportedFileId128: string;
      exportedSha256: Sha256;
      exportedAtUtc: string;
    }
  | {
      phase: 'failed';
      lastDurableCopy: SalvageInProgressPhase;
      errorCode: ErrorCode;
      failedAtUtc: string;
    };

type SalvageExportRecord = {
  recordVersion: 2;
  salvageOperationId: Uuid;
  sourceRecordId: Uuid;
  sourceRecordJournalSequence: U64String;
  temporaryRelativeName: string;
  finalRelativeName: string;
  sourcePreserved: true;
  preparedAtUtc: string;
  lastDurableAtUtc: string;
} &
  (
    | {
        sourceAbnormalState: 'damaged' | 'conflicted' | 'recoveryRequired';
        sourceEvidence: Extract<SalvageSourceEvidence, { kind: 'knownVerified' }>;
        copy: SalvageCopyPhase<'verifiedCopy'>;
      }
    | {
        sourceAbnormalState:
          | 'damaged'
          | 'conflicted'
          | 'orphaned'
          | 'recoveryRequired';
        sourceEvidence: Extract<
          SalvageSourceEvidence,
          { kind: 'incompleteOrOrphan' }
        >;
        copy: SalvageCopyPhase<'unverifiedCopy'>;
      }
  );

```
