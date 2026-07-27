# 清盘设计文档：运行时 API 与数据契约

> 文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 规范章节：第 8 章
> 状态与版本继承自主索引；本文件不单独构成批准对象。

[上一篇：安全与架构](02-safety-architecture.md#qpn-sec-5) · [主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>) · [下一篇：需求与规则](04-requirements.md#qpn-sec-9)

---

<a id="qpn-sec-8"></a>
## 8. 接口与数据契约

本节同时记录当前运行时 API 与目标文档契约。2026-07-24 首批代码切片新增带标识的扫描会话、不可变清理计划和一次性执行命令；2026-07-25 第二批切片新增明确带 `preview` 后缀的实验性隔离列表和副本导出。其余目标契约仍不代表已实现。代码迁移必须保持前后端类型同步，并为 schema 变化提供版本字段。

<a id="qpn-sec-8-1"></a>
### 8.1 当前 Tauri 命令

| 领域 | 当前命令 | 状态 |
|---|---|---|
| 磁盘与分区 | `list_disks`、`list_partition_disks`、`open_windows_disk_management` | 可用，分区只读 |
| 缓存清理 | `scan_cleanup_v2`、`create_cleanup_plan`、`execute_cleanup_plan` | 当前工作树已实现 `scanId → planId → one-shot execute`；待正式证据归档 |
| 空间分析 | `analyze_storage`、`scan_large_files`、`scan_duplicate_files`、`cancel_task` | 可用 |
| 大文件执行 | `delete_large_files` | 可用，当前为永久删除 |
| 应用 | `list_apps`、`get_app_icon`、`uninstall_app` | 可用 |
| 启动项 | `list_startup_entries`、`set_startup_enabled` | 读取部分可用，写入未实现 |
| 审计 | `list_operation_records` | 可用 |
| 实验性隔离 | `list_quarantine_preview`、`export_quarantine_copy_preview` | 仅 `temp`、同卷、固定导出根；不是第 8.2/8.3.9 节正式 QPC1 命令，限制见[实现说明](implementation/2026-07-25-quarantine-preview.md) |

#### 8.1.1 当前写命令的构建与发布门禁

“当前工作树可调用”只表示代码路径存在，不等于能力已达到发布条件。下表中的 internal 指非 debug 构建显式启用 Cargo feature `internal-write-preview`；该 feature 仅供内部故障注入和流程验证，不得用于生产签名、分发或发布证明。默认 Release 不启用该 feature，危险能力由 Rust 后端 fail-closed，不能依赖 UI 隐藏。

| 当前命令 | 工作树代码路径 | Debug / internal | 默认 Release | 后续生产开放条件 |
|---|---|---|---|---|
| `execute_cleanup_plan` | 一次性消费不可变计划；当前含 `DeleteMode::Permanent` 和实验性 `DeleteMode::Quarantine` 分派 | 可执行，用于 R1 删除和 `temp` 隔离 preview 验证 | 两种写分派均在进入扫描删除器或隔离暂存器前由后端拒绝；计划中的文件不处理 | R1 需通过 M1-01~03 门禁；R2 必须完成 M1-04 正式 QPC1、恢复和独立发布门禁，preview 不得直接转为生产能力 |
| `delete_large_files` | 按最近一次大文件扫描快照复检并请求原文件永久删除 | 明确确认后可用于内部验证 | 在读取或消费大文件快照前硬拒绝 | 原文件 R4 永久删除须单独通过 GATE-008，并由条件发布能力显式开放；不得以 M1-05 分析能力代替 |
| `uninstall_app` | 从最近一次应用枚举快照取得已注册卸载命令并启动旧式 Win32 卸载器 | 明确确认后可用于内部验证 | 在读取应用快照或启动外部进程前硬拒绝 | 仅经 M1-06/M1-07 验证的 MSI/AppX 适配器可逐项开放；旧式 Win32 路径仍是独立条件能力 |
| `list_quarantine_preview` | 读取现有实验性隔离库存 | 可调用 | 只读调用保留；不能据此创建新的 preview 记录 | 正式发布界面必须迁移到第 8.2 节 QPC1 列表契约，preview 记录不得冒充 recordVersion 5 |
| `export_quarantine_copy_preview` | 将已校验 preview 对象复制到后端固定的新目录，不覆盖目标且保留隔离对象 | 可调用 | 保留对象有效且 state=committed/sourceRetained 的普通 copy-only 副本导出；RecoveryRequired 后端拒绝，当前无救援入口 | 正式普通导出和异常救援须分别实现第 8.2 节授权、幂等和 QPC1 状态契约 |

<a id="qpn-sec-8-2"></a>
### 8.2 目标命令矩阵

本节展开 API-001。除上述清理计划 v2 与明确标注的隔离 preview 切片外，其余命令仍是目标契约，不得据此宣称已经实现；preview 命令不得冒充本表正式命令。

所有命令使用第 8.3 节 envelope。标识与活动授权分为三类：

- **瞬时能力 ID**：request、`RootGrant`、`LicenseDeactivationGrant`、`AppUpdateRecoveryPackageFileGrant`、`DiagnosticExportTargetGrant`、候选写授权和 elevation session，绑定 TokenUser SID、Logon SID、Session、应用实例、用途和对象状态；跨实例/会话失效。第 6.4 节精确 helper 的一次性委托是唯一显式跨进程例外，不允许普通新实例消费旧能力。
- **活动授权上下文**：task/plan 的取消、确认和执行权限只保存在普通权限 Rust 后端，绑定耐久对象 ID、TokenUser SID、Logon SID、Session、应用实例、用途和对象状态。它不是 `taskId/planId`，也不作为 bearer token 返回 WebView；命令分发器从当前原生调用上下文取得并校验，跨实例/会话失效。
- **耐久只读记录 ID**：task、plan、operation、exclusion/analysis/R1 automation policy、approval grant、scheduled job、quarantine record 和 audit record，绑定 Windows 账户 TokenUser SID、schema、资源状态和保留期，存放于原生受保护仓库。task/plan/operation 的耐久 ID 只允许查询和对账，不能恢复已失效的瞬时执行能力；活动取消仍额外校验原 Logon SID/Session。新进程必须重新校验账户 SID、任务主体和内部摘要。

查询命令对未知 ID 和错误所有者统一返回对应领域的 `*_NOT_FOUND`，避免所有者枚举；终态 task/plan/operation 在保留期内仍可通过 `get_*` 查询。只有写命令对已终态或状态不匹配对象返回 `STALE_PLAN`、`OPERATION_NOT_CANCELLABLE` 或对应 state error。`authorize_plan(planId)` 和 `execute_plan(planId)` 的显式参数虽只有耐久 ID，但缺失当前原生活动授权上下文时必须返回 `STALE_PLAN` 且不得认领；新应用实例不能仅凭旧 `planId` 执行。`execute_plan` 响应丢失时，`get_plan` 必须能通过耐久 `claimedByOperationId` 找回唯一 operation。除原生选择器内部外，WebView 请求不得包含绝对路径、URL、卸载命令行或系统工具参数。

| 领域 | 目标命令 | 输入要点 | 返回 / 事件 | 风险与约束 |
|---|---|---|---|---|
| 授权 | `create_license_activation_draft` / `activate_license` | 前者打开原生卡密控件；后者只接受 `activationDraftId` | 草稿 ID/期限 / `LicenseStatusView` | WebView 永不读取卡密、installation ID、设备密钥或 token；未发送草稿到期销毁，已 `prepared` 请求仅在 replay deadline 前续作原 WAL，之后只读对账 |
| 授权 | `get_license_status` / `validate_license` / `refresh_license` | 均无 WebView 业务字段；refresh ID 由 Rust 生成 | `LicenseStatusView` | token 仅在 Rust/Credential Manager；refresh 以内部业务 request ID、双截止时间和服务端长期结果栅栏耐久幂等；过期旧 token 不重发 |
| 授权 | `create_license_deactivation_grant` / `deactivate_license` | 前者仅接受枚举 reason 并显示原生影响确认；后者仅接受一次性 `deactivationGrantId` | grant 摘要/期限 / `LicenseStatusView` | 业务 request ID 与签名声明只由 Rust 在消费 grant 的事务中生成；首次发送前先刷盘 WAL，超过 replay deadline 只做设备证明对账 |
| 根授权 | `grant_analysis_root` | 精确 `analysisKind`；由 Rust 打开原生目录选择器 | 与请求一一对应且仅含该 scope 的 `userSelectedAnalysis` grant | R0；`storageUsage/largeFiles/duplicates` 不得交叉签发或接受其他 scope，UI 不提交路径，授权默认随应用实例失效 |
| 分析策略 | `save_analysis_policy` / `revoke_analysis_policy` | 实例根授权 ID、分析类型、排除策略 ID | 策略 ID、摘要和状态 | 原生确认后由后端加密保存根；重启时复核卷/File ID |
| 排除策略 | `grant_exclusion_path` / `upsert_exclusion_policy` / `delete_exclusion_policy` | 原生选择器签发的排除 entry ID | 策略 ID、摘要和状态 | UI 不提交路径；跨重启复核卷/File ID |
| R1 策略 | `create_r1_automation_policy` / `delete_r1_automation_policy` | 当前 active 包中的规则 ID、资源上限 | R1 策略 ID、摘要和状态 | 只创建待批准策略，不构成授权 |
| 扫描 | `start_scan` | `ScanRequest` 判别联合 | `TaskRef` + 进度事件 | R0；任务 ID 由后端生成 |
| 扫描 | `get_scan_results` | `ScopedPageRequest<{ taskId }, ScanResultFilter>` | `ScanResultsResponse` | 只读；终态前只返回已提交页 |
| 任务 | `get_task` / `cancel_task` | 任务 ID | `TaskView` | 终态可读；取消协作式且幂等 |
| 计划 | `create_plan` | 扫描任务 ID、候选 ID、每项枚举动作 | `PlanView` | 后端重建 `scanCandidates` source 和不可变 item；不接受路径 |
| 卸载计划 | `create_uninstall_plan` | 应用快照 ID | `PlanView` | 构造 R3 计划；不直接启动进程 |
| 计划读取 | `get_plan` / `get_plan_items` | 计划 ID / `ScopedPageRequest<{ planId }>` | `PlanView` / `CursorPage<PlanItemView>` | UI 只获得脱敏视图；内部快照和策略材料不出仓库 |
| 确认 | `authorize_plan` | 计划 ID | 更新后的 `PlanView` | R1/R2/R4 和无需提升的 R3 完成原生确认；需提升的 R3 只确认提权意图并进入 `readyForElevation` |
| 执行 | `execute_plan` | **仅计划 ID** | `OperationRef` + 进度事件 | 原子认领 `ready/readyForElevation`；需提升的 R3 再由 helper 高完整性确认 |
| 执行 | `get_operation` / `get_operation_items` / `cancel_operation` | 操作 ID / `ScopedPageRequest<{ operationId }>` | `OperationView` / `AppendCursorPage<OperationItemResult>` / 当前状态 | 取消仅在 item 边界生效；同一计划不可重试 |
| 隔离 | `list_quarantine` | `PageRequest<QuarantineFilter>` | `CursorPage<QuarantineRecordView>` | 不返回原路径明文，按需本地解密展示 |
| 恢复 | `grant_restore_target` / `start_restore` | `purpose=restore` 的原生目标授权；`StartRestoreRequest` | 仅 `restoreTarget` grant / `OperationRef(kind=restore)` | 同一签发 route 按 purpose 选择精确响应 schema；只消费 `allowedScopes=['restore']` 的 grant；去重并稳定排序；只复制到新目录并保留源；显式 idempotency key 耐久幂等 |
| 异常导出 | `grant_restore_target` / `start_quarantine_salvage_export` | `purpose=salvageExport` 的原生目标授权；`StartQuarantineSalvageExportRequest` | 仅 `salvageExportTarget` grant / `OperationRef(kind=quarantineSalvage)` | 同一签发 route 按 purpose 选择精确响应 schema；grant 不得与普通导出交叉消费；仅 copy-only 到新目录、源保留；不得开放清除 |
| 清除 | `create_quarantine_purge_plan` | `recordIds[1..500]` | R4 计划 | 去重、稳定排序且整批预检；必须再走 `authorize_plan/execute_plan` |
| 自动批准 | `create_automation_approval` / `get_automation_approval` / `revoke_automation_approval` | 创建含 idempotency key、待批准 R1 策略 ID、触发器、Windows 时区和运行上限；Rust 显示原生摘要 | 绑定唯一 job ID 的 grant、期限和状态 | 创建响应丢失可按同 key 或 grant ID 找回；一个 grant 只授权一个 job，最长 180 天 |
| 自动任务读取 | `list_scheduled_jobs` | `PageRequest<ScheduledJobFilter>` | `CursorPage<ScheduledJob>` | 固定 snapshot 分页；不返回 Windows 命令行或原始 SID |
| 自动任务写入 | `upsert_scheduled_job` | R0 使用 create/update 严格联合；R1 **只接受已绑定 grant ID**；带幂等键 | `ScheduledJobMutationResult(kind=upserted)` | R1 不接受 job ID、触发器、时区、运行上限、路径、摘要或命令；R2-R4 拒绝 |
| 自动任务删除 | `delete_scheduled_job` | `jobId + expectedRevision + idempotencyKey` | `ScheduledJobMutationResult(kind=deleted)` | 先禁用并验证，再删除；响应丢失返回原 mutation result |
| 更新通道 | `get_update_channel_setting` / `set_update_channel_setting` | 领域、目标通道；应用写入必带 `expectedRevision`，首次创建固定为 `"0"`；规则写入禁止该字段；Rust 原生确认 | 与 request domain/channel 成对的规则 per-user setting 或 `MachineAppUpdatePolicy` | 应用通道修改需同 SID 管理员高完整性确认与全机 CAS；internal 仅 internal 构建 |
| 规则 | `get_rule_status` / `request_rule_update` | 耐久规则通道 setting ID；无下载 URL | 当前/候选包状态 | Rust 客户端下载、验签、能力校验并激活 |
| 应用更新 | `check_app_update` / `stage_app_update` / `apply_staged_update` / `cancel_app_update` / `get_app_update` | 逐命令使用 `CommandContractMap` 的 machine policy/update ID 与 expected journal sequence | `AppUpdateCheckResult` 或 `AppUpdateJournalView` | 未知/错误 owner 的 update ID 返回 `UPDATE_NOT_FOUND`；全局 mutex、policy/base CAS 和调用前撤销重验；URL/参数来自已验签 manifest |
| 更新恢复 | `grant_app_update_recovery_package_file` / `reconcile_update_recovery` / `apply_signed_recovery_package` | 前两者接受 closed `InstallRecoverySource`；恢复包 payload、assessment 和 result 均按同一 `source.kind` 判别；首个命令不接受路径并打开原生文件选择器；最后一个只接受 grant ID | 文件 grant / `UpdateRecoveryAssessment` / `RecoveryResolutionRecord` | grant 瞬时绑定 source/admission、owner/Logon SID、Session、应用实例、用途、文件身份/大小/哈希且只能消费一次；包内 source 必须逐字段匹配。`productUninstall` 仅允许以 `VerifiedProductAbsenceEvidence` 产生 `uninstalledPreserved`，不得携带安装目标字段；应用前仍验证恢复阈值签名、代码签名和 machine anchor |
| 应用 | `list_apps` / `get_uninstall_operation` | `PageRequest<AppFilter>` / 操作 ID | `CursorPage<AppView>` / `UninstallResult` | 枚举 R0；卸载写动作统一走计划 ID |
| 启动项 | `list_startup_entries` | `PageRequest<StartupEntryFilter>` | `CursorPage<StartupEntryView>` | R0；V1 无写命令 |
| 磁盘与分区 | `list_disks` / `list_partition_disks` | 无 | `DiskView[]` / `PartitionDiskView[]` | R0；仅展示后端枚举结果，不接受设备路径 |
| 磁盘与分区 | `open_windows_disk_management` | 无 | `OpenDiskManagementResult` | 只启动固定 `diskmgmt.msc` 系统入口，不拼接参数、不执行分区写操作 |
| 审计 | `list_operation_records` / `grant_diagnostic_export_target` / `export_diagnostic_bundle` | 分页请求；原生保存选择器无 WebView 路径字段；`diagnosticExportTargetGrantId` | `CursorPage<OperationRecordView>` / 保存 grant / 本地文件 | grant 仅允许在固定父目录以 `CREATE_NEW` 写一个诊断包；不自动上传 |

命令硬上限由后端常量控制，V1 初始值为：序列化请求不超过 1 MiB；同时最多 3 个只读扫描、1 个写操作；单次最多 32 个根授权、256 条规则、50,000 个计划候选；`maxFiles` 不超过 5,000,000，`maxResults` 不超过 200,000，目录深度不超过 128，扫描墙钟时间不超过 4 小时；结果页不超过 500 项。调用方只能请求更小限制，超过上限返回 `LIMIT_EXCEEDED`，不得静默扩大。

`start_restore`、`start_quarantine_salvage_export` 和 `create_quarantine_purge_plan` 的 JSON Schema 对 `recordIds` 强制 `minItems=1、maxItems=500、uniqueItems=true`；重复 ID 必须在任何查库、grant 消费或副作用前以 `INVALID_REQUEST` 拒绝。合法 UUID 先按 16 字节 canonical UUID 值升序排列，再计算 record-set、请求 payload、计划 item 和确认摘要；RFC 8785 本身不重排数组，因此规范排序属于其前置步骤。副作用命令的 `idempotencyKey` 是独立于 envelope `requestId` 的随机不透明值；后端以 `owner SID + command + key digest` 为唯一键，并保存规范化请求 payload 摘要、command-specific `mutationRef`，以及与 `CommandResponse<C>` 完全同型的不可变 response 或完整 `ApiError` 快照，保留期不得短于对应耐久记录。处理重试时必须先查 idempotency record，再校验可能已消费的瞬时 grant：同 key、同 payload 的 `succeeded/failed` 使用新的 envelope `requestId`，但 result/error 必须逐字段来自原快照，不能重读后来已变化的 operation/job/grant 来重建；同 key、不同 payload 返回 `INVALID_REQUEST`；两个并发首请求最多一个创建对象。`pending` 是内部耐久状态，不是成功响应：相同请求只能等待仍存活的同一 mutation，或凭 command-specific `mutationRef` 和对应 WAL 以 CAS 取得对账权并续作该 mutation，不能创建第二个 operation、grant、job、journal 或重复已武装外部调用；期限内无法终态化时返回可重试的 `IDEMPOTENCY_PENDING`，记录不得因 TTL 删除，调用方只能用同 key 再查。idempotency record、operation/job/grant、授权消费和整批 record CAS 必须在相应的同一事务边界提交。`create_automation_approval/upsert_scheduled_job/delete_scheduled_job` 同样遵守此规则；相同 R1 grant 只能解析到其唯一 `boundScheduledJobId`，不得生成第二个任务。

<a id="qpn-sec-8-3"></a>
### 8.3 目标类型

以下八项是本轮要求的正式领域契约登记；表内类型是权威根，代码块中的其他 wire、view、WAL、evidence 和 helper 类型均为其子类型或支撑契约，不得另行替代根契约。运行时迁移可分阶段进行，但生成的 Rust/TypeScript/JSON Schema 必须引用同一 Contract ID 和版本。

| Contract ID | 正式契约 / 权威类型 | 版本 | 生产者 | 消费者 | 存储或传输边界 |
|---|---|---|---|---|---|
| CONTRACT-001 | 规则清单 / `RuleManifest` | `schemaVersion=1` | 规则发布流水线 | 受限规则引擎、扫描器 | 签名规则包；进入 active 双槽前验证 |
| CONTRACT-002 | 扫描请求 / `ScanRequest` | `RequestEnvelope.apiVersion=1` | UI 经 Tauri envelope | 普通权限扫描器 | 仅 IPC；不耐久保存任意路径 |
| CONTRACT-003 | 候选快照 / `CandidateSnapshot` | `schemaVersion=1` | 扫描器 | 计划生成器、执行前复检 | 当前用户受保护状态仓库 |
| CONTRACT-004 | 清理计划 / `CleanupPlan` | `schemaVersion=2` | 计划生成器、原生确认器 | 普通/提权执行协调器 | 不可变耐久计划；只以 ID 跨 IPC |
| CONTRACT-005 | 执行结果 / `ExecuteResult` | `schemaVersion=1` | 执行器、结果聚合器 | UI、本地审计、启动对账 | 耐久终态；脱敏 view 跨 IPC |
| CONTRACT-006 | 隔离记录 / `QuarantineRecord` | `recordVersion=5` | QPC1 隔离/导出/清除执行器 | 隔离中心、恢复对账、quota ledger | 每卷受保护仓库；不上传 |
| CONTRACT-007 | 计划任务 / `ScheduledJob` | `schemaVersion=1` | 原生批准与任务注册器 | Windows Task Scheduler runner | 用户受保护仓库 + 规范化系统 Task 摘要 |
| CONTRACT-008 | 出站策略 / `OutboundRequestPolicy` | `policyVersion=1` | 签名发布配置 | Rust 类型化 HTTP 客户端 | 编译/签名配置；WebView 不可修改 |

`BaselineManifest`、`M0BaselineVerificationRecord v2`、`ServiceDeliveryRecord`、`DesignDocumentApprovalRecord v2`、`PlatformTupleRegistry`、`ReleaseCapabilityManifest`、`GovernanceTrustPolicy v1`、`ReleaseSourceDerivationRecord v1`、`DependencySecurityReport v2`、`ReleaseGateManifest v4` 和 `SignedReleaseAttestation v1` 是发布治理支撑契约，不计入上述八个产品运行时领域根：它们分别绑定可重放基线、M0 正式通过、在线服务证据、可认证批准、精确平台/测试义务、实际签名发布配置、外部信任策略、M0 到最终源码派生、可复算依赖处置、同一构建的发布证据根和 gate 外最终签名。

`FormalContractRegistry` 必须恰好包含上表八项，Contract ID 不得缺失、重复或扩展；每项的权威根、版本字段和值必须与类型定义一致，并分别保存对应 Rust、TypeScript 和 JSON Schema 规范制品的 SHA-256，三者不要求彼此相等。`FormalContractRegistrySnapshot(registryVersion=2)` 中 Rust/TypeScript 制品按 pinned generator 输出的 UTF-8、LF、无 BOM 精确字节哈希，JSON Schema 按 RFC 8785 UTF-8 字节哈希；registry digest 精确为 `SHA256(UTF8(RFC8785({registryVersion, registryCanonicalization, artifactCanonicalization, entries})))`，明确排除 `registryDigestSha256` 自身。任一制品摘要、tuple 顺序、canonical profile、根类型或版本漂移都使 API-004、M1-11 和发布门禁失败。


#### 8.3 类型契约分片索引

以下文件共同构成第 8.3 节的唯一规范 TypeScript 契约。类型检查和生成器必须按文件名前缀顺序拼接，不能单独宽化任一片段。

| 顺序 | 契约片段 |
|---|---|
| 1 | [8.3.1 基础类型与正式契约登记](contracts/00-core.md#qpn-sec-8-3-1) |
| 2 | [8.3.2 授权与许可证事务](contracts/01-licensing.md#qpn-sec-8-3-2) |
| 3 | [8.3.3 运行时通用类型、授权与策略](contracts/02-runtime-common.md#qpn-sec-8-3-3) |
| 4 | [8.3.4 规则签名、更新与撤销](contracts/03-rule-update.md#qpn-sec-8-3-4) |
| 5 | [8.3.5 应用更新、安装器与可信恢复](contracts/04-app-update.md#qpn-sec-8-3-5) |
| 6 | [8.3.6 扫描、任务与候选快照](contracts/05-scan-snapshot.md#qpn-sec-8-3-6) |
| 7 | [8.3.7 调度授权与不可变计划](contracts/06-schedule-plan.md#qpn-sec-8-3-7) |
| 8 | [8.3.8 执行、逐项结果与空间核算](contracts/07-execution-result.md#qpn-sec-8-3-8) |
| 9 | [8.3.9 隔离、恢复、导出与清除](contracts/08-quarantine.md#qpn-sec-8-3-9) |
| 10 | [8.3.10 应用枚举与卸载事务](contracts/09-app-uninstall.md#qpn-sec-8-3-10) |
| 11 | [8.3.11 启动项、磁盘、审计与任务日志](contracts/10-inventory-jobs.md#qpn-sec-8-3-11) |
| 12 | [8.3.12 出站策略与在线服务交付](contracts/11-network-services.md#qpn-sec-8-3-12) |
| 13 | [8.3.13 命令请求响应闭合映射](contracts/12-command-map.md#qpn-sec-8-3-13) |
| 14 | [8.3.14 稳定错误码](contracts/13-error-codes.md#qpn-sec-8-3-14) |


`CommandContractMap` 是 Tauri 命令面的唯一类型登记。第 8.2 节每个用 `/` 并列的命令都必须在生成阶段展开为独立 route，并与 map、Rust handler、closed JSON Schema 恰好一一对应；route 缺失、多出、request/response 错配或绕过 envelope 均使构建失败。同一 route 使用 `CommandSpec` 成对联合时，生成器必须保留每个 request discriminator 与 response 的对应关系，不能分别投影、笛卡尔组合或合并成两个宽联合：handler 先按 closed request schema 解析请求，再只使用 distributive `CommandResponseFor<C, typeof parsedRequest>` 对应的成功响应 schema；`Q extends Req` 允许解析后更窄的 literal 类型命中所属分支，但不能让 Q 命中其他 discriminator。除两个 grant route 外，`create_license_deactivation_grant.reason`、`save_analysis_policy.scope`、`start_scan.kind`、`upsert_scheduled_job.kind/mutation`、更新 setting 的 `domain/channel` 都是正式配对键：响应必须逐字段回显同一 reason/scope/kind/domain/channel，R0 create/update 不能返回 R1 job。应用通道 set 的 `expectedRevision` 必填，首次创建用规范 U64 `"0"`；规则通道 set 必须没有该字段。`grant_analysis_root` 的三个 `analysisKind` 只能返回各自 singleton `allowedScopes` 的 `userSelectedAnalysis` grant；`grant_restore_target` 的 `restore/salvageExport` 只能分别返回 `restoreTarget/salvageExportTarget`。`CommandRequest<C>/CommandResponse<C>` 仅表示 route 的请求或响应全集，用于 envelope 生成与静态校验，不得单独用作任一成对 route 的成功响应校验，也不允许 handler 自行扩大字段。

许可证对账还必须执行跨字段领域校验：响应的 `mutationKind/mutationRequestId/requestBodySha256/installationId` 必须与本地 pending 和对账请求逐字节一致；本地 resolution 还必须绑定相同 installation ID 和设备公钥摘要；`committed` 分支中的原 wire response 必须带对应 mutation kind 的 request ID，任何交叉组合均返回 `LICENSE_RESPONSE_INVALID`。`pending.retryAfterSeconds` 只能为 1 至 300 的安全整数。`notCommitted` 必须带 `durableNegativeFence=true`、`mutationKeyReuseBlocked=true`、`fenceRetention=licenseSubjectLifecycle`，且 `fenceCreatedAtUtc <= checkedAtUtc`；这些字段是服务端生命周期 tombstone 的签名协议断言，客户端验证 schema/关联/时间顺序，服务端测试证明 tombstone 在 subject 存续期不可 GC、重装或后台操作绕过。

旧许可证 WAL 必须按原 state 迁移，不能整类降级：旧 `responseStored` 先验证原 response、credential slot/rotation result 和摘要，再只续作本地 pointer CAS；旧 `committed` 只验证 active pointer/终态并做允许的 secret GC，两者绝不重发 mutation 或回退到网络对账。只有旧 pre-response 状态且 request body digest 与原设备私钥均可用时，才迁入新版本 `reconciliationRequired`，固定 `mutationReplayDeadlineAtUtc=migratedAtUtc`（因此旧 mutation 立即禁止重发），并从迁移时刻建立 30 天 activation/refresh 或 90 天 deactivation 只读对账窗口。缺少 request digest 或私钥时必须写 `LegacyLicenseMutationRecoveryRecord(unresolvedLegacyMutation)`，保留现有 WAL/secret，设置 `blocksNewLicenseMutation=true`，且不得生成任何 `LicenseMutationResolutionRecord`、新 installation/request ID 或允许重新认证；只有服务端返回关联 committed、生命周期级 notCommitted fence，或签名支持决议后才能终态化。

所有契约使用严格 JSON Schema（`additionalProperties=false`）并在解析后、哈希前再次做领域校验。`U64String`、`I64String`、`FileTimeString`、`Uuid` 和 `Sha256` 必须满足类型旁的唯一文本形式；禁止前导零、大小写等价值、空白和超范围数。字段名以 `AtUtc/UntilUtc/ExpiresAtUtc/DeadlineUtc` 结尾时必须使用 `TimestampUtc` 的毫秒级 UTC 规范形式；JSON `number` 只用于 schema 明确限定且处于 JavaScript safe-integer 范围的计数、枚举或进程字段，文件大小、序列和 FILETIME 一律使用字符串整数。所有字符串按原始 UTF-8 字节验证，不做隐式 Unicode 规范化。

摘要生成分两步：先执行领域规范化，再对结果执行 RFC 8785。领域规范化必须拒绝重复键、重复集合成员和非法等价值；`recordIds` 按 canonical UUID 的 16 字节值排序，`ApprovalBinding` 按 `ruleId UTF-8 bytes + packageHash + ruleVersion + canonicalPolicyDigest` 排序，包哈希/撤销集合按原始摘要字节排序。数组若表示用户顺序或计划执行顺序则不得重排，并必须在 schema 中显式标注。规范化实现只有一个 Rust 入口，并为 TypeScript schema、计划、确认、幂等、签名和审计共享测试向量；直接对未规范数组运行 RFC 8785 不构成合法摘要。

`CandidateSnapshot`、`CleanupPlan`、`PlanSource`、`PlanItem`、`AppSnapshot`、`PersistentAnalysisPolicy`、`ApprovalGrant` 和原路径/策略密文只保存在 Rust/本地受保护仓库；UI 获得的是按需展示视图以及不透明 ID，不得回传其中路径、绑定、摘要或密文作为动作参数。所有 release/recovery signed payload 使用 UTF-8 JSON、拒绝重复键并按 RFC 8785 规范化；`payloadSha256` 是规范化 payload 字节的 SHA-256，签名覆盖相同字节。恢复签名按第 14.1 节 `RecoveryKeySet` 验证：key ID、公钥字节/指纹均唯一，只把不同公钥产生且 purpose/domain/channel 正确的有效签名计入阈值；同一公钥的别名 ID 或重复签名不得多计。

`CleanupPlan` 的联合把 source、非空 item 集合、确认方式和生命周期强关联：`cleanupRules` 只能包含同一 task 的规则 `fileCandidate`，其类型与 validator 都禁止 `permanentDeleteOriginal`；该 R4 动作只能来自新的、仍有效的 `largeFiles` 用户选择快照。`appSnapshot` 必须且只能包含一个同摘要 `appUninstall`；`quarantineSelection` 只能包含 digest 中的 `quarantinePurge`。不得混合来源或在执行时换动作。manual file/purge/non-elevated app 只允许 native lifecycle，scheduledR1 只允许 cleanupRules/R1 cache-delete，elevated app 只允许 nativeThenElevation lifecycle；不存在 `appSnapshot + scheduledR1`、`quarantineSelection + readyForElevation` 或免确认写计划。`claimed/consumed` 必须携带唯一 operation ID。`PlanView` 使用相同来源/提权判别联合，不表达内部类型禁止的组合。`categorySummary` 必须按 `ConfirmationCategoryId` 稳定排序、无重复并精确覆盖全部 item：每类 count、逻辑字节、最高风险、动作和恢复均从 item 派生；摘要缺类、多类、计数不一致或确认页折叠类别时以 `OPERATION_STATE_INVALID` 失败关闭。每个 item 固化 `snapshotDigestSha256、policyDigestSha256、risk、action、recovery、requiresElevation`，摘要规范如下：

- 文件 item 的 snapshot digest 必须等于对应 `CandidateSnapshot.snapshotDigestSha256`；policy digest 覆盖规则包/规则版本、最终动作、风险、恢复、保护、排除和执行资源限制。
- 隔离清除 item 的 snapshot digest 对 canonical `recordId + recordJournalSequence + committed objectIdentity + retention + export + logical/allocated bytes` 计算；policy digest 覆盖清除策略版本、允许状态、身份/DACL/链接/流复检、tombstone 保留和确认模板。
- 卸载 item 的 snapshot digest 必须等于 `AppSnapshot.snapshotDigest`；`sealedInvocation` 从同一快照复制 MSI ProductCode、AppX identity 或 Win32 注册资源/EXE 身份/固定参数并进入 planHash，适配器只从 `sealedInvocation.adapter` 判别，不存在可冲突的外层 adapter 字段。policy digest 覆盖适配器策略版本、位置/签名规则、`requiresElevation` 判定和 30 分钟观察策略。

`planHash` 覆盖版本、source、按稳定顺序排列的全部 item（包括 sealed invocation）、所有摘要、所有者/会话和期限，但不覆盖可变状态/时间戳。后端 schema validator 还必须拒绝非法组合：文件/隔离清除 item 的 `requiresElevation` 只能为 false；`appSnapshotId`、snapshot digest、`requiresElevation` 和 sealed invocation 必须与源 `AppSnapshot` 完全一致；`readyForElevation` 只能对应一个 `requiresElevation=true` 的卸载 item 和 `nativeThenElevation` 确认；`scheduledR1` 只能绑定全部为 R1/cache-delete 的文件 item；source/item 摘要或数量必须完全一致。状态仓库出现不可能组合时返回 `OPERATION_STATE_INVALID` 并停止写操作，不尝试修补。用户修改任一动作或选择时必须创建新计划。`PlanView/PlanItemView` 是脱敏投影，不是回写模型。

R1-R4 写候选必须填充卷 GUID/序列、128 位 File ID、父链摘要、逻辑/分配大小、三类 FILETIME、属性、链接数、流集合摘要和安全描述符摘要。`usn` 仅在卷启用并允许读取 USN Journal 时存在；缺失必须进入计划能力摘要，UI 和测试不得把它描述为已验证。`allocatedBytes` 只影响空间估算，不影响身份判定。

`canonicalPolicyDigest` 是对 schema 约束和上述领域规范化后的策略 JSON 执行 RFC 8785 再计算 SHA-256，覆盖第 9.4 节列出的全部策略字段。计划、确认、自动批准和审计必须使用同一函数及测试向量。哈希和 `ApprovalBinding` 是批准内容，不是授权凭据；只有后端在原生确认后签发、仍为 active 的不透明 `approvalGrantId` 可用于建任务。R1 grant 在签发时同时绑定后端生成的唯一 job ID、完整 trigger、Windows time-zone ID、task principal SID、`maximumRuns` 和 definition digest；`maximumRuns` 为 1 至 180，`runsStarted` 在每次启动首个 item 前与 grant revision 原子递增。一个 grant 只能对应一个 job，删除、触发器/时区/运行上限/主体变化或 job 重建均需要新的原生批准，旧 grant 不迁移。V1 调度仅支持指定 Windows time-zone ID 的 daily/weekly 本地墙钟触发，`localTime` 为严格 `HH:mm`；任务计划设置“错过后尽快运行”“禁止并发实例”和 `AllowStartOnDemand=false`，低磁盘事件触发不进入 V1。

每个触发必须派生 `occurrenceKey = jobId + jobRevision + timeZoneId + localCalendarDate + triggerSlot` 并建立唯一索引。同一 occurrence 在夏令时回拨的重复小时、系统时钟回拨、Task Scheduler 重试或两个 instance 竞争时最多 claim 一次；春季跳时不存在的墙钟时刻只允许由“错过后尽快运行”产生一次，并记录 `launchReason=startWhenAvailable`。Windows 时区规则更新后沿用已批准的 time-zone ID 与本地墙钟，不改变 trigger 字段；系统 task definition 的 UTC 投影/摘要变化必须经 scheduler journal 重新验证后才能运行。无法把 Task Scheduler instance 唯一关联到 occurrenceKey 时返回 `JOB_TRIGGER_ATTESTATION_INVALID`，不得猜测执行。

`QuarantineRecord` 是按 `state` 判别的 recordVersion 5 联合。已知来源的 `prepared/sourceDigestPrepared/containerPrepared/copying/copied/containerVerified/containerCommitted/sourceDeletePrepared/sourceRemovedVerified/committed/sourceRetained/restorePrepared/purgePrepared` 分支必须携带对应 data；只有 `sourceRemovedVerified + FileMutationAttempt(removedVerified)` 可形成成功 `committed`，`sourceRetained` 必须证明完整 QPC1 容器和原文件均存在并投影为失败。`orphaned` 不要求 operation、candidate 或 rule ID；`purged/purgedUnverified` 只能使用最小 tombstone，禁止保留 candidate ID、路径/安全描述符密文、仓库名、File ID 或内容哈希。只有 `purgedUnverified` 必须引用受保护的 `PurgeReconciliationEvidence`；证据按 record/operation/charge/entry/volume/File ID/sequence 全绑定，unresolved 时 tombstone retention 被阻塞，resolved verified absence 后才在同一账本事务释放/替换 charge 并开始保留期。证据不通过 UI 返回，也不能用于内容恢复。`SalvageExportRecord` 使用 recordVersion 2。

recordVersion 3/4 的 `movePrepared/moved/secured`、prepare ACL 草稿及 `QUARANTINE_MOVE_FAILED/QUARANTINE_SECURE_FAILED/QUARANTINE_COMMIT_FAILED` 只允许历史解析，v5 绝不生成这些状态或错误。旧记录不能被解释为 QPC1 容器，也不能续作 rename、覆盖 DACL或触发源删除；迁移器保留原始字节和摘要，将可只读定位的历史隔离对象标记为 `recoveryRequired` 供专项导出，不完整/身份不明对象标记 `damaged/orphaned`。只有用户通过新 R2 计划重新建立并完整验证 QPC1 后才产生 v5 `committed`；迁移失败时关闭新写入并保留原记录。实现必须按 recordVersion/state 分支执行 closed schema 校验，不能用可选字段跨版本绕过恢复前置条件。

分页分为两种一致性模型。plan item、apps、quarantine、startup、scheduled jobs 和 audit 的首请求固定一个版本化 as-of high-water，使用 `CursorPage.snapshotComplete`；后续写入只在新的 first request 出现，旧版本行在 cursor 链结束或租约到期前保持可读。运行中的 scan/operation item 使用 `AppendCursorPage`：`caughtUp` 仅表示读到该响应的 `asOfSequence`，`producerTerminal` 才表示不会再追加；四种合法组合与类型联合一一对应：非 caught-up 一律返回 next cursor，caught-up 且生产者未终态仍返回 next cursor，只有 caught-up 且生产者终态时禁止 next cursor。`TaskView.producerComplete` 描述生产者是否终态，不与“本页是否读尽”复用字段。

首请求默认 100 项、范围 1 至 500；有父对象的命令把 task/plan/operation ID 仅放在 `ScopedPageRequest` first 分支，next 只能带 cursor。cursor TTL 为最后访问后 15 分钟、单个 snapshot 最长 60 分钟，绑定 API、命令、对象、所有者/会话、过滤、页大小、high-water、下一序号和排序；过期必须重新 first request。每个 owner/session 最多 8 条 active chain、200,000 条 pinned version row 和 64 MiB pinned bytes，采用 `CursorResourceLimits` 的绝对值；达到任一上限时新的 first 请求返回 `LIMIT_EXCEEDED`，不得驱逐仍有效的其他链或放宽快照。排序及 tie-breaker 固定为：scan/operation 是 `durableSequence + result/item ID`，plan 是 `itemOrder + planItemId`，apps 是 invariant-case displayName + appSnapshotId，quarantine 是 journalSequence + recordId，startup 是 source + invariant-case displayName + entryId，scheduled jobs 是 maximumRisk + canonical jobId，audit 是 eventSequence + auditRecordId。一个有效 cursor 链内不得重复、遗漏或漂移。解析/过期返回 `CURSOR_INVALID/CURSOR_EXPIRED`，不得回退 offset。所有列表使用脱敏 view，任何页不超过 1 MiB。

调度写入使用 `ScheduledJobJournal` v2，upsert 固定按 `prepared → registered → verified → committed` 前进，删除固定按 `deletePrepared → disabled → deleted` 前进；每次状态转移均独立事务提交并刷盘。`prepared` 后只注册 disabled Task，`registered` 后重读并验证系统定义，`verified` 后才应用 `desiredDefinition.enabled` 并再次重读；只有 observed enabled 状态与 desired 完全相等且两个摘要均匹配才写 `committed`。若应用启用状态后、committed 前崩溃，runner 因找不到 committed job 必须零 item。启动时在接受任何触发前枚举产品固定命名空间：未知 task、无 committed job、owner/principal/action/系统定义摘要不匹配或无法精确续作的非终态 journal 必须先禁用并确认；否则整个自动任务子系统返回 `JOB_RECONCILIATION_REQUIRED` 并失败关闭。确定性 task 名由安装密钥对 `owner SID digest + jobId` 计算，不包含用户文本；同一幂等键必须返回原 `ScheduledJobMutationResult`。

<a id="qpn-sec-8-4"></a>
### 8.4 稳定错误码

| 领域 | 错误码 | 含义与默认处理 |
|---|---|---|
| API | `UNSUPPORTED_API_VERSION`、`INVALID_REQUEST`、`IDEMPOTENCY_PENDING`、`LIMIT_EXCEEDED`、`CAPABILITY_DISABLED`、`PLATFORM_UNSUPPORTED` | 非 pending 错误拒绝请求且不新增副作用；pending 只附着/对账同一耐久 mutation，关闭的条件能力和不支持平台不得回退到相近动作 |
| 分页 | `CURSOR_INVALID`、`CURSOR_EXPIRED` | 不返回猜测页；从首请求重新读取 |
| 根/任务 | `ROOT_GRANT_INVALID`、`RESTORE_GRANT_INVALID`、`RESTORE_AUTH_SNAPSHOT_INVALID`、`TASK_NOT_FOUND`、`TASK_CONFLICT` | 新动作重新授权；已 prepared 恢复仅按合法操作级快照对账 |
| 原生文件授权 | `DIAGNOSTIC_EXPORT_GRANT_INVALID` | 诊断导出目标 grant 缺失、过期、已消费、跨会话或父目录身份变化时不创建文件，重新打开原生保存选择器 |
| 策略 | `EXCLUSION_ENTRY_NOT_FOUND`、`EXCLUSION_POLICY_NOT_FOUND`、`EXCLUSION_POLICY_CHANGED`、`ANALYSIS_POLICY_NOT_FOUND`、`AUTOMATION_POLICY_NOT_FOUND`、`AUTOMATION_POLICY_INVALID` | 不使用缺失或身份变化的耐久策略；重新选择、保存并按需批准 |
| 批准/作业 | `APPROVAL_GRANT_NOT_FOUND`、`APPROVAL_GRANT_INVALID`、`APPROVAL_GRANT_EXPIRED`、`APPROVAL_GRANT_REVOKED`、`APPROVAL_GRANT_BINDING_MISMATCH`、`APPROVAL_GRANT_RUN_LIMIT_REACHED`、`JOB_NOT_FOUND`、`JOB_INVALID`、`JOB_CONFLICT`、`JOB_REGISTRATION_FAILED`、`JOB_DEFINITION_MISMATCH`、`JOB_TRIGGER_ATTESTATION_INVALID`、`JOB_RECONCILIATION_REQUIRED` | 不生成/继续新的自动 item；runner 必须证明当前进程是批准触发产生的 Task Scheduler instance；先禁用未知或不一致 Task，再按 journal 对账；绑定/运行上限和撤销通过 revision/CAS 线性化 |
| 通道 | `CHANNEL_SETTING_INVALID` | 不接受 UI 通道字符串；重新读取或原生签发耐久 setting |
| 授权 | `LICENSE_TRANSIENT_GRANT_INVALID`、`LICENSE_RATE_LIMITED`、`LICENSE_ACTIVATION_FAILED`、`LICENSE_PROOF_INVALID`、`LICENSE_RESPONSE_INVALID`、`LICENSE_TOKEN_STORE_FAILED`、`LICENSE_DEACTIVATION_FAILED`、`LICENSE_RECOVERY_REQUIRED` | 激活草稿或停用确认 grant 缺失、过期、已消费、跨会话或绑定不符时零请求；令牌未耐久保存前不报告激活成功；mutation replay deadline 前只续作已耐久的原业务 ID，之后只允许 PoP 只读对账；超过对账期限或服务端证据不闭合时保持显式恢复状态，不回退 Web Storage/裸 bearer、不发送旧 mutation |
| 规则可用性 | `RULES_UNAVAILABLE`、`RULE_SIGNATURE_INVALID`、`RULE_SCHEMA_INVALID` | 不开放规则清理；只读非规则分析可继续 |
| 规则控制面 | `RULE_INDEX_INVALID`、`RULE_KEY_AUTHORIZATION_INVALID`、`RULE_KEY_REVOKED`、`RULE_REVOCATION_INVALID`、`RULE_CONTROL_EXPIRED`、`RULE_DOWNLOAD_FAILED`、`RULE_PACKAGE_HASH_MISMATCH` | 不接受候选控制面/包；已验证 sticky 撤销仍生效，命中 active 时关闭规则清理 |
| 规则权限 | `RULE_CAPABILITY_VIOLATION`、`RULE_CHANNEL_MISMATCH` | 拒绝候选包并保留当前 active 包 |
| 规则序列 | `RULE_ROLLBACK_BLOCKED`、`RULE_PACKAGE_REVOKED`、`RULE_ACTIVATION_FAILED` | 不降低高水位；按第 10.1 节发布更高序号修复包 |
| 计划 | `PLAN_NOT_FOUND`、`PLAN_NOT_AUTHORIZED`、`PLAN_ALREADY_CLAIMED`、`STALE_PLAN` | 不开始候选循环；需要时重新扫描、确认和建计划 |
| 操作 | `OPERATION_NOT_FOUND`、`OPERATION_NOT_CANCELLABLE`、`OPERATION_STATE_INVALID` | 不推测状态或重复动作；读取最新 operation，终态不可取消 |
| 状态仓库 | `STATE_STORE_UNAVAILABLE`、`STATE_STORE_CORRUPT` | 关闭全部新写动作；只按已耐久 WAL 对账，禁止重建猜测状态后继续 |
| 卷/路径 | `VOLUME_CHANGED`、`UNSUPPORTED_FILESYSTEM`、`PATH_OUTSIDE_ROOT`、`PARENT_CHANGED` | 跳过并保留；越界同时记录安全事件 |
| 文件身份 | `IDENTITY_CHANGED`、`REPARSE_POINT`、`MULTIPLE_HARD_LINKS`、`UNEXPECTED_STREAM` | 在写操作前跳过，处置为 `originalPreserved` |
| 运行状态 | `PROCESS_RUNNING`、`PROCESS_STATE_UNKNOWN`、`FILE_NOT_FOUND`、`FILE_LOCKED`、`FILE_MUTATION_INTERRUPTED`、`FILE_MUTATION_OUTCOME_UNKNOWN` | 写前消失项以 `notApplicable` 跳过；删除调用可能发生但精确对象仍存在时用 interrupted 保留对象并消费旧计划；目录项/身份/证据不明时用 outcome unknown 进入 recoveryRequired，两者都不重调删除 API |
| 不支持对象 | `CLOUD_PLACEHOLDER`、`EFS_UNSUPPORTED` | 不触发召回、不解密、不删除 |
| 权限/空间 | `ACCESS_DENIED`、`DISK_FULL`、`QUARANTINE_QUOTA`、`QUARANTINE_ACCOUNTING_UNKNOWN`、`QUARANTINE_LEDGER_INVALID` | 停止对应候选或操作，保留原文件；unknown/invalid ledger 只关闭新隔离，不阻止列表、诊断和救援；`ACCESS_DENIED` 本身不触发跨账户 UAC |
| 隔离建立（v5） | `QUARANTINE_UNAVAILABLE`、`QUARANTINE_PREPARE_FAILED`、`QUARANTINE_CRYPTO_UNSUPPORTED`、`QUARANTINE_CONTENT_GUARD_UNAVAILABLE`、`QUARANTINE_KEY_UNAVAILABLE`、`QUARANTINE_CONTAINER_CREATE_FAILED`、`QUARANTINE_COPY_FAILED`、`QUARANTINE_COPY_INTERRUPTED`、`QUARANTINE_SOURCE_CHANGED_DURING_COPY`、`QUARANTINE_CONTAINER_INTEGRITY_FAILED`、`QUARANTINE_CONTAINER_COMMIT_FAILED` | 按 QPC1 WAL 阶段保守终态化；容器完整验证和提交前绝不删除源，不得降级永久删除 |
| 隔离历史解析（v3/v4） | `QUARANTINE_MOVE_FAILED`、`QUARANTINE_SECURE_FAILED`、`QUARANTINE_COMMIT_FAILED` | 只读取历史记录并进入受限迁移/救援；v5 执行器不得产生这些错误或续作旧 rename/DACL 流程 |
| 隔离记录 | `QUARANTINE_RECORD_NOT_FOUND`、`QUARANTINE_STATE_INVALID`、`QUARANTINE_IDENTITY_CHANGED`、`QUARANTINE_RECOVERY_REQUIRED`、`QUARANTINE_DAMAGED` | 不恢复或清除不合法记录；处置为 `unknownNeedsAttention` 并阻止自动动作 |
| 恢复/清除 | `RESTORE_TARGET_INVALID`、`RESTORE_TARGET_CONFLICT`、`RESTORE_INTEGRITY_FAILED`、`RESTORE_INTERRUPTED`、`QUARANTINE_SALVAGE_FAILED`、`PURGE_FAILED`、`PURGE_INTERRUPTED`、`PURGE_OUTCOME_UNKNOWN` | 保留隔离源和已成功导出文件；无临时目标的未处理恢复项回原状态，有不完整目标时进入对账；异常救援不改源状态；清除中断按 WAL 对账 |
| 提权 | `ELEVATION_SAME_USER_REQUIRED`、`UAC_CANCELLED`、`UAC_TIMEOUT` | 计划保持已消费；V1 不重试 over-the-shoulder UAC |
| IPC | `IPC_PEER_INVALID`、`IPC_PROTOCOL_INVALID`、`ELEVATED_ACTION_NOT_ALLOWED` | 终止提权进程并记录安全事件，不执行动作 |
| 应用快照/卸载 | `APP_SNAPSHOT_NOT_FOUND`、`APP_SNAPSHOT_STALE`、`UNINSTALL_TARGET_INVALID`、`UNINSTALL_TARGET_AMBIGUOUS`、`UNINSTALL_REBOOT_REQUIRED`、`UNINSTALL_NOT_REMOVED`、`UNINSTALL_OUTCOME_UNKNOWN`、`UNINSTALL_RECOVERY_REQUIRED` | 多 context MSI、枚举不足或目标变化零调用；待重启和未知 attempt 保持 machine lock 且不重调；退出码 0 不替代资源/进程树复检 |
| 更新信任 | `UPDATE_NOT_FOUND`、`UPDATE_SIGNATURE_INVALID`、`UPDATE_SIGNER_POLICY_INVALID`、`UPDATE_MANIFEST_INVALID`、`UPDATE_KEY_AUTHORIZATION_INVALID`、`UPDATE_KEY_REVOKED`、`UPDATE_REVOCATION_INVALID`、`UPDATE_EPOCH_MIGRATION_REQUIRED`、`UPDATE_EPOCH_MIGRATION_INVALID`、`UPDATE_REPLAY_BLOCKED`、`UPDATE_VERSION_NOT_NEWER` | 未知/错误 owner ID 不泄露存在性；不下载或安装；不降低/重置信任高水位，subject name 不替代 SPKI/chain/revocation policy |
| 更新传输/安装 | `UPDATE_DOWNLOAD_FAILED`、`UPDATE_PACKAGE_INVALID`、`UPDATE_PLATFORM_MISMATCH`、`UPDATE_STAGING_FAILED`、`UPDATE_INSTALLER_UNSUPPORTED`、`UPDATE_CONTROL_EXPIRED`、`UPDATE_CONFLICT`、`UPDATE_INSTALLER_DOWNGRADE_BLOCKED`、`UPDATE_LKG_UNAVAILABLE`、`UPDATE_ADMISSION_BLOCKED`、`UPDATE_MACHINE_ANCHOR_INVALID`、`UPDATE_ARTIFACT_REVOKED`、`UPDATE_NOT_CANCELLABLE`、`UPDATE_SYSTEM_REBOOT_PENDING`、`UPDATE_INSTALL_FAILED`、`UPDATE_REBOOT_REQUIRED` | 保留当前版本；非原生架构包在创建/读取锚或任何产品写入前拒绝；裸 MSI、无锚点/冲突锚点和无精确 LKG 均不进入安装；admission floor 仅在 `callArmed` 事务中推进且不回退，无调用取消不 fence 同一包；已 armed 后不可取消 |
| 更新试运行/恢复 | `UPDATE_TRIAL_FAILED`、`UPDATE_TRIAL_TIMEOUT`、`UPDATE_MIGRATION_FAILED`、`UPDATE_USER_MIGRATION_FAILED`、`UPDATE_OUTCOME_UNKNOWN`、`UPDATE_ROLLBACK_FAILED`、`UPDATE_RECOVERY_REQUIRED`、`UPDATE_RECOVERY_PACKAGE_INVALID`、`UPDATE_RECOVERY_PACKAGE_GRANT_INVALID`、`UPDATE_RECOVERY_REPLAY_BLOCKED` | 恢复文件 grant 无效时不读取文件；精确 LKG 仍受 sticky 撤销；结果未知只读；恢复 sequence/hash 在 machine anchor 单调 CAS，可信恢复只追加 resolution；单用户迁移失败不回滚全机 binary |
| 对账/取消 | `OPERATION_OUTCOME_UNKNOWN`、`USER_CANCELLED` | 前者只标记因既有未知结果而尚未开始的后续 item，并使 operation 聚合为 `recoveryRequired`；后者返回已提交与未处理明细 |

`partiallySucceeded` 是 `OperationView/ExecuteResult.status`，不是 item 错误码。每个 item 错误必须同时返回 `phase`、`disposition` 和 `retryable`。`ApiError.retryable=true` 只表示在不复用已消费授权对象的前提下可安全重新发起等价读取，或在重新扫描、建计划和确认后再次尝试业务动作；单次 `execute_plan` 一旦认领，无论错误是否瞬时，对同一 plan 都固定为 `retryable=false`。`OperationItemResult.retryable=true` 仅表示该对象在新快照/新计划中可能再试，不授权重放旧 item。

`safeDetails` 只允许计数、限制值、布尔值、稳定枚举和不透明本地 ID；禁止文件内容、文件名、路径、文件哈希、SID、令牌、URL、命令行、用户标签、扫描/审计内容和原始 Win32 错误。Windows 原始错误及敏感诊断值只写入受保护本地审计。

---

