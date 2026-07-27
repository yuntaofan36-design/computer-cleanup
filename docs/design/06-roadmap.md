# 清盘设计文档：路线图与退出条件

> 文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 规范章节：第 15 章
> 状态与版本继承自主索引；本文件不单独构成批准对象。

[上一篇：测试与发布](05-test-release.md#qpn-sec-13) · [主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>) · [下一篇：附录](07-appendices.md#qpn-app-a)

---

<a id="qpn-sec-15"></a>
## 15. 路线图与退出条件

<a id="qpn-sec-15-1"></a>
### 15.1 待验证的 2026-07-23 M0 候选基线

范围（2026-07-23 冻结候选）：允许列表扫描、快照复检、空间分析、大文件扫描/永久删除、重复文件只读识别、应用卸载、分区只读和本地审计。当前 2026-07-25 工作树已超越该冻结观测；不得将本候选误引为当前状态或发布基线。

退出条件：本文第 3.2 节状态与代码、测试一致；任何未完成 UI 不再以已交付能力展示。**当前 M0 状态为 blocked**：第 3.2.1 节尚未归档闭合 baseline manifest、可重放 snapshot bundle、完整 untracked manifest、绑定 run ID/摘要的 capability evidence 和 `M0BaselineVerificationRecord v2(result=passed)` 五角色可认证记录；这些工件缺失时不得仅凭当前工作树、可重放 bundle 或本文件宣称 M0 已通过。

<a id="qpn-sec-15-2"></a>
### 15.2 M1：V1 安全闭环

M1 总体有一个不可替代的共同前置：M0 必须由 `M0BaselineVerificationRecord v2(result=passed)` 证明，且第 3.2.1 节的 `BaselineManifest v1`、可重放 snapshot bundle、完整 tracked/untracked implementation manifest、lockfile/toolchain、每个 capability 的 run ID/证据 SHA-256、M0 CI provenance、有效治理策略和五角色可认证批准都已归档、复算并通过。M1 子项可在 M0 blocked 时并行开发或生成局部证据，但不得宣称任何 M1 总体退出、GA 基线或可发布结论；当前因 M0 blocked，M1 总体同样 blocked。

| 交付项 | 前置依赖 | 产物 / API | 验收 ID | 发布证据 | 开发起始状态 / V1 开放条件 |
|---|---|---|---|---|---|
| M1-01 计划与身份闭环 | M0 passed、状态仓库 schema | 第 7 章状态机、`create/authorize/execute_plan`、根/父链固定、同句柄复检和唯一结果聚合 | SAFE-001~005、CLEAN-003、PLAN-001、RESULT-001、API-001~003、PAGE-001~002、REL-001 | T-PLAN、T-FS/T-FMUT、T-RESULT、T-API、T-PAGE、T-STATE-001 | 写执行关闭，门禁通过后开放 R1 |
| M1-02 签名规则供应链 | M1-SVC-02 staging passed、应用内能力包络、恢复密钥流程 | 签名 index、key authorization/revocation、规则双槽、sticky 高水位和 Rust 更新客户端 | RULE-001~006 | T-RULE、T-SVC-002、恢复签名仪式记录 | 开发期远程规则关闭；规则服务 staging、生产 origin 与 RULE/NET 门禁通过后 stable 必须开放，否则不满足 M1 退出 |
| M1-03 低风险缓存清理 | M1-01、M1-02 | R1 规则、预览和结果核算 | CLEAN-001~004、UI-001 | T-CACHE、T-UI | 手动 R1 开启；通用 Temp 不纳入 R1 |
| M1-04 R2 隔离与导出副本 | M1-01、每卷仓库 | QPC1 v1、DPAPI DEK、copy/full-verify/delete WAL、content guard、partial artifact/quota 对账、批量导出/救援、R4 容器清除 | REC-001~007、R4-001、API-002~003、REL-001、UI-001 | T-QUAR-001~026、T-QUOTA、T-BATCH、T-R4-PURGE、T-RESTORE-IDEMP | 已有 `temp` 同卷明文 preview 供开发验证，但不满足本行任何退出条件；R2 与隔离清除保持发布关闭，QPC1 vectors、真实 mapped-view 和各自门禁通过后独立开放 |
| M1-05 分析与排除 | 根授权、原生设置存储 | 空间/大文件/重复文件 R0、三扫描 slot 协调、大文件 R2 隔离、原生持久化排除 | SCAN-001~002、STORAGE-001、LARGE-001、DUP-001~002、EXCL-001 | T-SCAN-001~002、T-STORAGE、T-LARGE、T-DUP、T-EXCL | R0 开启；重复文件始终只读；原文件 R4 删除不在本项 |
| M1-06 应用、启动项与分区 | 安全卸载快照、machine broker/store | MSI singleton-context/AppX 适配器、user/machine journal、跨管理员 attachment、typed reboot/result；启动项/分区只读 | APP-001~003、STARTUP-001、PART-001 | T-APP-001~013、T-APP-018~019、T-STARTUP、T-PART | MSI/AppX 门禁通过后开放；Win32 不计入本行；其他项隐藏 |
| M1-07 提权边界 | M1-01、签名安装目录 | 一次性助手、命名管道、冻结 bundle、高完整性确认 | IPC-001~002 | T-IPC | R3 适配器逐项关闭，评审后开放 |
| M1-08 审计、隐私与授权 | M1-SVC-01 staging passed、原生安全存储、Rust 网络客户端 | 脱敏审计、出站策略、同 body/新 proof 激活 PoP、普通 PoP/强保护停用双密钥和 challenge | AUDIT-001、NET-001、LICENSE-001~003 | T-AUDIT、T-NET、T-LICENSE-001~004、T-SVC-001、抓包报告 | 遥测关闭；授权服务 staging 未通过、activation proof 负测失败或开发旁路存在时构建失败 |
| M1-09 任务计划 | M1-01~03、持久分析策略、批准 grant | Windows 任务计划 R0/R1 一次性 runner、永久绑定、run claim、upsert/delete WAL | AUTO-001~006 | T-AUTO-001~015、T-IDEMP-002、计划任务导出 | 默认关闭；R2-R4 硬拒绝 |
| M1-10 签名应用更新 | M1-SVC-03 staging passed、签名 bootstrapper、machine-global anchor、高完整性协调器、多用户迁移 | callArmed floor、`AppUpdateJournal/MachineFullInstallerJournal/MachineProductUninstallJournal`、原生架构拒绝、trial、精确 LKG/三来源可信恢复与卸载 absence 终态 | INV-013、UPDATE-001~013、REL-001 | T-UPDATE-001~042、T-SVC-003 故障注入 | 开发期关闭；更新服务 staging 与门禁通过后 MSI stable 可限流分批但能力必须存在，NSIS 实例明确不支持自动迁移 |
| M1-SVC-01 授权服务 | 授权服务设计和部署流水线 | `QPN-SVC-LIC-001` staging/production `ServiceDeliveryRecord` | SVC-001 | T-SVC-001、contract/negative/packet/E2E run | owner/设计/origin/artifact/隐私或运行证据任一缺失均 blocked |
| M1-SVC-02 规则控制面 | 规则服务设计和部署流水线 | `QPN-SVC-RULE-001` staging/production `ServiceDeliveryRecord` | SVC-002 | T-SVC-002、签名/撤销/回滚/隐私 E2E run | 仅客户端测试不能通过；production record 前远程规则保持关闭 |
| M1-SVC-03 应用更新控制面 | 更新服务设计和部署流水线 | `QPN-SVC-UPDATE-001` staging/production `ServiceDeliveryRecord` | SVC-003 | T-SVC-003、manifest/包/撤销/回滚/隐私 E2E run | 仅客户端测试不能通过；production record 前应用更新保持关闭 |
| M1-11 平台与发布工件 | M0 verification passed、M1-01~10、M1-SVC-01~03 production passed | Win10/11 x64/ARM64 包、SBOM/findings/dispositions/依赖报告、性能报告、`PlatformTupleRegistry v3`、`FormalContractRegistrySnapshot v2`、`ReleaseCapabilityManifest v2`、治理策略、M0 三文件、源码派生/CI provenance、六服务 records 和当前文档 `DesignDocumentApprovalRecord v2`，由唯一 `ReleaseGateManifest v4` 汇总并由 gate 外 `SignedReleaseAttestation v1` 签署 | API-004、PLAT-001~002、PERF-001、RELEASE-001、UI-001~003、GOAL-006、GATE-001~010 | T-CONTRACT-001~009、T-DOC-001、T-PLAT-001~004、T-RELEASE-001、T-UI-001~005、T-WIN-001~008、T-SVC-001~003、性能基准与发布追踪证据 | 任一客户端、服务、252 test obligation、required tuple、M0 三文件、19 个顶层文件、传递 refs、实际 capability/SBOM/依赖配置、源码派生、可信签名或文档批准未通过均不得发布；P1 也不得豁免 |

退出条件：M0 已由 `M0BaselineVerificationRecord v2(result=passed)` 证明，且第 3.2.1 节三份 M0 文件及全部内部证据均已归档、复算并由五角色批准；第 13.5 节全部门禁通过，`ReleaseGateManifest v4` 的 19 个固定顶层文件、六服务记录和全部传递 refs 闭合，`SignedReleaseAttestation v1` 由外部 trust root 授权，252 个发布测试义务均已物化，且第 15.2 节每个 M1 交付项列出的全部验收 ID（包括 P1）均有通过证据。关闭、隐藏或豁免 M1 必交能力不能满足退出条件；确需移出时，必须先由必需评审角色正式批准并同步修订第 1.2 节 V1 范围、第 13.6 节追踪矩阵、`RELEASE_CAPABILITY_POLICY/RELEASE_TEST_OBLIGATIONS` 和本节 M1 基线，再重新执行受影响门禁。“开发起始状态”不是 GA 豁免：签名规则更新或 MSI 应用更新的生产 origin、服务和门禁未就绪时，M1 不得退出；应用更新可以按用户比例限流，但发布制品中的完整信任/回滚路径必须可验收。大文件不得绕过 R2 隔离；R2 未就绪时相关写入口保持关闭。聊天数据库、媒体和文件不属于 M1，当前工作树中的相关写入口保持关闭。M1 必须交付的 R4 仅是已验证隔离记录的清除；原文件永久删除属于另一个 `actionKind`，只有单独通过 GATE-008 后才可由发布配置开放高级入口，并始终要求独立确认。

第 15.3、15.4 节仅记录经产品评审保留的候选方向，不构成 V1.x/V2 已承诺范围、发布日期或发布验收基线。任一方向只有在另立并批准包含稳定需求/里程碑 ID、支持矩阵、能力开关、专项门禁、测试定义和发布证据的设计后，才可进入具体版本；在此之前 UI、官网和发布说明不得宣称支持。

<a id="qpn-sec-15-3"></a>
### 15.3 候选路线：V1.x 高风险系统能力（非承诺）

候选方向 A：

- 注册表只读诊断、证据展示和导出。
- 系统组件、驱动和还原点只读盘点。

候选方向 B：

- 仅对通过专项安全评审的规则开放逐项修复。
- 注册表有单项备份和恢复；系统组件/驱动只调用受支持接口。

进入具体 V1.x 版本承诺的准入条件：每类能力另立独立威胁模型、稳定需求与测试 ID、能力开关、回滚演练、Windows VM 矩阵和真实升级/卸载场景测试；未完成并获批前仅为研究项，不得提供入口或发布声明。

<a id="qpn-sec-15-4"></a>
### 15.4 候选路线：V2 安全擦除（非承诺）

候选方向：

- HDD 尽力覆写，明确能力限制。
- 满足第 11.3 节全部前置条件时的全盘加密密码学擦除指导。
- 设备明确支持时的 ATA/NVMe sanitize 独立流程。

进入具体 V2 版本承诺的准入条件：目标设备识别、掉电恢复、防选错盘、不可逆确认、厂商能力矩阵及对应稳定需求/测试 ID 均已另文批准；未满足前不得开放或宣传。SSD 不规划单文件多次覆写模式。

<a id="qpn-sec-15-5"></a>
### 15.5 后续独立方案

企业集中策略、管理员部署、RBAC、集中审计、远程执行和数据驻留需要独立架构与隐私评审，不纳入 V1/V1.x 本地客户端设计。

---

