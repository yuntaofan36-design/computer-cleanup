<a id="qpn-sec-13-6-7"></a>
# 13.6.7 发布门禁根与签名声明

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
type OnlineServiceEnvironment = 'staging' | 'production';

type ServiceDeliveryRecordRelativePath<
  D extends OnlineServiceDesignId,
  E extends OnlineServiceEnvironment,
> = D extends 'QPN-SVC-LIC-001'
  ? `services/license/${E}.json`
  : D extends 'QPN-SVC-RULE-001'
    ? `services/rules/${E}.json`
    : `services/update/${E}.json`;

interface ReleaseServiceEvidenceRef<
  D extends OnlineServiceDesignId,
  E extends OnlineServiceEnvironment,
> extends CanonicalJsonEvidenceFileRef<ServiceDeliveryRecordRelativePath<D, E>> {
  serviceDesignId: D;
  environment: E;
}

interface ReleaseGateEvidenceFiles {
  testDefinitionsJsonl: EvidenceFileRef<'quality/test-definitions.jsonl'>;
  testRunsJsonl: EvidenceFileRef<'quality/test-runs.jsonl'>;
  traceRegisterJsonl: EvidenceFileRef<'quality/trace-register.jsonl'>;
  waiversJsonl: EvidenceFileRef<'quality/waivers.jsonl'>;
  platformTupleRegistry: CanonicalJsonEvidenceFileRef<'quality/platform-tuple-registry.json'>;
  formalContractRegistrySnapshot: CanonicalJsonEvidenceFileRef<'quality/formal-contract-registry-snapshot.json'>;
  designDocumentApprovalRecord: CanonicalJsonEvidenceFileRef<'release/design-document-approval-record.json'>;
  releaseCapabilityManifest: CanonicalJsonEvidenceFileRef<'release/release-capability-manifest.json'>;
  dependencySecurityReport: CanonicalJsonEvidenceFileRef<'release/dependency-security-report.json'>;
  sbom: EvidenceFileRef<'release/sbom.cdx.json'>;
  baselineManifest: CanonicalJsonEvidenceFileRef<'m0/baseline-manifest.json'>;
  snapshotBundle: EvidenceFileRef<'m0/snapshot-bundle.qps1'>;
  m0BaselineVerificationRecord: CanonicalJsonEvidenceFileRef<'m0/verification-record.json'>;
  governanceTrustPolicy: CanonicalJsonEvidenceFileRef<'trust/governance-trust-policy.json'>;
  sourceDerivationRecord: CanonicalJsonEvidenceFileRef<'release/source-derivation-record.json'>;
  sourcePatchBundle: EvidenceFileRef<'release/source-patch.bundle'>;
  dependencyFindingsJsonl: EvidenceFileRef<'release/dependency-findings.jsonl'>;
  dependencyDispositionsJsonl: EvidenceFileRef<'release/dependency-dispositions.jsonl'>;
  ciProvenance: CanonicalJsonEvidenceFileRef<'release/ci-build-provenance.json'>;
}

interface ReleaseGateManifestCanonicalPayload {
  schemaVersion: 4;
  releaseId: Uuid;
  canonicalization: 'RFC8785';
  releaseArtifacts: ReleaseArtifactSet;
  releaseArtifactSetDigestSha256: Sha256;
  sourceCommit: string;
  buildId: string;
  ciProvenanceDigestSha256: Sha256;
  governanceTrustPolicyDigestSha256: Sha256;
  sourceDerivationRecordDigestSha256: Sha256;
  m0BaselineId: string;
  evidenceFiles: ReleaseGateEvidenceFiles;
  serviceDeliveryRecords: readonly [
    ReleaseServiceEvidenceRef<'QPN-SVC-LIC-001', 'staging'>,
    ReleaseServiceEvidenceRef<'QPN-SVC-LIC-001', 'production'>,
    ReleaseServiceEvidenceRef<'QPN-SVC-RULE-001', 'staging'>,
    ReleaseServiceEvidenceRef<'QPN-SVC-RULE-001', 'production'>,
    ReleaseServiceEvidenceRef<'QPN-SVC-UPDATE-001', 'staging'>,
    ReleaseServiceEvidenceRef<'QPN-SVC-UPDATE-001', 'production'>,
  ];
  createdAtUtc: TimestampUtc;
}

interface ReleaseGateManifest extends ReleaseGateManifestCanonicalPayload {
  manifestDigestSha256: Sha256;
}

interface SignedReleaseStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.release-statement.v1+json';
  domain: 'qingpan.release-attestation.v1';
  canonicalization: 'RFC8785';
  releaseId: Uuid;
  releaseArtifacts: ReleaseArtifactSet;
  releaseArtifactSetDigestSha256: Sha256;
  releaseGateManifest: EvidenceFileRef<'release/release-gate-manifest.json'>;
  releaseGateManifestDigestSha256: Sha256;
  sourceCommit: string;
  buildId: string;
  ciProvenance: CanonicalJsonEvidenceFileRef<'release/ci-build-provenance.json'>;
  sourceDerivationRecordDigestSha256: Sha256;
  governanceTrustPolicyDigestSha256: Sha256;
  issuedAtUtc: TimestampUtc;
}

interface SignedReleaseAttestation {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.release-attestation.v1+json';
  statement: SignedReleaseStatementCanonicalPayload;
  statementDigestSha256: Sha256;
  signingKeyId: string;
  algorithm: GovernanceSignatureAlgorithm;
  signatureBase64: string;
}
```
