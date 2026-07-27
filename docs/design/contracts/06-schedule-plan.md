<a id="qpn-sec-8-3-7"></a>
# 8.3.7 调度授权与不可变计划

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface ApprovalBinding {
  ruleId: string;
  packageHash: Sha256;
  ruleVersion: string;
  canonicalPolicyDigest: Sha256;
}

type ScheduledTrigger =
  | { kind: 'daily'; localTime: string }
  | {
      kind: 'weekly';
      dayOfWeek: 0 | 1 | 2 | 3 | 4 | 5 | 6;
      localTime: string;
    };

interface ScheduledJobDefinitionMaterial {
  schemaVersion: 1;
  jobId: Uuid;
  trigger: ScheduledTrigger;
  windowsTimeZoneId: string;
  workload:
    | {
        kind: 'R0Analysis';
        analysisPolicyId: Uuid;
        canonicalPolicyDigest: Sha256;
      }
    | {
        kind: 'R1Cleanup';
        approvalGrantId: Uuid;
        automationPolicyId: Uuid;
        automationPolicyRevision: U64String;
        automationPolicyDigest: Sha256;
        bindings: NonEmptyArray<ApprovalBinding>;
        maximumRuns: number;
      };
  runAs: 'currentUser';
  taskPrincipalSidDigest: Sha256;
  runnerId: 'qingpan-scheduled-runner-v1';
  taskSettings: {
    startWhenAvailable: true;
    allowStartOnDemand: false;
    multipleInstances: 'ignoreNew';
    runOnlyIfNetworkAvailable: false;
    allowHardTerminate: false;
  };
}

interface ApprovalGrant {
  approvalGrantId: Uuid;
  revision: U64String;
  usageRevision: U64String;
  automationPolicyId: Uuid;
  automationPolicyRevision: U64String;
  automationPolicyDigest: Sha256;
  bindings: NonEmptyArray<ApprovalBinding>;
  ownerSidDigest: Sha256;
  taskPrincipalSidDigest: Sha256;
  boundScheduledJobId: Uuid;
  approvedTrigger: ScheduledTrigger;
  windowsTimeZoneId: string;
  maximumRuns: number;
  runsStarted: number;
  authorizedScheduleDigestSha256: Sha256;
  nativeConfirmationDigest: Sha256;
  createdAtUtc: string;
  expiresAtUtc: string;
  state:
    | 'active'
    | 'revoked'
    | 'policyChanged'
    | 'packageRevoked'
    | 'expired'
    | 'runLimitReached';
}

interface ApprovalGrantJobBinding {
  approvalGrantId: Uuid;
  scheduledJobId: Uuid;
  createdAtUtc: string;
  state: 'active' | 'terminal';
  terminalAtUtc?: string;
}

interface ScheduledRunClaimBase {
  scheduledRunClaimId: Uuid;
  jobId: Uuid;
  jobRevision: U64String;
  runOrdinal: number;
  taskSchedulerInstanceGuid: Uuid;
  taskEnginePid: number;
  taskEngineCreatedAtFiletime: FileTimeString;
  runnerPid: number;
  runnerCreatedAtFiletime: FileTimeString;
  actionProcessCorrelation:
    | 'operationalEventActionPid'
    | 'verifiedActionDescendantOfTaskEngine';
  occurrenceKeySha256: Sha256;
  scheduledOccurrenceAtUtc: string;
  launchReason: 'scheduledTime' | 'startWhenAvailable';
  launchAttestationDigestSha256: Sha256;
  claimedAtUtc: string;
}

type ScheduledRunClaim = ScheduledRunClaimBase &
  (
    | {
        workloadKind: 'R0Analysis';
        approvalGrantId?: never;
        approvalGrantUsageRevision?: never;
      }
    | {
        workloadKind: 'R1Cleanup';
        approvalGrantId: Uuid;
        approvalGrantUsageRevision: U64String;
      }
  );

interface CreateAutomationApprovalRequest {
  idempotencyKey: IdempotencyKey;
  automationPolicyId: Uuid;
  trigger: ScheduledTrigger;
  windowsTimeZoneId: string;
  maximumRuns: number;
}

type PlanState =
  | 'awaitingConfirmation'
  | 'ready'
  | 'readyForElevation'
  | 'claimed'
  | 'invalidated'
  | 'expired'
  | 'consumed';

type ScanCandidatePlanSource =
  | {
      kind: 'scanCandidates';
      taskId: Uuid;
      taskKind: 'cleanupRules';
      taskResultDigestSha256: Sha256;
      rulePackageHash: Sha256;
    }
  | {
      kind: 'scanCandidates';
      taskId: Uuid;
      taskKind: 'largeFiles';
      taskResultDigestSha256: Sha256;
      rulePackageHash?: never;
    };

type PlanSource =
  | ScanCandidatePlanSource
  | {
      kind: 'quarantineSelection';
      recordCount: number;
      recordsDigestSha256: Sha256;
    }
  | {
      kind: 'appSnapshot';
      appSnapshotId: Uuid;
      snapshotDigestSha256: Sha256;
    };

type FilePlanAction =
  | {
      action: 'deleteRebuildableCache';
      risk: 'R1';
      recovery: 'rebuildable';
    }
  | {
      action: 'quarantine';
      risk: 'R2';
      recovery: 'quarantine';
    }
  | {
      action: 'permanentDeleteOriginal';
      risk: 'R4';
      recovery: 'none';
    };

interface PlanItemCommon<
  C extends ConfirmationCategoryId = ConfirmationCategoryId,
> {
  planItemId: Uuid;
  snapshotDigestSha256: Sha256;
  policyDigestSha256: Sha256;
  confirmationCategoryId: C;
  expectedLogicalBytes: U64String;
  expectedAllocatedBytes?: U64String;
  requiresElevation: boolean;
}

interface PlanSizeSummary {
  itemCount: number;
  logicalBytes: U64String;
  allocatedKnownBytes: U64String;
  allocatedKnownItemCount: number;
  allocatedUnknownItemCount: number;
}

interface PlanConfirmationCategorySummary {
  categoryId: ConfirmationCategoryId;
  itemCount: number;
  logicalBytes: U64String;
  maximumRisk: Exclude<RiskLevel, 'R0'>;
  actionSummary: Partial<Record<ActionKind, number>>;
  recoverySummary: Partial<Record<RecoveryKind, number>>;
  summaryDigestSha256: Sha256;
}

type RuleFileCandidatePlanItem = PlanItemCommon<FileConfirmationCategoryId> &
  Extract<
    FilePlanAction,
    { action: 'deleteRebuildableCache' | 'quarantine' }
  > & {
    kind: 'fileCandidate';
    sourceKind: 'cleanupRules';
    candidateId: Uuid;
    rulePackageHash: Sha256;
    requiresElevation: false;
  };

type LargeFileCandidatePlanItem = PlanItemCommon<'userLargeFiles'> &
  Extract<
    FilePlanAction,
    { action: 'quarantine' | 'permanentDeleteOriginal' }
  > & {
    kind: 'fileCandidate';
    sourceKind: 'largeFiles';
    candidateId: Uuid;
    rulePackageHash?: never;
    requiresElevation: false;
  };

type FileCandidatePlanItem =
  | RuleFileCandidatePlanItem
  | LargeFileCandidatePlanItem;

type QuarantinePurgePlanItem = PlanItemCommon<'quarantinePurge'> & {
  kind: 'quarantinePurge';
  quarantineRecordId: Uuid;
  recordJournalSequence: U64String;
  action: 'purgeQuarantine';
  risk: 'R4';
  recovery: 'none';
  requiresElevation: false;
};

type AppUninstallPlanItem<E extends boolean = boolean> =
  PlanItemCommon<'applicationUninstall'> & {
  kind: 'appUninstall';
  appSnapshotId: Uuid;
  sealedInvocation: Extract<
    SealedUninstallInvocation,
    { requiresElevation: E }
  >;
  action: 'launchUninstaller';
  risk: 'R3';
  recovery: 'none';
  requiresElevation: E;
};

type PlanItem =
  | FileCandidatePlanItem
  | QuarantinePurgePlanItem
  | AppUninstallPlanItem;

interface CleanupPlanBase<E extends boolean = boolean> {
  schemaVersion: 2;
  planId: Uuid;
  planHash: Sha256;
  itemsDigestSha256: Sha256;
  rulePackageHashes: Sha256[];
  policyDigests: Sha256[];
  createdAtUtc: string;
  expiresAtUtc: string;
  userSidDigest: Sha256;
  logonSidDigest: Sha256;
  sessionId: number;
  expectedLogicalBytes: U64String;
  sizeSummary: PlanSizeSummary;
  categorySummary: NonEmptyArray<PlanConfirmationCategorySummary>;
  maximumRisk: RiskLevel;
  requiresElevation: E;
  riskSummary: Partial<Record<RiskLevel, number>>;
  actionSummary: Partial<Record<ActionKind, number>>;
}

type PendingPlanConfirmation =
  | { kind: 'native'; status: 'pending'; summaryDigest: Sha256 }
  | {
      kind: 'nativeThenElevation';
      status: 'pending';
      summaryDigest: Sha256;
    };

type ReadyPlanConfirmation =
  | {
      kind: 'native';
      status: 'recorded';
      summaryDigest: Sha256;
      confirmedAtUtc: string;
    }
  | {
      kind: 'scheduledR1';
      status: 'bound';
      scheduledJobId: Uuid;
      scheduledJobRevision: U64String;
      approvalGrantId: Uuid;
      approvalGrantRevision: U64String;
      approvalBindings: NonEmptyArray<ApprovalBinding>;
    };

type ElevationIntentConfirmation = {
  kind: 'nativeThenElevation';
  status: 'intentRecorded';
  summaryDigest: Sha256;
  intentConfirmedAtUtc: string;
};

type ElevatedPlanConfirmation = Omit<
  ElevationIntentConfirmation,
  'status'
> & {
  status: 'elevatedRecorded';
  elevatedConfirmation: {
    operationId: Uuid;
    confirmedAtUtc: string;
    executionBundleDigestSha256: Sha256;
    confirmationDigest: Sha256;
  };
};

type PlanLifecycle =
  | {
      state: 'awaitingConfirmation';
      confirmation: PendingPlanConfirmation;
      claimedByOperationId?: never;
      terminalReason?: never;
    }
  | {
      state: 'ready';
      confirmation: ReadyPlanConfirmation;
      claimedByOperationId?: never;
      terminalReason?: never;
    }
  | {
      state: 'readyForElevation';
      confirmation: ElevationIntentConfirmation;
      claimedByOperationId?: never;
      terminalReason?: never;
    }
  | {
      state: 'claimed';
      confirmation:
        | ReadyPlanConfirmation
        | ElevationIntentConfirmation
        | ElevatedPlanConfirmation;
      claimedByOperationId: Uuid;
      terminalReason?: never;
    }
  | {
      state: 'invalidated';
      confirmation:
        | PendingPlanConfirmation
        | ReadyPlanConfirmation
        | ElevationIntentConfirmation;
      claimedByOperationId?: never;
      terminalReason: ErrorCode;
    }
  | {
      state: 'expired';
      confirmation:
        | PendingPlanConfirmation
        | ReadyPlanConfirmation
        | ElevationIntentConfirmation;
      claimedByOperationId?: never;
      terminalReason: 'STALE_PLAN';
    }
  | {
      state: 'consumed';
      confirmation:
        | ReadyPlanConfirmation
        | ElevationIntentConfirmation
        | ElevatedPlanConfirmation;
      claimedByOperationId: Uuid;
      terminalReason?: ErrorCode;
    };

type NarrowPlanLifecycleByConfirmationKind<L, K extends string> =
  L extends { confirmation: infer C }
    ? Extract<C, { kind: K }> extends never
      ? never
      : Omit<L, 'confirmation'> & {
          confirmation: Extract<C, { kind: K }>;
        }
    : never;

type NativePlanLifecycle = NarrowPlanLifecycleByConfirmationKind<
  PlanLifecycle,
  'native'
>;
type ScheduledR1PlanLifecycle = NarrowPlanLifecycleByConfirmationKind<
  PlanLifecycle,
  'scheduledR1'
>;
type ElevatedPlanLifecycle = NarrowPlanLifecycleByConfirmationKind<
  PlanLifecycle,
  'nativeThenElevation'
>;

type CleanupPlan =
  | (CleanupPlanBase<false> & {
        source: Extract<
          ScanCandidatePlanSource,
          { taskKind: 'cleanupRules' }
        >;
        items: NonEmptyArray<RuleFileCandidatePlanItem>;
      } & (NativePlanLifecycle | ScheduledR1PlanLifecycle))
  | (CleanupPlanBase<false> & {
        source: Extract<ScanCandidatePlanSource, { taskKind: 'largeFiles' }>;
        items: NonEmptyArray<LargeFileCandidatePlanItem>;
      } & NativePlanLifecycle)
  | (CleanupPlanBase<false> & {
        source: Extract<PlanSource, { kind: 'quarantineSelection' }>;
        items: NonEmptyArray<QuarantinePurgePlanItem>;
      } & NativePlanLifecycle)
  | (CleanupPlanBase<false> & {
        source: Extract<PlanSource, { kind: 'appSnapshot' }>;
        items: readonly [AppUninstallPlanItem<false>];
      } & NativePlanLifecycle)
  | (CleanupPlanBase<true> & {
        source: Extract<PlanSource, { kind: 'appSnapshot' }>;
        items: readonly [AppUninstallPlanItem<true>];
      } & ElevatedPlanLifecycle);

type PlanSourceView =
  | {
      kind: 'scanCandidates';
      taskId: Uuid;
      taskKind: 'cleanupRules' | 'largeFiles';
    }
  | { kind: 'quarantineSelection'; recordCount: number }
  | {
      kind: 'appSnapshot';
      appSnapshotId: Uuid;
      displayName: string;
      displayPublisher?: string;
      requiresElevation: false;
    }
  | {
      kind: 'appSnapshot';
      appSnapshotId: Uuid;
      displayName: string;
      displayPublisher?: string;
      requiresElevation: true;
    };

interface FilePlanItemViewBase {
  kind: 'fileCandidate';
  planItemId: Uuid;
  candidateId: Uuid;
  displayPath: string;
  evidence: string;
  expectedLogicalBytes: U64String;
  expectedAllocatedBytes?: U64String;
}

type PlanItemView =
  | (FilePlanItemViewBase & {
      sourceKind: 'cleanupRules';
      confirmationCategoryId: FileConfirmationCategoryId;
    } & (
      | {
          risk: 'R1';
          action: 'deleteRebuildableCache';
          recovery: 'rebuildable';
        }
      | { risk: 'R2'; action: 'quarantine'; recovery: 'quarantine' }
    ))
  | (FilePlanItemViewBase & {
      sourceKind: 'largeFiles';
      confirmationCategoryId: 'userLargeFiles';
    } & (
      | { risk: 'R2'; action: 'quarantine'; recovery: 'quarantine' }
      | {
          risk: 'R4';
          action: 'permanentDeleteOriginal';
          recovery: 'none';
        }
    ))
  | {
      kind: 'quarantinePurge';
      planItemId: Uuid;
      quarantineRecordId: Uuid;
      confirmationCategoryId: 'quarantinePurge';
      displayLabel: string;
      risk: 'R4';
      action: 'purgeQuarantine';
      recovery: 'none';
      expectedLogicalBytes: U64String;
      expectedAllocatedBytes?: U64String;
    }
  | {
      kind: 'appUninstall';
      planItemId: Uuid;
      appSnapshotId: Uuid;
      confirmationCategoryId: 'applicationUninstall';
      displayName: string;
      displayPublisher?: string;
      adapter: 'msi' | 'appx' | 'win32Exe';
      risk: 'R3';
      action: 'launchUninstaller';
      recovery: 'none';
      requiresElevation: boolean;
    };

interface PlanViewBase<
  S extends PlanSourceView = PlanSourceView,
  E extends boolean = boolean,
> {
  planId: Uuid;
  source: S;
  createdAtUtc: string;
  expiresAtUtc: string;
  itemCount: number;
  expectedLogicalBytes: U64String;
  sizeSummary: PlanSizeSummary;
  categorySummary: NonEmptyArray<PlanConfirmationCategorySummary>;
  maximumRisk: RiskLevel;
  requiresElevation: E;
  riskSummary: Partial<Record<RiskLevel, number>>;
  actionSummary: Partial<Record<ActionKind, number>>;
}

type PendingPlanViewConfirmation =
  | { kind: 'native'; status: 'pending' }
  | { kind: 'nativeThenElevation'; status: 'pending' };

type ReadyPlanViewConfirmation =
  | { kind: 'native'; status: 'recorded'; recordedAtUtc: string }
  | { kind: 'scheduledR1'; status: 'bound' };

type ElevationPlanViewConfirmation =
  | {
      kind: 'nativeThenElevation';
      status: 'intentRecorded';
      recordedAtUtc: string;
    }
  | {
      kind: 'nativeThenElevation';
      status: 'elevatedRecorded';
      recordedAtUtc: string;
    };

type PlanViewLifecycle =
  (
    | {
        state: 'awaitingConfirmation';
        confirmation: PendingPlanViewConfirmation;
        claimedByOperationId?: never;
        terminalReason?: never;
      }
    | {
        state: 'ready';
        confirmation: ReadyPlanViewConfirmation;
        claimedByOperationId?: never;
        terminalReason?: never;
      }
    | {
        state: 'readyForElevation';
        confirmation: Extract<
          ElevationPlanViewConfirmation,
          { status: 'intentRecorded' }
        >;
        claimedByOperationId?: never;
        terminalReason?: never;
      }
    | {
        state: 'claimed';
        confirmation:
          | ReadyPlanViewConfirmation
          | ElevationPlanViewConfirmation;
        claimedByOperationId: Uuid;
        terminalReason?: never;
      }
    | {
        state: 'invalidated' | 'expired';
        confirmation:
          | PendingPlanViewConfirmation
          | ReadyPlanViewConfirmation
          | ElevationPlanViewConfirmation;
        claimedByOperationId?: never;
        terminalReason: ErrorCode;
      }
    | {
        state: 'consumed';
        confirmation:
          | ReadyPlanViewConfirmation
          | ElevationPlanViewConfirmation;
        claimedByOperationId: Uuid;
        terminalReason?: ErrorCode;
      }
  );

type NarrowPlanViewLifecycleByConfirmationKind<L, K extends string> =
  L extends { confirmation: infer C }
    ? Extract<C, { kind: K }> extends never
      ? never
      : Omit<L, 'confirmation'> & {
          confirmation: Extract<C, { kind: K }>;
        }
    : never;

type NativePlanViewLifecycle = NarrowPlanViewLifecycleByConfirmationKind<
  PlanViewLifecycle,
  'native'
>;
type ScheduledR1PlanViewLifecycle = NarrowPlanViewLifecycleByConfirmationKind<
  PlanViewLifecycle,
  'scheduledR1'
>;
type ElevatedPlanViewLifecycle = NarrowPlanViewLifecycleByConfirmationKind<
  PlanViewLifecycle,
  'nativeThenElevation'
>;

type PlanView =
  | (PlanViewBase<
      Extract<
        PlanSourceView,
        { kind: 'scanCandidates'; taskKind: 'cleanupRules' }
      >,
      false
    > & (NativePlanViewLifecycle | ScheduledR1PlanViewLifecycle))
  | (PlanViewBase<
      Extract<
        PlanSourceView,
        { kind: 'scanCandidates'; taskKind: 'largeFiles' }
      >,
      false
    > & NativePlanViewLifecycle)
  | (PlanViewBase<
      Extract<PlanSourceView, { kind: 'quarantineSelection' }>,
      false
    > &
      NativePlanViewLifecycle)
  | (PlanViewBase<
      Extract<PlanSourceView, { kind: 'appSnapshot'; requiresElevation: false }>,
      false
    > & NativePlanViewLifecycle)
  | (PlanViewBase<
      Extract<PlanSourceView, { kind: 'appSnapshot'; requiresElevation: true }>,
      true
    > & ElevatedPlanViewLifecycle);

```
