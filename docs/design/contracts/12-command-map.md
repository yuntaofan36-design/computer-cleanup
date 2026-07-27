<a id="qpn-sec-8-3-13"></a>
# 8.3.13 命令请求响应闭合映射

> 所属文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 索引：[第 8.3 节运行时契约索引](../03-runtime-api.md#qpn-sec-8-3)
> 本文件是规范 TypeScript 契约片段；校验时必须按文件名前缀顺序拼接。

```ts
interface CommandSpec<Q, R> {
  request: Q;
  response: R;
}

type CreateLicenseDeactivationGrantCommandSpec = {
  [R in LicenseDeactivationReason]: CommandSpec<
    CreateLicenseDeactivationGrantRequest & { reason: R },
    LicenseDeactivationGrantView & { reason: R }
  >;
}[LicenseDeactivationReason];

type SaveAnalysisPolicyCommandSpec = {
  [S in PersistentAnalysisPolicy['scope']]: CommandSpec<
    {
      rootGrantId: Uuid;
      scope: S;
      exclusionPolicyId?: Uuid;
    },
    PersistentAnalysisPolicy & { scope: S }
  >;
}[PersistentAnalysisPolicy['scope']];

type StartScanCommandSpec = {
  [K in ScanRequest['kind']]: CommandSpec<
    Extract<ScanRequest, { kind: K }>,
    TaskRef<K>
  >;
}[ScanRequest['kind']];

type ScheduledJobForKind<K extends ScheduledJob['workload']['kind']> =
  K extends 'R0Analysis'
    ? ScheduledJobBase & {
        workload: {
          kind: 'R0Analysis';
          analysisPolicyId: Uuid;
          canonicalPolicyDigest: Sha256;
        };
        maximumRisk: 'R0';
        approvalGrantRevision?: never;
        maximumRuns?: never;
        runsStarted?: never;
      }
    : ScheduledJobBase & {
        workload: { kind: 'R1Cleanup'; approvalGrantId: Uuid };
        maximumRisk: 'R1';
        approvalGrantRevision: U64String;
        maximumRuns: number;
        runsStarted: number;
      };

type ScheduledJobUpsertResultFor<
  K extends ScheduledJob['workload']['kind'],
> = { kind: 'upserted'; job: ScheduledJobForKind<K> };

type UpsertScheduledJobCommandSpec =
  | CommandSpec<
      Extract<UpsertScheduledJobRequest, { kind: 'R0Analysis' }> & {
        mutation: 'create';
      },
      ScheduledJobUpsertResultFor<'R0Analysis'>
    >
  | CommandSpec<
      Extract<UpsertScheduledJobRequest, { kind: 'R0Analysis' }> & {
        mutation: 'update';
      },
      ScheduledJobUpsertResultFor<'R0Analysis'>
    >
  | CommandSpec<
      Extract<UpsertScheduledJobRequest, { kind: 'R1Cleanup' }>,
      ScheduledJobUpsertResultFor<'R1Cleanup'>
    >;

type UpdateSettingDomain = UpdateChannelSetting['domain'];
type UpdateChannelSettingForDomain<D extends UpdateSettingDomain> =
  D extends 'rules' ? RuleUpdateChannelSetting : MachineAppUpdatePolicy;
type UpdateChannelSettingFor<
  D extends UpdateSettingDomain,
  C extends UpdateChannel,
> = UpdateChannelSettingForDomain<D> & { channel: C };

type GetUpdateChannelSettingCommandSpec = {
  [D in UpdateSettingDomain]: CommandSpec<
    { domain: D },
    UpdateChannelSettingForDomain<D>
  >;
}[UpdateSettingDomain];

type SetUpdateChannelSettingCommandSpec = {
  [D in UpdateSettingDomain]: {
    [C in UpdateChannel]: CommandSpec<
      D extends 'rules'
        ? { domain: D; channel: C; expectedRevision?: never }
        : { domain: D; channel: C; expectedRevision: U64String },
      UpdateChannelSettingFor<D, C>
    >;
  }[UpdateChannel];
}[UpdateSettingDomain];

interface LicenseActivationDraftView {
  activationDraftId: Uuid;
  expiresAtUtc: TimestampUtc;
}

interface ScanResultsResponse {
  page: AppendCursorPage<ScanResultView>;
  task: TaskView;
}

interface AutomationApprovalView {
  approvalGrantId: Uuid;
  revision: U64String;
  boundScheduledJobId: Uuid;
  maximumRuns: number;
  runsStarted: number;
  expiresAtUtc: TimestampUtc;
  state: ApprovalGrant['state'];
}

interface RuleStatusView {
  channelSettingId: Uuid;
  activePackageHash?: Sha256;
  candidatePackageHash?: Sha256;
  state: 'unavailable' | 'active' | 'updatePending' | 'failedClosed';
  lastErrorCode?: ErrorCode;
}

interface RuleUpdateResult {
  status: RuleStatusView;
  activated: boolean;
}

interface UpdateRecoveryAssessmentBase<S extends InstallRecoverySource> {
  source: S;
  sourceJournalSequence: U64String;
  observedStateDigestSha256: Sha256;
}

type UpdateRecoveryResolution = StrictUnion<
  | { kind: 'exactTarget'; sideEffectRequired: false }
  | { kind: 'exactLkg'; sideEffectRequired: false }
  | {
      kind: 'signedRecoveryRequired';
      sideEffectRequired: true;
      recoveryAction: 'installRecoveryTarget';
    }
  | {
      kind: 'uninstalledPreserved';
      sideEffectRequired: false;
      absenceEvidence: VerifiedProductAbsenceEvidence;
      anchorLifecycleRevisionAfterCommit: U64String;
    }
  | {
      kind: 'signedRecoveryRequired';
      sideEffectRequired: true;
      recoveryAction: 'completeProductUninstall';
      expectedProductIdentity: ExpectedProductUninstallIdentity;
    }
>;

type InstallTargetRecoveryResolution =
  | Extract<UpdateRecoveryResolution, { kind: 'exactTarget' }>
  | Extract<UpdateRecoveryResolution, { kind: 'exactLkg' }>
  | Extract<
      UpdateRecoveryResolution,
      { recoveryAction: 'installRecoveryTarget' }
    >;

type ProductUninstallRecoveryResolution =
  | Extract<UpdateRecoveryResolution, { kind: 'uninstalledPreserved' }>
  | Extract<
      UpdateRecoveryResolution,
      { recoveryAction: 'completeProductUninstall' }
    >;

type UpdateRecoveryAssessment = StrictUnion<
  | (UpdateRecoveryAssessmentBase<InstallTargetRecoverySource> & {
      resolution: InstallTargetRecoveryResolution;
    })
  | (UpdateRecoveryAssessmentBase<ProductUninstallRecoverySource> & {
      resolution: ProductUninstallRecoveryResolution;
    })
>;

interface DiagnosticBundleExportResult {
  localExportId: Uuid;
  createdAtUtc: TimestampUtc;
  bundleSha256: Sha256;
}

interface CommandContractMap {
  create_license_activation_draft: CommandSpec<
    Record<string, never>,
    LicenseActivationDraftView
  >;
  activate_license: CommandSpec<ActivateLicenseRequest, LicenseStatusView>;
  get_license_status: CommandSpec<Record<string, never>, LicenseStatusView>;
  validate_license: CommandSpec<Record<string, never>, LicenseStatusView>;
  refresh_license: CommandSpec<RefreshLicenseRequest, LicenseStatusView>;
  create_license_deactivation_grant: CreateLicenseDeactivationGrantCommandSpec;
  deactivate_license: CommandSpec<DeactivateLicenseRequest, LicenseStatusView>;

  grant_analysis_root:
    | CommandSpec<
        { analysisKind: 'storageUsage' },
        Extract<
          RootGrant,
          {
            kind: 'userSelectedAnalysis';
            allowedScopes: readonly ['storageUsage'];
          }
        >
      >
    | CommandSpec<
        { analysisKind: 'largeFiles' },
        Extract<
          RootGrant,
          {
            kind: 'userSelectedAnalysis';
            allowedScopes: readonly ['largeFiles'];
          }
        >
      >
    | CommandSpec<
        { analysisKind: 'duplicates' },
        Extract<
          RootGrant,
          {
            kind: 'userSelectedAnalysis';
            allowedScopes: readonly ['duplicates'];
          }
        >
      >;
  save_analysis_policy: SaveAnalysisPolicyCommandSpec;
  revoke_analysis_policy: CommandSpec<
    { analysisPolicyId: Uuid },
    PersistentAnalysisPolicy
  >;
  grant_exclusion_path: CommandSpec<
    { appliesTo: NonEmptyArray<ExclusionEntry['appliesTo'][number]> },
    ExclusionEntry
  >;
  upsert_exclusion_policy: CommandSpec<
    { exclusionPolicyId?: Uuid; entryIds: NonEmptyArray<Uuid> },
    ExclusionPolicy
  >;
  delete_exclusion_policy: CommandSpec<
    { exclusionPolicyId: Uuid },
    ExclusionPolicy
  >;
  create_r1_automation_policy: CommandSpec<
    {
      ruleIds: NonEmptyArray<string>;
      maximumFiles: number;
      maximumBytes: U64String;
    },
    R1AutomationPolicy
  >;
  delete_r1_automation_policy: CommandSpec<
    { automationPolicyId: Uuid },
    R1AutomationPolicy
  >;

  start_scan: StartScanCommandSpec;
  get_scan_results: CommandSpec<
    ScopedPageRequest<{ taskId: Uuid }, ScanResultFilter>,
    ScanResultsResponse
  >;
  get_task: CommandSpec<{ taskId: Uuid }, TaskView>;
  cancel_task: CommandSpec<{ taskId: Uuid }, TaskView>;
  create_plan: CommandSpec<
    {
      taskId: Uuid;
      selections: NonEmptyArray<{
        candidateId: Uuid;
        action:
          | 'deleteRebuildableCache'
          | 'quarantine'
          | 'permanentDeleteOriginal';
      }>;
    },
    PlanView
  >;
  create_uninstall_plan: CommandSpec<{ appSnapshotId: Uuid }, PlanView>;
  get_plan: CommandSpec<{ planId: Uuid }, PlanView>;
  get_plan_items: CommandSpec<
    ScopedPageRequest<{ planId: Uuid }>,
    CursorPage<PlanItemView>
  >;
  authorize_plan: CommandSpec<{ planId: Uuid }, PlanView>;
  execute_plan: CommandSpec<
    { planId: Uuid },
    Extract<OperationRef, { kind: 'planExecution' }>
  >;
  get_operation: CommandSpec<{ operationId: Uuid }, OperationView>;
  get_operation_items: CommandSpec<
    ScopedPageRequest<{ operationId: Uuid }>,
    AppendCursorPage<OperationItemResult>
  >;
  cancel_operation: CommandSpec<{ operationId: Uuid }, OperationView>;

  list_quarantine: CommandSpec<
    PageRequest<QuarantineFilter>,
    CursorPage<QuarantineRecordView>
  >;
  grant_restore_target:
    | CommandSpec<
        { purpose: 'restore' },
        Extract<RootGrant, { kind: 'restoreTarget' }>
      >
    | CommandSpec<
        { purpose: 'salvageExport' },
        Extract<RootGrant, { kind: 'salvageExportTarget' }>
      >;
  start_restore: CommandSpec<
    StartRestoreRequest,
    Extract<OperationRef, { kind: 'restore' }>
  >;
  start_quarantine_salvage_export: CommandSpec<
    StartQuarantineSalvageExportRequest,
    Extract<OperationRef, { kind: 'quarantineSalvage' }>
  >;
  create_quarantine_purge_plan: CommandSpec<
    CreateQuarantinePurgePlanRequest,
    PlanView
  >;

  create_automation_approval: CommandSpec<
    CreateAutomationApprovalRequest,
    AutomationApprovalView
  >;
  get_automation_approval: CommandSpec<
    { approvalGrantId: Uuid },
    AutomationApprovalView
  >;
  revoke_automation_approval: CommandSpec<
    { approvalGrantId: Uuid },
    AutomationApprovalView
  >;
  list_scheduled_jobs: CommandSpec<
    PageRequest<ScheduledJobFilter>,
    CursorPage<ScheduledJob>
  >;
  upsert_scheduled_job: UpsertScheduledJobCommandSpec;
  delete_scheduled_job: CommandSpec<
    DeleteScheduledJobRequest,
    Extract<ScheduledJobMutationResult, { kind: 'deleted' }>
  >;

  get_update_channel_setting: GetUpdateChannelSettingCommandSpec;
  set_update_channel_setting: SetUpdateChannelSettingCommandSpec;
  get_rule_status: CommandSpec<{ channelSettingId: Uuid }, RuleStatusView>;
  request_rule_update: CommandSpec<
    { channelSettingId: Uuid },
    RuleUpdateResult
  >;
  check_app_update: CommandSpec<
    { machinePolicyId: Uuid },
    AppUpdateCheckResult
  >;
  stage_app_update: CommandSpec<
    { updateId: Uuid; expectedJournalSequence: U64String },
    AppUpdateJournalView
  >;
  apply_staged_update: CommandSpec<
    { updateId: Uuid; expectedJournalSequence: U64String },
    AppUpdateJournalView
  >;
  cancel_app_update: CommandSpec<
    CancelAppUpdateRequest,
    AppUpdateJournalView
  >;
  get_app_update: CommandSpec<{ updateId: Uuid }, AppUpdateJournalView>;
  grant_app_update_recovery_package_file: CommandSpec<
    { source: InstallRecoverySource },
    AppUpdateRecoveryPackageFileGrant
  >;
  reconcile_update_recovery: CommandSpec<
    { source: InstallRecoverySource },
    UpdateRecoveryAssessment
  >;
  apply_signed_recovery_package: CommandSpec<
    { installerRecoveryPackageFileGrantId: Uuid },
    RecoveryResolutionRecord
  >;

  list_apps: CommandSpec<PageRequest<AppFilter>, CursorPage<AppView>>;
  get_uninstall_operation: CommandSpec<{ operationId: Uuid }, UninstallResult>;
  list_startup_entries: CommandSpec<
    PageRequest<StartupEntryFilter>,
    CursorPage<StartupEntryView>
  >;
  list_disks: CommandSpec<Record<string, never>, DiskView[]>;
  list_partition_disks: CommandSpec<
    Record<string, never>,
    PartitionDiskView[]
  >;
  open_windows_disk_management: CommandSpec<
    Record<string, never>,
    OpenDiskManagementResult
  >;
  list_operation_records: CommandSpec<
    PageRequest<AuditFilter>,
    CursorPage<OperationRecordView>
  >;
  grant_diagnostic_export_target: CommandSpec<
    Record<string, never>,
    DiagnosticExportTargetGrant
  >;
  export_diagnostic_bundle: CommandSpec<
    { diagnosticExportTargetGrantId: Uuid },
    DiagnosticBundleExportResult
  >;
}

type CommandName = keyof CommandContractMap;
type CommandExchange<C extends CommandName> = CommandContractMap[C];
type CommandRequest<C extends CommandName> = CommandContractMap[C]['request'];
type CommandResponse<C extends CommandName> = CommandContractMap[C]['response'];
type ResponseFromCommandExchange<E, Q> =
  E extends CommandSpec<infer Req, infer Res>
    ? Q extends Req
      ? Res
      : never
    : never;
type CommandResponseFor<
  C extends CommandName,
  Q extends CommandRequest<C>,
> = ResponseFromCommandExchange<CommandExchange<C>, Q>;

```
