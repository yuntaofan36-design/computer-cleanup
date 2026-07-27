<a id="qpn-sec-8-3-4"></a>
# 8.3.4 规则签名、更新与撤销

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface RuleManifest {
  schemaVersion: 1;
  ruleId: string;
  ruleVersion: string;
  envelopeId: string;
  displayName: string;
  category: FileConfirmationCategoryId;
  supportedBuilds: Array<{
    min: number;
    max?: number;
    architectures: Array<'x64' | 'arm64'>;
  }>;
  rootSelections: Array<{
    resolverId: string;
    leafTemplateId: string;
    narrowerRelativePath?: string;
  }>;
  matcher: RuleMatcher;
  minimumAgeSeconds?: number;
  processGuardIds: string[];
  risk: RiskLevel;
  action: ActionKind;
  requiredPrivilege: 'user' | 'administrator';
  exclusionClasses: CapabilityEnvelope['mandatoryExclusionClasses'];
  recovery: RecoveryKind;
  automationAllowed: boolean;
  resourceLimits: RuleResourceLimits;
  evidence: string;
  acceptanceIds: string[];
}

type UpdateChannel = 'stable' | 'beta' | 'internal';

interface RecoverySignature {
  recoveryKeyId: string;
  signatureEd25519Base64: string;
}

type RecoverySignaturePurpose =
  | 'releaseKeyAuthorization'
  | 'ruleRevocation'
  | 'appRevocation'
  | 'appEpochMigration'
  | 'appRecoveryPackage'
  | 'machineMsiUpgradeCodeMigration'
  | 'authenticodeOfflineStatus'
  | 'authenticodeSignerRotation';

interface RecoveryPublicKey {
  recoveryKeyId: string;
  publicKeyEd25519Base64: string;
  publicKeyFingerprintSha256: Sha256;
}

interface RecoveryKeySet {
  recoveryKeySetId: string;
  keySetEpoch: U64String;
  threshold: number;
  keys: [RecoveryPublicKey, ...RecoveryPublicKey[]];
  allowedPurposes: RecoverySignaturePurpose[];
}

interface ReleaseSigned<T> {
  canonicalization: 'RFC8785';
  payload: T;
  payloadSha256: Sha256;
  signatureEd25519Base64: string;
}

interface RecoverySigned<T> {
  canonicalization: 'RFC8785';
  payload: T;
  payloadSha256: Sha256;
  signatures: RecoverySignature[];
}

interface ReleaseKeyAuthorizationPayload {
  schemaVersion: 1;
  authorizationSequence: U64String;
  recoveryKeySetId: string;
  releaseKeyId: string;
  publicKeyEd25519Base64: string;
  scope:
    | {
        domain: 'rules';
        channel: UpdateChannel;
        minimumPackageSequence: U64String;
        maximumPackageSequence?: U64String;
      }
    | {
        domain: 'application';
        channel: UpdateChannel;
        releaseEpoch: U64String;
        minimumManifestSequence: U64String;
        maximumManifestSequence?: U64String;
      };
  allowedMediaTypes: string[];
  issuedAtUtc: string;
  expiresAtUtc: string;
}

type RecoverySignedReleaseKeyAuthorization =
  RecoverySigned<ReleaseKeyAuthorizationPayload> & {
    mediaType: 'application/vnd.qingpan.release-key-authorization+json';
  };

interface RulePackagePayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  sequence: U64String;
  packageVersion: string;
  releaseKeyId: string;
  issuedAtUtc: string;
  minimumAppVersion: string;
  maximumAppVersion?: string;
  rules: RuleManifest[];
}

type SignedRulePackage = ReleaseSigned<RulePackagePayload> & {
  mediaType: 'application/vnd.qingpan.rules+json';
};

interface RuleRevocationPayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  revocationSequence: U64String;
  recoveryKeySetId: string;
  issuedAtUtc: string;
  expiresAtUtc: string;
  revokedReleaseKeyIds: string[];
  revokedIndexPayloadHashes: Sha256[];
  revokedPackageHashes: Sha256[];
  minimumAcceptedPackageSequence: U64String;
  reasonCode: 'keyCompromise' | 'unsafeRule' | 'packageDefect' | 'channelReset';
}

type RecoverySignedRuleRevocation = RecoverySigned<RuleRevocationPayload> & {
  mediaType: 'application/vnd.qingpan.rule-revocations+json';
};

interface RuleIndexPayload {
  schemaVersion: 1;
  channel: UpdateChannel;
  indexSequence: U64String;
  releaseKeyId: string;
  keyAuthorizationPayloadSha256: Sha256;
  revocationPayloadSha256: Sha256;
  packagePayloadSha256: Sha256;
  packageSequence: U64String;
  packageVersion: string;
  packageSizeBytes: U64String;
  issuedAtUtc: string;
  expiresAtUtc: string;
}

type SignedRuleIndex = ReleaseSigned<RuleIndexPayload> & {
  mediaType: 'application/vnd.qingpan.rule-index+json';
};

interface RuleTrustState {
  channel: UpdateChannel;
  highestAuthorizationSequence: U64String;
  highestAuthorizationPayloadSha256: Sha256;
  highestRevocationSequence: U64String;
  highestRevocationPayloadSha256: Sha256;
  highestIndexSequence: U64String;
  highestIndexPayloadSha256: Sha256;
  highestPackageSequence: U64String;
  highestPackagePayloadSha256: Sha256;
  acceptedKeyAuthorizations: RecoverySignedReleaseKeyAuthorization[];
  stickyMinimumAcceptedPackageSequence: U64String;
  stickyRevokedReleaseKeyIds: string[];
  stickyRevokedIndexPayloadHashes: Sha256[];
  stickyRevokedPackageHashes: Sha256[];
  activeReleaseKeyId?: string;
  activeIndexPayloadSha256?: Sha256;
  activePackageSequence?: U64String;
  activePackagePayloadSha256?: Sha256;
  lastDurableAtUtc: string;
}

```
