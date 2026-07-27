# 清盘设计文档：产品范围与现状基线

> 文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 规范章节：第 1-4 章
> 状态与版本继承自主索引；本文件不单独构成批准对象。

[主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>) · [下一篇：安全与架构](02-safety-architecture.md#qpn-sec-5)

---

<a id="qpn-sec-1"></a>
## 1. 执行摘要

<a id="qpn-sec-1-1"></a>
### 1.1 产品定位

清盘是一款面向 Windows 10/11 64 位系统的本地磁盘分析与清理工具。产品的核心原则是：

> 看清空间去向，只处理有明确证据的内容；分析与执行分离，未知内容默认保留。

V1 聚焦以下结果：

1. 识别允许列表中的 Windows、浏览器和应用可重建缓存。
2. 在执行前展示命中依据、路径、大小、风险、影响和恢复方式。
3. 基于最近一次扫描快照执行，并在删除前重新校验路径和文件身份。
4. 提供只读空间分析、大文件发现和重复文件确认能力。
5. 按风险使用只读分析、直接删除、产品隔离、系统备份或不可逆操作。
6. 在本机保存脱敏操作记录，不上传文件或扫描信息。
7. 仅为授权、签名规则包和签名应用更新建立最小联网通道。

本产品不承诺“零误删”“绝对安全”“全覆盖”或“无法恢复”。安全目标通过保守默认、失败关闭、恢复能力和测试门禁来实现。

<a id="qpn-sec-1-2"></a>
### 1.2 V1 范围

V1 包含：

- Windows 10/11 64 位 x64、ARM64 安装包。
- 允许列表缓存扫描、预览、计划和执行。
- 风险分级、应用进程保护、排除规则和执行前复检。
- 磁盘占用分析、大文件发现，以及完整 SHA-256 后逐字节终检的重复文件确认。
- 用户主动选择的大文件可创建非默认 R2 隔离计划；不自动勾选、不进入默认缓存清理或自动任务。
- 已安装应用枚举，以及 MSI 和 AppX/MSIX 受支持接口卸载；通用 Win32 EXE 卸载仅在专项门禁通过的发布构建开放。
- 启动项只读枚举；V1 不启用、禁用或删除启动项。
- 物理磁盘和分区的只读展示，以及打开 Windows 磁盘管理。
- 本地审计、导出隔离副本（不还原原位置，隔离源保留）、签名规则更新、签名应用更新。
- 对 `committed/sourceRetained` 且加密容器身份和完整性可验证的隔离记录提供独立 R4 手动清除：每批 1 至 500 项、默认不选中、原生二次确认、不得自动化。该动作只删除产品加密容器，不是安全擦除；它不同于条件性的原文件永久删除，也不同于 V2 设备擦除。
- 预先配置的 R0 只读分析任务和用户预先批准的 R1 清理任务；默认均不开启。

本节“V1 包含”描述的是 M1 退出时必须存在并通过门禁的目标能力，不表示当前工作树已经交付，也不表示开发期初始功能开关已经开放。生产 origin 未配置、签名更新链未通过门禁，或任一 **M1 必交能力** 仍被发布配置关闭时，均不得宣称 V1/M1 已完成；明确标记“P0（条件）”且不属于 M1 默认交付的能力可按 GATE-003/GATE-008 保持关闭。

V1 不包含：

- 注册表自动修复。
- 聊天数据库、聊天媒体和聊天文件的主动清理；当前工作树中的相关写入口必须保持关闭，未来需按产品、版本和精确数据根另立需求与兼容矩阵。
- 直接删除 Windows 组件仓库、驱动仓库、还原点或启动关键文件。
- 磁盘碎片整理。
- SSD 单文件“多次覆写”或任何不可验证的取证级擦除承诺。
- 企业集中策略、远程执行、集中日志上报和多租户管理。
- 病毒查杀、恶意软件判断或文件内容云分析。
- 网络盘、云端副本和其他设备上的数据清理。
- 重复文件的自动整理或删除；V1 只提供只读确认和保留建议。

<a id="qpn-sec-1-3"></a>
### 1.3 成功标准

| 编号 | 目标 | V1 验收标准 |
|---|---|---|
| GOAL-001 | 未知内容默认保留 | 未命中有效签名规则的路径不能进入默认清理计划 |
| GOAL-002 | 扫描和写执行隔离 | `execute_plan` 只接受未过期、未消费的通用计划 ID，不接受快照/记录 ID、任意路径或命令；扫描候选、卸载和隔离清除先封装为第 8.3 节不可变 `PlanItem`，恢复使用受限目标 grant/record ID 协议 |
| GOAL-003 | 文件变化失败关闭 | 路径、父目录链或可观察的文件身份、大小、时间、USN、链接和流信息发生变化时跳过该文件 |
| GOAL-004 | 风险与恢复一致 | 每个候选项显示风险、执行方式和恢复能力，执行结果分别统计回收估算、可用空间变化观测与隔离占用 |
| GOAL-005 | 本地隐私 | 抓包测试确认本地用户文件内容、文件名、完整路径、文件哈希、扫描结果和本地日志不会外发；规则与应用制品摘要仅可用于固定更新端点 |
| GOAL-006 | 可发布 | 支持矩阵、安全语料、故障注入、升级回滚和代码签名门禁全部通过 |

---

<a id="qpn-sec-2"></a>
## 2. 用户、场景与边界

<a id="qpn-sec-2-1"></a>
### 2.1 目标用户

1. 希望安全释放系统盘空间的个人用户。
2. 需要了解磁盘占用但不熟悉 Windows 目录结构的用户。
3. 需要在单台办公电脑上整理缓存、大文件和已安装应用的用户。
4. 需要保留本机操作记录、但不需要集中管控的专业用户。

企业集中策略、管理员强制执行、集中审计和数据驻留属于独立企业版方案，不通过扩展本文中的本地授权后台来隐式实现。

<a id="qpn-sec-2-2"></a>
### 2.2 核心任务流

| 场景 | 用户目标 | 默认路径 |
|---|---|---|
| 安全释放空间 | 清理明确可重建的缓存 | 扫描 → 预览 → 选择 → 复检 → 执行 → 查看结果 |
| 找出空间大户 | 理解目录和类型占用 | 选择本地磁盘 → 只读分析 → 逐层浏览 |
| 处理大文件 | 手动确认单个用户文件 | 只读发现 → 查看敏感性 → 显式选择 → 默认 R2 隔离；条件 R4 能力另行确认 |
| 查找重复文件 | 找出字节内容一致的文件 | 大小分组 → 采样 SHA-256 → 完整 SHA-256 → 逐字节终检 → 只读建议 |
| 卸载应用 | 启动原厂或系统卸载器 | 枚举注册表快照 → 查看信息 → 确认 → 启动卸载器 |
| 导出隔离副本 | 将仍在隔离仓库的内容复制到新目录供人工核对；不还原原位置、不删除隔离源 | 隔离与导出中心 → 选择 `committed/sourceRetained` 记录（包括 retention 已到期项）→ 导出副本到新建目录 → 人工核对 → 如需释放隔离占用，另行确认清除 |

<a id="qpn-sec-2-3"></a>
### 2.3 设计边界

- UI 不判断文件是否安全，也不构造原生命令。
- 规则引擎不执行脚本，不接受远程下发的任意绝对路径。
- 只读分析页面不提供隐式删除入口。
- 默认清理不包含 Cookie、密码、登录令牌、浏览历史、聊天记录、下载内容或用户文档。
- 云占位文件、网络路径、重解析点、EFS 文件和无法确认身份的对象默认跳过。
- 分区写操作交给 Windows 磁盘管理；清盘只展示经过校验的只读信息。

---

<a id="qpn-sec-3"></a>
## 3. 当前实现基线与差距

<a id="qpn-sec-3-1"></a>
### 3.1 当前技术边界

仓库由三个运行边界组成：

- `desktop/`：Tauri 2、React、TypeScript 和 Rust 桌面客户端。
- `server/`：Express 和 SQLite 授权服务。
- `web/`：授权管理后台。

本设计主文覆盖桌面客户端，并规定三类在线控制面的客户端契约与发布前置条件。服务端内部数据模型、管理员权限、部署和运维由具名服务设计维护，但“另文”不能是不可追踪豁免：授权、规则和应用更新服务分别使用保留 ID `QPN-SVC-LIC-001`、`QPN-SVC-RULE-001`、`QPN-SVC-UPDATE-001`。三份服务设计当前均为 `version=TBD/owner=TBD/status=blocked`；第 5.7、13.6 和 15.2 节要求的记录未变为 `passed` 前，依赖它们的 M1 能力及 M1 总体均不得退出。

<a id="qpn-sec-3-2"></a>
### 3.2 能力矩阵

| 能力 | 当前工作树状态 | 证据与限制 | V1 目标 |
|---|---|---|---|
| Tauri/React/Rust 客户端 | 已实现 | 原生构建和浏览器预览已分离 | 保持边界，预览数据不得伪装为原生结果 |
| 内置允许列表缓存扫描 | 已实现 | 规则限定 Local/Roaming/微信数据根和明确叶子目录；当前不是签名规则 | 迁移为可签名、受限的规则清单 |
| 扫描快照复检 | 已实现，需增强 | 缓存和大文件路径已共享卷/File ID、规范路径、父目录链、大小、修改时间、单硬链接、ADS、重解析点和云占位复检；仍没有固定句柄删除或 oplock | 迁移到同句柄/固定目录句柄协议并完成故障注入门禁 |
| 应用进程保护 | 已实现 | 浏览器、VS Code、Discord、Figma、微信运行或状态不明时保守跳过 | 将进程保护写入规则契约并覆盖执行阶段 |
| 空间占用分析 | 已实现 | 本地只读、资源有上限、支持取消 | 完成兼容矩阵和基准测试 |
| 大文件扫描 | 已实现 | 排除重解析点、云占位和无法建立稳定身份的文件 | 默认只读；永久删除作为 R4 高级动作 |
| 大文件永久删除 | 已实现，风险待收敛 | 使用最近扫描快照和明确确认，不进入回收站 | M1 迁移为 R2 隔离；原文件 R4 默认关闭，仅按 R4-DELETE-001 条件开放 |
| 重复文件识别 | 已实现，只读 | 大小、头尾采样 SHA-256、完整 SHA-256；排除硬链接别名和异常数据流 | 增加稳定句柄逐字节终检；V1 只提供分组证据和保留建议 |
| 应用枚举与卸载 | 已实现 | 仅调用最近注册表快照中的卸载命令，不接受前端命令行 | MSI 与 AppX/MSIX 受支持适配器为 V1 必交；通用 Win32 EXE 仅在 APP-004 专项门禁通过的构建中条件开放，并增加确认、完成状态和失败审计 |
| 分区管理 | 已实现，只读 | 展示布局并打开 Windows 磁盘管理 | 保持只读，不在客户端实现分区写入 |
| 本地审计 | 部分实现 | JSONL 可追加并容忍损坏行；当前敏感字段未统一加密 | 增加字段最小化、ACL、DPAPI 和保留策略 |
| 隔离与导出中心 | 部分实现 | 已有仅限 `temp`、同卷、100 文件/1 GiB 的 `quarantine-preview-v1`：哈希链日志、验证复制后删除、真实库存和固定新目录副本导出；对象未加密，未达到 QPC1/M1-04 | 实现 QPC1、DPAPI、quota/content guard、每卷仓库、正式目标 grant、批量导出与清除门禁 |
| 排除规则 | 部分实现 | 用户排除项保存在前端 localStorage，只用于部分分析任务 | 迁移到原生持久化，统一应用于所有扫描入口 |
| 启动项管理 | 部分实现 | 可读取当前用户 Run 项；启用/禁用命令为占位实现，前端未接线 | V1 只读；写操作另行安全设计 |
| 自动清理 | 未实现 | UI 固定关闭，未提供设置或任务接口 | 使用 Windows 任务计划程序，仅允许固定 R0 分析或完整策略绑定的预批准 R1 |
| 应用更新 | 部分实现 | 插件和 UI 已接入，但公钥及端点为空 | 配置签名更新、失败回滚和发布通道 |
| 签名规则更新 | 未实现 | 当前规则编译在客户端代码中 | 实现能力包络、签名验证、双槽激活和更高序号修复 |
| 授权校验 | 部分实现 | 已有设备激活和令牌校验；开发离线口令及 localStorage 令牌不得进入生产包 | 生产环境仅 TLS，移除开发旁路，凭据交由系统安全存储 |
| 生产级加密隔离、注册表修复、系统组件/驱动清理、安全擦除 | 未实现 | 实验性隔离切片不得作为 QPC1 或 M1-04 已交付能力宣传 | 按第 15 章门禁分阶段交付 |

<a id="qpn-sec-3-2-1"></a>
#### 3.2.1 可复现基线与当前快照

正式基线使用第 13.6 节闭合的 `BaselineManifest v1`，固定 19 个 `BaselineCapabilityId`，而不是“至少包含”的开放对象。每个 capability 必须记录 `verified/presentUnverified/partial/absent`、实现文件与 symbol、blob SHA-256、feature/release gate、当前命令、限制、验证命令、run ID、证据路径/SHA-256 和 owner role。`manifestDigestSha256` 只对排除自身的 RFC 8785 canonical payload 计算。CI 以二进制方式重放 snapshot bundle，复算 commit、patch、未跟踪实现文件、lockfile、workspace manifest，并按第 13.6 节固定成员、排序和 domain 复算 capability evidence root；任一缺失或摘要不符使基线无效。

正式文件固定为 `quality/baselines/<baselineId>/baseline-manifest.json` 与 manifest 所指向的单一 snapshot bundle。可重放只证明材料一致，不等于 M0 `passed`；只有闭合的 `M0BaselineVerificationRecord v2(result=passed)` 才表示正式通过。该记录必须绑定 baseline 原始文件/canonical 摘要、bundle 原始摘要、workspace/capability evidence root、source commit、可解析的 M0 CI provenance、受外部 trust root 认证的治理策略，以及五个不同自然人角色对同一精确 approval statement 的可验证批准。M1 发布根分别绑定三份 M0 文件，要求所有内部/外部摘要相等；缺记录、错角色、重复自然人、不可验签证据、错摘要、`blocked` 或仅有可重放 bundle 都不能宣称 M0 通过。

下表是 2026-07-23 的冻结观测值，已被 2026-07-24/25 开发工作树切片超越，不代表当前文件摘要。本轮仍不创建 `quality/baselines/` 产物；`snapshotBundlePath/snapshotBundleSha256/toolchain/runId/evidencePath` 为 `TBD`，因此 `reproducible=false`、M0 退出被明确阻断：

| 字段 | 冻结值 |
|---|---|
| `baselineId` | `M0-WORKTREE-20260723-01` |
| `sourceCommit` | `57e2245060da4691e90aff22373d0bf6ff8f0d8c` |
| `trackedPatchSha256` | `f7e5d377e8e217a55e2e741b20f18e431e02ae48de0e0701aec8ed59576057aa`；对 `git diff --binary --full-index --no-ext-diff --no-textconv HEAD -- desktop server web package.json pnpm-lock.yaml pnpm-workspace.yaml` 的原始 stdout 字节计算 |
| `lockfiles` | `pnpm-lock.yaml = a014e92df8d6052cbdac3e14aba0d13f6c3355cb176bcdbf0176fe1466785df3` |
| `workspaceManifestSha256` | `76c5ebfba0765cd1d979757f7b62d9fce15517d17eda4630bd0f03d7d2949f24`；规范 JSON 包含本表 commit/patch/lockfile 及下列实现文件清单，不包含本文 |
| `snapshotBundle` | `TBD`；未归档，当前快照不可重放，正式评审不得签署 M0 |
| `m0BaselineVerificationRecord` | `TBD`；当前不存在 `result=passed` 的五角色验证记录 |

2026-07-23 冻结的 dirty implementation file manifest（路径按 manifest 规范顺序，状态只取 Git porcelain 两字符）为：

| 状态 | 路径 | SHA-256 |
|---|---|---|
| ` M` | `desktop/src/App.tsx` | `79d8b377b55978e716d13e68e314b97a3d39fbcc1e7db8f3aed149e8e72c57ba` |
| ` M` | `desktop/src/components.test.tsx` | `6ce410397c38a11e3d68d89125b8f286cf734d28cb3eecc329e81004b26d3ee5` |
| ` M` | `desktop/src/components.tsx` | `ea28e4ddaa2188ee4cd95a5ebd2ad075c59f1896c2cd31b8fd5d47b2c51c6d39` |
| ` M` | `desktop/src/native.test.ts` | `be6b8903779a2f9ed95ac332064dbc8317e0339ab1c3adb54fbdda704a5ddc41` |
| ` M` | `desktop/src/native.ts` | `0af5c46a4d0d5654f92047272f9294d1e28125dd0e317151e3b18871ef536e38` |
| ` M` | `desktop/src/pages/CleanupCenter.test.tsx` | `e13e050f0ffd150fb44ec53aac0ec9782470988c5bdb5ed4d4b39eafa94c1859` |
| ` M` | `desktop/src/pages/CleanupCenter.tsx` | `1cb282077142591099cafdb8d8e1e2a365f8b6cc7e97aeb56ee7d4209d0f98ca` |
| `??` | `desktop/src/pages/FileDiscovery.test.tsx` | `0fb02a747c6d18157bf13106dc36c0e61e530a180a8482a39feda29e79a4355b` |
| ` M` | `desktop/src/pages/FileDiscovery.tsx` | `eb97c89a9cda2caefddfd39f359272ab2e1377b5cc648f1d15fd96cb7489901a` |
| ` M` | `desktop/src/styles.css` | `522ad5511d32b079b467f4e6ac650df1c68939bedd966fa97ef377115cbd6e40` |
| ` M` | `desktop/src/types.ts` | `767a7a510f76179ae6c43c6710d4b4d36fdbc1903371735a862b6773842bdc14` |
| ` M` | `desktop/src-tauri/src/lib.rs` | `5da0066af42e88a4d3f56c10949c7c0ceca0dae01efed9b343b54ef14bbdd0c5` |
| ` M` | `desktop/src-tauri/src/models.rs` | `ec4e50aea93052aba54c7419f80714eea55d3d1dd2a8091e1dde123bd210d7eb` |
| ` M` | `desktop/src-tauri/src/scanner.rs` | `27116bd2ed8fbf3f2c1196018e9348f658677ff087dbab11a6acad0c502abc42` |
| ` M` | `desktop/src-tauri/src/storage.rs` | `8aed0fd757a5c0dc1fe856b8a366cdf713416670264aa1b51a1a13a4ebe10c1f` |

能力证据登记必须逐行关联第 3.2 节；以下只给出实现入口和待运行命令，不构成通过证据：

| 能力组 | 覆盖第 3.2 节条目 | 实现入口 | feature/release 状态 | 验证命令与当前证据 |
|---|---|---|---|---|
| `BASE-DESKTOP` | 客户端、扫描快照、进程保护、缓存、大文件/重复文件、空间分析 | `desktop/src-tauri/src/{lib,scanner,browsers,storage,models}.rs`；`desktop/src/{native,App}.ts(x)`；相关 pages | 运行时入口存在；无统一受保护 capability manifest | `pnpm --filter qingpan test`、`cargo test --manifest-path desktop/src-tauri/Cargo.toml`；run/evidence `TBD`，状态 `presentUnverified` |
| `BASE-APPS` | 应用枚举/卸载、启动项、分区 | `desktop/src-tauri/src/{apps,partitions,lib}.rs`；`desktop/src/pages/{AppManagement,DiskPartition}.tsx` | MSI/AppX/Win32 尚未达到目标协议；启动项写占位必须隐藏 | 同上；run/evidence `TBD`，状态 `partial` |
| `BASE-STATE` | 审计、隔离/导出、排除、自动任务 | `desktop/src-tauri/src/{audit,quarantine,commands/quarantine,lib}`；`desktop/src/features/quarantine/`；`desktop/src/pages/{RecoveryCenter,SettingsPage}.tsx` | 实验性明文隔离和真实副本导出已存在；QPC1、目标 grant、清除、scheduler 仍未实现 | 同上；run/evidence `TBD`，状态 `partial/absent` |
| `BASE-UPDATE-LICENSE` | 应用更新、签名规则更新、授权 | `desktop/src/{license,LicenseGate}.tsx?`、`desktop/src/pages/SettingsPage.tsx`、`desktop/src-tauri/tauri.conf.json`、`server/src/` | 生产 origin/key/服务证据未配置；开发旁路必须在发布构建失败 | `pnpm --filter qingpan-server test`、`pnpm build`；run/evidence `TBD`，状态 `partial` |

上表中的 `desktop/src/{license,LicenseGate}.tsx?` 是模式说明而不是路径字面量；manifest 生成器必须展开为实际存在的 `desktop/src/license.ts` 与 `desktop/src/LicenseGate.tsx`，拒绝 glob、问号或未解析路径。正式 manifest 还必须记录 Node/pnpm/rustc/cargo/Tauri CLI/Windows SDK 与 x64/ARM64 target 版本。M0 退出必须同时满足：snapshot bundle 可重放、19 个 capability key 完整、声称 `verified` 的行均有通过 run、`M0BaselineVerificationRecord v2` 的 CI、治理策略和五角色批准闭合；当前状态不满足。

<a id="qpn-sec-3-3"></a>
### 3.3 当前必须先关闭的风险

1. `temp` 已接入实验性隔离，但其他缓存和大文件动作仍主要使用永久删除；预览仓库未加密且没有生产 quota/content guard，不能进入 V1 发布配置。
2. 微信聊天、媒体和附件等高风险用户数据当前只读展示并由后端阻断；在生产级 R2 隔离完成前不得重新开放写入口。
3. README 中“尚未开放用户文件删除”的口径已与当前工作树不一致，发布前必须统一。
4. 前端存在“已签名规则”的说明，但代码中的规则仍为内置规则，签名更新尚未实现。
5. 更新器尚未配置签名公钥和发布端点，CSP 的 `https:` 也不是域名级出站白名单。
6. 授权客户端包含开发用途的离线旁路，令牌保存在 Web Storage；生产构建必须移除旁路并使用系统安全存储。
7. 当前本地审计是明文 JSONL；写入路径时必须先完成字段最小化和敏感字段保护。

---

<a id="qpn-sec-4"></a>
## 4. 关键设计决策

| 决策编号 | 决策 | 理由 |
|---|---|---|
| ADR-001 | V1 支持 Windows 10/11 64 位 x64、ARM64；不支持 Win7、Win8 和 32 位 | 与当前 Tauri 2、WebView2 和打包链路一致，避免不可维护的兼容承诺 |
| ADR-002 | 未知内容默认保留，失败策略为跳过 | 清理收益不能优先于用户数据和系统完整性 |
| ADR-003 | UI 只提交计划 ID/候选 ID，不提交任意删除路径或命令 | 缩小前端受攻击后的写入能力 |
| ADR-004 | R1 可直接删除，V1 的 R2 固定使用产品隔离，R3 使用受支持系统接口和专用备份，R4 单独不可逆确认 | 使风险、恢复和用户预期一致；V1 不依赖外部回收站恢复语义 |
| ADR-005 | 普通扫描及 R1/R2、无需提升的 R3 以当前用户权限运行；只有 `requiresElevation=true` 的 R3 使用按需、一次性提权执行器 | 避免常驻高权限服务和过宽权限，同时给无需管理员权限的受支持卸载接口明确执行归属 |
| ADR-006 | 远程规则使用无脚本清单、离线签名、单调序列和应用内能力包络；远程内容只能收窄能力 | 防止有效签名、误签或密钥泄露把规则更新变成任意文件删除通道 |
| ADR-007 | 仅授权、规则更新和应用更新允许联网，不提供默认遥测 | 保持可验证的本地隐私边界 |
| ADR-008 | 自动化通过 Windows 任务计划程序运行一次性任务，只允许预配置 R0 或绑定完整策略摘要的 R1 | 兼容“无常驻进程”和安全确认要求 |
| ADR-009 | 注册表、系统组件、驱动和擦除能力分阶段交付 | 每类能力需要不同的 Windows API、恢复模型和测试门禁 |
| ADR-010 | 竞品只作为需求参考，所有规则和实现独立设计 | 降低许可证、商标、供应链和错误继承风险 |

---

