<a id="qpn-sec-8-3-12"></a>
# 8.3.12 出站策略与在线服务交付

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface OutboundRequestPolicy {
  policyVersion: 1;
  service: 'license' | 'ruleUpdate' | 'appUpdate';
  allowedRequests: Array<{
    originId: string;
    resolvedOriginFromSignedBuildConfig: true;
    method: 'GET' | 'POST';
    pathTemplate: string;
    allowedHeaderNames: string[];
    fixedHeaderValues: Record<string, string>;
    allowedQueryNames: string[];
    allowedBodyFields: string[];
    requestContentType?: 'application/json';
    responseContentTypes: string[];
    redirect: 'deny' | 'revalidateEachHop';
  }>;
  requireTls: true;
  forbiddenDataClasses: Array<
    'fileContent' | 'fileName' | 'fullPath' | 'fileHash' | 'scanResult' | 'auditLog'
  >;
}

type OnlineServiceDesignId =
  | 'QPN-SVC-LIC-001'
  | 'QPN-SVC-RULE-001'
  | 'QPN-SVC-UPDATE-001';

type OnlineServiceMilestoneId = 'M1-SVC-01' | 'M1-SVC-02' | 'M1-SVC-03';

interface ServiceDeliveryRecordBase<D extends OnlineServiceDesignId> {
  schemaVersion: 1;
  accountableOwner: string;
  designDoc: {
    id: D;
    version: string;
    relativePath: string;
    sha256: Sha256;
    approvalEvidenceDigestSha256: Sha256;
  };
  source: { repositoryId: string; commit: string };
  artifact: {
    sha256: Sha256;
    sbomSha256: Sha256;
    provenanceSha256: Sha256;
  };
  contract: {
    schemaVersion: number;
    schemaDigestSha256: Sha256;
    endpointSetDigestSha256: Sha256;
  };
  deployment: {
    environment: 'staging' | 'production';
    revision: string;
    originId: string;
    signedOriginConfigDigestSha256: Sha256;
  };
  persistence: {
    migrationId: string;
    idempotencyModelDigestSha256: Sha256;
    retentionPolicyDigestSha256: Sha256;
  };
  trust: {
    keySetDigestSha256: Sha256;
    ceremonyEvidenceIds: NonEmptyArray<string>;
  };
  privacy: {
    acceptedLogSchemaDigestSha256: Sha256;
    headerAndBodyStrippingEvidenceDigestSha256: Sha256;
    retentionDeletionEvidenceDigestSha256: Sha256;
  };
  operations: {
    rollbackRunbookDigestSha256: Sha256;
    incidentRunbookDigestSha256: Sha256;
  };
  tests: {
    contractRunIds: NonEmptyArray<Uuid>;
    negativeRunIds: NonEmptyArray<Uuid>;
    packetRunIds: NonEmptyArray<Uuid>;
    e2eRunIds: NonEmptyArray<Uuid>;
  };
  readiness: 'passed' | 'failed' | 'blocked';
  assessedAtUtc: TimestampUtc;
}

type ServiceDeliveryRecord = StrictUnion<
    | (ServiceDeliveryRecordBase<'QPN-SVC-LIC-001'> & {
        serviceMilestoneId: 'M1-SVC-01';
        serviceDesignId: 'QPN-SVC-LIC-001';
        domain: 'license';
      })
    | (ServiceDeliveryRecordBase<'QPN-SVC-RULE-001'> & {
        serviceMilestoneId: 'M1-SVC-02';
        serviceDesignId: 'QPN-SVC-RULE-001';
        domain: 'ruleUpdate';
      })
    | (ServiceDeliveryRecordBase<'QPN-SVC-UPDATE-001'> & {
        serviceMilestoneId: 'M1-SVC-03';
        serviceDesignId: 'QPN-SVC-UPDATE-001';
        domain: 'appUpdate';
      })
>;

```
