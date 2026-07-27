<a id="qpn-sec-8-3-11"></a>
# 8.3.11 启动项、磁盘、审计与任务日志

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface StartupEntryView {
  startupEntryId: Uuid;
  source: 'registryRun' | 'startupFolder' | 'scheduledTask' | 'service';
  displayName: string;
  displayPublisher?: string;
  scope: 'currentUser' | 'perMachine';
  enabledState: 'enabled' | 'disabled' | 'unknown';
  writeSupported: false;
}

interface DiskView {
  diskId: Uuid;
  displayName: string;
  busType: 'nvme' | 'sata' | 'usb' | 'virtual' | 'unknown';
  mediaType: 'ssd' | 'hdd' | 'scm' | 'unspecified';
  partitionStyle: 'gpt' | 'mbr' | 'raw' | 'unknown';
  sizeBytes: U64String;
  isBootDisk: boolean;
  isSystemDisk: boolean;
  health: 'healthy' | 'warning' | 'unknown';
  writeSupported: false;
}

interface PartitionVolumeView {
  partitionId: Uuid;
  partitionNumber: number;
  driveLetter?: string;
  fileSystem?: 'NTFS' | 'ReFS' | 'FAT32' | 'exFAT' | 'other' | 'unknown';
  sizeBytes: U64String;
  freeBytes?: U64String;
  role: 'system' | 'boot' | 'recovery' | 'data' | 'unknown';
  writeSupported: false;
}

interface PartitionDiskView {
  disk: DiskView;
  partitions: PartitionVolumeView[];
  snapshotCapturedAtUtc: TimestampUtc;
  writeSupported: false;
}

interface OpenDiskManagementResult {
  launched: true;
  systemTool: 'diskmgmt.msc';
  launchedAtUtc: TimestampUtc;
}

type OperationKind =
  | 'planExecution'
  | 'restore'
  | 'quarantineSalvage'
  | 'ruleUpdate'
  | 'appUpdate'
  | 'diagnosticExport';

interface OperationRecordView {
  auditRecordId: Uuid;
  operationId: Uuid;
  operationKind: OperationKind;
  status: OperationStatus;
  startedAtUtc: string;
  completedAtUtc?: string;
  itemCounts: Record<ItemOutcome, number>;
  accounting: SpaceAccounting;
  terminalCode?: ErrorCode;
}

interface ScheduledJobBase {
  schemaVersion: 1;
  jobId: Uuid;
  revision: U64String;
  enabled: boolean;
  trigger: ScheduledTrigger;
  windowsTimeZoneId: string;
  maximumRisk: 'R0' | 'R1';
  runAs: 'currentUser';
  taskPrincipalSidDigest: Sha256;
  deterministicWindowsTaskName: string;
  authorizedScheduleDigestSha256: Sha256;
  windowsTaskDefinitionDigestSha256: Sha256;
}

type ScheduledJob = ScheduledJobBase &
  (
    | {
        workload: {
          kind: 'R0Analysis';
          analysisPolicyId: Uuid;
          canonicalPolicyDigest: Sha256;
        };
        maximumRisk: 'R0';
        approvalGrantRevision?: never;
        maximumRuns?: never;
        runsStarted?: never;
      }
    | {
        workload: { kind: 'R1Cleanup'; approvalGrantId: Uuid };
        maximumRisk: 'R1';
        approvalGrantRevision: U64String;
        maximumRuns: number;
        runsStarted: number;
      }
  );

type ScheduledJobMutationResult =
  | { kind: 'upserted'; job: ScheduledJob }
  | {
      kind: 'deleted';
      jobId: Uuid;
      deletedRevision: U64String;
      deletedAtUtc: string;
      tombstoneDigestSha256: Sha256;
    };

type UpsertScheduledJobRequest =
  | ((
      | {
          mutation: 'create';
          jobId?: never;
          expectedRevision?: never;
        }
      | {
          mutation: 'update';
          jobId: Uuid;
          expectedRevision: U64String;
        }
    ) & {
      kind: 'R0Analysis';
      idempotencyKey: IdempotencyKey;
      enabled: boolean;
      trigger: ScheduledTrigger;
      windowsTimeZoneId: string;
      analysisPolicyId: Uuid;
    })
  | {
      kind: 'R1Cleanup';
      idempotencyKey: IdempotencyKey;
      approvalGrantId: Uuid;
    };

interface DeleteScheduledJobRequest {
  idempotencyKey: IdempotencyKey;
  jobId: Uuid;
  expectedRevision: U64String;
}

interface ScheduledJobJournalBase {
  journalVersion: 2;
  mutationId: Uuid;
  jobId: Uuid;
  targetRevision: U64String;
  ownerSidDigest: Sha256;
  deterministicWindowsTaskName: string;
  idempotencyKeyDigestSha256: Sha256;
  canonicalPayloadDigestSha256: Sha256;
  expectedAuthorizedScheduleDigestSha256?: Sha256;
  expectedWindowsTaskDefinitionDigestSha256?: Sha256;
  previousCommittedRevision?: U64String;
  createdAtUtc: string;
  lastDurableAtUtc: string;
}

type ScheduledJobJournal = ScheduledJobJournalBase &
  (
    | {
        state: 'prepared';
        desiredDefinition: ScheduledJob;
        observedWindowsTaskDefinitionDigestSha256?: never;
      }
    | {
        state: 'registered';
        desiredDefinition: ScheduledJob;
        registeredAtUtc: string;
        observedWindowsTaskDefinitionDigestSha256?: never;
      }
    | {
        state: 'verified';
        desiredDefinition: ScheduledJob;
        registeredAtUtc: string;
        observedWindowsTaskDefinitionDigestSha256: Sha256;
        verifiedAtUtc: string;
      }
    | {
        state: 'committed';
        desiredDefinition: ScheduledJob;
        registeredAtUtc: string;
        observedWindowsTaskDefinitionDigestSha256: Sha256;
        verifiedAtUtc: string;
        desiredEnabledAppliedAtUtc: string;
        observedEnabled: boolean;
        committedAtUtc: string;
      }
    | {
        state: 'deletePrepared';
        desiredDefinition?: never;
        observedWindowsTaskDefinitionDigestSha256?: never;
      }
    | {
        state: 'disabled';
        desiredDefinition?: never;
        observedWindowsTask:
          | { kind: 'present'; definitionDigestSha256: Sha256 }
          | { kind: 'absent' };
        disabledAtUtc: string;
      }
    | {
        state: 'deleted';
        desiredDefinition?: never;
        observedWindowsTaskDefinitionDigestSha256?: never;
        disabledAtUtc: string;
        deletedAtUtc: string;
      }
    | {
        state: 'reconciliationRequired';
        desiredDefinition?: ScheduledJob;
        evidenceDigestSha256: Sha256;
        code: ErrorCode;
      }
  );

```
