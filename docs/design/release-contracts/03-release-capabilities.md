<a id="qpn-sec-13-6-4"></a>
# 13.6.4 发布能力策略与清单

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
const RELEASE_CAPABILITY_POLICY = {
  planFileIdentity: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  signedRuleSupplyChain: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  rebuildableCacheCleanup: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  quarantineExportPurge: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  storageLargeDuplicateAnalysis: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  applicationUninstall: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  startupPartitionAnalysis: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  elevationBoundary: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'notApplicable',
    enabledBackendEnforcement: 'enabled',
  },
  auditNetworkLicensing: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  scheduledTasks: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  signedApplicationUpdate: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  onlineLicenseService: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'notApplicable',
    enabledBackendEnforcement: 'enabled',
  },
  onlineRuleService: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'notApplicable',
    enabledBackendEnforcement: 'enabled',
  },
  onlineUpdateService: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'notApplicable',
    enabledBackendEnforcement: 'enabled',
  },
  formalContracts: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'notApplicable',
    enabledBackendEnforcement: 'enabled',
  },
  uiPlatformRelease: {
    deliveryClass: 'm1Required',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  genericWin32Uninstall: {
    deliveryClass: 'conditional',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
  permanentOriginalFileDelete: {
    deliveryClass: 'conditional',
    enabledUiExposure: 'visible',
    enabledBackendEnforcement: 'enabled',
  },
} as const;

type ReleaseCapabilityId = keyof typeof RELEASE_CAPABILITY_POLICY;

type EnabledReleaseCapabilityRecord<I extends ReleaseCapabilityId> = {
  deliveryClass: (typeof RELEASE_CAPABILITY_POLICY)[I]['deliveryClass'];
  state: 'enabled';
  uiExposure: (typeof RELEASE_CAPABILITY_POLICY)[I]['enabledUiExposure'];
  backendEnforcement:
    (typeof RELEASE_CAPABILITY_POLICY)[I]['enabledBackendEnforcement'];
  configurationDigestSha256: Sha256;
};

type DisabledConditionalReleaseCapabilityRecord = {
  deliveryClass: 'conditional';
  state: 'disabledByReleasePolicy';
  uiExposure: 'absent';
  backendEnforcement: 'failClosedDisabled';
  configurationDigestSha256: Sha256;
};

type ReleaseCapabilityRecordFor<I extends ReleaseCapabilityId> =
  (typeof RELEASE_CAPABILITY_POLICY)[I]['deliveryClass'] extends 'm1Required'
    ? EnabledReleaseCapabilityRecord<I>
    : StrictUnion<
        | EnabledReleaseCapabilityRecord<I>
        | DisabledConditionalReleaseCapabilityRecord
      >;

type ReleaseCapabilityMap = {
  readonly [I in ReleaseCapabilityId]: ReleaseCapabilityRecordFor<I>;
};

interface ReleaseCapabilityManifestCanonicalPayload {
  schemaVersion: 2;
  manifestId: 'QPN-RELEASE-CAPABILITIES-V2-001';
  releaseId: Uuid;
  canonicalization: 'RFC8785';
  sourceCommit: string;
  buildId: string;
  capabilityPolicy: typeof RELEASE_CAPABILITY_POLICY;
  capabilityPolicyDigestSha256: Sha256;
  capabilities: ReleaseCapabilityMap;
}

interface ReleaseCapabilityManifest
  extends ReleaseCapabilityManifestCanonicalPayload {
  manifestDigestSha256: Sha256;
}

```
