<a id="qpn-sec-13-6-1"></a>
# 13.6.1 平台登记与发布测试义务

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 13.6 节发布与追踪契约索引](../05-test-release.md#qpn-sec-13-6)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
const PLATFORM_BUILDS = {
  win10_22h2_19045: { windowsVersion: '10 22H2', windowsBuild: 19045 },
  win11_24h2_26100: { windowsVersion: '11 24H2', windowsBuild: 26100 },
  win11_25h2_26200: { windowsVersion: '11 25H2', windowsBuild: 26200 },
} as const;

const PLATFORM_ARCHITECTURES = ['x64', 'arm64'] as const;

const DATA_VOLUME_PROFILES = {
  ntfsSsd: { fileSystem: 'NTFS', mediaType: 'ssd', maximumRisk: 'R4' },
  ntfsHdd: { fileSystem: 'NTFS', mediaType: 'hdd', maximumRisk: 'R4' },
  refsSsdReadOnly: {
    fileSystem: 'ReFS',
    mediaType: 'ssd',
    maximumRisk: 'R0',
  },
} as const;

const INITIATING_PRIVILEGE_PROFILES = {
  standardUser: {
    privilegeContext: 'standardUser',
    elevationPolicy: 'rejectOverTheShoulder',
  },
  splitTokenAdminMedium: {
    privilegeContext: 'splitTokenAdminMedium',
    elevationPolicy: 'linkedTokenSameUserOnly',
  },
} as const;

const PLATFORM_COVERAGE_PROFILES = {
  all36: {
    dataVolumeProfileIds: ['ntfsSsd', 'ntfsHdd', 'refsSsdReadOnly'],
    expectedTupleCount: 36,
  },
  ntfsWrite24: {
    dataVolumeProfileIds: ['ntfsSsd', 'ntfsHdd'],
    expectedTupleCount: 24,
  },
  refsReadOnly12: {
    dataVolumeProfileIds: ['refsSsdReadOnly'],
    expectedTupleCount: 12,
  },
} as const;

type PlatformBuildId = keyof typeof PLATFORM_BUILDS;
type PlatformArchitecture = (typeof PLATFORM_ARCHITECTURES)[number];
type DataVolumeProfileId = keyof typeof DATA_VOLUME_PROFILES;
type InitiatingPrivilegeProfileId =
  keyof typeof INITIATING_PRIVILEGE_PROFILES;
type PlatformCoverageProfileId = keyof typeof PLATFORM_COVERAGE_PROFILES;

type MandatoryTestCoverageBinding = StrictUnion<
  | {
      kind: 'hostIndependent';
    }
  | {
      kind: 'registryProfile';
      profileId: PlatformCoverageProfileId;
    }
>;

const RELEASE_TEST_OBLIGATIONS = [
  {
    capabilityId: 'formalContracts',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-API-003', 'T-API-004', 'T-CONTRACT-001', 'T-CONTRACT-002',
      'T-CONTRACT-003', 'T-CONTRACT-004', 'T-CONTRACT-005',
      'T-CONTRACT-006', 'T-CONTRACT-007', 'T-CONTRACT-008',
      'T-CONTRACT-009',
    ],
  },
  {
    capabilityId: 'applicationUninstall',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-APP-001', 'T-APP-002', 'T-APP-003', 'T-APP-004', 'T-APP-005',
      'T-APP-006', 'T-APP-007', 'T-APP-008', 'T-APP-009', 'T-APP-010',
      'T-APP-011', 'T-APP-012', 'T-APP-013', 'T-APP-018', 'T-APP-019',
    ],
  },
  {
    capabilityId: 'genericWin32Uninstall',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: ['T-APP-014', 'T-APP-015', 'T-APP-016', 'T-APP-017'],
  },
  {
    capabilityId: 'auditNetworkLicensing',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-AUDIT-001', 'T-AUDIT-002', 'T-AUDIT-003', 'T-AUDIT-004',
      'T-LICENSE-001', 'T-LICENSE-002', 'T-LICENSE-003', 'T-LICENSE-004',
      'T-NET-001', 'T-NET-002', 'T-NET-003', 'T-NET-004', 'T-NET-005',
      'T-NET-006', 'T-NET-007', 'T-NET-008', 'T-NET-009', 'T-NET-010',
    ],
  },
  {
    capabilityId: 'scheduledTasks',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-AUTO-001', 'T-AUTO-002', 'T-AUTO-003', 'T-AUTO-004',
      'T-AUTO-005', 'T-AUTO-006', 'T-AUTO-007', 'T-AUTO-008',
      'T-AUTO-009', 'T-AUTO-010', 'T-AUTO-011', 'T-AUTO-012',
      'T-AUTO-013', 'T-AUTO-014', 'T-AUTO-015', 'T-IDEMP-002',
    ],
  },
  {
    capabilityId: 'quarantineExportPurge',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-BATCH-001', 'T-BATCH-PURGE-001', 'T-BATCH-RESTORE-001',
      'T-QUAR-001', 'T-QUAR-002', 'T-QUAR-003', 'T-QUAR-004',
      'T-QUAR-005', 'T-QUAR-006', 'T-QUAR-007', 'T-QUAR-008',
      'T-QUAR-009', 'T-QUAR-010', 'T-QUAR-011', 'T-QUAR-012',
      'T-QUAR-013', 'T-QUAR-014', 'T-QUAR-015', 'T-QUAR-016',
      'T-QUAR-017', 'T-QUAR-018', 'T-QUAR-019', 'T-QUAR-020',
      'T-QUAR-022', 'T-QUAR-023', 'T-QUAR-024',
      'T-QUAR-025', 'T-QUAR-026', 'T-QUOTA-001', 'T-QUOTA-002',
      'T-R4-PURGE-001', 'T-R4-PURGE-002', 'T-R4-PURGE-003',
      'T-R4-PURGE-004', 'T-R4-PURGE-005', 'T-RESTORE-IDEMP-001',
      'T-RESTORE-IDEMP-002',
    ],
  },
  {
    capabilityId: 'quarantineExportPurge',
    platformCoverage: { kind: 'registryProfile', profileId: 'all36' },
    testIds: ['T-QUAR-021'],
  },
  {
    capabilityId: 'rebuildableCacheCleanup',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-CACHE-001', 'T-CACHE-002', 'T-CACHE-003', 'T-CACHE-004',
      'T-CACHE-005', 'T-CACHE-006',
    ],
  },
  {
    capabilityId: 'uiPlatformRelease',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: ['T-DOC-001', 'T-PLAT-001', 'T-PLAT-004', 'T-RELEASE-001'],
  },
  {
    capabilityId: 'uiPlatformRelease',
    platformCoverage: { kind: 'registryProfile', profileId: 'all36' },
    testIds: [
      'T-PLAT-002', 'T-PLAT-003', 'T-UI-001', 'T-UI-002', 'T-UI-003',
      'T-UI-004', 'T-UI-005', 'T-WIN-001', 'T-WIN-002', 'T-WIN-003',
      'T-WIN-004', 'T-WIN-005', 'T-WIN-006', 'T-WIN-007', 'T-WIN-008',
    ],
  },
  {
    capabilityId: 'storageLargeDuplicateAnalysis',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-DUP-001', 'T-DUP-002', 'T-DUP-003', 'T-DUP-004', 'T-DUP-005',
      'T-EXCL-001', 'T-EXCL-002', 'T-EXCL-003', 'T-EXCL-004',
      'T-EXCL-005', 'T-EXCL-006', 'T-LARGE-001', 'T-LARGE-002',
      'T-LARGE-003', 'T-LARGE-004', 'T-LARGE-005', 'T-STORAGE-001',
      'T-STORAGE-002', 'T-STORAGE-003', 'T-STORAGE-004',
    ],
  },
  {
    capabilityId: 'storageLargeDuplicateAnalysis',
    platformCoverage: { kind: 'registryProfile', profileId: 'all36' },
    testIds: ['T-SCAN-001', 'T-SCAN-002'],
  },
  {
    capabilityId: 'planFileIdentity',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-FMUT-001', 'T-FMUT-002', 'T-FS-001', 'T-FS-002', 'T-FS-003',
      'T-FS-004', 'T-FS-005', 'T-FS-006', 'T-FS-007', 'T-FS-008',
      'T-FS-009', 'T-IDEMP-001', 'T-PAGE-001', 'T-PAGE-002',
      'T-PLAN-001', 'T-PLAN-002', 'T-RESULT-001', 'T-STATE-001',
    ],
  },
  {
    capabilityId: 'planFileIdentity',
    platformCoverage: { kind: 'registryProfile', profileId: 'all36' },
    testIds: ['T-FMUT-003'],
  },
  {
    capabilityId: 'elevationBoundary',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-IPC-001', 'T-IPC-002', 'T-IPC-003', 'T-IPC-004', 'T-IPC-005',
      'T-IPC-006', 'T-IPC-007', 'T-IPC-008', 'T-IPC-009', 'T-IPC-010',
    ],
  },
  {
    capabilityId: 'startupPartitionAnalysis',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-PART-001', 'T-PART-002', 'T-STARTUP-001', 'T-STARTUP-002',
      'T-STARTUP-003',
    ],
  },
  {
    capabilityId: 'signedRuleSupplyChain',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-RULE-001', 'T-RULE-002', 'T-RULE-003', 'T-RULE-004',
      'T-RULE-005', 'T-RULE-006', 'T-RULE-007', 'T-RULE-008',
      'T-RULE-009', 'T-RULE-010', 'T-RULE-011', 'T-RULE-012',
      'T-RULE-013', 'T-RULE-014', 'T-RULE-015', 'T-RULE-016',
      'T-RULE-017', 'T-RULE-018', 'T-RULE-019', 'T-RULE-020',
    ],
  },
  {
    capabilityId: 'signedApplicationUpdate',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-UPDATE-001', 'T-UPDATE-002', 'T-UPDATE-003', 'T-UPDATE-004',
      'T-UPDATE-005', 'T-UPDATE-006', 'T-UPDATE-007', 'T-UPDATE-008',
      'T-UPDATE-009', 'T-UPDATE-010', 'T-UPDATE-011', 'T-UPDATE-012',
      'T-UPDATE-013', 'T-UPDATE-014', 'T-UPDATE-015', 'T-UPDATE-016',
      'T-UPDATE-017', 'T-UPDATE-018', 'T-UPDATE-019', 'T-UPDATE-020',
      'T-UPDATE-021', 'T-UPDATE-022', 'T-UPDATE-023', 'T-UPDATE-024',
      'T-UPDATE-025', 'T-UPDATE-026', 'T-UPDATE-027', 'T-UPDATE-028',
      'T-UPDATE-029', 'T-UPDATE-030', 'T-UPDATE-031', 'T-UPDATE-032',
      'T-UPDATE-033', 'T-UPDATE-034', 'T-UPDATE-035', 'T-UPDATE-036',
      'T-UPDATE-037', 'T-UPDATE-038', 'T-UPDATE-039', 'T-UPDATE-041',
      'T-UPDATE-042',
    ],
  },
  {
    capabilityId: 'signedApplicationUpdate',
    platformCoverage: { kind: 'registryProfile', profileId: 'all36' },
    testIds: ['T-UPDATE-040'],
  },
  {
    capabilityId: 'onlineLicenseService',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: ['T-SVC-001'],
  },
  {
    capabilityId: 'onlineRuleService',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: ['T-SVC-002'],
  },
  {
    capabilityId: 'onlineUpdateService',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: ['T-SVC-003'],
  },
  {
    capabilityId: 'permanentOriginalFileDelete',
    platformCoverage: { kind: 'hostIndependent' },
    testIds: [
      'T-R4-DELETE-001', 'T-R4-DELETE-002', 'T-R4-DELETE-003',
      'T-R4-DELETE-004',
    ],
  },
] as const satisfies readonly {
  capabilityId: ReleaseCapabilityId;
  platformCoverage: MandatoryTestCoverageBinding;
  testIds: readonly string[];
}[];

type ReleaseTestObligationGroup = (typeof RELEASE_TEST_OBLIGATIONS)[number];
type ReleaseTestId = ReleaseTestObligationGroup['testIds'][number];
type ReleaseTestObligationGroupFor<
  I extends ReleaseTestId,
  G extends ReleaseTestObligationGroup = ReleaseTestObligationGroup,
> = G extends G ? (I extends G['testIds'][number] ? G : never) : never;

type PlatformTupleId =
  `${PlatformBuildId}__${PlatformArchitecture}__${DataVolumeProfileId}__${InitiatingPrivilegeProfileId}`;

type PlatformTupleFor<I extends PlatformTupleId> =
  I extends `${infer B extends PlatformBuildId}__${infer A extends PlatformArchitecture}__${infer D extends DataVolumeProfileId}__${infer P extends InitiatingPrivilegeProfileId}`
    ? {
        platformTupleId: I;
        platformBuildId: B;
        windowsBuild: (typeof PLATFORM_BUILDS)[B]['windowsBuild'];
        architecture: A;
        dataVolumeProfileId: D;
        dataFileSystem: (typeof DATA_VOLUME_PROFILES)[D]['fileSystem'];
        dataMediaType: (typeof DATA_VOLUME_PROFILES)[D]['mediaType'];
        maximumRisk: (typeof DATA_VOLUME_PROFILES)[D]['maximumRisk'];
        initiatingPrivilegeProfileId: P;
        privilegeContext: (typeof INITIATING_PRIVILEGE_PROFILES)[P]['privilegeContext'];
        elevationPolicy: (typeof INITIATING_PRIVILEGE_PROFILES)[P]['elevationPolicy'];
      }
    : never;

type PlatformTupleMap = {
  readonly [I in PlatformTupleId]: PlatformTupleFor<I>;
};

interface PlatformTupleRegistryCanonicalPayload {
  schemaVersion: 3;
  registryId: 'QPN-PLATFORM-TUPLES-V3-001';
  canonicalization: 'RFC8785';
  frozenUbrByBuild: Readonly<Record<PlatformBuildId, number>>;
  coverageProfiles: typeof PLATFORM_COVERAGE_PROFILES;
  releaseTestObligations: typeof RELEASE_TEST_OBLIGATIONS;
  releaseTestObligationsDigestSha256: Sha256;
  tuples: PlatformTupleMap;
}

interface PlatformTupleRegistry extends PlatformTupleRegistryCanonicalPayload {
  registryDigestSha256: Sha256;
}

type TestPlatformCoverageFor<I extends ReleaseTestId> =
  ReleaseTestObligationGroupFor<I>['platformCoverage'] extends {
    kind: 'hostIndependent';
  }
    ? { kind: 'hostIndependent' }
    : ReleaseTestObligationGroupFor<I>['platformCoverage'] extends {
          kind: 'registryProfile';
          profileId: infer P extends PlatformCoverageProfileId;
        }
      ? {
          kind: 'registryProfile';
          profileId: P;
          platformTupleRegistryDigestSha256: Sha256;
        }
      : never;

type RunPlatformBinding = StrictUnion<
  | {
      kind: 'hostIndependent';
    }
  | {
      kind: 'registryTuple';
      platformTupleId: PlatformTupleId;
      platformTupleRegistryDigestSha256: Sha256;
    }
>;

```
