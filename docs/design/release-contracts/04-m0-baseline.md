<a id="qpn-sec-13-6-5"></a>
# 13.6.5 M0 基线与验证记录

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
type BaselineCapabilityId =
  | 'desktopClient'
  | 'builtInAllowlistCacheScan'
  | 'scanSnapshotRevalidation'
  | 'applicationProcessProtection'
  | 'storageAnalysis'
  | 'largeFileScan'
  | 'largeFilePermanentDelete'
  | 'duplicateIdentification'
  | 'applicationInventoryUninstall'
  | 'partitionReadOnly'
  | 'localAudit'
  | 'quarantineExportCenter'
  | 'exclusionRules'
  | 'startupInventory'
  | 'scheduledCleanup'
  | 'applicationUpdate'
  | 'signedRuleUpdate'
  | 'licenseValidation'
  | 'highRiskFutureCapabilities';

interface BaselineImplementationRef {
  relativePath: EvidenceRelativePath;
  symbol: string;
  blobSha256: Sha256;
}

interface BaselineCapabilityEvidence {
  state: 'verified' | 'presentUnverified' | 'partial' | 'absent';
  implementationRefs: BaselineImplementationRef[];
  featureReleaseGate: string;
  currentCommands: string[];
  limitations: string[];
  verificationCommands: string[];
  runIds: string[];
  evidenceFiles: readonly EvidenceFileRef[];
  ownerRole: 'development' | 'security' | 'test' | 'release';
}

interface BaselineManifestCanonicalPayload {
  schemaVersion: 1;
  baselineId: string;
  canonicalization: 'RFC8785';
  capturedAtUtc: TimestampUtc;
  sourceCommit: string;
  trackedPatchSha256: Sha256;
  untrackedImplementationFiles: readonly {
    relativePath: EvidenceRelativePath;
    blobSha256: Sha256;
  }[];
  workspaceManifestSha256: Sha256;
  snapshotBundleRelativePath: 'm0/snapshot-bundle.qps1';
  snapshotBundleSha256: Sha256;
  lockfiles: NonEmptyArray<{
    relativePath: EvidenceRelativePath;
    sha256: Sha256;
  }>;
  toolchain: {
    node: string;
    pnpm: string;
    rustc: string;
    cargo: string;
    tauriCli: string;
    windowsSdk: string;
    rustTargets: readonly ['x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'];
  };
  document: {
    documentId: 'QPN-DOC-DESKTOP-001';
    documentVersion: string;
    documentSha256: Sha256;
  };
  capabilities: Readonly<
    Record<BaselineCapabilityId, BaselineCapabilityEvidence>
  >;
}

interface BaselineManifest extends BaselineManifestCanonicalPayload {
  manifestDigestSha256: Sha256;
}

interface M0CiProvenanceStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.m0-ci-provenance.v1+json';
  domain: 'qingpan.m0-ci-provenance.v1';
  canonicalization: 'RFC8785';
  baselineId: string;
  baselineManifest: CanonicalJsonEvidenceFileRef<'m0/baseline-manifest.json'>;
  snapshotBundle: EvidenceFileRef<'m0/snapshot-bundle.qps1'>;
  sourceCommit: string;
  workspaceManifestSha256: Sha256;
  capabilityEvidenceRootSha256: Sha256;
  replayToolchainDigestSha256: Sha256;
  governanceTrustPolicyDigestSha256: Sha256;
  builderPrincipalId: string;
  result: 'passed';
}

interface M0CiProvenanceRecord {
  schemaVersion: 1;
  statement: M0CiProvenanceStatementCanonicalPayload;
  statementDigestSha256: Sha256;
  signingKeyId: string;
  algorithm: GovernanceSignatureAlgorithm;
  signatureBase64: string;
}

interface M0BaselineApprovalStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.m0-baseline-approval-statement.v1+json';
  domain: 'qingpan.m0-baseline-approval.v1';
  canonicalization: 'RFC8785';
  baselineId: string;
  baselineManifest: CanonicalJsonEvidenceFileRef;
  snapshotBundle: EvidenceFileRef<'m0/snapshot-bundle.qps1'>;
  workspaceManifestSha256: Sha256;
  capabilityEvidenceRootSha256: Sha256;
  sourceCommit: string;
  m0CiProvenance: CanonicalJsonEvidenceFileRef<'m0/ci-provenance.json'>;
  governanceTrustPolicyDigestSha256: Sha256;
  result: 'passed';
}

interface M0BaselineVerificationRecordCanonicalPayload {
  schemaVersion: 2;
  verificationId: Uuid;
  baselineId: string;
  canonicalization: 'RFC8785';
  baselineManifest: CanonicalJsonEvidenceFileRef;
  snapshotBundle: EvidenceFileRef<'m0/snapshot-bundle.qps1'>;
  workspaceManifestSha256: Sha256;
  capabilityEvidenceRootSha256: Sha256;
  sourceCommit: string;
  m0CiProvenance: CanonicalJsonEvidenceFileRef<'m0/ci-provenance.json'>;
  governanceTrustPolicyDigestSha256: Sha256;
  result: 'passed';
  verifiedAtUtc: TimestampUtc;
  approvalStatement: M0BaselineApprovalStatementCanonicalPayload;
  approvalStatementDigestSha256: Sha256;
  approvals: readonly [
    GovernanceRoleApproval<
      'product',
      'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'desktop',
      'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'security',
      'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'test',
      'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'release',
      'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
    >,
  ];
}

interface M0BaselineVerificationRecord
  extends M0BaselineVerificationRecordCanonicalPayload {
  recordDigestSha256: Sha256;
}

```
