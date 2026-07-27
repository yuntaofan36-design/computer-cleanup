# 清盘 Windows 磁盘清理工具开发设计文档

<a id="qpn-sec-0"></a>
## 0. 文档信息

| 字段 | 内容 |
|---|---|
| 文档 ID | `QPN-DOC-DESKTOP-001` |
| 文档状态 | Draft，供产品、开发、安全、测试和发布评审 |
| 文档版本 | 0.8 |
| 最后更新 | 2026-07-25 |
| 产品名称 | 清盘（Qingpan） |
| 目标版本 | V1；V1.x/V2 仅为非承诺候选方向 |
| 代码基线 | `M0-WORKTREE-20260723-01`；精确 commit、工作树和锁文件摘要见第 3.2.1 节；当前为 `presentUnverified`，尚未归档可重放 snapshot bundle，不能作为已通过 M0 的发布基线 |
| 目标读者 | 产品负责人、桌面端开发、Windows/Rust 开发、安全评审、测试和发布工程师 |
| 目标用户 | 个人用户及单机办公用户 |
| 文档负责人 | 清盘桌面客户端技术负责人（责任角色；具体姓名在项目评审系统绑定） |
| 必需评审角色 | 产品、桌面端、安全、测试、发布负责人 |

<a id="qpn-sec-0-1"></a>
### 0.1 版本记录

| 版本 | 日期 | 说明 |
|---|---|---|
| 0.1 | 2026-07-23 前 | 以竞品能力聚合为主的产品愿景稿 |
| 0.2 | 2026-07-23 | 按当前实现重构为可评审、可开发、可验收的设计文档 |
| 0.3 | 2026-07-23 | 补齐隔离 WAL、五类状态机、目标命令/契约、能力包络、更新回滚和追踪矩阵 |
| 0.4 | 2026-07-23 | 闭合通用计划、隔离对账、供应链撤销、应用更新恢复、错误码和机器追踪契约 |
| 0.5 | 2026-07-23 | 闭合冻结提权执行包、耐久恢复授权、卸载外部进程事务、多用户更新与发布证据契约 |
| 0.6 | 2026-07-23 | 同步删除/恢复/调度/更新协议，补齐稳定错误、验收需求、故障注入与无上下文复核闭环 |
| 0.7 | 2026-07-24 | 将单体文档拆为领域文档集，并落地首批扫描会话、不可变清理计划和一次性执行代码切片 |
| 0.8 | 2026-07-25 | 落地稳定文件身份复检、实验性同卷隔离仓库、真实库存与不覆盖副本导出纵向切片；保持生产级 QPC1/M1-04 为未完成 |

<a id="qpn-sec-0-2"></a>
### 0.2 文档使用约定

- 本文中的“已实现”仅表示当前工作树中存在可调用代码，不等同于已经通过发布验收。
- 第 3.2 节所有“已实现/部分实现”在没有绑定通过的 run ID 与证据前统一解释为 `presentUnverified/partial`；发布材料不得把它们写成“已交付”。正式 M0 批准必须绑定第 3.2.1 节定义的完整 baseline manifest、可重放 snapshot bundle、capability evidence 及 `M0BaselineVerificationRecord v2`，不能只引用日期、分支名或当前目录。
- 本文处于 `Draft` 时，“目标设计”均表示拟议的 V1/M1 目标；只有本文变为 `Approved`，且精确版本/SHA-256 的 `DesignDocumentApprovalRecord v2` 通过 GATE-010 后，才构成批准基线。第 15.3、15.4 节的 V1.x/V2 候选方向不属于“目标设计”，不构成版本、日期或能力承诺。
- 规范性要求使用“必须”“不得”“仅允许”；建议使用“应当”；说明性内容使用“可以”。
- 每条可独立验收的需求使用稳定编号：产品/工程需求见第 9 章，安全不变量使用 `INV-*`，发布门禁使用 `GATE-*`，里程碑产物使用 `M1-*`。其他章节是这些编号的展开，不新增无编号交付承诺。
- 代码、规则、测试用例、发布检查表和豁免记录应引用对应需求编号；第 13.6 节维护全链路映射。
- 竞品内容仅作为公开行为和用户需求的参考，不代表复制其实现、规则库、商标或专有素材。

<a id="qpn-sec-0-3"></a>
### 0.3 草案复核记录（非正式批准）

| 日期 | 评审视角 | 结论 | 已处理的主要问题 | 状态 |
|---|---|---|---|---|
| 2026-07-23 | 无上下文范围理解 | 主范围可理解，部分风险/路线口径漂移 | 统一大文件、重复文件、自动任务与恢复语义 | 已修订，待最终复核 |
| 2026-07-23 | 可实施性 | 可拆 Epic，原协议不足以直接拆工单 | 补目标命令、硬上限、五类状态机、错误与追踪矩阵 | 已修订，待最终复核 |
| 2026-07-23 | 安全与隐私 | 能力包络、隔离事务、IPC、网络和更新需加固 | 补应用内包络、WAL、同 SID UAC、Rust 出站和更新日志 | 已修订，待最终复核 |

正式批准需由第 0 节所列五个必需评审角色分别产生第 13.6 节可验签的 detached signature，或由受信项目评审系统产生可回放的签名 receipt；五个角色必须映射到五个不同自然人身份。批准结果保存为文档外部的 `release/design-document-approval-record.json`，由 `ReleaseGateManifest v4` 引用原始文件和 canonical payload 摘要，不能写回被批准的 Markdown 后再循环重算。无上下文读者测试、本表和当前 `Draft` 均不构成批准证据。

---


---

## 文档集阅读路径

原长文件名保留为文档集入口。规范正文按领域拆分，章节编号、需求 ID、门禁 ID 和里程碑 ID 保持不变。

| 顺序 | 专题 | 规范章节 |
|---|---|---|
| 1 | [产品范围、现状基线与关键决策](docs/design/01-product-scope.md#qpn-sec-1) | 第 1-4 章 |
| 2 | [安全、恢复、架构与状态机](docs/design/02-safety-architecture.md#qpn-sec-5) | 第 5-7 章 |
| 3 | [运行时 API 与数据契约](docs/design/03-runtime-api.md#qpn-sec-8) | 第 8 章；正式类型继续按领域拆分 |
| 4 | [需求、规则与支持矩阵](docs/design/04-requirements.md#qpn-sec-9) | 第 9-12 章 |
| 5 | [测试、发布门禁、更新与运维](docs/design/05-test-release.md#qpn-sec-13) | 第 13-14 章；发布契约继续按领域拆分 |
| 6 | [路线图与退出条件](docs/design/06-roadmap.md#qpn-sec-15) | 第 15 章 |
| 7 | [竞品、风险与术语附录](docs/design/07-appendices.md#qpn-app-a) | 附录 A-C |

## 当前代码落地里程碑

- 文档集当前状态仍为 `Draft`，拆分本身不构成正式批准。
- 2026-07-24 已落地首批纵向代码切片：Rust [cleanup_plan](desktop/src-tauri/src/cleanup_plan/mod.rs) 与 [commands/cleanup](desktop/src-tauri/src/commands/cleanup/mod.rs) 分离领域和 Tauri 适配，前端 [cleanup-plan feature](desktop/src/features/cleanup-plan/index.ts) 分离类型、API、预览适配器和测试；旧的按条目 ID 直接执行入口不再注册。该事实仍按 `presentUnverified` 管理，直至测试结果和构建产物进入正式证据归档。
- 2026-07-25 已落地第二个纵向切片：共享 [fs_safety](desktop/src-tauri/src/fs_safety/mod.rs)、实验性 [quarantine](desktop/src-tauri/src/quarantine/mod.rs)、独立 [commands/quarantine](desktop/src-tauri/src/commands/quarantine/mod.rs) 和前端 [quarantine feature](desktop/src/features/quarantine/index.ts)。它仅支持 `temp` 规则、同卷、明文对象和固定目录副本导出，不等于生产级 QPC1 或 M1-04；精确边界见[实现说明](docs/design/implementation/2026-07-25-quarantine-preview.md)。
- 当前代码事实与证据入口见[第 3.2 节](docs/design/01-product-scope.md#qpn-sec-3-2)和[第 3.2.1 节](docs/design/01-product-scope.md#qpn-sec-3-2-1)；未绑定通过证据的能力仍是 `presentUnverified/partial`。
- [M0](docs/design/06-roadmap.md#qpn-sec-15-1)当前仍为 blocked；[M1](docs/design/06-roadmap.md#qpn-sec-15-2)可并行开发子项，但在 M0 验证、全部门禁和发布证据闭合前不得宣称完成。
- 代码落地以[第 9 章稳定需求 ID](docs/design/04-requirements.md#qpn-sec-9)、[第 13.6 节追踪矩阵](docs/design/05-test-release.md#qpn-sec-13-6)和[第 15 章退出条件](docs/design/06-roadmap.md#qpn-sec-15)为准。

所有专题文件均继承本页的文档 ID、版本和状态；正式批准与发布证据必须覆盖完整文档集，不能只计算本索引文件。
