<a id="qpn-sec-13-6-2"></a>
# 13.6.2 测试定义、运行、追踪与豁免

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface TestDefinitionRecordBase {
  schemaVersion: 2;
  title: string;
  requirementIds: NonEmptyArray<string>;
  priority: 'P0' | 'P1';
  risks: NonEmptyArray<RiskLevel | 'all'>;
  given: string;
  when: string;
  then: string;
  fixtureIds: string[];
  evidenceContractId: string;
  ownerRole: 'development' | 'security' | 'test' | 'release';
}

type TestDefinitionRecord = {
  [I in ReleaseTestId]: TestDefinitionRecordBase & {
    testId: I;
    capabilityId: ReleaseTestObligationGroupFor<I>['capabilityId'];
    platformCoverage: TestPlatformCoverageFor<I>;
  };
}[ReleaseTestId];

type TestFailureCode = ErrorCode | 'ASSERTION_FAILED' | 'EVIDENCE_INVALID';

interface TestedComponentIdentity {
  role:
    | 'desktopBinary'
    | 'elevationHelper'
    | 'scheduledRunner'
    | 'bootstrapper'
    | 'msiPayload'
    | 'nsisInstaller'
    | 'updateCoordinator'
    | 'recoveryExecutor'
    | 'rulePackage';
  sha256: Sha256;
}

interface TestUserContext {
  role: 'initiatingUser' | 'elevatedHelper' | 'trialChild' | 'scheduledRunner';
  tokenUserSidDigestSha256: Sha256;
  logonSidDigestSha256: Sha256;
  sessionId: number;
  authenticationId?: U64String;
  integrity: 'medium' | 'high';
}

interface TestRunRecordBase {
  schemaVersion: 2;
  releaseId: Uuid;
  testId: ReleaseTestId;
  releaseTestObligationsDigestSha256: Sha256;
  runId: Uuid;
  testDefinitionDigestSha256: Sha256;
  releaseCapabilityManifestCanonicalDigestSha256: Sha256;
  platform: RunPlatformBinding;
  fixtureVersions: Record<string, string>;
  windowsEdition: string;
  windowsVersion: string;
  windowsBuild: number;
  windowsUbr: number;
  architecture: 'x64' | 'arm64';
  fileSystem: 'NTFS' | 'ReFS';
  mediaType: 'ssd' | 'hdd';
  installerContext: 'none' | 'bootstrapper' | 'msiPayload' | 'nsis';
  privilegeContext: 'standardUser' | 'splitTokenAdminMedium' | 'highIntegrity';
  userContexts: NonEmptyArray<TestUserContext>;
  startedAtUtc: TimestampUtc;
  completedAtUtc: TimestampUtc;
  sourceCommit: string;
  buildId: string;
  releaseArtifactSetDigestSha256: Sha256;
  testedComponents: NonEmptyArray<TestedComponentIdentity>;
  evidenceSha256: Sha256;
  evidenceRelativePath: EvidenceRelativePath;
  ciProvenanceDigestSha256: Sha256;
  executedBy: string;
}

type TestRunRecord = TestRunRecordBase &
  (
    | {
        result: 'passed';
        failureCode?: never;
        blockedReason?: never;
      }
    | {
        result: 'failed';
        failureCode: TestFailureCode;
        blockedReason?: never;
      }
    | {
        result: 'blocked';
        failureCode?: never;
        blockedReason:
          | 'fixtureUnavailable'
          | 'platformUnavailable'
          | 'externalDependencyUnavailable'
          | 'evidenceCollectionFailed';
      }
  );

type M1MilestoneId =
  | 'M1-01'
  | 'M1-02'
  | 'M1-03'
  | 'M1-04'
  | 'M1-05'
  | 'M1-06'
  | 'M1-07'
  | 'M1-08'
  | 'M1-09'
  | 'M1-10'
  | 'M1-11'
  | 'M1-SVC-01'
  | 'M1-SVC-02'
  | 'M1-SVC-03';

const m1MandatoryP1RequirementBindings = {
  'APP-001': ['M1-06'],
  'STARTUP-001': ['M1-06'],
  'PART-001': ['M1-06'],
  'PAGE-001': ['M1-01'],
  'PERF-001': ['M1-11'],
  'SCAN-002': ['M1-05'],
  'UI-002': ['M1-11'],
  'UI-003': ['M1-11'],
} as const satisfies Record<string, readonly M1MilestoneId[]>;

type FormalP1RequirementId =
  | 'APP-001'
  | 'STARTUP-001'
  | 'PART-001'
  | 'PAGE-001'
  | 'PERF-001'
  | 'SCAN-002'
  | 'UI-002'
  | 'UI-003';
type M1MandatoryP1RequirementId =
  keyof typeof m1MandatoryP1RequirementBindings;
type WaivableP1RequirementId = Exclude<
  FormalP1RequirementId,
  M1MandatoryP1RequirementId
>;

interface TraceRegisterRecordBase {
  schemaVersion: 2;
  releaseId: Uuid;
  requirementId: string;
  milestoneIds: M1MilestoneId[];
  milestoneBindingDigestSha256: Sha256;
  testId: ReleaseTestId;
  runId: Uuid;
  priority: 'P0' | 'P1';
  risks: NonEmptyArray<RiskLevel | 'all'>;
  capabilityState: 'enabled' | 'disabledByReleasePolicy';
  implementationModule: string;
  implementationVersion: string;
  platform: RunPlatformBinding;
  testResult: TestRunRecord['result'];
  evidenceRelativePath: EvidenceRelativePath;
  evidenceSha256: Sha256;
  releaseArtifactSetDigestSha256: Sha256;
  releaseCapabilityManifestCanonicalDigestSha256: Sha256;
  releaseTestObligationsDigestSha256: Sha256;
  sourceCommit: string;
  buildId: string;
  ciProvenanceDigestSha256: Sha256;
  serviceDeliveryRecordDigestSha256?: Sha256;
  serviceDeploymentEnvironment?: 'staging' | 'production';
  executedBy: string;
  completedAtUtc: TimestampUtc;
}

type TraceRegisterRecord = TraceRegisterRecordBase &
  (
    | {
        disposition: 'passed';
        testResult: 'passed';
        waiverId?: never;
      }
    | {
        disposition: 'waived';
        requirementId: WaivableP1RequirementId;
        milestoneIds: [];
        priority: 'P1';
        testResult: 'failed' | 'blocked';
        waiverId: string;
      }
    | {
        disposition: 'gateFailed';
        testResult: 'failed' | 'blocked';
        waiverId?: never;
      }
  );

interface WaiverRecord {
  schemaVersion: 1;
  waiverId: string;
  scope: 'nonM1P1Only';
  requirementId: WaivableP1RequirementId;
  milestoneIds: [];
  milestoneBindingDigestSha256: Sha256;
  testId: ReleaseTestId;
  runId: Uuid;
  testDefinitionDigestSha256: Sha256;
  testRunDigestSha256: Sha256;
  releaseArtifactSetDigestSha256: Sha256;
  releaseCapabilityManifestCanonicalDigestSha256: Sha256;
  owner: string;
  approvedByRole: 'security' | 'test' | 'release';
  approvalEvidenceDigestSha256: Sha256;
  reasonDigestSha256: Sha256;
  createdAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
  remediationVersion: string;
}

type EvidenceRelativePath = string;

interface EvidenceFileRef<
  P extends EvidenceRelativePath = EvidenceRelativePath,
> {
  relativePath: P;
  fileSha256: Sha256;
}

interface CanonicalJsonEvidenceFileRef<
  P extends EvidenceRelativePath = EvidenceRelativePath,
> extends EvidenceFileRef<P> {
  canonicalPayloadDigestSha256: Sha256;
}

type ReleaseArtifactArchitecture = 'x64' | 'arm64';
type ReleaseArtifactRole = 'bootstrapper' | 'msiPayload' | 'nsisInstaller';

type ReleaseArtifactRelativePath<
  R extends ReleaseArtifactRole,
  A extends ReleaseArtifactArchitecture,
> = R extends 'bootstrapper'
  ? `artifacts/${A}/qingpan-setup.exe`
  : R extends 'msiPayload'
    ? `artifacts/${A}/qingpan.msi`
    : `artifacts/${A}/qingpan-migration.exe`;

interface ReleaseArtifactRef<
  R extends ReleaseArtifactRole,
  A extends ReleaseArtifactArchitecture,
> extends EvidenceFileRef<ReleaseArtifactRelativePath<R, A>> {
  role: R;
  architecture: A;
}

type ReleaseArtifactSet = readonly [
  ReleaseArtifactRef<'bootstrapper', 'x64'>,
  ReleaseArtifactRef<'msiPayload', 'x64'>,
  ReleaseArtifactRef<'nsisInstaller', 'x64'>,
  ReleaseArtifactRef<'bootstrapper', 'arm64'>,
  ReleaseArtifactRef<'msiPayload', 'arm64'>,
  ReleaseArtifactRef<'nsisInstaller', 'arm64'>,
];

```
