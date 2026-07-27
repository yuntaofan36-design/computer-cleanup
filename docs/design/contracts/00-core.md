<a id="qpn-sec-8-3-1"></a>
# 8.3.1 基础类型与正式契约登记

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
type ApiVersion = 1;
type U64String = string;       // ^(?:0|[1-9][0-9]*)$，且 <= 2^64-1
type I64String = string;       // ^(?:0|-?[1-9][0-9]*)$，在 i64 范围内；拒绝 +0/-0
type FileTimeString = string;  // U64String 编码的 Windows 100 ns FILETIME
type Uuid = string;            // RFC 4122 小写连字符规范形式；拒绝 nil UUID
type Sha256 = string;          // 64 个小写十六进制字符
type TimestampUtc = string;    // YYYY-MM-DDTHH:mm:ss.sssZ；拒绝偏移、闰秒和非规范等价值
type NonEmptyArray<T> = readonly [T, ...T[]];
type UnionKeys<T> = T extends T ? keyof T : never;
type StrictUnionHelper<T, All> = T extends T
  ? T & Partial<Record<Exclude<UnionKeys<All>, keyof T>, never>>
  : never;
type StrictUnion<T> = StrictUnionHelper<T, T>;

interface FormalContractRegistryEntry<
  I extends string,
  R extends string,
  F extends string,
  V extends number,
> {
  contractId: I;
  rootTypeName: R;
  versionField: F;
  versionValue: V;
  artifactDigests: {
    rustContractSha256: Sha256;
    typeScriptContractSha256: Sha256;
    jsonSchemaSha256: Sha256;
  };
}

type FormalContractRegistry = readonly [
  FormalContractRegistryEntry<'CONTRACT-001', 'RuleManifest', 'schemaVersion', 1>,
  FormalContractRegistryEntry<'CONTRACT-002', 'ScanRequest', 'RequestEnvelope.apiVersion', 1>,
  FormalContractRegistryEntry<'CONTRACT-003', 'CandidateSnapshot', 'schemaVersion', 1>,
  FormalContractRegistryEntry<'CONTRACT-004', 'CleanupPlan', 'schemaVersion', 2>,
  FormalContractRegistryEntry<'CONTRACT-005', 'ExecuteResult', 'schemaVersion', 1>,
  FormalContractRegistryEntry<'CONTRACT-006', 'QuarantineRecord', 'recordVersion', 5>,
  FormalContractRegistryEntry<'CONTRACT-007', 'ScheduledJob', 'schemaVersion', 1>,
  FormalContractRegistryEntry<'CONTRACT-008', 'OutboundRequestPolicy', 'policyVersion', 1>,
];

interface FormalContractRegistrySnapshot {
  registryVersion: 2;
  registryCanonicalization: 'RFC8785_UTF8';
  artifactCanonicalization: {
    rust: 'UTF8_LF_NO_BOM';
    typeScript: 'UTF8_LF_NO_BOM';
    jsonSchema: 'RFC8785_UTF8';
  };
  entries: FormalContractRegistry;
  registryDigestSha256: Sha256;
}

type RiskLevel = 'R0' | 'R1' | 'R2' | 'R3' | 'R4';
type ActionKind =
  | 'analyze'
  | 'deleteRebuildableCache'
  | 'quarantine'
  | 'purgeQuarantine'
  | 'permanentDeleteOriginal'
  | 'windowsApi'
  | 'launchUninstaller';
type RecoveryKind = 'none' | 'rebuildable' | 'quarantine' | 'systemBackup';
type FileConfirmationCategoryId =
  | 'rebuildableCache'
  | 'sharedTemporaryFiles'
  | 'logsAndCrashData'
  | 'userLargeFiles'
  | 'chatUserData';
type ConfirmationCategoryId =
  | FileConfirmationCategoryId
  | 'quarantinePurge'
  | 'applicationUninstall';

interface RequestEnvelope<T> {
  apiVersion: ApiVersion;
  requestId: Uuid;
  payload: T;
}

type ResponseEnvelope<T> =
  | { apiVersion: ApiVersion; requestId: Uuid; ok: true; result: T }
  | { apiVersion: ApiVersion; requestId: Uuid; ok: false; error: ApiError };

interface ApiError {
  code: ErrorCode;
  messageKey: string;
  retryable: boolean;
  safeDetails?: Record<string, string | number | boolean>;
}

type LicenseDeactivationReason =
  | 'userDeactivatedDevice'
  | 'resetAuthorizationIdentity'
  | 'uninstallDeleteLocalData';

```
