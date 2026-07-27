<a id="qpn-sec-8-3-3"></a>
# 8.3.3 运行时通用类型、授权与策略

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface ProgressEvent {
  apiVersion: ApiVersion;
  taskOrOperationId: Uuid;
  sequence: U64String;
  phase: string;
  processedItems: U64String;
  totalItems?: U64String;
  processedBytes?: U64String;
  terminal: boolean;
}

type PageCursor = string;

type PageRequest<F = never> =
  | { kind: 'first'; pageSize?: number; filter?: F }
  | { kind: 'next'; cursor: PageCursor };

type ScopedPageRequest<S, F = never> =
  | ({ kind: 'first'; pageSize?: number; filter?: F } & S)
  | { kind: 'next'; cursor: PageCursor };

interface SnapshotPageBase<T> {
  items: T[];
  asOfSequence: U64String;
  totalItems: U64String;
}

type CursorPage<T> = SnapshotPageBase<T> &
  (
    | { snapshotComplete: false; nextCursor: PageCursor }
    | { snapshotComplete: true; nextCursor?: never }
  );

interface AppendPageBase<T> {
  items: T[];
  asOfSequence: U64String;
  durableItemsThroughAsOf: U64String;
}

type AppendCursorPage<T> = AppendPageBase<T> &
  (
    | {
        caughtUp: false;
        producerTerminal: false;
        nextCursor: PageCursor;
        totalItems?: never;
      }
    | {
        caughtUp: false;
        producerTerminal: true;
        nextCursor: PageCursor;
        totalItems: U64String;
      }
    | {
        caughtUp: true;
        producerTerminal: false;
        nextCursor: PageCursor;
        totalItems?: never;
      }
    | {
        caughtUp: true;
        producerTerminal: true;
        nextCursor?: never;
        totalItems: U64String;
      }
  );

interface CursorLease {
  cursorDigestSha256: Sha256;
  ownerSidDigest: Sha256;
  sessionId: number;
  apiName: string;
  objectId?: Uuid;
  consistency: 'snapshot' | 'append';
  asOfSequence: U64String;
  pinnedVersionRows: U64String;
  pinnedBytes: U64String;
  lastAccessExpiresAtUtc: string;
  absoluteExpiresAtUtc: string;
}

interface CursorResourceLimits {
  maximumActiveChainsPerOwnerSession: 8;
  maximumPinnedVersionRowsPerOwnerSession: 200000;
  maximumPinnedBytesPerOwnerSession: 67108864;
  idleTtlSeconds: 900;
  absoluteTtlSeconds: 3600;
}

type ScanResultFilter = { resultKinds?: ScanResultView['kind'][] };
type QuarantineFilter = {
  states?: QuarantineRecord['state'][];
  retention?: Array<'active' | 'expired'>;
};
type AppFilter = {
  search?: string;
  adapters?: Array<'msi' | 'appx' | 'win32Exe'>;
  supportedOnly?: boolean;
};
type StartupEntryFilter = {
  sources?: Array<'registryRun' | 'startupFolder' | 'scheduledTask' | 'service'>;
};
type ScheduledJobFilter = {
  risks?: Array<'R0' | 'R1'>;
  enabled?: boolean;
};
type AuditFilter = {
  operationKinds?: OperationKind[];
  statuses?: OperationStatus[];
  sinceUtc?: string;
  untilUtc?: string;
};

type IdempotencyKey = string;

type DurableIdempotentCommand = Extract<
  CommandName,
  | 'start_restore'
  | 'start_quarantine_salvage_export'
  | 'create_automation_approval'
  | 'upsert_scheduled_job'
  | 'delete_scheduled_job'
>;

type DeepReadonly<T> =
  T extends string | number | boolean | null | undefined
    ? T
    : T extends readonly (infer U)[]
      ? readonly DeepReadonly<U>[]
      : T extends object
        ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
        : T;

type DurableIdempotencyMutationRefByCommand = {
  start_restore: { kind: 'operation'; operationId: Uuid };
  start_quarantine_salvage_export: {
    kind: 'operation';
    operationId: Uuid;
  };
  create_automation_approval: {
    kind: 'automationApproval';
    approvalGrantId: Uuid;
    boundScheduledJobId: Uuid;
  };
  upsert_scheduled_job: {
    kind: 'scheduledJobMutation';
    jobId: Uuid;
    expectedResultKind: 'upserted';
  };
  delete_scheduled_job: {
    kind: 'scheduledJobMutation';
    jobId: Uuid;
    expectedResultKind: 'deleted';
  };
};

interface DurableIdempotencyRecordBase<
  C extends DurableIdempotentCommand,
> {
  command: C;
  idempotencyKeyDigestSha256: Sha256;
  ownerSidDigest: Sha256;
  canonicalPayloadDigestSha256: Sha256;
  createdAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

type DurableIdempotencyState<C extends DurableIdempotentCommand> =
  | {
      state: 'pending';
      mutationId: Uuid;
      mutationRef: DurableIdempotencyMutationRefByCommand[C];
      response?: never;
      responsePayloadSha256?: never;
      error?: never;
      errorPayloadSha256?: never;
    }
  | {
      state: 'succeeded';
      mutationId: Uuid;
      mutationRef?: never;
      response: DeepReadonly<CommandResponse<C>>;
      responsePayloadSha256: Sha256;
      error?: never;
      errorPayloadSha256?: never;
    }
  | {
      state: 'failed';
      mutationId: Uuid;
      mutationRef?: never;
      response?: never;
      responsePayloadSha256?: never;
      error: DeepReadonly<ApiError>;
      errorPayloadSha256: Sha256;
      failureEvidenceDigestSha256: Sha256;
    };

type DurableIdempotencyRecord = {
  [C in DurableIdempotentCommand]:
    DurableIdempotencyRecordBase<C> & DurableIdempotencyState<C>;
}[DurableIdempotentCommand];

interface StartRestoreRequest {
  idempotencyKey: IdempotencyKey;
  targetGrantId: Uuid;
  recordIds: [Uuid, ...Uuid[]];
}

interface StartQuarantineSalvageExportRequest {
  idempotencyKey: IdempotencyKey;
  targetGrantId: Uuid;
  recordIds: [Uuid, ...Uuid[]];
}

interface CreateQuarantinePurgePlanRequest {
  recordIds: [Uuid, ...Uuid[]];
}

interface TransientGrantBinding {
  ownerSidDigest: Sha256;
  logonSidDigest: Sha256;
  sessionId: number;
  issuedToAppInstanceId: Uuid;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

interface RootGrantBase extends TransientGrantBinding {
  rootGrantId: Uuid;
  grantDigestSha256: Sha256;
  displayName: string;
  volumeGuid: string;
  rootFileId128: string;
}

type UserSelectedAnalysisRootGrant =
  | (RootGrantBase & {
      kind: 'userSelectedAnalysis';
      allowedScopes: readonly ['storageUsage'];
    })
  | (RootGrantBase & {
      kind: 'userSelectedAnalysis';
      allowedScopes: readonly ['largeFiles'];
    })
  | (RootGrantBase & {
      kind: 'userSelectedAnalysis';
      allowedScopes: readonly ['duplicates'];
    });

type RootGrant =
  | (RootGrantBase & {
      kind: 'builtInResolver';
      allowedScopes: readonly ['cleanupRules'];
    })
  | UserSelectedAnalysisRootGrant
  | (RootGrantBase & {
      kind: 'restoreTarget';
      allowedScopes: readonly ['restore'];
    })
  | (RootGrantBase & {
      kind: 'salvageExportTarget';
      allowedScopes: readonly ['salvageExport'];
    });

interface AppUpdateRecoveryPackageFileGrant extends TransientGrantBinding {
  installerRecoveryPackageFileGrantId: Uuid;
  purpose: 'applySignedInstallerRecoveryPackage';
  source: InstallRecoverySource;
  sourceBindingDigestSha256: Sha256;
  volumeGuid: string;
  fileId128: string;
  sizeBytes: U64String;
  lastWriteTimeFiletime: FileTimeString;
  sha256: Sha256;
  grantDigestSha256: Sha256;
}

interface DiagnosticExportTargetGrant extends TransientGrantBinding {
  diagnosticExportTargetGrantId: Uuid;
  purpose: 'exportDiagnosticBundle';
  parentVolumeGuid: string;
  parentFileId128: string;
  reservedLeafNameDigestSha256: Sha256;
  createDisposition: 'createNew';
  grantDigestSha256: Sha256;
}

interface ExclusionEntry {
  exclusionEntryId: Uuid;
  encryptedCanonicalPath: string;
  volumeGuid: string;
  fileId128: string;
  appliesTo: Array<'cleanupRules' | 'storageUsage' | 'largeFiles' | 'duplicates'>;
  ownerSidDigest: Sha256;
  state: 'active' | 'revoked' | 'identityChanged';
}

interface ExclusionPolicy {
  exclusionPolicyId: Uuid;
  entryIds: Uuid[];
  canonicalPolicyDigest: Sha256;
  ownerSidDigest: Sha256;
  createdAtUtc: string;
  updatedAtUtc: string;
  state: 'active' | 'revoked' | 'entryChanged';
}

interface PersistentAnalysisPolicy {
  analysisPolicyId: Uuid;
  encryptedCanonicalRootPath: string;
  volumeGuid: string;
  rootFileId128: string;
  scope: 'storageUsage' | 'largeFiles' | 'duplicates';
  exclusionPolicyId?: Uuid;
  canonicalPolicyDigest: Sha256;
  userSidDigest: Sha256;
  createdAtUtc: string;
  state: 'active' | 'revoked' | 'rootChanged';
}

interface R1AutomationPolicyRule {
  ruleId: string;
  packageHash: Sha256;
  ruleVersion: string;
  canonicalPolicyDigest: Sha256;
}

interface R1AutomationPolicy {
  automationPolicyId: Uuid;
  revision: U64String;
  rules: NonEmptyArray<R1AutomationPolicyRule>;
  maximumFiles: number;
  maximumBytes: U64String;
  ownerSidDigest: Sha256;
  policyDigest: Sha256;
  createdAtUtc: string;
  state: 'draft' | 'approved' | 'revoked' | 'policyChanged' | 'packageRevoked';
}

interface RuleUpdateChannelSetting {
  channelSettingId: Uuid;
  domain: 'rules';
  channel: 'stable' | 'beta' | 'internal';
  ownerSidDigest: Sha256;
  confirmedAtUtc: string;
  state: 'active' | 'revoked';
}

interface MachineAppUpdatePolicy {
  channelSettingId: Uuid;
  domain: 'application';
  scope: 'machine';
  machineInstallAnchorId: Uuid;
  installationInstanceId: Uuid;
  canonicalMsiUpgradeCode: string;
  nativeMachineArchitecture: 'x64' | 'arm64';
  channel: 'stable' | 'beta' | 'internal';
  policyRevision: U64String;
  authorizedByAdminSidDigest: Sha256;
  confirmedAtUtc: string;
  state: 'active' | 'revoked';
}

type UpdateChannelSetting =
  | RuleUpdateChannelSetting
  | MachineAppUpdatePolicy;

interface RuleResourceLimits {
  maximumFiles: number;
  maximumResults: number;
  maximumDepth: number;
  maximumDurationSeconds: number;
}

type RuleMatcher =
  | {
      kind: 'allFilesInLeafRecursive';
      names?: never;
      prefix?: never;
      suffix?: never;
    }
  | {
      kind: 'rootFileNames';
      names: NonEmptyArray<string>;
      prefix?: never;
      suffix?: never;
    }
  | ({ kind: 'descendantDirectoryNames' } &
      (
        | { names: NonEmptyArray<string>; prefix?: string; suffix?: string }
        | { names?: never; prefix: string; suffix?: string }
        | { names?: never; prefix?: never; suffix: string }
      ));

interface RuleControlLimits {
  signedIndexMaxBytes: 262144;
  signedKeyAuthorizationMaxBytes: 262144;
  signedRevocationMaxBytes: 262144;
  signedPackageMaxBytes: 8388608;
  maximumRulesPerPackage: 256;
  maximumRootSelectionsPerRule: 16;
  maximumMatcherNamesPerRule: 256;
  maximumSupportedBuildRangesPerRule: 32;
  maximumProcessGuardsPerRule: 64;
  maximumExclusionClassesPerRule: 16;
  maximumAcceptanceIdsPerRule: 64;
  maximumGenericArrayItems: 256;
  maximumUtf8BytesPerString: 512;
  maximumJsonNestingDepth: 32;
  maximumTotalUtf8StringBytesPerPackage: 4194304;
}

// 编译进应用；远程包不能新增或扩大这些能力。
interface CapabilityEnvelope {
  envelopeId: string;
  ruleFamily: string;
  rootResolvers: Array<{
    resolverId: string;
    rootClass:
      | 'localAppData'
      | 'roamingAppData'
      | 'knownApplicationProfile'
      | 'userDocuments';
    allowedLeafTemplateIds: string[];
  }>;
  allowedMatcherKinds: RuleMatcher['kind'][];
  resourceCeiling: RuleResourceLimits;
  minimumAgeFloorSeconds?: number;
  mandatoryProcessGuardIds: string[];
  mandatoryExclusionClasses: Array<
    | 'reparsePoint'
    | 'efs'
    | 'cloudPlaceholder'
    | 'unknownStream'
    | 'multipleHardLinks'
  >;
  allowedOutcomes: Array<{
    risk: RiskLevel;
    action: ActionKind;
    recovery: RecoveryKind;
    requiredPrivilege: 'user' | 'administrator';
  }>;
  automationCeiling: 'none' | 'R0' | 'R1';
}

```
