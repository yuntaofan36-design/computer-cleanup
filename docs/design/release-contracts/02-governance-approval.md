<a id="qpn-sec-13-6-3"></a>
# 13.6.3 治理信任与文档批准

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
const GOVERNANCE_APPROVAL_ROLES = [
  'product',
  'desktop',
  'security',
  'test',
  'release',
] as const;

type GovernanceApprovalRole = (typeof GOVERNANCE_APPROVAL_ROLES)[number];
type GovernanceSignatureAlgorithm = 'ed25519' | 'ecdsa-p256-sha256';
type GovernanceApprovalStatementMediaType =
  | 'application/vnd.qingpan.document-approval-statement.v1+json'
  | 'application/vnd.qingpan.m0-baseline-approval-statement.v1+json'
  | 'application/vnd.qingpan.dependency-disposition-statement.v1+json';

type TrustedApprovalBinding = StrictUnion<
  | {
      kind: 'detachedSigningKey';
      keyId: string;
      algorithm: GovernanceSignatureAlgorithm;
      publicKeySpkiDerBase64: string;
      publicKeySpkiSha256: Sha256;
    }
  | {
      kind: 'reviewSystemIdentity';
      reviewSystemId: string;
      tenantId: string;
      reviewerSubjectId: string;
    }
>;

interface TrustedApprovalPrincipal {
  principalId: string;
  humanIdentityId: string;
  allowedRoles: readonly GovernanceApprovalRole[];
  bindings: readonly TrustedApprovalBinding[];
}

interface TrustedReviewSystem {
  reviewSystemId: string;
  tenantId: string;
  receiptKeyId: string;
  receiptAlgorithm: GovernanceSignatureAlgorithm;
  receiptPublicKeySpkiDerBase64: string;
  receiptPublicKeySpkiSha256: Sha256;
}

interface TrustedPurposeSigningKey {
  keyId: string;
  principalId: string;
  purpose:
    | 'm0CiProvenance'
    | 'ciBuildProvenance'
    | 'qingpanReleaseAttestation';
  algorithm: GovernanceSignatureAlgorithm;
  publicKeySpkiDerBase64: string;
  publicKeySpkiSha256: Sha256;
}

interface GovernanceTrustPolicyCanonicalPayload {
  schemaVersion: 1;
  policyId: 'QPN-GOVERNANCE-TRUST-V1-001';
  policySequence: U64String;
  organizationTrustRootId: string;
  canonicalization: 'RFC8785';
  principals: NonEmptyArray<TrustedApprovalPrincipal>;
  reviewSystems: readonly TrustedReviewSystem[];
  purposeSigningKeys: NonEmptyArray<TrustedPurposeSigningKey>;
  trustedReplayToolchainDigestsSha256: NonEmptyArray<Sha256>;
  revokedPrincipalIds: readonly string[];
  revokedKeyIds: readonly string[];
  separationOfDuties: {
    requireDistinctHumanIdentityPerRole: true;
    maximumRolesPerApprovalStatementPerHuman: 1;
    releaseAttestationSignerDistinctFromApproversAndBuilder: true;
  };
  validFromUtc: TimestampUtc;
  validUntilUtc: TimestampUtc;
}

interface GovernanceTrustPolicy
  extends GovernanceTrustPolicyCanonicalPayload {
  policyDigestSha256: Sha256;
  organizationRootSignatures: NonEmptyArray<{
    rootKeyId: string;
    algorithm: GovernanceSignatureAlgorithm;
    signatureBase64: string;
  }>;
}

type ApprovalAttestation = StrictUnion<
  | {
      kind: 'signedStatement';
      signingKeyId: string;
      algorithm: GovernanceSignatureAlgorithm;
      signatureBase64: string;
    }
  | {
      kind: 'reviewSystemDecision';
      reviewSystemId: string;
      tenantId: string;
      reviewerSubjectId: string;
      changeId: string;
      decisionId: string;
      decisionRevision: U64String;
      decision: 'approved';
      signedReceipt: EvidenceFileRef;
    }
>;

interface GovernanceRoleApproval<
  R extends GovernanceApprovalRole,
  M extends GovernanceApprovalStatementMediaType,
> {
  role: R;
  principalId: string;
  statementMediaType: M;
  approvalStatementDigestSha256: Sha256;
  approvedAtUtc: TimestampUtc;
  attestation: ApprovalAttestation;
}

interface DocumentApprovalStatementCanonicalPayload {
  schemaVersion: 1;
  mediaType: 'application/vnd.qingpan.document-approval-statement.v1+json';
  domain: 'qingpan.document-approval.v1';
  canonicalization: 'RFC8785';
  documentId: 'QPN-DOC-DESKTOP-001';
  status: 'approved';
  documentVersion: string;
  documentSha256: Sha256;
  accountableOwner: string;
  governanceTrustPolicyDigestSha256: Sha256;
}

interface DesignDocumentApprovalRecordCanonicalPayload {
  schemaVersion: 2;
  statement: DocumentApprovalStatementCanonicalPayload;
  approvalStatementDigestSha256: Sha256;
  approvals: readonly [
    GovernanceRoleApproval<
      'product',
      'application/vnd.qingpan.document-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'desktop',
      'application/vnd.qingpan.document-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'security',
      'application/vnd.qingpan.document-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'test',
      'application/vnd.qingpan.document-approval-statement.v1+json'
    >,
    GovernanceRoleApproval<
      'release',
      'application/vnd.qingpan.document-approval-statement.v1+json'
    >,
  ];
}

interface DesignDocumentApprovalRecord
  extends DesignDocumentApprovalRecordCanonicalPayload {
  recordDigestSha256: Sha256;
}

```
