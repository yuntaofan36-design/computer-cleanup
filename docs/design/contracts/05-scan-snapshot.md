<a id="qpn-sec-8-3-6"></a>
# 8.3.6 扫描、任务与候选快照

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface ScanLimits {
  maxFiles: number;
  maxResults: number;
  maxDepth: number;
  maxDurationSeconds: number;
}

type ScanRequest =
  | {
      kind: 'cleanupRules';
      ruleIds?: string[];
      exclusionPolicyId?: Uuid;
      limits?: Partial<ScanLimits>;
    }
  | {
      kind: 'storageUsage';
      rootGrantIds: Uuid[];
      exclusionPolicyId?: Uuid;
      limits?: Partial<ScanLimits>;
    }
  | {
      kind: 'largeFiles';
      rootGrantIds: Uuid[];
      minimumSizeBytes: U64String;
      exclusionPolicyId?: Uuid;
      limits?: Partial<ScanLimits>;
    }
  | {
      kind: 'duplicates';
      rootGrantIds: Uuid[];
      minimumSizeBytes: U64String;
      exclusionPolicyId?: Uuid;
      limits?: Partial<ScanLimits>;
    };

interface TaskRef<K extends ScanRequest['kind'] = ScanRequest['kind']> {
  taskId: Uuid;
  kind: K;
  state: 'created' | 'loadingRules' | 'running';
}

type TaskStatus =
  | 'created'
  | 'loadingRules'
  | 'running'
  | 'cancelRequested'
  | 'completed'
  | 'limitReached'
  | 'cancelled'
  | 'failed';

interface TaskViewBase {
  taskId: Uuid;
  kind: ScanRequest['kind'];
  createdAtUtc: string;
  updatedAtUtc: string;
  committedResultCount: U64String;
  resultSequence: U64String;
}

type TaskView = TaskViewBase &
  (
  | {
      status:
        | 'created'
        | 'loadingRules'
        | 'running'
        | 'cancelRequested';
      producerComplete: false;
      completedAtUtc?: never;
    }
  | {
      status: 'limitReached';
      producerComplete: true;
      completedAtUtc: string;
      limit: {
        kind: 'files' | 'results' | 'depth' | 'duration';
        maximum: U64String;
        observed: U64String;
      };
    }
  | {
      status: 'completed';
      producerComplete: true;
      completedAtUtc: string;
    }
  | {
      status: 'cancelled';
      producerComplete: true;
      completedAtUtc: string;
      terminalCode: 'USER_CANCELLED';
    }
  | {
      status: 'failed';
      producerComplete: true;
      completedAtUtc: string;
      terminalCode: ErrorCode;
    }
  );

type ScanResultView =
  | {
      kind: 'cleanupCandidate' | 'largeFile';
      resultId: Uuid;
      candidateId: Uuid;
      displayPath: string;
      logicalBytes: U64String;
      risk: RiskLevel;
      evidence: string;
    }
  | {
      kind: 'storageNode';
      resultId: Uuid;
      displayPath: string;
      logicalBytes: U64String;
      allocatedBytes?: U64String;
    }
  | {
      kind: 'duplicateGroup';
      resultId: Uuid;
      memberCount: number;
      logicalBytesPerMember: U64String;
      confirmation: 'sampleHash' | 'fullHash' | 'byteCompared';
    };

interface CandidateSnapshot {
  schemaVersion: 1;
  candidateId: Uuid;
  taskId: Uuid;
  snapshotAtUtc: string;
  ruleId?: string;
  ruleVersion?: string;
  rulePackageHash?: Sha256;
  displayPath: string;
  canonicalRootId: string;
  relativePath: string;
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
  reparseTag?: number;
  hardLinkCount: number;
  streamCount: number;
  streamSetDigestSha256: Sha256;
  securityDescriptorDigestSha256?: Sha256;
  risk: RiskLevel;
  recommendedAction: ActionKind;
  evidence: string;
  recovery: RecoveryKind;
  snapshotDigestSha256: Sha256;
}

```
