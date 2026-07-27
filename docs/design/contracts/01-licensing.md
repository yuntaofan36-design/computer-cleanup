<a id="qpn-sec-8-3-2"></a>
# 8.3.2 授权与许可证事务

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface LicenseTokenWireBundle {
  accessToken: string;
  accessTokenExpiresAtUtc: TimestampUtc;
  refreshToken: string;
  refreshTokenExpiresAtUtc: TimestampUtc;
  tokenFamilyId: Uuid;
  devicePublicKeySpkiSha256: Sha256;
}

interface LicenseActivationWireResult {
  schemaVersion: 1;
  kind: 'activated';
  installationId: Uuid;
  activationRequestId: Uuid;
  licenseSubjectId: Uuid;
  seatAssignmentId: Uuid;
  deactivationAuthorizationKeyId: Uuid;
  deactivationAuthorizationPublicKeySpkiSha256: Sha256;
  entitlements: string[];
  tokens: LicenseTokenWireBundle;
  activatedAtUtc: TimestampUtc;
}

interface LicenseValidationWireResult {
  schemaVersion: 1;
  kind: 'valid';
  installationId: Uuid;
  licenseSubjectId: Uuid;
  seatAssignmentId: Uuid;
  entitlements: string[];
  validatedAtUtc: TimestampUtc;
  nextValidationAtUtc: TimestampUtc;
}

interface LicenseRefreshWireResult {
  schemaVersion: 1;
  kind: 'refreshed';
  installationId: Uuid;
  refreshRequestId: Uuid;
  rotationResultId: Uuid;
  tokens: LicenseTokenWireBundle;
  refreshedAtUtc: TimestampUtc;
}

interface LicenseDeactivationWireResult {
  schemaVersion: 1;
  kind: 'deactivated';
  installationId: Uuid;
  deactivationRequestId: Uuid;
  revokedSeatAssignmentId: Uuid;
  revokedTokenFamilyId: Uuid;
  deactivatedAtUtc: TimestampUtc;
}

interface LicenseActivateWireRequest {
  licenseKey: string;
  installationId: Uuid;
  activationRequestId: Uuid;
  appVersion: string;
  devicePublicKeySpki: string;
  deactivationAuthorizationPublicKeySpki: string;
}

interface LicenseActivationProofProtectedHeader {
  schemaVersion: 1;
  alg: 'ES256';
  typ: 'qingpan-license-activation-pop+jws';
  devicePublicKeySpkiSha256: Sha256;
}

interface LicenseActivationProofPayload {
  schemaVersion: 1;
  domain: 'qingpan.license.activation-pop.v1';
  purpose: 'licenseActivationProof';
  method: 'POST';
  path: '/api/license/activate';
  bodySha256: Sha256;
  installationId: Uuid;
  activationRequestId: Uuid;
  iat: number;
  jti: Uuid;
}

type LicenseActivationProofCompact = `${string}.${string}.${string}`;

type LicenseValidateWireRequest = Record<string, never>;

interface LicenseRefreshWireRequest {
  refreshRequestId: Uuid;
}

interface LicenseDeactivationChallengeWireRequest {
  installationId: Uuid;
  deactivationRequestId: Uuid;
  deactivationAuthorizationKeyId: Uuid;
}

interface LicenseDeactivationChallengeWireResult {
  schemaVersion: 1;
  challengeId: Uuid;
  installationId: Uuid;
  deactivationRequestId: Uuid;
  deactivationAuthorizationKeyId: Uuid;
  challengeNonceBase64: string;
  issuedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
  singleUse: true;
}

interface LicenseDeactivateWireRequest {
  installationId: Uuid;
  deactivationRequestId: Uuid;
  deactivationAuthorizationKeyId: Uuid;
  challengeId: Uuid;
  issuedAtUtc: TimestampUtc;
  reason: LicenseDeactivationReason;
  deactivationGrantDigestSha256: Sha256;
  canonicalStatementSha256: Sha256;
  statementSignature: string;
  userVerificationEvidenceDigestSha256: Sha256;
}

type LicenseMutationKind = 'activation' | 'refresh' | 'deactivation';

interface LicenseMutationReconcileWireRequest {
  installationId: Uuid;
  mutationKind: LicenseMutationKind;
  mutationRequestId: Uuid;
  requestBodySha256: Sha256;
  devicePublicKeySpki: string;
}

type LicenseReconciledMutation =
  | {
      mutationKind: 'activation';
      mutationRequestId: Uuid;
      response: LicenseActivationWireResult;
    }
  | {
      mutationKind: 'refresh';
      mutationRequestId: Uuid;
      response: LicenseRefreshWireResult;
    }
  | {
      mutationKind: 'deactivation';
      mutationRequestId: Uuid;
      response: LicenseDeactivationWireResult;
    };

type LicenseMutationReconcileWireResult =
  | {
      schemaVersion: 1;
      kind: 'committed';
      installationId: Uuid;
      requestBodySha256: Sha256;
      mutation: LicenseReconciledMutation;
      checkedAtUtc: TimestampUtc;
    }
  | {
      schemaVersion: 1;
      kind: 'pending';
      installationId: Uuid;
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      requestBodySha256: Sha256;
      retryAfterSeconds: number;
      checkedAtUtc: TimestampUtc;
    }
  | {
      schemaVersion: 1;
      kind: 'notCommitted';
      installationId: Uuid;
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      requestBodySha256: Sha256;
      durableNegativeFence: true;
      mutationKeyReuseBlocked: true;
      fenceRetention: 'licenseSubjectLifecycle';
      fenceCreatedAtUtc: TimestampUtc;
      checkedAtUtc: TimestampUtc;
    }
  | {
      schemaVersion: 1;
      kind: 'recoveryRequired';
      installationId: Uuid;
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      requestBodySha256: Sha256;
      checkedAtUtc: TimestampUtc;
    };

type LicenseReconciliationState =
  | { state: 'notAttempted' }
  | {
      state: 'pending';
      checkedAtUtc: TimestampUtc;
      retryAfterSeconds: number;
    }
  | {
      state: 'unavailable';
      checkedAtUtc: TimestampUtc;
      code:
        | 'LICENSE_RATE_LIMITED'
        | 'LICENSE_PROOF_INVALID'
        | 'LICENSE_RESPONSE_INVALID'
        | 'LICENSE_RECOVERY_REQUIRED';
    };

type LicenseMutationResolutionRecordBase<K extends LicenseMutationKind> = {
  recordVersion: 2;
  installationId: Uuid;
  devicePublicKeySpkiSha256: Sha256;
  mutationKind: K;
  mutationRequestId: Uuid;
  requestBodySha256: Sha256;
  resolvedAtUtc: TimestampUtc;
};

type LicenseNotCommittedFenceResolution = {
  resolution: 'notCommittedFenced';
  durableNegativeFence: true;
  mutationKeyReuseBlocked: true;
  fenceRetention: 'licenseSubjectLifecycle';
  fenceCreatedAtUtc: TimestampUtc;
};

type LicenseMutationResolutionRecord = StrictUnion<
  | (LicenseMutationResolutionRecordBase<'activation'> &
    (
      | {
          resolution: 'committedApplied';
          responsePayloadSha256: Sha256;
        }
      | {
          resolution: 'committedRequiresReauthentication';
          responsePayloadSha256: Sha256;
          code: 'LICENSE_RECOVERY_REQUIRED';
        }
      | (LicenseNotCommittedFenceResolution & {
          credentialsIssued: false;
          activationSecretDisposition: 'destroyedAfterTerminalFence';
        })
    ))
  | (LicenseMutationResolutionRecordBase<'refresh'> &
    (
      | {
          resolution: 'committedApplied';
          responsePayloadSha256: Sha256;
        }
      | {
          resolution: 'committedRequiresReauthentication';
          responsePayloadSha256: Sha256;
          code: 'LICENSE_RECOVERY_REQUIRED';
        }
      | (LicenseNotCommittedFenceResolution & {
          oldCredentials: {
            state: 'retained';
            credentialSetId: Uuid;
            refreshTokenValidUntilUtc: TimestampUtc;
          };
          activeCredentialPointerUnchanged: true;
        })
    ))
  | (LicenseMutationResolutionRecordBase<'deactivation'> &
    (
    | {
        resolution: 'committedApplied';
        responsePayloadSha256: Sha256;
        activeCredentials: {
          state: 'destroyed';
          destroyedAtUtc: TimestampUtc;
        };
        reconciliationKeyState: {
          state: 'destroyed';
          destroyedAtUtc: TimestampUtc;
        };
        deactivationAuthorizationKeyState: {
          state: 'destroyed';
          deactivationAuthorizationKeyId: Uuid;
          destroyedAtUtc: TimestampUtc;
        };
      }
    | (LicenseNotCommittedFenceResolution &
        (
          | {
              activeCredentials: {
                state: 'retained';
                credentialSetId: Uuid;
                devicePublicKeySpkiSha256: Sha256;
              };
              reconciliationKeyState: {
                state: 'restoredToActiveDeviceKey';
                credentialId: Uuid;
                restoredCredentialSetId: Uuid;
                restoredAtUtc: TimestampUtc;
              };
              deactivationAuthorizationKeyState: {
                state: 'destroyed';
                deactivationAuthorizationKeyId: Uuid;
                destroyedAtUtc: TimestampUtc;
              };
              seatMayRemainOccupied: true;
            }
          | {
              activeCredentials: {
                state: 'destroyed';
                destroyedAtUtc: TimestampUtc;
              };
              reconciliationKeyState: {
                state: 'destroyed';
                destroyedAtUtc: TimestampUtc;
              };
              deactivationAuthorizationKeyState: {
                state: 'destroyed';
                destroyedAtUtc: TimestampUtc;
              };
              seatMayRemainOccupied: true;
            }
        ))
    ))>;

interface LegacyLicenseMutationRecoveryRecord {
  recordVersion: 1;
  legacyRecoveryRecordId: Uuid;
  state: 'unresolvedLegacyMutation';
  sourceRecordVersion: number;
  installationId: Uuid;
  mutationKind: LicenseMutationKind;
  mutationRequestId: Uuid;
  legacyRecordSha256: Sha256;
  missingEvidence: NonEmptyArray<'requestBodySha256' | 'devicePrivateKey'>;
  blocksNewLicenseMutation: true;
  requiredResolution: 'signedServerDecisionOrSupport';
  recordedAtUtc: TimestampUtc;
}

interface ActivateLicenseRequest {
  activationDraftId: Uuid;
}

type RefreshLicenseRequest = Record<string, never>;

interface CreateLicenseDeactivationGrantRequest {
  reason: LicenseDeactivationReason;
}

interface DeactivateLicenseRequest {
  deactivationGrantId: Uuid;
}

type LicenseStatusView =
  | { state: 'notActivated' }
  | {
      state: 'active';
      entitlements: string[];
      accessValidUntilUtc: TimestampUtc;
      refreshValidUntilUtc: TimestampUtc;
      lastValidatedAtUtc?: TimestampUtc;
    }
  | {
      state: 'deactivationPending';
      deactivationRequestId: Uuid;
      reason: LicenseDeactivationReason;
      seatMayRemainOccupied: true;
      queuedAtUtc: TimestampUtc;
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      reconciliationDeadlineAtUtc: TimestampUtc;
      reason: 'reconciliationPending';
      nextAction: 'retryReconciliation';
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      reconciliationDeadlineAtUtc: TimestampUtc;
      reason:
        | 'reconciliationDeadlineExpired'
        | 'serverStateUnavailable';
      nextAction: 'contactSupport';
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: LicenseMutationKind;
      mutationRequestId: Uuid;
      legacyRecoveryRecordId: Uuid;
      legacyRecordSha256: Sha256;
      reconciliationDeadlineAtUtc?: never;
      reason: 'unresolvedLegacyMutation';
      nextAction: 'contactSupport';
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: 'activation' | 'refresh';
      mutationRequestId: Uuid;
      reconciliationDeadlineAtUtc: TimestampUtc;
      reason: 'committedRefreshCredentialExpired';
      nextAction: 'reauthenticateKnownInstallation';
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: 'refresh';
      mutationRequestId: Uuid;
      reconciliationDeadlineAtUtc?: never;
      fenceCreatedAtUtc: TimestampUtc;
      priorRefreshTokenValidUntilUtc: TimestampUtc;
      reason: 'notCommittedRefreshCredentialExpired';
      nextAction: 'reauthenticateKnownInstallation';
    }
  | {
      state: 'licenseRecoveryRequired';
      mutationKind: 'deactivation';
      mutationRequestId: Uuid;
      reconciliationDeadlineAtUtc?: never;
      fenceCreatedAtUtc: TimestampUtc;
      activeCredentialState: 'destroyed';
      seatMayRemainOccupied: true;
      reason: 'notCommittedDeactivationCredentialsDestroyed';
      nextAction: 'contactSupport';
    }
  | { state: 'revoked'; revokedAtUtc: TimestampUtc };

interface LicenseDeactivationGrantView {
  deactivationGrantId: Uuid;
  reason: LicenseDeactivationReason;
  confirmedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
}

interface LicenseCredentialSetRef {
  credentialSetId: Uuid;
  tokenFamilyId: Uuid;
  refreshTokenSha256: Sha256;
  accessTokenExpiresAtUtc: TimestampUtc;
  refreshTokenExpiresAtUtc: TimestampUtc;
  devicePublicKeySpkiSha256: Sha256;
}

interface LicenseReconciliationKeyRef {
  credentialId: Uuid;
  devicePublicKeySpkiSha256: Sha256;
  nonExportable: true;
  allowedPurposes: readonly ['licenseReconciliation'];
  retention: 'untilMutationTerminalOrSignedSupportDecision';
}

interface LicenseDeactivationAuthorizationKeyRef {
  deactivationAuthorizationKeyId: Uuid;
  publicKeySpkiSha256: Sha256;
  provider:
    | 'MicrosoftPassportKeyStorageProvider'
    | 'MicrosoftSoftwareKeyStorageProvider';
  algorithm: 'ECDSA_P256';
  exportPolicy: 'nonExportable';
  uiPolicy:
    | 'windowsHelloUserVerificationRequired'
    | 'NCRYPT_UI_FORCE_HIGH_PROTECTION_FLAG';
  registeredAtActivation: true;
  ordinaryDevicePopUseAllowed: false;
}

interface PendingLicenseDevicePopKeyRef {
  localCredentialId: Uuid;
  publicKeySpki: string;
  publicKeySpkiSha256: Sha256;
  provider:
    | 'MicrosoftPlatformCryptoProvider'
    | 'MicrosoftSoftwareKeyStorageProvider';
  algorithm: 'ECDSA_P256';
  exportPolicy: 'nonExportable';
  uiPolicy: 'silentNoUserVerification';
  allowedPurposes: readonly [
    'licenseActivationProof',
    'licenseValidation',
    'licenseRefresh',
    'licenseReconciliation',
  ];
  registrationState: 'pendingActivation';
}

interface PendingLicenseDeactivationAuthorizationKeyRef {
  localCredentialId: Uuid;
  publicKeySpki: string;
  publicKeySpkiSha256: Sha256;
  provider:
    | 'MicrosoftPassportKeyStorageProvider'
    | 'MicrosoftSoftwareKeyStorageProvider';
  algorithm: 'ECDSA_P256';
  exportPolicy: 'nonExportable';
  uiPolicy:
    | 'windowsHelloUserVerificationRequired'
    | 'NCRYPT_UI_FORCE_HIGH_PROTECTION_FLAG';
  allowedPurposes: readonly ['licenseDeactivationAuthorization'];
  registrationState: 'pendingActivation';
  ordinaryDevicePopUseAllowed: false;
}

interface PendingLicenseActivationRequestMaterial {
  wireSchemaVersion: 1;
  contentType: 'application/json';
  canonicalization: 'RFC8785';
  canonicalRequestCredentialId: Uuid;
  canonicalRequestByteLength: U64String;
  canonicalRequestSha256: Sha256;
  installationId: Uuid;
  activationRequestId: Uuid;
  appVersion: string;
  activationSecretValueSha256: Sha256;
  devicePublicKeySpkiSha256: Sha256;
  deactivationAuthorizationPublicKeySpkiSha256: Sha256;
  activationProofBinding: {
    proofVersion: 1;
    typ: 'qingpan-license-activation-pop+jws';
    domain: 'qingpan.license.activation-pop.v1';
    purpose: 'licenseActivationProof';
    httpMethod: 'POST';
    httpPath: '/api/license/activate';
    signingKeySpkiSha256: Sha256;
  };
}

interface RegisteredLicenseActivationKeyBinding {
  devicePublicKeySpkiSha256: Sha256;
  deactivationAuthorizationKeyId: Uuid;
  deactivationAuthorizationPublicKeySpkiSha256: Sha256;
  registeredAtUtc: TimestampUtc;
}

interface DeactivationAuthorizationEvidence {
  evidenceVersion: 1;
  deactivationGrantId: Uuid;
  deactivationGrantDigestSha256: Sha256;
  reason: LicenseDeactivationReason;
  key: LicenseDeactivationAuthorizationKeyRef;
  challenge: LicenseDeactivationChallengeWireResult;
  signedStatementDomain: 'qingpan.license.deactivation.v2';
  canonicalStatementSha256: Sha256;
  statementSignatureBase64: string;
  userVerification: {
    status: 'verified';
    verifiedAtUtc: TimestampUtc;
    nativeUiReceiptDigestSha256: Sha256;
  };
  evidenceDigestSha256: Sha256;
}

type PendingLicenseActivation = {
  recordVersion: 3;
  recordSequence: U64String;
  sourceActivationDraftId: Uuid;
  installationId: Uuid;
  activationRequestId: Uuid;
  appVersion: string;
  requestMaterial: PendingLicenseActivationRequestMaterial;
  devicePopKey: PendingLicenseDevicePopKeyRef;
  deactivationAuthorizationKey: PendingLicenseDeactivationAuthorizationKeyRef;
  requestBodySha256: Sha256;
  preparedAtUtc: TimestampUtc;
  mutationReplayDeadlineAtUtc: TimestampUtc;
  reconciliationDeadlineAtUtc: TimestampUtc;
} &
  StrictUnion<
    | {
        state: 'prepared';
        activationResult?: never;
        newCredentialSet?: never;
        registeredKeyBinding?: never;
        responsePayloadSha256?: never;
      }
    | {
        state: 'reconciliationRequired';
        reconciliation: LicenseReconciliationState;
        activationResult?: never;
        newCredentialSet?: never;
        registeredKeyBinding?: never;
        responsePayloadSha256?: never;
      }
    | {
        state: 'responseStored';
        activationResult: Omit<LicenseActivationWireResult, 'tokens'>;
        newCredentialSet: LicenseCredentialSetRef;
        registeredKeyBinding: RegisteredLicenseActivationKeyBinding;
        responsePayloadSha256: Sha256;
        responseStoredAtUtc: TimestampUtc;
      }
    | {
        state: 'committed';
        activationResult: Omit<LicenseActivationWireResult, 'tokens'>;
        newCredentialSet: LicenseCredentialSetRef;
        registeredKeyBinding: RegisteredLicenseActivationKeyBinding;
        responsePayloadSha256: Sha256;
        committedAtUtc: TimestampUtc;
      }
  >;

interface LicenseDeactivationGrant {
  recordVersion: 1;
  deactivationGrantId: Uuid;
  reason: LicenseDeactivationReason;
  ownerSidDigest: Sha256;
  logonSidDigest: Sha256;
  sessionId: number;
  issuedToAppInstanceId: Uuid;
  confirmationSummaryDigestSha256: Sha256;
  confirmedAtUtc: TimestampUtc;
  expiresAtUtc: TimestampUtc;
  state: 'active' | 'consumed';
}

type PendingLicenseDeactivation = {
  recordVersion: 4;
  recordSequence: U64String;
  installationId: Uuid;
  deactivationRequestId: Uuid;
  reason: LicenseDeactivationReason;
  issuedAtUtc: TimestampUtc;
  deactivationAuthorization: DeactivationAuthorizationEvidence;
  deactivationGrantDigestSha256: Sha256;
  reconciliationKey: LicenseReconciliationKeyRef;
  preparedAtUtc: TimestampUtc;
  mutationReplayDeadlineAtUtc: TimestampUtc;
  reconciliationDeadlineAtUtc: TimestampUtc;
} &
  (
    | {
        state: 'prepared';
        activeCredentials: { state: 'retained' };
        reconciliationKeyState: { state: 'retained' };
        response?: never;
      }
    | {
        state: 'queued';
        activeCredentials: {
          state: 'destroyed';
          destroyedAtUtc: TimestampUtc;
        };
        reconciliationKeyState: { state: 'retained' };
        queuedAtUtc: TimestampUtc;
        response?: never;
      }
    | {
        state: 'reconciliationRequired';
        activeCredentials:
          | { state: 'retained' }
          | { state: 'destroyed'; destroyedAtUtc: TimestampUtc };
        reconciliationKeyState: { state: 'retained' };
        reconciliation: LicenseReconciliationState;
        response?: never;
      }
    | {
        state: 'responseStored';
        activeCredentials:
          | { state: 'retained' }
          | { state: 'destroyed'; destroyedAtUtc: TimestampUtc };
        reconciliationKeyState: { state: 'retained' };
        response: LicenseDeactivationWireResult;
        responsePayloadSha256: Sha256;
        responseStoredAtUtc: TimestampUtc;
      }
    | {
        state: 'committed';
        activeCredentials: {
          state: 'destroyed';
          destroyedAtUtc: TimestampUtc;
        };
        reconciliationKeyState: {
          state: 'destroyed';
          destroyedAtUtc: TimestampUtc;
        };
        response: LicenseDeactivationWireResult;
        responsePayloadSha256: Sha256;
        committedAtUtc: TimestampUtc;
      }
  );

type PendingLicenseRefresh = {
  recordVersion: 2;
  recordSequence: U64String;
  installationId: Uuid;
  refreshRequestId: Uuid;
  requestBodySha256: Sha256;
  oldCredentialSet: LicenseCredentialSetRef;
  preparedAtUtc: TimestampUtc;
  mutationReplayDeadlineAtUtc: TimestampUtc;
  reconciliationDeadlineAtUtc: TimestampUtc;
} &
  (
    | {
        state: 'prepared';
        newCredentialSet?: never;
        rotationResultId?: never;
        responsePayloadSha256?: never;
      }
    | {
        state: 'reconciliationRequired';
        reconciliation: LicenseReconciliationState;
        newCredentialSet?: never;
        rotationResultId?: never;
        responsePayloadSha256?: never;
      }
    | {
        state: 'responseStored';
        newCredentialSet: LicenseCredentialSetRef;
        rotationResultId: Uuid;
        responsePayloadSha256: Sha256;
        responseStoredAtUtc: TimestampUtc;
      }
    | {
        state: 'committed';
        newCredentialSet: LicenseCredentialSetRef;
        rotationResultId: Uuid;
        responsePayloadSha256: Sha256;
        committedAtUtc: TimestampUtc;
      }
  );

```
