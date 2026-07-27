# 2026-07-25 实现说明：实验性隔离与副本导出

> 状态：`presentUnverified/partial`。本文件记录当前工作树事实，不修改第 8.3.9 节生产级 QPC1 目标契约，也不构成 M1-04 或发布门禁通过证据。

## 1. 本轮落地范围

当前代码新增了一条可运行的纵向切片：

1. `scan_cleanup_v2` 保存带稳定文件身份的扫描快照。
2. `create_cleanup_plan` 生成一次性不可变计划。
3. 只有 `LocalAppData\Temp` 的 `temp` 规则可以进入实验性隔离执行。
4. 执行器验证并提交隔离对象后才尝试移除源文件。
5. 应用重启后可从真实隔离仓库列出记录。
6. 恢复中心可把有效对象导出到后端新建目录；不覆盖原路径，不删除隔离对象。

微信聊天、媒体、附件等高风险用户数据仍只读展示，并由后端 `blockedReason` 阻断。大文件仍未迁移到该隔离切片。

### 1.1 构建通道与写入门禁

该切片能否进入隔离暂存和源删除分派由编译通道决定：

| 构建通道 | 启用方式 | 实验性隔离写入行为 |
|---|---|---|
| Debug | 默认调试构建 | 可运行本切片，用于本地开发验证 |
| Internal | 非 Debug 构建显式启用 Cargo feature `internal-write-preview` | 可运行本切片，只用于受控内部验证，不是生产发布配置 |
| Production Release | 默认非 Debug 构建，未启用 `internal-write-preview` | 在下游隔离执行器分派前硬拒绝，不创建隔离暂存，也不尝试移除源文件 |

该门禁是编译通道约束，不接受 WebView 参数、环境变量或运行时设置降级。默认 Release 构建通过计划创建或 UI 操作也不能绕过执行前拒绝；本文后续“可运行”均仅指 Debug 或显式 Internal 构建。

## 2. 代码边界

| 领域 | 入口 |
|---|---|
| 文件安全公共能力 | `desktop/src-tauri/src/fs_safety/{identity,metadata}.rs` |
| 扫描快照和执行复检 | `desktop/src-tauri/src/scanner.rs` |
| 隔离日志、仓库、状态、暂存和导出 | `desktop/src-tauri/src/quarantine/` |
| 清理计划隔离分派 | `desktop/src-tauri/src/cleanup_plan/quarantine_executor.rs` |
| Tauri 隔离命令 | `desktop/src-tauri/src/commands/quarantine/` |
| 前端真实隔离库存和导出交互 | `desktop/src/features/quarantine/` |

每个领域使用独立模块入口；恢复页不再根据审计记录的 `stagedBytes` 推断可恢复性。

## 3. 当前安全流程

单文件执行顺序固定为：

`扫描身份快照 -> 执行前复检 -> Prepared/Copying 日志刷盘 -> 同卷对象复制 -> SHA-256 -> 对象刷盘 -> 对象全量复读 -> create_new 清单 -> ObjectCommitted 日志刷盘 -> 源身份与内容再次复检 -> SourceDeletePrepared 日志刷盘 -> 尝试移除源 -> 终态刷盘`

执行复检包含：

- 规则根、父目录链、规范路径和扫描快照路径一致；
- 卷与文件 ID、大小、修改时间一致；
- 普通文件、非链接、非重解析点；
- 硬链接计数必须为 1；
- 只允许默认数据流，拒绝 ADS；
- 拒绝 offline/recall 云占位属性；
- 源文件与隔离对象必须位于同一卷。

任一身份、元数据、数据流、哈希、刷盘或目录验证无法确定时失败关闭。源删除 API 返回失败后不会直接形成 `sourceRetained`：执行器必须再次完成候选身份复检、完整 SHA-256 内容复检和最终身份复检，三步均成功才能追加 `SourceRetained`。文件已消失、被替换、内容变化或任一后置证明无法完成时，不追加该终态；日志保留在 `SourceDeletePrepared`，重启后推导为 `recoveryRequired`，且不会自动重试删除。

实验性计划硬上限为 100 个文件、1 GiB；超过上限在计划创建阶段拒绝。

### 3.1 Preview 状态与重启处置

| 最后已刷盘状态/事件 | 可能的磁盘事实 | 重启后的当前行为 | 当前允许操作 | 发布判断 |
|---|---|---|---|---|
| `Prepared` | 只有 journal，尚未形成可提交对象 | 无 manifest，不进入库存列表；不自动续作或删除源 | 仅人工取证 | 自动对账未实现，阻断发布 |
| `Copying` | 可能存在不完整的明文 `.blob` 对象和 journal | 无 manifest，不进入库存列表；不自动清理或续传 | 仅人工取证 | 孤立明文对象自动对账未实现，阻断发布 |
| `ObjectVerified` | 完整明文对象可能已刷盘，但 manifest 尚未提交 | 无 manifest，不进入库存列表；不自动收编或清理 | 仅人工取证 | 孤立明文对象自动对账未实现，阻断发布 |
| `ObjectCommitted` | 对象与 manifest 已提交，源删除尚未武装 | 推导为 `sourceRetained`；对象无损时进入库存 | 普通导出隔离副本；不自动删源 | Preview 可观察状态，不是 QPC1 发布证据 |
| `SourceDeletePrepared` | 删除已经武装，但源是否仍在无法确定 | 推导为 `recoveryRequired`；不自动重删 | 当前无普通导出或救援入口；仅未来专用救援/取证流程 | 阻断发布 |
| `Committed` | 删除调用成功且终态已刷盘 | 推导为 `committed` | 对象完整性验证通过后可普通导出 | 仍受 Preview 总体门禁限制 |
| `SourceRetained` | 删除未发生，或删除失败后已重新证明源身份和内容未变 | 推导为 `sourceRetained` | 对象完整性验证通过后可普通导出，并明确提示两份均保留 | 仍受 Preview 总体门禁限制 |
| `Damaged` 或日志/manifest 无效 | 对象缺失、大小不符或元数据链无法信任 | 标记不可导出或计入损坏记录，不降级为安全状态 | 仅人工取证 | 阻断发布 |

特别是 `Prepared`、`Copying` 和 `ObjectVerified`：当前仓库列表以 manifest 为入口，无法发现并自动处置这些阶段遗留的 journal 或明文对象。孤立对象枚举、归属证明、幂等收编/清除和审计均未实现，因此这不是可接受的 Release 恢复策略，而是明确发布阻断项。

## 4. 仓库和当前命令

仓库位置：

```text
%LOCALAPPDATA%\Qingpan\quarantine-preview-v1\
  journal\<record-id>.jsonl
  objects\<record-id>.blob
  manifests\<record-id>.json

%LOCALAPPDATA%\Qingpan\restore-exports\
  Qingpan-Export-<operation-id>\<original-file-name>
```

日志使用连续序号和前一条原始 JSON 行的 SHA-256 链。截断尾行、序号跳跃、链摘要不符或非法状态跳转均拒绝继续操作。仓库根和固定子目录不得是链接或重解析点。

当前命令是明确带 `preview` 后缀的临时合同：

| 命令 | 输入 | 输出要点 |
|---|---|---|
| `list_quarantine_preview` | `limit`，范围 1 至 500 | 真实记录、`corruptRecords`；不返回原路径 |
| `export_quarantine_copy_preview` | `recordId` | 唯一导出目录、文件名、字节数、隔离源保留和审计落盘状态 |

导出目标不由 WebView 提交。后端只在固定根下创建唯一目录和新文件，复制后再次验证完整 SHA-256。普通导出仅允许对象有效且状态为 `committed` 或 `sourceRetained` 的记录；`recoveryRequired` 即使对象本身可读也必须拒绝，前端同样不会显示普通导出按钮。该状态只允许未来独立设计的专用救援/取证流程，当前没有 rescue、salvage 或其他绕行入口。`damaged` 同样不可导出。导出成功后隔离对象始终保留，因此不释放隔离占用。

## 5. 明确未实现

该切片不是生产级隔离协议，尚未实现：

- QPC1 分块格式、AES-GCM、DPAPI 包装密钥和专用 ACL；
- 固定句柄删除、oplock、content guard 和完整崩溃故障注入矩阵；
- 每卷仓库选择、跨卷隔离、quota ledger 和孤立对象自动对账；
- 原生目录 grant、用户选择导出目标、批量导出和耐久幂等键；
- 原位置恢复、覆盖恢复、自动清除、手动清除和安全擦除；
- 大文件或用户数据隔离；
- M1-04 的 REC-001~007、R4-001、正式 API-002~003 和发布证据。

因此 UI 固定显示“实验性隔离”，发布材料不得使用“完整恢复”“加密隔离”“零误删”或“已通过 M1-04”等表述。

其中孤立明文对象自动对账不是一般性后续优化，而是当前 Preview 进入 Release 的直接阻断项；仅有默认 Release capability gate 不能替代仓库恢复协议和崩溃一致性证据。

## 6. 本地验证结果

2026-07-25 当前工作树完成了以下非发布验证：

- Rust 库测试：85 项通过；
- Rust Debug `cargo check` 与 `cargo build`：通过；
- Rust 默认 Release `cargo check --release` 与 `cargo build --release`：通过；
- 前端 Vitest：52 项通过；
- 前端 TypeScript/Vite 生产构建：通过。

这些结果尚未绑定 run ID、环境清单、制品摘要或 Windows x64/ARM64 VM 证据，只能证明当前开发工作树通过本地检查。
