<a id="qpn-sec-13-6-6"></a>
# 13.6.6 依赖安全、源码派生与构建来源

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface DependencyFindingRecordCanonicalPayload {
  schemaVersion: 1;
  findingKeySha256: Sha256;
  releaseId: Uuid;
  releaseArtifactSetDigestSha256: Sha256;
  sbomFileSha256: Sha256;
  scannerIdentity: string;
  scannerDatabaseSnapshotSha256: Sha256;
  componentBomRef: string;
  componentPurl: string;
  advisoryId: string;
  severity: 'critical' | 'high' | 'medium' | 'low' | 'unknown';
}

interface DependencyFindingRecord
  extends DependencyFindingRecordCanonicalPayload {
  findingRecordDigestSha256: Sha256;
}

interface DependencyDispositionStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.dependency-disposition-statement.v1+json';
  domain: 'qingpan.dependency-disposition.v1';
  canonicalization: 'RFC8785';
  releaseId: Uuid;
  findingKeySha256: Sha256;
  findingRecordDigestSha256: Sha256;
  decision: 'fixed' | 'notAffected' | 'falsePositive';
  rationaleDigestSha256: Sha256;
  evidenceFiles: NonEmptyArray<EvidenceFileRef>;
  expiresAtUtc: TimestampUtc;
  governanceTrustPolicyDigestSha256: Sha256;
}

interface DependencyDispositionRecord {
  schemaVersion: 1;
  statement: DependencyDispositionStatementCanonicalPayload;
  approvalStatementDigestSha256: Sha256;
  approvals: readonly [
    GovernanceRoleApproval<
      'security',
      'application/vnd.qingpan.dependency-disposition-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'release',
      'application/vnd.qingpan.dependency-disposition-statement.v1+json'
    >,
  ];
}

interface DependencySecurityReportCanonicalPayload {
  schemaVersion: 2;
  reportId: Uuid;
  releaseId: Uuid;
  canonicalization: 'RFC8785';
  releaseArtifactSetDigestSha256: Sha256;
  sourceCommit: string;
  buildId: string;
  ciProvenanceDigestSha256: Sha256;
  sbomFormat: 'CycloneDX-JSON-1.6';
  sbomFileSha256: Sha256;
  scannerIdentity: string;
  scannerDatabaseSnapshotSha256: Sha256;
  findingsFile: EvidenceFileRef<'release/dependency-findings.jsonl'>;
  dispositionsFile: EvidenceFileRef<'release/dependency-dispositions.jsonl'>;
  findingRecordCount: number;
  criticalOrHighFindingCount: number;
  dispositionRecordCount: number;
  resolvedCriticalOrHighCount: number;
  unresolvedCriticalOrHighCount: 0;
  findingsRootSha256: Sha256;
  dispositionsRootSha256: Sha256;
  scannedAtUtc: TimestampUtc;
  result: 'passed';
}

interface DependencySecurityReport
  extends DependencySecurityReportCanonicalPayload {
  reportDigestSha256: Sha256;
}

interface ReleaseSourceDerivationRecordCanonicalPayload {
  schemaVersion: 1;
  derivationId: Uuid;
  releaseId: Uuid;
  canonicalization: 'RFC8785';
  m0BaselineId: string;
  baselineManifest: CanonicalJsonEvidenceFileRef<'m0/baseline-manifest.json'>;
  snapshotBundle: EvidenceFileRef<'m0/snapshot-bundle.qps1'>;
  m0VerificationRecord: CanonicalJsonEvidenceFileRef<'m0/verification-record.json'>;
  m0ReplayedSourceTreeRootSha256: Sha256;
  sourcePatchBundle: EvidenceFileRef<'release/source-patch.bundle'>;
  patchFormat: 'git-binary-full-index-no-renames-v1';
  patchInputSourceTreeRootSha256: Sha256;
  patchOutputSourceTreeRootSha256: Sha256;
  finalRepositoryId: string;
  finalSourceCommit: string;
  finalSourceTreeRootSha256: Sha256;
  buildInputSourceTreeRootSha256: Sha256;
  replayToolchainDigestSha256: Sha256;
  replayRunId: Uuid;
  result: 'passed';
}

interface ReleaseSourceDerivationRecord
  extends ReleaseSourceDerivationRecordCanonicalPayload {
  recordDigestSha256: Sha256;
}

interface CiBuildProvenanceStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.ci-build-provenance.v1+json';
  domain: 'qingpan.ci-build-provenance.v1';
  canonicalization: 'RFC8785';
  releaseId: Uuid;
  releaseArtifacts: ReleaseArtifactSet;
  releaseArtifactSetDigestSha256: Sha256;
  sourceCommit: string;
  buildId: string;
  sourceDerivationRecord: CanonicalJsonEvidenceFileRef<'release/source-derivation-record.json'>;
  buildInputSourceTreeRootSha256: Sha256;
  releaseCapabilityManifest: CanonicalJsonEvidenceFileRef<'release/release-capability-manifest.json'>;
  governanceTrustPolicyDigestSha256: Sha256;
  builderPrincipalId: string;
  result: 'passed';
}

interface CiBuildProvenanceRecord {
  schemaVersion: 1;
  statement: CiBuildProvenanceStatementCanonicalPayload;
  statementDigestSha256: Sha256;
  signingKeyId: string;
  algorithm: GovernanceSignatureAlgorithm;
  signatureBase64: string;
}

```
