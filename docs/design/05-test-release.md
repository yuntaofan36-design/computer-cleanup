# 清盘设计文档：测试、发布门禁与运维

> 文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 规范章节：第 13-14 章
> 状态与版本继承自主索引；本文件不单独构成批准对象。

[上一篇：需求与规则](04-requirements.md#qpn-sec-9) · [主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>) · [下一篇：路线图](06-roadmap.md#qpn-sec-15)

---

<a id="qpn-sec-13"></a>
## 13. 测试、验收与发布门禁

<a id="qpn-sec-13-1"></a>
### 13.1 测试层次

| 层次 | 覆盖内容 |
|---|---|
| 单元测试 | 规则解析、签名、路径规范化、父目录链、风险映射、计划状态、错误码、空间核算 |
| 属性/模糊测试 | 路径、Unicode、长路径、规则 manifest、审计行和恢复清单 |
| Rust 集成测试 | 临时文件树、快照变化、链接、锁定、部分失败、隔离提交和恢复导出 |
| 前端测试 | 风险展示、确认门槛、不可选状态、部分成功、实际空间和无障碍 |
| Windows VM 测试 | 系统版本、架构、文件系统、UAC、安装/升级/卸载和系统健康 |
| 安全测试 | 恶意规则、签名篡改、降级、IPC 伪造、命令注入、网络字段泄露 |
| 故障注入 | 磁盘满、断电、崩溃、权限变化、文件并发替换、规则损坏、更新中断 |

<a id="qpn-sec-13-2"></a>
### 13.2 文件系统安全语料

必须覆盖：

- 符号链接、Junction、挂载点和嵌套重解析点；至少包含扫描前已存在的中间 Junction，以及应用重启后从卷 GUID 根逐组件重建持久授权根的用例，禁止用完整配置路径一次打开来跳过中间检查。
- 硬链接、额外数据流、稀疏、压缩、EFS 和只读文件。
- Windows 长路径、保留名、尾随点/空格、大小写差异和多语言 Unicode。
- OneDrive 等云占位、离线属性、网络盘和可移动介质。
- 文件被进程锁定、ACL 拒绝、父目录在扫描后替换、内容变化但大小相同。后者必须验证 FILETIME change time 或 USN 变化会被拦截；若测试刻意恢复全部可观察元数据且无可用 USN，只能验证文档所述能力限制，不得宣称必然检测。
- 扫描后新增文件、删除后重建同名文件、卷卸载/重挂载和 File ID 变化。
- 隔离 WAL 覆盖 QPC1 `prepared/sourceDigestPrepared/containerPrepared/copying/copied/containerVerified/containerCommitted/sourceDeletePrepared/sourceRemovedVerified/committed` 每个刷盘边界、content guard break、映射写、源身份/内容变化、DPAPI/AEAD/格式验证、容器发布、源删除单次调用、`sourceRetained`、导出副本和 R4 清除；不得再生成旧 rename/DACL 用例作为 v5 成功路径。

首个文件或外部进程副作用之前，任何安全前置条件不确定都应产生“跳过并保留”，而不是强制执行；副作用调用可能已经发生后，不得伪装为跳过，必须按对应 WAL 进入 `outcomeUnknown/recoveryRequired` 并停止重试。

<a id="qpn-sec-13-2-1"></a>
#### 13.2.1 必跑协议与竞态用例

| Test ID | 需求 | 故障点 / When | 必须观察到的结果 |
|---|---|---|---|
| T-IPC-009 | IPC-002 | helper 验证后及高完整性确认后分别篡改 plan、AppSnapshot、sealed invocation、摘要和数据库页 | 高完整性页面展示的 bundle 与系统调用字节一致；任一差异零调用并消费计划 |
| T-IPC-010 | IPC-001~002 | 委托并发双连、错 PID/创建时间/helper hash/session、过期、重放或 helper 在确认刷盘前后崩溃 | 只有精确委托一次成功；UAC 取消/超时/崩溃均终态化且旧 bundle 不恢复 |
| T-QUAR-015 | REC-002/004 | 导出副本在 prepared、temporaryCreated、copying、verified、published 每个 flush 后崩溃并由新实例恢复，同时核对按钮/确认/成功/到期文案 | 只按内嵌授权快照续作；旧瞬时 grant 不能新开操作；文案不暗示原位还原或立即释放空间，隔离源始终保留 |
| T-QUAR-016 | REC-004 | 替换目标父/子目录、temp File ID/DACL/marker 或制造 final 冲突 | 不覆盖、不删除未知目标，记录进入 recoveryRequired 并保留隔离源 |
| T-QUAR-017 | REC-004 | damaged 与 orphan 分别救援，期间改变 source evidence/state/sequence 或崩溃 | known 完整校验才是 verifiedCopy；orphan 永远 unverifiedCopy，异常主状态不变且 purge 仍关闭 |
| T-QUAR-018 | REC-003 | 两个近配额 R2 prepare 并发；再放入 allocated/logical size 均不可读的 orphan | reservation 最多一个成功；未知核算阻止新隔离但列表/诊断/救援可用，不自动清除 |
| T-QUAR-019 | REC-001/SAFE-003~005 | 在 QPC1 十个主状态、container/ledger flush、源 `callPrepared/callAccepted/removedVerified` 的每个边界崩溃并重启对账 | containerCommitted 前零源删除；删除可能调用后绝不重调；只有 container 与 removedVerified 均耐久才成功，其他按 sourceRetained/recoveryRequired 保守终态化 |
| T-QUAR-020 | REC-007 | writer、reader、独立恢复工具运行 `qpc1-v1-vectors.json` 的 zero/single/multi-chunk golden vectors，并逐项注入 header preimage、非规范 CBOR、tag、index、长度、截断、尾随和溢出错误 | 三实现生成/验证相同字节和 digest；全部 negative vector 在分配或源删除前失败关闭 |
| T-QUAR-021 | REC-001/007/SAFE-004 | 在 `all36` 每个 tuple 中由另一进程先建立真实 `PAGE_READWRITE` 映射再关句柄，随后请求 oplock；另以不同普通 SID 尝试读取/改写容器 owner/group/DACL/label | 未证明映射写可观察 break 时 capability 不开放；break 后零新源删除；另一 SID 无访问，任一安全描述符重读不一致使容器不可提交；ReFS tuple 必须保持 R2 不支持且零写入 |
| T-QUAR-022 | REC-001/007 | 对 copy、full decrypt verify、container rename/reopen、ledger charge 和 source delete 调用计时并在每点强制失败 | 任一完整容器验证/提交/charge 前源删除调用计数为 0；提交后源删失败保留完整容器和源并报告 sourceRetained |
| T-QUAR-023 | REC-001/006 | 在 containerPrepared/copying 的每个 frame 截断或崩溃，重启时组合源 hash 相同/变化、temp identity 相同/变化和 final object 有/无 | 只有精确产品临时对象可经独立 discard WAL 清除并释放 reservation；未知对象/源变化保留双方并 recoveryRequired，不跨实例续写 |
| T-QUAR-024 | REC-001/007 | 在 pre-hash、copy 中、container verify 后和 source delete 紧前通过普通写、映射写或同名替换改变源 | 任一 hash/identity/guard 差异零源删除；已发布容器转 sourceRetained，未发布对象按受限 partial-artifact 对账 |
| T-QUAR-025 | REC-001/007 | 删除/替换 DPAPI key material，篡改 wrapped DEK、manifest/chunk tag、chunk 顺序或 ciphertext，并尝试普通导出与异常救援 | 不发布部分明文、不删除源/容器；正常导出失败，异常路径只可导出明确标记未验证的 `.qpc1` 容器 |
| T-QUAR-026 | REC-002~007/R4-001 | reservation/实际 allocation 边界、复制或 ledger 时磁盘满、sourceRetained 导出、1/500/501 项 R4 清除及配额/保留期触发 | 不超卖、不因磁盘满删源、不自动清除；committed/sourceRetained 均可手动导出/清除，sourceRetained 明确只删容器且原文件保留 |
| T-FS-009 | SAFE-001/004~005 | 从卷根到直接父目录逐级尝试 rename/delete 祖先，组合 share-delete 被拒绝/无法取得、最终拓扑复检后并发移动和 R1/R2/R4 mutation | 全部授权根/祖先句柄保持拒绝 share-delete 至 item 终态；无法取得窗口返回 FILE_LOCKED/PARENT_CHANGED；mutation 不会作用于移到允许根外的对象 |
| T-R4-PURGE-005 | REC-005/R4-001 | 确认后、首删前改变任一 record journal/state/identity/分配大小摘要 | 整批零删除、计划 consumed、结果明确失败，不能继续其余项 |
| T-IDEMP-001 | API-002 | 五种 command 注入错配 response、pending 同 mutation attach/对账竞争、TTL、响应前崩溃、底层 grant/job 后续变化及同 key 异 payload | schema 拒绝 command/response 错配；pending 不新建/不过期；同 payload 逐字段返回原不可变 response/ApiError 快照而非重读可变对象，异 payload INVALID_REQUEST |
| T-AUTO-011 | AUTO-003 | 撤销先于 claim、claim 先于撤销，以及在 grant 重验与 item prepared 之间插入屏障 | revision/CAS 决定唯一顺序；撤销提交后没有新 item prepared |
| T-AUTO-012 | AUTO-003 | 单 grant 内多 binding 任一失效、item 间重启和两个任务进程竞争 | 整个 scheduled plan 停止，已提交 item 保留，余项 unprocessed 且无漏执行 |
| T-APP-011 | APP-002~003 | MSI GUID 大小写/花括号别名、per-user/machine context、AppX 同 family 不同 FullName及两个管理员 SID 并发 | 规范测试向量唯一；machine store/mutex 只允许一个 attempt/OS 调用，另一请求只创建 owner 隔离 attachment 或返回既有状态，不创建第二锁 |
| T-APP-012 | APP-003 | MSI launchPrepared/调用边界/返回刷盘及 AppX deployment started/completed 各边界崩溃 | 同一规范资源不重复调用；只有已证明未调用可释放锁，其余进入对账或 recoveryRequired |
| T-APP-013 | APP-003 | MSI return/AppX completed 分别组合目标 absent/present/unknown，AppX 仅 started 后重启；再向 attempt、成功 item 和 completed view 注入 launch 缺失/仅 `started`、completed + target present/unknown，向 notRemoved item/view 注入 absent/unknown 或无 completed launch，并交叉替换 source attempt sequence/digest | 仅同一 deployment 的 completed launch + absent observation 可生成 AppX removed attempt/item/view；completed + present 只能生成带同一 attempt 证据的 notRemoved；所有伪造、错配和 only-started/unknown 组合被 closed schema/领域校验拒绝且锁不释放 |
| T-APP-018 | APP-003 | 管理员 A 发起 machine MSI 后，管理员 B/C 以各自 plan/operation 附着；交叉替换 owner SID、local plan/item、source sequence/digest，组合 rebootPending/removed/notRemoved/unknown | 只有 machine journal 中精确 attachment 可投影到各自本地 operation；用户只读自己脱敏结果，三者共享一次 OS 调用，错配不泄露源 operation 并失败关闭 |
| T-APP-019 | APP-002~003 | `MsiEnumProductsExW/MsiGetProductInfoExW` 枚举零/一/多 context、权限不足和调用前后集合变化；对 singleton 分别返回 0/3010/1641/失败并跨 boot 对账 | 只有精确 singleton 可调用；多 context/不完整返回 UNINSTALL_TARGET_AMBIGUOUS；3010 保持锁至 boot 变化，1641/失败/未知不重调且进入对应恢复状态 |
| T-PAGE-001 | PAGE-001 | snapshot/append 翻页期间追加、删除、同排序键、TTL 过期 | 有效 cursor 链无重复遗漏；caughtUp 与 producerTerminal 不混淆；过期从 first 重读 |
| T-UPDATE-027 | UPDATE-006~007 | 双管理员竞争、旧 staged、旧完整 MSI/NSIS、apply 前撤销/离线过期、LKG 后撤销 | 最多一个安装；降级/撤销制品零 MSI 调用，离线不能绕过 normal install 复验 |
| T-UPDATE-028 | UPDATE-008 | trial pipe 抢占、远程 client、错 child/token/session、nonce 泄露替身、重放及 child 丢失 | 收据全部拒绝并进入 rollback；nonce 不出现在 argv/env；不启动第二 trial child |
| T-UPDATE-029 | UPDATE-008~009 | 用户 A 更新后换用户/重启，用户 B lazy migration 失败；恢复文件 grant 伪造/过期/跨会话/文件身份变化，或恢复包状态摘要/签名/代码签名错误 | 非法 grant 零文件读取/零副作用；B 只读恢复且不误回滚 machine binary；伪造恢复失败，可信恢复只追加 resolution |
| T-UPDATE-030 | UPDATE-003/009 | key-set 重复 ID、同公钥不同 ID、重复签名、错误 purpose、n-1 票和轮换中断 | 不同公钥有效票未达阈值一律拒绝；轮换只在目标 committed 激活 |
| T-FMUT-001 | SAFE-005 | 在 `prepared/callPrepared/callRejected/callAccepted/removedVerified` 每次刷盘后崩溃并以两个 executor 对账 | 删除 API 最多调用一次；只有 `removedVerified` 补交成功，其余按存在性保守终态化 |
| T-FMUT-002 | SAFE-001~005 | `callPrepared` 后替换父目录、同名重建、File ID 变化、delete-pending 或证据不可读 | 不重调删除；精确对象可证明存在则 preserved，否则 outcome unknown/recoveryRequired |
| T-BATCH-RESTORE-001 | REC-004~005 | committed/sourceRetained 混合批在 0/N 项成功后取消、进程故障、SID 或目标目录身份变化 | 成功导出保留；无 temp 项回各自 previousMainState/unprocessed；不完整项受限对账；终态无 restorePrepared且 sourceRetained 不伪报隔离成功 |
| T-BATCH-PURGE-001 | REC-003/005 | committed/sourceRetained 混合批 prepared 后在首删前、删中和 item 间停止，并组合 mutation phase 与 present/absent/changed/DeletePending | `removedVerified` 补交 purged；无该证据的 absent 才是 purgedUnverified；present/changed/unknown 按 previousMainState/damaged/recoveryRequired，绝不自动续删 |
| T-QUOTA-001 | REC-006 | 并发 reservation、边界等式、64 MiB reserve、基础设施 CAS、各隔离 WAL 边界、孤立 reservation、missing charge 和 unknown orphan | 同 revision 等式只允许可容纳请求；不超卖、不按 TTL 释放；unknown/invalid ledger 阻止新隔离但保留读/救援 |
| T-QUOTA-002 | REC-003/006 | restore、purge interrupted、verified purge 和 tombstone GC | 前两者保持 charge；verified purge 替换为 tombstone charge；只有 verified GC 最终释放 |
| T-RESTORE-IDEMP-001 | API-002/REC-004 | 首事务前后、目录 CREATE_NEW 后、directoryReady 前后及响应前崩溃 | 同 key 只产生一个 operation、一个目录和一批 record claim，旧 grant 不被二次消费 |
| T-RESTORE-IDEMP-002 | API-002/REC-004 | 同 key 异 payload、并发同 key、错误 marker 或目录身份变化 | 零 copy、不复用 grant、不采用错误目录，终态不遗留 owned prepared record |
| T-RULE-019 | RULE-006/API-003 | 三种 matcher 深度 fixture、names OR、prefix/suffix AND、case-sensitive 目录、空/重复/转义/超限输入 | 仅规范匹配产生候选；任一非法或超限整包拒绝，active 包不变 |
| T-RULE-020 | RULE-004~005/REL-001 | active 包、四类高水位、sticky 撤销和激活事件事务各边界崩溃，并与 claimed operation/GC 竞态 | 旧或新状态原子可证明；claimed 固定包仍可对账，引用归零前不删包 |
| T-PLAN-002 | PLAN-001/R4-DELETE-001 | 对 source/items/confirmation/lifecycle/category summary 做合法组合表，并尝试 cleanupRules+R4、app+scheduled、purge+elevation 及摘要篡改 | 只有严格联合规定组合通过；远程规则不能构造原文件 R4，非法组合零 item 并报状态错误 |
| T-RESULT-001 | RESULT-001 | 从 `operationItemResultContract` 生成 ref/action/adapter 与 code/outcome/phase/disposition/retryable/unknownEvidence 全组合；对 MSI/AppX/Win32 的 succeeded/completed 注入缺 launch、缺 observation、target present/unknown、错 source attempt；对 Win32 再注入 running/detached/unavailable、未 drain Job 或缺 launch WAL，对三种 adapter 的 notRemoved 注入 absent/unknown/未完成 launch；并覆盖退出码、purge 错 phase、operation unknown、counts/status/空间合计篡改 | 只有契约表及适配器 evidence 联合中的完整 tuple 通过；MSI 需 completed call + absent，AppX 需 completed deployment + absent，Win32 需耐久 launch + exited + 受控 Job 树 drained + absent；notRemoved 必须是对应完成证据 + present；所有跨领域码、非法证据、错 attempt 绑定和错 outcome schema 均拒绝，其余按第 7.7 节唯一优先级 |
| T-API-003 | API-003 | 非规范 U64/I64/UUID/SHA/timestamp、未知字段、重复集合与 Unicode 等价值 | 非规范输入拒绝；Rust/schema/签名/审计对合法向量产生同一摘要 |
| T-API-004 | API-001 | 除两个 grant route 的跨用途注入外，分别交叉替换停用 reason、分析策略 scope、四种 scan kind、R0 create/update 与 R1 job kind、rules/application domain、stable/beta/internal channel；向规则 set 注入 revision、向应用 set 删除 revision 或在首次创建不用 `"0"`；并用带更窄 literal 的已解析请求实例化 `CommandResponseFor` | 每个合法请求只接受其成对 `CommandSpec` 的单用途/同 discriminator 响应；所有笛卡尔错配、宽 scope、错 kind/mutation/domain/channel、非法 CAS 字段均在 closed request/response schema 或领域校验失败；窄合法请求推导非 `never` 且只得到对应响应，handler/客户端不得保存或消费错配对象 |
| T-CONTRACT-001 | API-004 | 对 `CONTRACT-001/RuleManifest` 删除、重复或替换 ID/root，把 `schemaVersion=1` 改为别名字段/值，或改动任一生成 schema 而不更新 digest | 恰好一个登记且 Rust/TypeScript/JSON Schema 的 ID、版本与 digest 全等才通过 |
| T-CONTRACT-002 | API-004 | 对 `CONTRACT-002/ScanRequest` 删除分支、扩展未知 kind，或使登记版本与 `RequestEnvelope.apiVersion=1` 不等 | closed scan union 与 envelope 版本逐生成物相等才通过 |
| T-CONTRACT-003 | API-004 | 对 `CONTRACT-003/CandidateSnapshot` 删除或修改 `schemaVersion=1`，或使登记/root digest 漂移 | 类型、本地存储 schema 和登记三者一致才通过 |
| T-CONTRACT-004 | API-004 | 把 `CONTRACT-004/CleanupPlan` 登记为 v1、删除 `schemaVersion=2` 或使 plan JSON Schema digest 不同 | 三种生成物都为 schema v2 且 digest 相等才通过 |
| T-CONTRACT-005 | API-004 | 对 `CONTRACT-005/ExecuteResult` 缺失/篡改 `schemaVersion=1`，或放宽终态分支后不更新 digest | 根版本与全分支 schema digest 一致才通过 |
| T-CONTRACT-006 | API-004 | 对 `CONTRACT-006/QuarantineRecord` 登记非 `recordVersion=5`，或混入 v3/v4/跨 state 字段 | 只有 v5 closed union 与三生成物 digest 一致才通过 |
| T-CONTRACT-007 | API-004 | 对 `CONTRACT-007/ScheduledJob` 删除/修改 `schemaVersion=1`，或使 R0/R1 分支、登记与 Task 存储 schema 漂移 | 根版本、两分支与生成 digest 一致才通过 |
| T-CONTRACT-008 | API-004 | 对 `CONTRACT-008/OutboundRequestPolicy` 修改 `policyVersion=1`、宽化网络字段或只更新一种生成物 | 政策版本、白名单 schema 和三生成物 digest 一致才通过 |
| T-CONTRACT-009 | API-004 | 独立篡改 Rust/TypeScript/JSON Schema 任一制品，交换两个 Contract 的具名摘要，改变八项 tuple 顺序、canonical profile、未知字段或把 `registryDigestSha256` 自身放入哈希输入 | 三制品摘要分别按 profile 复算，v2 registry 只哈希排除自身的 RFC 8785 canonical payload；任一漂移都使 API-004/M1-11 失败 |
| T-PLAT-001 | PLAT-002 | host-independent 生成 3×2×3×2 key map 和当前 23 组/252 个 `RELEASE_TEST_OBLIGATIONS`，注入缺失/重复/额外 test ID、同 ID 多组、key/value 分解错配、profile 计数 35/23/11、错误 capability/profile、未知维度和 obligation/registry digest 篡改 | 只接受精确 36 项、profile 36/24/12、23 个规范有序组、252 个唯一 test ID 与两个 RFC 8785 摘要；JSON Schema/TypeScript/生成器集合相同 |
| T-PLAT-002 | PLAT-001~002 | 对 `all36` 每个 tuple 执行适用的安装、升级、缓存、重启、卸载流程，并在 ReFS profile 尝试 R1-R4 | 每个 required tuple 至少一条 passed run；ReFS 只有 R0，任一缺失/blocked run 或写动作成功均使 GATE-003 失败 |
| T-PLAT-003 | PLAT-001~002/IPC-001 | 对 `all36` 分别以 standardUser 和 splitTokenAdminMedium 发起 R3；尝试 over-the-shoulder 账户、错 SID linked token 和高完整性桌面 child | 标准用户凭据式提升拒绝，同 SID split token 才能进入高完整性 helper，桌面 child 保持原 Medium/Limited 身份 |
| T-PLAT-004 | PLAT-001~002/RELEASE-001 | 跨两个 build 交换 UBR、tuple registry/profile、run/trace；把 `T-PLAT-002/T-SCAN-001/T-UI-002/T-FMUT-003/T-QUAR-021/T-UPDATE-040/T-WIN-001` 改为较小 profile，或把任一 test 改绑无关 capability，再注入未来 Windows build、自由 selector、pairwise 子集或 digest 正确但实际文件错误 | definition 的 capability/coverage 必须与摘要保护的 obligation 精确相等，registry/file/manifest/provenance 必须同 build 且覆盖权威集合；任一缩小、错绑或交换阻断 GATE-003/M1-11 |
| T-SCAN-001 | SCAN-001 | 在 `all36` 超过文件/结果/深度/墙钟限制并在枚举、哈希、阻塞 OS 读取边界取消 | 返回准确 `limitReached/cancelled`，阻塞调用结束后不再枚举且不产生写副作用 |
| T-SCAN-002 | SCAN-002 | 在 `all36` 同时保持 3 个只读扫描运行，再并发提交第 4 个、重复 request，并在取消/终态释放前后重试 | 只有前三个唯一 task 耐久存在；第 4 个无排队/无对象返回 `LIMIT_EXCEEDED`，slot 释放后新 task 才成功 |
| T-UI-001 | UI-001 | 在 `all36` 组合 R1/R2/R4、部分成功、结果未知和空间观测四状态渲染结果页 | 五类空间/结果概念分别显示且 basis 正确，R2 明示隔离占用，不把缺测当零或宣称精确归因 |
| T-UI-002 | UI-002 | 在 `all36` 的 125%/150%/200% 缩放、最长本地化文本和最小支持窗口遍历核心页面并保存截图 | 文字、按钮、表格、确认摘要和错误不重叠/截断，命令可见或可滚动到达 |
| T-UI-003 | UI-003 | 在 `all36` 仅用键盘和 Windows 屏幕阅读器完成扫描、预览、确认、结果、隔离和错误恢复 | Tab/箭头顺序稳定、焦点可见且不丢失，名称/角色/风险/状态/动态通知均可感知 |
| T-UI-004 | CLEAN-002 | 在 `all36` 对每类候选和计划逐项比对依据、影响、风险、动作与恢复摘要，并在确认后篡改分类/item | UI 与不可变 plan 一致；任一变更创建新计划并重新确认，不以折叠或默认勾选隐藏风险 |
| T-UI-005 | REC-002/UI-001 | 在 `all36` 覆盖隔离导出成功/失败/到期/sourceRetained 和清除入口 | 全程使用“导出副本”，明确不还原原位置、不自动释放占用；异常状态不伪报隔离成功 |
| T-BATCH-001 | API-002~003 | restore/salvage/purge 的重复、乱序和 1/500/501 个 record ID | 重复/超限零查库副作用；合法乱序得到相同 canonical record-set/确认摘要 |
| T-PAGE-002 | PAGE-001~002 | 跨 API/对象/owner/session cursor，8-chain/row/byte/1 MiB 上限与 append 四种组合 | 不越权、不驱逐有效链；超限稳定失败；next cursor 的有无与真值表一致 |
| T-AUTO-013 | AUTO-003~004 | 直接启动 runner、on-demand Run、instance GUID 重放、两个 runner 竞争最后一次，再改变 trigger/timezone/principal/binding/grant revision | 只有一个经 Task Scheduler 证明的 run ordinal；手动/重放/批准材料变化均零扫描、零新 item |
| T-AUTO-014 | AUTO-005 | upsert/delete 每个 journal 刷盘点崩溃，Task 缺失、孤立、篡改或禁用失败 | 精确状态幂等前滚；未 committed runner 零 item；无法确认禁用时自动子系统失败关闭 |
| T-IDEMP-002 | API-002/AUTO-005 | scheduler pending 各 WAL 阶段、同/异 payload 重放、响应前崩溃和两个首请求竞争 | 同 payload 附着/对账后返回原完整 mutation result；异 payload 拒绝；最多一个 Windows Task |
| T-AUTO-015 | AUTO-006/API-002 | approval 提交后响应丢失、create/update 缺半个 CAS 字段、DST 跳时/重复小时、时区规则/系统时钟回拨和两个 occurrence 竞争 | 同 key 找回唯一 grant/job；非法 request schema 拒绝；每个 occurrenceKey 最多一个 claim，无法证明触发则零扫描 |
| T-LICENSE-001 | LICENSE-001~003/API-001 | 原生草稿过期/限速、WebView 注入路径分片或伪造 deactivation request ID、跨会话/重复停用 grant、请求 reason 与 grant view/耐久 grant reason 交叉错配、服务/代理日志检查及凭据存储失败 | 停用 schema 无业务 request ID；只发送固定 Rust 字段；reason 只能按成对 `CommandSpec` 原样绑定且错配响应拒绝；原始卡密/本地字段不进日志；非法 grant 零请求且未安全存储不报告成功 |
| T-LICENSE-002 | LICENSE-002 | `PendingLicenseActivation(recordVersion=3)/Refresh` 在 proof 生成前后、send/response、prepared、reconciliationRequired、responseStored、pointer CAS、slot、resolution 与 GC 各边界崩溃；省略 proof，篡改 alg/typ/domain/purpose/method/path/body/ID/SPKI/`iat/jti`，换普通 key或强保护 key 代签，重放 proof，并以非 fresh 变量/JSON/Rust 向四个 activation state 注入其他 state 字段；另覆盖双密钥/request slot、v2 迁移、refresh fence | 只有同一 slot 的逐字节 body、双密钥/profile 全绑定且每次新的有效 proof 才可在 deadline 前发送；无效 proof 不查询/污染 mutation、不占席或返回旧 token，跨 state 字段在 TS compile-negative、closed JSON Schema 与 Rust unknown-field 层拒绝；响应丢失重试 body 相同但 `iat/jti/signature` 新鲜，业务结果最多一次 |
| T-LICENSE-003 | LICENSE-003 | 在线/离线停用在 grant 消费、prepared、首次发送、responseStored、active credentials 销毁、reconciliation key/WAL 跨卸载保留、v2 fenced resolution 与终态 GC 各边界崩溃；对 notCommitted 交叉 active credentials retained/destroyed、key restored/destroyed，并注入缺失/非法组合 | retained 分支只能与 `restoredToActiveDeviceKey` 原子提交，重启恢复 active 且可重新确认停用；destroyed 分支只能与 key destroyed 提交，并固定显示 `notCommittedDeactivationCredentialsDestroyed + seatMayRemainOccupied + contactSupport`；非法组合/缺 disposition 不清 WAL、不猜测；replay deadline 后旧停用零发送，服务端最多撤销一次 |
| T-LICENSE-004 | LICENSE-003 | 同 SID 非产品进程直接打开普通设备 key 调用 `NCryptSignHash` 并直连 `/deactivate`；另重放/替换 challenge、key ID、grant/statement/UV digest，取消 Windows Hello/高保护 UI | 普通 key 签名、无强保护用户验证、错/过期/已消费 challenge 全部被客户端或服务端拒绝且席位不变；只有登记的独立 key 当次原生验证可停用一次 |
| T-SVC-001 | SVC-001 | 对授权服务 staging/production 删除 owner、设计、artifact/provenance、origin、幂等迁移、key ceremony、日志 stripping/deletion 或 contract/negative/packet/E2E 任一证据；交叉替换三个服务的 `designDoc.id/serviceDesignId/serviceMilestoneId/domain` | 缺项/失败记录无法构造 passed readiness；任一 ID/domain 错配在 TS/JSON Schema 层拒绝；M1-SVC-01 与 M1-08/M1-11 阻断，完整不可变证据才通过 |
| T-SVC-002 | SVC-002 | 对规则服务注入未知字段接受、错误 origin、签名/撤销重放、回滚失败、日志泄露、仅客户端 mock 证据或服务 ID/domain 交叉错配 | closed contract/撤销/隐私或任一分支绑定失败即 blocked；M1-SVC-02 与 M1-02/M1-11 不通过 |
| T-SVC-003 | SVC-003 | 对应用更新服务注入 manifest/epoch/revocation/package 错配、跨 origin 重定向、回滚失败、日志泄露、缺 E2E run 或服务 ID/domain 交叉错配 | 分支绑定或服务制品/部署/包分发/隐私证据失败时，M1-SVC-03 与 M1-10/M1-11 不通过 |
| T-DOC-001 | RELEASE-001 | 用 Draft 文档、错版本/SHA-256、缺任一角色、复制签名、同一自然人换 key/账号承担多角色、未授权或已撤销 key、篡改/重放评审 receipt，或在批准后修改任一字节 | GATE-010 与 M1-11 失败；只有同一 `Approved` 版本/SHA-256、同一有效 trust policy 下五个不同自然人身份的可验签/可回放批准全部通过时有效，读者测试不能替代 |
| T-RELEASE-001 | RELEASE-001/API-004/PLAT-002 | 在 A/B build 间交换 M0 三文件、治理策略、源码派生/patch、CI provenance、capability manifest、SBOM/findings/dispositions/report、test/trace、platform/formal registry、document approval 或六服务记录；注入 M0 patch input/output 断链、final tree 与 build input 不同、审批伪造/重复、路径逃逸/大小写碰撞、finding 重复/孤立 disposition/过期证据/错误 count 或 root、M1 capability 降级、错 test coverage、缺/多顶层 evidence、canonical/self-digest 环，以及零字节/LF-only `waivers.jsonl` | `ReleaseGateManifest v4` 只接受同一 release/build 的 19 个固定顶层文件、六服务记录、传递 refs、实际 artifact/provenance 和 `result=passed` 的 M0/derivation/dependency 证据；源码从 M0 重放到 build input 可复算，零 waiver 只接受零字节；gate 外 `SignedReleaseAttestation v1` 必须由外部信任锚授权且不得被 gate 反向引用，任一混用或自证阻断 GATE-001/GATE-003/GATE-010/M1-11 |
| T-AUDIT-004 | AUDIT-001/API-001 | 伪造、过期、重放、跨会话诊断保存 grant，替换父目录或预占目标名 | 非法 grant 返回 `DIAGNOSTIC_EXPORT_GRANT_INVALID` 且零文件；合法 grant 只在固定父目录 `CREATE_NEW` 一个包并消费一次，不产生网络请求 |
| T-APP-014 | APP-004 | capability 关闭、`currentUserOnly` 目标需提升、路径/参数变化、HKCU、可写 HKLM、可写 EXE、任意有效签名、未知 template 和参数注入 | 关闭时前后端均无可执行入口；普通权限分支绝不转 helper；只有受保护 HKLM + 精确签名 + 编译模板组合可进入高完整性确认 |
| T-APP-015 | APP-004 | 用户可写 CWD/PATH/TEMP、同名 DLL、依赖替换和 mitigation 缺失 | child 零启动并记录目标无效；每个开放 Win32 adapter 的 planting suite 全部通过 |
| T-APP-016 | APP-004 | Win32 `launchPrepared`、创建 kill-on-close Job、`CREATE_SUSPENDED`、child/token/image 校验、禁止 breakaway 并加入 Job、child+Job+launch evidence 刷盘、`ResumeThread` 的每个边界崩溃；并尝试在证据刷盘前恢复线程 | 固定顺序不可跳步；耐久 launch evidence 前应用代码零执行，schema/执行器拒绝提前 resume；任何无法证明的创建边界不重复启动，锁转 recoveryRequired，旧计划不可重放 |
| T-APP-017 | APP-004 | 退出码 0 但资源仍在/已消失、Job 仍有后继、breakaway/detached、30 分钟仍运行、Job drain 证据缺失；向 succeeded/completed/notRemoved item/view 注入 running、detached、unavailable、present/absent/unknown 与错 attempt digest | 只有耐久 launch、子进程 exited、禁止 breakaway 的同一受控 Job 树已清空且目标 absent 才 removed；同证据但资源 present 才 notRemoved；其余为 unknown、不可重试、schema 不接受成功/已验证未移除且锁不释放 |
| T-STATE-001 | REL-001/RESULT-001 | 状态仓库打开、flush、CAS、schema/checksum/sequence 在每个写边界失败 | 新 R1-R4/调度/更新全部关闭；只按已有 WAL 对账且不猜测修复后继续 |
| T-UPDATE-031 | UPDATE-005~006/010 | app/full admission、10 分钟 ticket 过期/错 provenance/base/anchor；对 `MachineFullInstallerJournal` 的 admitted/callArmed/inFlight/resolvedWithoutCall/resolved/recoveryRequired 六态逐事务崩溃，替换 installation request/installer attempt/admission ID 或 sequence，并发同/异包 | 只有三 ID + sequence 与 embedded attempt/admission 完全一致才对账；admitted 零调用取消不推进 floor，callArmed 与 floor 同事务，armed/inFlight 不重调，非终态/被引用 journal 不按 TTL GC |
| T-UPDATE-032 | UPDATE-010 | `PreviousLkg` 各 unavailable reason、LKG 缺失/撤销，以及四个合法零调用取消源、同包重新 admission 和 callArmed 后取消 | 无 LKG 不进 installPrepared；合法取消磁盘仍为 base、floor 保持 admission 前值；同包可重建 admission，armed 后不可取消且 floor 不回退 |
| T-UPDATE-033 | UPDATE-003/007/011 | A 通道接受 package/version 撤销后切 B、回滚、卸载、重装和可信恢复 | 全局集合只增不减；target/LKG 命中时零调用，current artifact 跨通道只读 |
| T-UPDATE-034 | UPDATE-001/005/011 | journal v4 全部 16 个状态 × activeAdmission/floor/实际 MSI/二进制组合 | 每个状态只走第 14.1 节唯一对账；armed 调用不重复，旧版存在不伪造 rolledBack |
| T-UPDATE-035 | UPDATE-004~005 | MSI `0/3010/1641/1603/其他非零` 返回前后和 evidence flush 前崩溃，分别组合可/不可证明 transaction、目标与 reboot requirement | 每类先写唯一合法 evidence；失败码不进成功态且无裸 return code；三类证据齐全才 reconciled，否则 recoveryRequired，MSI 不重调 |
| T-UPDATE-036 | UPDATE-010/012 | 先以 UpgradeCode U1 安装，再以合法单调版本升级到 U5、卸载并运行仍受信的旧 U1；另覆盖首装、正常卸载/重装、anchor 删除/复制/ACL 篡改、同产品族不同 UpgradeCode、双击 MSI、直接 `msiexec /i`，以及针对同一 anchor 的正确/错误恢复阈值签名 UpgradeCode 迁移 | 合法重装按产品族复用唯一全机 anchor/floor/sticky；旧 U1 与未授权不同 UpgradeCode 不能创建新 anchor且在首产品写前拒绝；只有 expected current/revision/sequence/原生架构全匹配且序号严格递增的阈值签名迁移原子推进原 anchor，重放、降序、同序号异 payload 和跨 anchor 迁移均零 MSI 调用 |
| T-UPDATE-037 | UPDATE-008 | trial launchArmed、CREATE_SUSPENDED、加入 kill-on-close Job、child identity flush、ResumeThread 各边界崩溃，并尝试错 token/session/AuthId | 只有发起 split-token admin 的 linked Medium/Limited child 在耐久 identity 后恢复；任一边界不启动第二 child |
| T-UPDATE-038 | UPDATE-013 | 离线撤销状态/签名者轮换的错误 purpose、过期、低/同序号异 hash、错 artifact/signer set，以及 package binary provenance 变化 | 两类独立 floor 只增不减；伪造/重放失败关闭，admission 只绑定安装前提取且与 manifest/active policy 一致的 provenance |
| T-UPDATE-039 | UPDATE-009/012 | trusted recovery T1/T2/T3 各事务边界崩溃，低/同序号同异 payload及卸载重装后重放；product uninstall T3 注入缺/错 expected identity、MSI/服务/binary absence、journal sequence 或 anchor revision | pending 返回原 attempt 且不重调，resolved 返回原 resolution；卸载只有 resolution + absence evidence + journal `uninstalledPreserved` + anchor lifecycle/revision + floor 同事务才通过，原 attempt 仍为 recoveryRequired；其他 replay/组合拒绝 |
| T-UPDATE-040 | UPDATE-012/PLAT-001 | 在 `all36` 交叉运行 ARM64/x64 受信 old/current package，并尝试在架构检查前创建/读取 anchor | `IsWow64Process2` 与 MSI metadata 在任何 anchor/state 访问前拒绝，返回 UPDATE_PLATFORM_MISMATCH；每个产品族始终只有一个全机 anchor |
| T-UPDATE-041 | UPDATE-001/005/009 | full installer journal 六态每次刷盘后崩溃，随后以匹配/错 installationRequestId、installerAttemptId、admissionId、journal sequence/source 的 assessment 与恢复包处理；在各种 grant/attempt/resolution 引用下尝试 GC | 无 AppUpdateJournal 也能唯一对账；错 tuple 零读取/副作用，正确 source 只追加一次 resolution且不重调；非终态/被引用记录不删，裁剪后仍有产品族生命周期 tombstone |
| T-UPDATE-042 | UPDATE-001/005/009~012 | ARP bootstrapper、裸 `msiexec /x`、productUninstall ticket/REMOVE 错配、0/3010/1641/失败/结果丢失与恢复包重放；向 payload/assessment/resolution 交叉注入 target/absence 字段，并用非 fresh 变量、JSON、Rust 构造 `MachineProductUninstallJournal.recoveryRequired + absence/completion/resolutionRecord` 及 `exactTarget + absence/anchorLifecycle` | 只有精确 ticket 调用一次；外层与嵌套 `StrictUnion`、JSON Schema `unevaluatedProperties=false` 和 Rust closed enum/struct 都拒绝跨分支字段；只有安装器成功或 `completeProductUninstall` 加完整 verified absence 才以对应 completionKind 写 uninstalledPreserved，原 recoveryRequired attempt 不改写且 anchor/floor/sticky 不降低 |
| T-FMUT-003 | SAFE-005 | 在 `all36` 尝试 POSIX/ON_CLOSE/IGNORE_READONLY flags、只读文件和预置 DeletePending | NTFS 只调用精确 DELETE flag 或合格 fallback；ReFS 零写入并返回不支持；禁用 flags 永不出现，DeletePending/只读不明时零调用并失败关闭 |

<a id="qpn-sec-13-3"></a>
### 13.3 Windows 验收矩阵

以下自然语言维度由 PLAT-002 的 `PlatformTupleRegistry v3` 和覆盖 profile 机器化，不能由测试定义自行缩小。registry canonical payload 中 23 个 `RELEASE_TEST_OBLIGATIONS` 组必须无重复地展开为第 13.6/M1 当前要求的精确 252 个 test ID；definition 的 `capabilityId/kind/profileId` 必须逐 ID 等于该登记，registry profile 还必须绑定同一 registry 与 obligation digest。对标记 `all36` 的用例，CI 必须展开精确 36 个 tuple 并要求每个 `(requirementId, testId, platformTupleId)` 至少一条 passed run；`ntfsWrite24` 和 `refsReadOnly12` 分别只能由固定数据卷谓词生成 24/12 项，不接受手写 ID 数组、抽样或 pairwise 替代。每个发布候选至少执行干净安装、升级安装、缓存清理、重启和卸载的适用分支：

- Windows 10 22H2 build 19045 x64、ARM64。
- Windows 11 24H2 build 26100 和 25H2 build 26200 的 x64、ARM64。
- NTFS SSD、NTFS HDD；ReFS 只读分析。
- 标准用户、管理员用户、UAC 默认级别。
- 拆分令牌管理员的同 SID 提权必须成功；标准用户 over-the-shoulder 凭据提升必须返回 `ELEVATION_SAME_USER_REQUIRED` 且不执行计划。
- 每个平台 tuple 都测试 bootstrapper 首装/升级、卸载后重装、应用内更新，以及双击 MSI 和 `msiexec /i` 直接安装；后二者必须在 `InstallInitialize`/首个产品写前因无 admission ticket 拒绝。
- trial VM 读取 child token 的 TokenUser、Logon SID、Session、AuthenticationId、Integrity Level、ElevationType、group attributes 和 privileges，证明桌面进程未继承高完整性协调器令牌。
- 开放 APP-004 的每个 Win32 adapter 都在用户可写 CWD/PATH/TEMP 和同名 DLL/依赖替换语料下运行 planting suite；任一 payload 被加载即门禁失败并保持 capability disabled。

R3 功能还必须在操作后完成：

- 连续 3 次正常重启。
- `DISM /Online /Cleanup-Image /ScanHealth`。
- `sfc /verifyonly`。
- Windows Update 检查。
- 设备管理器、网络、启动和关键应用冒烟测试。

<a id="qpn-sec-13-4"></a>
### 13.4 网络与隐私验收

- 对 license activate/validate/refresh/deactivate/reconcile、规则更新和应用更新分别抓包。
- 验证请求仅包含第 5.7 节允许字段。
- 在包含特殊文件名、完整路径和已知哈希的测试数据集上确认这些值不会出现在网络流量、服务日志或 URL 中。
- 验证生产 WebView 的直接 fetch、导航、图片、字体、WebSocket 和重定向外带均被阻止；仅 Rust 类型化客户端能建立允许连接。
- 验证未知请求字段、未知响应字段、跨 origin 重定向、凭据头跨服务传递和超过跳数的响应均失败关闭。
- 验证 license reconcile 不携带 access/refresh token、不接受 WebView 字段、不产生席位/轮换/撤销副作用；`committed/pending/notCommitted/recoveryRequired` 与 mutation 摘要、设备 PoP、期限和 negative fence 逐分支通过正负向测试。
- 断网时本地只读分析和已安装规则仍可工作；更新失败不删除当前有效规则。
- 不存在未声明的遥测、错误上报或第三方分析请求。

<a id="qpn-sec-13-5"></a>
### 13.5 发布门禁

V1 发布必须同时满足：

1. **GATE-001 构建与供应链证据**：`pnpm build`、`pnpm test` 和 Rust 测试全部通过；可信 builder 签名的 CI provenance 必须把最终 artifact 绑定到已重放的 M0→最终 build input 源码派生及 capability manifest。最终 artifact 的 CycloneDX JSON 1.6 SBOM、canonical JSONL findings/dispositions 和 `DependencySecurityReport v2` 全部可解析、复算且 `unresolvedCriticalOrHighCount=0`；`unknown` severity、孤立/重复/过期处置或仅自报计数均失败。
2. **GATE-002 平台签名**：Windows x64、ARM64 安装包均完成代码签名和干净机验证。
3. **GATE-003 P0 与平台完备证据**：M1 默认 P0 及本构建实际启用能力对应的“P0（条件）”均有自动化或可重复人工验收记录；PLAT-002/T-PLAT-001~004 必须证明 registry 精确 36 项、三个 profile 计数正确、252 个发布 test obligation 与第 13.6/M1 双向相等、definition 的 capability/coverage 与摘要保护登记相等，且每个 required tuple 都有 passed run。缺失/blocked tuple、已知 test ID 错绑 capability/降级 coverage、自由 selector 缩小或 pairwise 抽样均失败；关闭的条件能力还必须同时通过“后端拒绝 + UI/发布声明无入口”测试。
4. **GATE-004 缺陷阈值**：任何已启用能力均不得存在未关闭的 P0，且不得存在未关闭的 P1 安全/数据损失、权限或授权、自动化、规则或应用更新/安装器供应链、隐私、恢复或发布完整性问题；其他 P1 只能按第 13.6 节限期豁免。绑定任一 M1 必交里程碑的 P1 不属于“其他 P1”，机器登记必须为 passed，不能以 waived、关闭或隐藏入口通过。
5. **GATE-005 故障注入**：QPC1 隔离/导出/清除、许可证 challenge/WAL 时钟推进与对账、三类服务控制面、网络白名单、MSI 安装/卸载 0/3010/1641、重启边界、试运行和三类 `InstallRecoverySource` 完成故障注入。
6. **GATE-006 性能基线**：性能基准已归档且没有未批准的回归。
7. **GATE-007 读者测试**：当前文档通过无上下文读者测试，读者能正确回答范围、风险、恢复、联网、失败和路线图；该测试不构成正式批准。
8. **GATE-008 R4 分能力门禁**：每个拟开放 R4 `actionKind` 分别关联 R4-001、专项安全用例和产品/安全/测试签署；通过隔离清除门禁不自动开放原文件永久删除或设备擦除。
9. **GATE-009 恢复签名证据**：归档 release-key authorization、revocation、epoch migration/recovery package 的离线恢复签名仪式、应用内 key-set 对照、key ID/公钥字节/指纹唯一性、同公钥别名不计票、阈值/用途验证和高水位测试证据。
10. **GATE-010 正式文档批准**：`DesignDocumentApprovalRecord v2` 必须引用状态为 `Approved` 的精确文档版本/SHA-256、责任人、精确 approval statement 和受外部组织信任根认证的 `GovernanceTrustPolicy v1`；产品、桌面端、安全、测试、发布五个角色各由不同自然人通过授权 key 签名或受信评审系统 receipt 批准。复制证据、同人换 key/账号、已撤销身份、策略降级、任一字节变更、`Draft`、缺角色或只有 GATE-007 均失败。

本文档当前仍为 `Draft`，因此 GATE-010 当前状态是 **blocked**；这是发布前待完成的真实门禁，不得把草案自评或读者测试写成已批准。

<a id="qpn-sec-13-6"></a>
### 13.6 V1 追踪矩阵

下表是便于阅读的**能力级追踪摘要**，不是逐需求登记表。P0 必须有通过证据；P1 限期豁免仅允许非数据安全/权限/隐私偏差，或同时将关联能力关闭并从该构建的已交付范围、UI 和发布说明中移除。后一种“关闭并移除”只适用于不属于 M1 必交范围的能力；任何 M1 必交能力及其验收 ID（包括 P1）都不得靠关闭入口或豁免满足 M1 退出。豁免必须写明责任人、到期日和补救版本，不能继续宣称该能力完整交付。“已实现”只表示目标构建上的用例通过且证据可定位，不以代码或 UI 存在代替。

| 能力 | 风险 / 默认动作 | 需求 ID | 目标命令或模块 | 最低测试 ID | M1 交付项 | 发布证据位置 / 初始状态 |
|---|---|---|---|---|---|---|
| 计划与文件身份 | R1-R4 / 复检后执行 | INV-002~007、SAFE-001~005、CLEAN-003、PLAN-001、RESULT-001、API-001~003、PAGE-001~002、REL-001 | `create_plan`、`authorize_plan`、`execute_plan`、`grant_analysis_root`、`grant_restore_target` | T-PLAN-001~002、T-FS-001~009、T-FMUT-001~003、T-RESULT-001、T-API-003~004、T-IDEMP-001、T-PAGE-001~002、T-STATE-001 | M1-01 | `artifacts/<build>/plan-fs/` / 待实现 |
| 规则供应链 | R1-R4 / 失败关闭 | INV-001、INV-010、RULE-001~006 | 规则控制面/双槽、能力包络、`request_rule_update` | T-RULE-001~020 | M1-02 | `artifacts/<build>/rules/` / 待实现 |
| 可重建缓存 | R1 / 单文件永久删除 | INV-001、INV-007、CLEAN-001、CLEAN-002、CLEAN-004 | 扫描器、规则引擎、当前用户执行器 | T-CACHE-001~006 | M1-03 | `artifacts/<build>/cache/` / 待实现 |
| 隔离、导出副本、清除 | R2 隔离；R4 清除 | INV-011、INV-014、REC-001~007、R4-001、API-002~003、REL-001、UI-001 | QPC1 copy/full-verify/delete WAL、导出/救援、清除计划 | T-QUAR-001~026、T-QUOTA-001~002、T-BATCH-001、T-BATCH-RESTORE-001、T-BATCH-PURGE-001、T-RESTORE-IDEMP-001~002、T-R4-PURGE-001~005 | M1-04 | `artifacts/<build>/quarantine/` / 待实现 |
| 空间、大文件与排除 | R0 分析；R2 隔离 | STORAGE-001、LARGE-001、SCAN-001~002、EXCL-001 | `start_scan`、三扫描 slot 协调器、排除策略、计划命令 | T-SCAN-001~002、T-STORAGE-001~004、T-LARGE-001~005、T-EXCL-001~006 | M1-05 | `artifacts/<build>/analysis/` / 待实现 |
| 重复文件 | R0 / 只读 | DUP-001、DUP-002 | `start_scan(kind=duplicates)` | T-DUP-001~005 | M1-05 | `artifacts/<build>/duplicates/` / 待实现 |
| 应用卸载 | R0 枚举；R3 卸载 | APP-001~003 | user/machine journal、`MachineUninstallAttachment`、`create_uninstall_plan → authorize_plan → execute_plan → get_uninstall_operation` | T-APP-001~013、T-APP-018~019 | M1-06 | `artifacts/<build>/apps/` / MSI/AppX 待实现 |
| 启动项与分区 | R0 / 只读 | STARTUP-001、PART-001 | `list_startup_entries`、分区模块 | T-STARTUP-001~003、T-PART-001~002 | M1-06 | `artifacts/<build>/startup-partitions/` / 待实现 |
| 提权边界 | R3 / 同 SID UAC | INV-009、IPC-001~002 | 一次性提权执行器、冻结 execution bundle | T-IPC-001~010 | M1-07 | `artifacts/<build>/ipc/` / 待实现 |
| 审计与联网 | 全部 / 本地最小化 | INV-012、AUDIT-001、NET-001、LICENSE-001~003 | 审计存储、Rust 出站客户端、activation PoP、双密钥停用 challenge | T-AUDIT-001~004、T-NET-001~010、T-LICENSE-001~004 | M1-08 | `artifacts/<build>/privacy/` / 待实现 |
| 自动任务 | R0/R1 / 默认关闭 | INV-008、AUTO-001~006 | `save_analysis_policy`、批准/run claim revision CAS、任务计划 WAL | T-AUTO-001~015、T-IDEMP-002 | M1-09 | `artifacts/<build>/scheduler/` / 待实现 |
| 应用更新 | 发布变更 / 签名试运行 | INV-013、UPDATE-001~013、REL-001 | machine-global anchor/trust/admission、`AppUpdateJournal`、`MachineFullInstallerJournal`、`MachineProductUninstallJournal`、bootstrapper、trial/recovery、`VerifiedProductAbsenceEvidence → uninstalledPreserved` 产品缺失终态 | T-UPDATE-001~042 | M1-10 | `artifacts/<build>/update/` / 待实现 |
| 授权在线服务 | 全部 / 服务端副作用 | SVC-001 | `QPN-SVC-LIC-001`、staging/production `ServiceDeliveryRecord` | T-SVC-001 | M1-SVC-01 | `artifacts/<build>/services/license/` / blocked |
| 规则在线服务 | R1-R4 / 供应链 | SVC-002 | `QPN-SVC-RULE-001`、staging/production `ServiceDeliveryRecord` | T-SVC-002 | M1-SVC-02 | `artifacts/<build>/services/rules/` / blocked |
| 应用更新在线服务 | 发布变更 / 供应链 | SVC-003 | `QPN-SVC-UPDATE-001`、staging/production `ServiceDeliveryRecord` | T-SVC-003 | M1-SVC-03 | `artifacts/<build>/services/update/` / blocked |
| 正式领域契约 | 全部 / schema 漂移时构建失败 | API-004 | `CONTRACT-001~008`、`FormalContractRegistrySnapshot v2`、三类独立制品摘要与 canonical registry 复算 | T-CONTRACT-001~009 | M1-11 | `artifacts/<build>/contracts/` / 待实现 |
| UI 与平台发布 | 全部 / 保守展示 | PLAT-001~002、PERF-001、RELEASE-001、UI-001~003、GOAL-006、GATE-001~010 | React UI、安装包、`PlatformTupleRegistry v3`、`ReleaseCapabilityManifest v2`、`GovernanceTrustPolicy v1`、源码派生/CI provenance、SBOM/`DependencySecurityReport v2`、`ReleaseGateManifest v4`、`SignedReleaseAttestation v1`、三服务 records、`DesignDocumentApprovalRecord v2` | T-DOC-001、T-PLAT-001~004、T-RELEASE-001、T-UI-001~005、T-WIN-001~008、T-SVC-001~003 | M1-11 | `artifacts/<build>/release/`，含 19 个固定顶层证据、六服务记录、传递 refs、252 test obligations、36 tuple、实际发布能力/依赖配置和同一 `Approved` 文档版本/SHA-256 的五角色可认证证据 / 当前 Draft，blocked |

条件能力不混入 M1 摘要。`RELEASE_CAPABILITY_POLICY` 中只有 `genericWin32Uninstall/permanentOriginalFileDelete` 两项为 conditional；其余 16 项类型上只能 enabled。启用任一通用 Win32 EXE 卸载适配器的构建必须单独登记 `APP-004 → T-APP-014~017 → GATE-003`，证据放在 `artifacts/<build>/win32-uninstall/`；启用 `permanentDeleteOriginal` 的构建同样单独登记 `R4-DELETE-001 → T-R4-DELETE-001~004 → GATE-008`，证据放在 `artifacts/<build>/r4-original-delete/`。未启用构建分别登记 `disabledByReleasePolicy + UI absent + backend failClosedDisabled` 的后端/UI 测试，不把两项计入 M1 必交通过率。

每个发布构建生成并归档机器可读治理证据。`test-definitions.jsonl` 按 test ID 唯一定义 Given/When/Then、需求映射、权威 capability 和闭合平台 coverage；`test-runs.jsonl` 按 `runId` 记录一次具体平台执行；`trace-register.jsonl` 以 `requirementId + testId + runId` 为唯一键物化可追踪结果；`waivers.jsonl` 只保存非受保护 P1 的具名豁免。其余文件包括 `PlatformTupleRegistry v3`、`FormalContractRegistrySnapshot v2`、`DesignDocumentApprovalRecord v2`、`ReleaseCapabilityManifest v2`、`GovernanceTrustPolicy v1`、M0 三文件、源码 patch/派生记录、CI build provenance、CycloneDX SBOM、dependency findings/dispositions 及 `DependencySecurityReport v2`。

`ReleaseGateManifest v4.evidenceFiles` 固定恰好 19 个具名顶层 `EvidenceFileRef`：四份 quality JSONL、platform/formal registry、document approval、release capability manifest、dependency report、SBOM、M0 baseline manifest/snapshot/verification、governance trust policy、source derivation record/source patch、dependency findings/dispositions 和 CI provenance；字段及相对路径由类型中的 literal 固定，缺项、多项或改名均失败。另有 staging/production × 三服务共六份 `ReleaseServiceEvidenceRef`，每份也携带固定相对路径、原始文件 SHA-256 与 canonical payload digest。审批 receipt、M0 CI provenance、dependency disposition evidence 等由这些文件继续引用的对象属于传递证据；验证器必须遍历整个可达图并复算每个原始文件，不能只验证顶层父文件摘要。trace 至少包含优先级、风险、capability 状态、实现模块/版本、平台/obligation 绑定、结果、证据路径/SHA-256、发布 artifact SHA-256、release capability manifest canonical digest、source commit、构建号、CI provenance/attestation digest、执行人和日期。重复平台轮次各有独立 runId，不能覆盖或聚合成一行；能力摘要中的范围字符串必须展开为实际 ID。

gate 完成后才生成固定路径 `release/signed-release-attestation.json` 的 `SignedReleaseAttestation v1`；它不属于上述 19 项且 gate 不得引用它。发布验证入口必须同时读取固定路径 `release/release-gate-manifest.json` 和该 attestation，以分发系统预置、独立于 bundle 的组织 trust root 验证 governance policy 与 release signer，再核对 gate 原始摘要、canonical digest 和 artifact。当前文档为 `Draft`，因此不得生成声称有效的 M0 verification、document approval、passed release manifest 或 signed release attestation，GATE-010/M1-11 保持 blocked。

JSONL 编码对零记录有单独规范：`waivers.jsonl` 等任一零记录 JSONL 必须是 0 字节文件，其 SHA-256 按空字节串复算；不得写 BOM、空行或单独 LF。只有非空 JSONL 才要求每条记录是 RFC 8785 JSON、UTF-8、无 BOM、记录后单 LF且文件末尾恰一 LF。解析器必须用空文件和 LF-only negative vector 验证该规则。

全部 `EvidenceRelativePath` 都以 release bundle 根解析，wire 值必须是 NFC、小写 ASCII 的 `/` 分隔相对路径，每段非空且不得为 `.` 或 `..`；禁止绝对路径、盘符、UNC/device path、反斜杠、冒号/ADS、控制字符、尾随点/空格、重复分隔符和 Windows ordinal-ignore-case 碰撞。验证器从已固定的 bundle 根逐段拒绝重解析点并读取原始字节；同一路径被多次引用时只能对应同一 SHA-256，路径相同而摘要不同、摘要相同但声明路径逃逸或任何传递引用不可解析都失败关闭。

权威定义文件固定为仓库 `quality/v1/test-definitions.jsonl`；本轮只定义契约，不创建该代码产物。第 13.2 节 97 行是高风险/跨域用例的详细说明，不是发布测试全集；`RELEASE_TEST_OBLIGATIONS` 已把第 13.6/M1 的所有范围展开为当前精确 252 个 ID。M1 退出前每个 ID 必须在权威文件恰好出现一次，并至少映射一条第 9 章需求；definition 的 capability/coverage 必须从 obligation 派生。未来新增 test ID 必须在同一次受评审变更中更新 obligation、obligation digest、platform registry digest和受影响追踪/M1 基线。未物化、悬空、重复、错 capability 或未登记 coverage 的 ID 都使对应能力保持 `notImplemented/capabilityDisabled`。条件能力关闭时也必须物化专门的 disabled 后端/UI 测试，不能用“无测试”表示关闭。


#### 13.6 发布与追踪契约分片索引

以下文件共同构成第 13.6 节的机器可读治理契约。类型检查和发布验证必须按文件名前缀顺序拼接，并先加载第 8.3 节运行时契约。

| 顺序 | 发布契约片段 |
|---|---|
| 1 | [13.6.1 平台登记与发布测试义务](release-contracts/00-platform-obligations.md#qpn-sec-13-6-1) |
| 2 | [13.6.2 测试定义、运行、追踪与豁免](release-contracts/01-test-trace.md#qpn-sec-13-6-2) |
| 3 | [13.6.3 治理信任与文档批准](release-contracts/02-governance-approval.md#qpn-sec-13-6-3) |
| 4 | [13.6.4 发布能力策略与清单](release-contracts/03-release-capabilities.md#qpn-sec-13-6-4) |
| 5 | [13.6.5 M0 基线与验证记录](release-contracts/04-m0-baseline.md#qpn-sec-13-6-5) |
| 6 | [13.6.6 依赖安全、源码派生与构建来源](release-contracts/05-dependency-provenance.md#qpn-sec-13-6-6) |
| 7 | [13.6.7 发布门禁根与签名声明](release-contracts/06-release-gate.md#qpn-sec-13-6-7) |


`TestDefinitionRecord/TestRunRecord/TraceRegisterRecord` 的发布 schema 固定为 v2，v1 不得进入 M1 证据。`TestDefinitionRecord` 是以 252 项 `ReleaseTestId` 判别的闭合联合；每份 definition 的 `capabilityId/platformCoverage` 由 `PlatformTupleRegistry v3.releaseTestObligations` 唯一决定，不接受 definition 自行选择。23 个 obligation group 按规范顺序保存，组内 ID 按 UTF-8 字节排序，展开后必须无重复且与第 13.6/M1 的测试范围双向相等；`releaseTestObligationsDigestSha256` 只对该规范数组计算。每份 run 至少包含 `releaseId/testId/runId`、定义、obligation 与 capability manifest 摘要、fixture/安全语料版本、平台绑定、Windows edition/version/build/UBR、架构、文件系统、介质、安装器/权限及全部参与用户/Logon SID/session/AuthId 上下文、开始/结束时间、严格结果、原始证据和带角色的二进制/MSI/NSIS/规则包 SHA-256。`registryTuple` 的重复维度必须与 tuple key、registry 和主机/卷/token 实测值完全相等；trace 的 release/platform/obligation/artifact/capability manifest/source/build/CI 字段必须逐字段来自引用 run，不能独立填写。CI 校验 definition、run、trace、waiver、需求、规则和实现双向可达，无重复/悬空，并对每个 profile 展开的 `(requirementId, testId, platformTupleId)` 要求至少一条 passed trace。

CI 还必须分别复算：`FormalContractRegistrySnapshot v2` 的精确八项/三制品/canonical digest；`PlatformTupleRegistry v3` 的 36 key、字段分解、UBR、profile 36/24/12、23 组/252 个唯一 test obligation、每个 ID 的 capability/coverage、obligation digest 与排除自身的 registry digest；`DesignDocumentApprovalRecord v2` 的精确 document approval statement、当前文件版本/SHA-256、责任人、五角色批准、记录摘要和治理策略绑定。

`GovernanceTrustPolicy.policyDigestSha256 = SHA256(UTF8(RFC8785(GovernanceTrustPolicyCanonicalPayload)))`。`organizationRootSignatures` 不进入该 payload；它们以固定 domain `qingpan.governance-trust-policy.v1\0` 签署 policy digest。发布验证器必须从安装器或分发验证环境预置的外部组织 trust root 取得 root key、阈值、最小 `policySequence` 和撤销 floor，不能相信候选 bundle 自带的根。policy 必须在有效期内，principal/key/SPKI/review identity 全局唯一，撤销集合生效；同一 key 或评审 identity 只能映射一个稳定 principal/human identity。

每份 `GovernanceRoleApproval` 的验签输入精确为 `UTF8(RFC8785({domain:"qingpan.governance-role-approval.v1",statementMediaType,approvalStatementDigestSha256,role,principalId,approvedAtUtc}))`。`signedStatement` 使用 policy 中该 principal 的授权 key 验签；`reviewSystemDecision` 的 signed receipt 必须是同一字段外加 `reviewSystemId/tenantId/reviewerSubjectId/changeId/decisionId/decisionRevision/decision="approved"` 的 RFC 8785 对象，并由 policy 固定 receipt key 验签。五角色 tuple 中 principal 与从 policy 解析出的 `humanIdentityId` 必须分别两两不同；换 key、评审账号或 attestation 分支不能绕过去重。`DocumentApprovalStatementCanonicalPayload`、`M0BaselineApprovalStatementCanonicalPayload` 和 `DependencyDispositionStatementCanonicalPayload` 的 digest 均为各自完整 RFC 8785 字节的 SHA-256，角色、media type 或 statement 任一错配都失败。

`BaselineManifest.manifestDigestSha256`、`M0BaselineVerificationRecord v2.recordDigestSha256`、`ReleaseCapabilityManifest v2.manifestDigestSha256`、`ReleaseSourceDerivationRecord v1.recordDigestSha256` 和 `DependencySecurityReport v2.reportDigestSha256` 均分别对排除自身摘要字段的 RFC 8785 canonical payload 计算。capability evidence root 的唯一输入是 `{domain:"qingpan.m0-capability-evidence-root.v1",baselineId,capabilities}`：`capabilities` 必须是固定 19 个 `BaselineCapabilityId` 按 UTF-8 字节序排列的数组，每项恰含 `capabilityId/state`、去重并排序的 `runIds`，以及按 `relativePath` 排序且路径全局唯一的 `{relativePath,fileSha256}`；计算 root 前必须逐文件读取原始字节复算 SHA-256。缺 capability、把证据改挂另一 capability、重复/不可解析文件、run/evidence 顺序非规范或声称 `verified` 却无通过 run 均失败。

`M0BaselineVerificationRecord v2` 内嵌的 approval statement 必须逐字段等于同一记录的 baseline manifest 原始/canonical 摘要、snapshot、workspace/capability root、source、M0 CI provenance、trust policy 与 `result=passed`，`approvalStatementDigestSha256` 只对该精确对象计算；五份 approval 必须绑定同一 digest。M0 的 source commit 只描述重放起点，不要求等于最终发布 commit。最终源码树 root 的叶子固定为 `{relativePath,fileMode,byteLength,fileSha256}`，路径按本节规则规范化、按 UTF-8 字节排序且无重复/大小写碰撞，root 为 `SHA256(UTF8(RFC8785({domain:"qingpan.source-tree.v1",entries})))`。

`ReleaseSourceDerivationRecord v1` 必须从 gate 引用的精确 M0 manifest/snapshot/verification 重放出 `m0ReplayedSourceTreeRootSha256`。`git-binary-full-index-no-renames-v1` patch 由固定 Git/replay toolchain 以 binary、full-index、no-renames、no-ext-diff 模式生成，在无网络、空白输出目录中仅对规范相对路径应用；patch input 必须等于 M0 replay root，patch output、final source tree 和 builder 实际 build input 三个 root 必须完全相等。可信 builder 再签署 `CiBuildProvenanceStatementCanonicalPayload`，其 statement 同时绑定 derivation record、最终 artifact、最终 source/build 和 release capability manifest；key 必须由有效 policy 授权 `ciBuildProvenance`。缺 patch 字节、输入/输出断链、重放工具不符或 build 使用另一源码树均失败。

dependency findings/dispositions 两份文件按本节 JSONL 规则解析。`findingKeySha256 = SHA256(UTF8(RFC8785({scannerIdentity,scannerDatabaseSnapshotSha256,componentBomRef,componentPurl,advisoryId})))`；key 或完整记录重复均拒绝。`findingsRootSha256` 和 `dispositionsRootSha256` 分别为 `SHA256(UTF8(RFC8785({domain:"qingpan.dependency-findings.v1"|"qingpan.dependency-dispositions.v1",records})))`，其中完整记录按 `findingKeySha256` UTF-8 排序。每个 critical/high finding 必须恰好 join 一条未过期、证据文件全部可解析、security/release 两个不同受信自然人批准的 `fixed/notAffected/falsePositive` disposition；orphan disposition、`unknown` severity 和 risk-accepted 不能通过。报告中的全部 count 与两个 root 必须由 join 结果复算，不能信任自报的 `unresolvedCriticalOrHighCount=0`。

`RELEASE_CAPABILITY_POLICY` 和其 digest 必须精确固定 16 个 enabled M1 项与两个条件项，mapped discriminated type/closed JSON Schema/Rust validator 均拒绝重分类、M1 disabled、enabled+absent 或 enabled+failClosed。capability manifest 只含预构建可知的 release/source/build/configuration，不含最终 artifact 或发布后 attestation；artifact 内嵌 **release capability manifest canonical digest**，可信 build provenance 以该 manifest 和源码派生为 material，gate 再绑定 provenance，因而不存在摘要环。

`ReleaseGateManifest v4.manifestDigestSha256 = SHA256(UTF8(RFC8785(ReleaseGateManifestCanonicalPayload)))`，明确排除自身。验证器按固定 literal 路径和原始字节复算 19 个顶层 evidence file、六份 service record 以及所有可达的传递 evidence refs，并拒绝不可达替代文件、路径逃逸、大小写碰撞、同路径异摘要和额外顶层字段。零记录 JSONL 必须为 0 字节；非空 JSONL 固定每行 RFC 8785、LF、无 BOM且末尾单 LF。验证器重放 M0 与 source derivation，要求三个 M0 文件、各内部摘要、`result=passed`、五角色批准、M0 root、patch chain、final/build-input root 全部相等；再要求 capability、obligation、SBOM/findings/dispositions/report、run/trace、service、artifact 和 builder provenance 逐字段绑定同一 release/source/build。

`SignedReleaseStatementCanonicalPayload` 的 digest 只对其 RFC 8785 字节计算；`SignedReleaseAttestation v1` 用有效 policy 中 purpose=`qingpanReleaseAttestation` 的 key 对该 digest 验签，并绑定 gate 原始文件 SHA-256、gate canonical digest、artifact、CI provenance、source derivation、release/source/build 和 trust policy。release signer 必须与本次五角色 approver 和 builder 的 human identity 均不同，key 未撤销且 policy 有效；分发验证器还必须用外部组织 trust root 验证该 policy。gate 与 build provenance 都不得引用 attestation，attestation 只在二者完成后生成，因此摘要图无环。任一跨 release/build 的 baseline、capability、dependency、trace、platform、contract、approval、service、derivation 或 attestation 混用都失败；文档任一字节变化后旧批准、gate 与 attestation 立即失效。

`SVC-001~003` 的每条 trace 还必须填写 `serviceDeliveryRecordDigestSha256` 和精确 `serviceDeploymentEnvironment`，并由 CI 按 `SVC-001→M1-SVC-01/QPN-SVC-LIC-001`、`SVC-002→M1-SVC-02/QPN-SVC-RULE-001`、`SVC-003→M1-SVC-03/QPN-SVC-UPDATE-001` 的闭合映射读取不可变 `ServiceDeliveryRecord`；其他需求禁止填写这两个字段。staging/production 是两份独立 record/run，不能用同一 mock 结果复用；只有 record `readiness=passed` 且所有 NonEmpty run 集合均可解析为通过证据时，trace 才可为 passed。

`m1MandatoryP1RequirementBindings` 是当前 V1/M1 的权威 P1 里程碑登记，CI 从受评审的登记生成规范 `milestoneBindingDigestSha256`，不得信任 trace 自报的 `milestoneIds`。登记中的 requirement 必须与第 13.6、15.2 节双向完全相等；缺项、多项、错 milestone 或摘要不符均使构建失败。P0 一律禁止豁免；登记中的 M1 P1 只允许 `passed` 或 `gateFailed`，不允许 `waived`，即使能力被隐藏或关闭也不例外。当前全部正式 P1 都属于 M1，因此 `WaivableP1RequirementId` 为 `never`，不会生成合法 `WaiverRecord`；未来只有先批准并加入正式需求、且明确不属于任何 M1 必交里程碑的 P1，才可扩展该联合并按限期豁免流程处理。M1 退出对每个权威 requirement/test/platform tuple 只接受 `disposition=passed`，`failed/blocked/waived` 任一存在即失败。

---

<a id="qpn-sec-14"></a>
## 14. 发布、更新与运维

<a id="qpn-sec-14-1"></a>
### 14.1 安装与应用更新

- Windows 分别生成原生 x64/ARM64 MSI payload、签名 `QingpanSetup.exe` bootstrapper 与 NSIS 完整迁移包。所有 MSI 发布（首次安装、手动升级、应用内更新和产品卸载）都必须由受保护 admission broker 先验证 bootstrapper/MSI 签名与 metadata、锁定 `MachineInstallAnchor`、提交并刷盘 `InstallerAdmissionAttempt(admitted)` 和用途绑定 ticket，再调用 Windows Installer；应用内协调器和 ARP 卸载入口复用同一 broker。ticket 最长有效 10 分钟，只能在 `callArmed` 前消费，并完整绑定 subject、package/current artifact、caller/installer/target provenance、expected base、anchor revision、精确安装/卸载属性和 invocation nonce。MSI 内还有一个 immediate fail-closed ticket validator，排序在 `InstallInitialize`、deferred custom action、服务/文件/注册表写入和任何产品副作用之前；NSIS 同样在首个产品写入前校验。任何 `MsiInstallProduct/CreateProcess` 或 NSIS 产品写入前必须先刷盘 `callArmed`，调用边界再写 `inFlight`；两状态崩溃后只对账、绝不第二次调用。裸 `msiexec /i`、`msiexec /x`、双击 MSI、缺失/过期/不匹配 ticket 一律返回 `UPDATE_ADMISSION_BLOCKED` 且零产品写入，因此 MSI payload 不是可直接安装或卸载的独立入口。`MajorUpgrade(AllowDowngrades=no)` 与包内校验只是第二道防线；旧但签名有效的完整包不得清空或降低 installer、epoch、manifest、恢复序号或撤销高水位。
- 完整安装不依赖 `AppUpdateJournal`。broker 必须在同一受保护 machine 事务中生成不可变 `installerAttemptId`，并同时写入 `InstallerAdmissionAttempt(attemptVersion=3)` 与 `MachineFullInstallerJournal(admitted)`；journal 以 `installationRequestId + installerAttemptId + admissionId` 建唯一索引，其 sequence、anchor/installation ID 及 embedded admission 必须逐字段相等。`admitted → callArmed → inFlight → resolved/recoveryRequired` 及零调用的 `resolvedWithoutCall` 每次都与 attempt、ticket 消费、anchor floor 和 active-admission pointer 在同一 machine CAS 提交；`InstallerTrustState.activeAdmission` 只是当前锁/指针，不是历史事实源。`admitted/callArmed/inFlight/recoveryRequired` 及被 file grant、idempotency、`TrustedRecoveryAttempt` 或 resolution 引用的记录禁止按 TTL 删除；终态详情只能在零引用的高完整性 GC 事务后裁剪，且必须保留不短于产品族 anchor 生命周期的 source tuple、最终 state、sequence 和 evidence digest tombstone。
- 在读取、创建或更新任何 anchor/security state 前，bootstrapper 必须用 `IsWow64Process2` 取得原生机器架构，并从 MSI Summary Information/签名发布 metadata 独立确认包架构；x64 包在 ARM64 或任何非原生组合均返回 `UPDATE_PLATFORM_MISMATCH`，零 anchor 读写和零 MSI 调用。首次安装时 bootstrapper 才以受保护 HKLM/ProgramData 事务创建 `MachineInstallAnchor` 和随机 `installationInstanceId`；唯一键固定为 `productFamilyId`，`nativeMachineArchitecture` 是锚内不可变属性，UpgradeCode 和架构都不参与另建唯一键。首个 `canonicalMsiUpgradeCode` 只能来自该 bootstrapper 的编译期产品常量，解析为 canonical GUID 后写入锚；不得从待安装 MSI、manifest 或命令行取不同值来创建另一锚。正常卸载仅把 anchor 标为 `uninstalledPreserved`，不得删除 instance ID、架构、规范 UpgradeCode、installer floor、sticky 撤销、UpgradeCode migration floor 或 recovery floor；重装必须复用同一锚。因此 U5 卸载后运行仍受信的 U1 或不同架构旧包，只能命中/拒绝现有产品族锚并在首个产品副作用前失败，不能伪装首次安装。合法 UpgradeCode 变更必须提交 `RecoverySignedMsiUpgradeCodeMigration`：purpose、同一 anchor ID、产品族/原生架构、expected current code、expected anchor revision 与 prior sequence 全匹配，`migrationSequence` 严格递增且同序号异 payload 拒绝；在一个 machine CAS 中更新原锚的 code、revision、sequence/hash 后才可继续，绝不创建第二锚。若机器已有产品/安全状态但 anchor 缺失、重复、DACL/owner 不符或字段冲突，则普通安装/更新失败为 `UPDATE_MACHINE_ANCHOR_INVALID`，只能走阈值签名恢复流程，不能生成新 identity 绕过历史。
- ARP 的 UninstallString 只允许指向受信 `QingpanSetup.exe --uninstall`，不得指向裸 MSI 或拼接用户参数。bootstrapper 重新枚举唯一 anchor、ProductCode/UpgradeCode/current binary provenance，创建 `InstallSubject(kind=productUninstall)` 和 `MachineProductUninstallJournal`；ticket 精确绑定 `REMOVE=ALL`、`REBOOT=ReallySuppress` 及当前制品/anchor。调用返回 `0` 后仍须验证产品、服务和目标 binary 全部 absent 才在同一事务写 `uninstalledPreserved`；`3010` 保持 `rebootPending` 与 active admission，boot ID 变化后再验证；`1641` 或结果边界不明进入 `recoveryRequired`；其他失败保持产品状态和证据，不自动重调。任何分支均不删除/降低 anchor、installer/recovery floor 或 sticky 撤销。该 journal 与 app update、`MachineFullInstallerJournal` 一样可通过 `InstallRecoverySource` 进入只读对账和阈值签名恢复。
- `MachineProductUninstallJournal.uninstalledPreserved` 有两种且仅有两种合法完成方式：`installerCall` 绑定原 attempt `resolved`；`trustedRecovery` 保留原 attempt `recoveryRequired` 并追加 `recoveryResolutionRecordId`。两者都必须保存 `VerifiedProductAbsenceEvidence`，并在同一 machine 事务中 CAS anchor lifecycle/revision、journal sequence 和 recovery floor；不得把恢复完成改写成原安装器成功。
- 规则通道 setting 是 per-user；应用通道是 `MachineAppUpdatePolicy`，由拆分令牌管理员在高完整性原生页设置，并以 `machineInstallAnchorId + installationInstanceId` 唯一。规则 setting 没有 machine revision，set 请求禁止 `expectedRevision`；应用 set 必须带当前 `policyRevision`，不存在旧策略时用规范 `"0"`，并在 machine 事务中 CAS，缺失、非零首次值或冲突均返回 `UPDATE_CONFLICT`。release-key/manifest 授权、高水位和精确 LKG 按 stable、beta、internal 每机隔离；package hash 与 blocked version 撤销同时并入机器锚点下全局 sticky 状态。切换策略 revision 原子使候选/暂存失效，但不得清空全局撤销或 installer floor。每个 machine anchor 只有一个具名全局 update mutex，journal sequence、policy revision 和实际已安装身份还必须用 CAS 防止两个管理员/会话竞争；冲突返回 `UPDATE_CONFLICT`。
- 应用内 `RecoveryKeySet` 在构建期和运行时同时校验：key ID、规范化公钥字节和指纹分别唯一，`1 <= threshold <= distinct public-key count`；同一公钥的不同 ID、重复签名字节、未知 key、混合 key set 或错误 purpose/domain/channel 不计票并使文档失败。恢复签名输入使用固定 domain separator，覆盖 media type、业务 domain、channel、schema 与 canonical payload hash；`machineMsiUpgradeCodeMigration` 是独立 purpose，不能由 epoch migration、恢复包或普通 release 签名代替。恢复 key set 轮换只能随代码签名应用事务安装，在目标 committed 后激活；回滚保持旧 set。
- Rust 从固定 manifest、key authorization、revocation 和 epoch migration 端点获取第 8.3 节类型。验证顺序固定为：格式/大小 → recovery key-set/目的/不同公钥阈值 → key authorization/revocation → 连续 epoch migration 链 → manifest 高水位与 release 签名 → 版本/平台/安装器约束 → 固定端点包下载 → 字节数/哈希/代码签名/MSI 元数据。
- 每通道在高完整性状态中事务维护授权、release-key/manifest 撤销、epoch migration、manifest 的最高序号/哈希和通道 sticky 集合。低序号、同序号异哈希、撤销集合缩小、过期控制面、未知/撤销 key、架构/通道不符均拒绝。`releaseEpoch` 只能沿恢复阈值签名且 `fromEpoch → toEpoch` 连续的迁移链递增；迁移 payload 哈希必须与 manifest 按顺序完全相等。任一通道接受有效 package hash 或 blocked version 撤销时，必须在同一 machine 事务把它们并入 `InstallationArtifactSecurityState` 的跨通道集合，再使相应 staged journal 失效；通道切换、取消、回滚、卸载、重装和可信恢复均不得缩小该集合。`blockedApplicationVersions` 入库时必须是 canonical SemVer，重复等价值拒绝，匹配时忽略 build metadata。命中 target/LKG 时零安装器调用；命中 `currentArtifact` 时将其持久化为 `readOnlyBlocked`，跨全部用户/通道关闭新的 R1-R4 和自动任务，只保留 R0、隔离副本导出/诊断及安装严格更高且未撤销版本。
- `targetVersion` 使用 canonical SemVer 2.0.0；其 numeric core 必须在 MSI 范围内并精确等于 `msiProductVersion`，`binaryFileVersion` 必须等于发布流水线按 `major.minor.patch.installerBuildSequence` 生成的四段版本，beta/internal 的 prerelease 顺序由 signed build sequence 判定。manifest、MSI Property、已安装产品和目标二进制四者任一不一致即拒绝。创建新的 normal admission 必须同时满足 `targetVersion > 当前实际版本` 且 `installerBuildSequence > MachineInstallAnchor.installerAdmissionFloorBuildSequence`；该 anchor 字段是唯一 installer admission floor，`InstallerTrustState` 不复制第二份。服务器响应不能指定降级，唯一降级路径是本机 journal 的精确 LKG。
- manifest 不包含 `packageUrl`；`packageSha256` 只能映射到固定包端点。manifest/控制面响应上限 256 KiB，MSI 上限 500 MiB，下载墙钟 30 分钟；`Content-Length`、实际字节数和声明大小必须一致，未知字段、归档/压缩嵌套、超限或超时均失败关闭。
- 更新事务为 `downloaded → verified → staged → installPrepared → installing → trialPending → trialLaunchArmed → trialRunning → committed`；重启走 `rebootPending`，失败走 `rollbackPending → rollingBack → rolledBack/recoveryRequired`，安装调用前的合法放弃走 `cancelled`，人工可信恢复只追加 `recoveredExternally`。每次转移、安装/回滚调用前后均刷盘 machine journal；全机二进制提交与任一用户 profile 迁移是不同事务。
- 普通权限客户端下载后只产生候选更新 ID。高完整性协调器从受保护状态读取 manifest，通过保持源句柄复制到仅 `SYSTEM/Administrators` 可写的随机暂存文件，重算哈希并刷盘。随后按编译期 `ProductAuthenticodePolicy` 验证 bootstrapper/MSI：leaf SPKI 必须命中 pin set、EKU 必须是 code signing、时间戳和链有效，证书撤销状态只能来自新鲜在线结果或 `RecoverySignedAuthenticodeOfflineStatus`；后者绑定精确 artifact/evidence、短有效期和独立单调 sequence/hash，unknown 失败关闭。发布者 subject/display name 只用于展示，不是信任锚。SPKI 轮换必须由仍受信代码签名版本随安装事务交付，或由 `RecoverySignedAuthenticodeSignerRotation` 从当前 signer-set digest、policy version 和最低 build sequence 严格推进；离线状态与轮换各自的 floor 在卸载、重装、回滚和可信恢复后均不得降低，同序号异哈希拒绝。再验证 MSI ProductName/ProductCode/UpgradeCode、架构和版本元数据；MSI 与 manifest 的 UpgradeCode 必须彼此相等，并等于已命中锚的 `canonicalMsiUpgradeCode`（或刚在同一 admission 前由有效阈值迁移推进的值），历史包中不同但有效签名的值不得触发新锚。随后从 MSI 文件表/签名发布证据独立提取 `Qingpan.exe` 的预安装 hash、相对路径、FileVersion 和 signer SPKI，与 manifest 完全一致后计算 `targetBinaryProvenanceDigestSha256` 并写入 admission；不得安装后读取当前 binary 再“自绑定”。更新协调器和 recovery executor 自身也执行同一代码签名与安全加载上下文策略。UI 路径、哈希和安装参数均不被接受。
- 统一 `authorizeArtifactUse(normalInstall|rollback)` 在 `verified→staged`、`staged→installPrepared`、回滚前和每次启动对账时执行。normalInstall 必须在全局 mutex 内证明 machine policy/revision 未变、控制面仍未过期、manifest/key 仍授权、通道与全局 key/manifest/package/目标版本均未撤销、epoch/sequence 不低于 sticky floor，并重新读取实际已安装 ProductCode/UpgradeCode/ProductVersion/AppVersion/build sequence 与 journal expected base 做 CAS；刷新失败或离线且控制面已过期时保留 staged 但禁止安装。接受新撤销时，在同一事务使相应 staged journal 失效。
- 应用内自动更新只有在 `previousLkg.kind=available`，且其通道、release key、manifest、包字节、签名、MSI 元数据和全局撤销全部精确可用时才允许 `staged→installPrepared`；否则保持 staged，返回 `UPDATE_LKG_UNAVAILABLE/UPDATE_ARTIFACT_REVOKED`，不推进 admission floor、不调用 MSI。此限制不影响用户主动运行更高版本完整安装包，但完整包也必须通过共同 admission primitive。
- `staged→installPrepared` 在全局 mutex 内用一个 machine 事务同时提交：journal v4 的 `InstallAdmission(admissionVersion=2, floorAdvancedAtAdmission=false)`、`InstallerTrustState.activeAdmission=admitted`、expected-base CAS、target-binary provenance/authenticode policy 和 post-commit revision；它只记录 `installerAdmissionFloorBeforeCall`，不得推进 anchor floor。admission 绑定 machine anchor、`InstallRecoverySource`、installer kind、package/caller/target provenance、build sequence、base identity、ticket 和期限。只有即将产生首个外部副作用时，`callArmed` 事务才同时记录 `bootIdBeforeCall/callArmedAtUtc`、消费 ticket、把 attempt 置为 `callArmed`，并以 anchor revision CAS 将唯一 `MachineInstallAnchor.installerAdmissionFloorBuildSequence` 提高到 target sequence；事务刷盘后才允许 `inFlight` 和唯一一次安装器调用。floor 一经 armed，在失败、回滚、卸载、重装或可信恢复后均不得降低。floor 已等于 target 时，只有同一尚未 resolved 的 armed/in-flight admission 可对账；其他 `sequence <= floor` 的 MSI/NSIS/update 一律在首个副作用前返回 `UPDATE_INSTALLER_DOWNGRADE_BLOCKED/UPDATE_ADMISSION_BLOCKED`。
- `cancel_app_update` 只允许从 `downloaded/verified/staged/installPrepared` 进入 `cancelled`；请求必须 CAS expected journal sequence。`installPrepared` 取消还必须证明安装/回滚 attempt count 均为 0、Windows Installer 无在途调用且实际版本精确等于 expected base，并在同一事务把 active admission 置为 `resolvedWithoutCall`，记录 `floorRemainedAtPreAdmissionValue=true` 和 `exactSamePackageReadmissionAllowed=true`。`callArmed/installing` 及之后返回 `UPDATE_NOT_CANCELLABLE`。artifact 缺失、策略变化、撤销或 superseded 可走同一零调用状态约束的系统取消；它们不回退任何既有高水位，但也不为从未调用的包制造新 floor。
- 安装前若已有 pending reboot 则不调用 MSI。固定 `REBOOT=ReallySuppress` 调用返回 `0` 时先写 `msiReturnSuccess(0)` 再进入 `trialPending`；`3010` 先写 `msiReturnSuccess(3010)` 和调用前 boot ID 再进入 `rebootPending`；`1641` 写 `msiUnexpectedRestart` 后进入 `recoveryRequired`；其他合法非零码写排除 `0/3010/1641` 的 `msiReturnFailure` 后进入 `rollbackPending`。`trialPending/trialLaunchArmed/trialRunning/committed` 只接受 `SuccessfulInstallEvidence`；失败/恢复分支保存完整 `InstallEvidence`，不得另放无来源的裸返回码。MSI 返回码只能来自同一调用栈的实际返回并先刷盘，不得由安装后状态推断。若调用完成但返回证据在刷盘前丢失，只有 Windows Installer transaction、精确目标制品和 reboot requirement 三类独立证据均可验证时，才可写 `reconciledExactTarget`；任一证据不明则进入 `recoveryRequired`。`3010` 不是失败或已完成；再次 apply 只返回 `UPDATE_REBOOT_REQUIRED`，不重复调用。
- trial 管道必须使用 `FILE_FLAG_FIRST_PIPE_INSTANCE`、`PIPE_REJECT_REMOTE_CLIENTS` 和仅协调器/预期 child/SYSTEM 的 DACL。高完整性协调器不得继承自身令牌启动桌面应用；它必须取得发起拆分令牌管理员的 linked medium token，并在 `CreateProcessAsUserW` 前后证明与发起者的 TokenUser SID、Logon SID、Session、AuthenticationId 完全一致，Integrity Level 为 Medium、`TokenElevationTypeLimited`、Administrators 为 deny-only 且无启用的管理员特权。无法取得或复核精确 token 时 trial 失败并进入 rollback，不得以高完整性或另一账户 token 代替。启动顺序固定为：同一事务创建 `TrialSessionRecord(launchArmed)` 并把 journal 转 `trialLaunchArmed`，刷盘；创建带 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 的 Job；以 `CREATE_SUSPENDED` 创建 child，验证 token/映像后先加入 Job；同一事务写 child identity 并转 `trialRunning`，刷盘后才 `ResumeThread`。在 `trialLaunchArmed` 或进程创建边界崩溃只进入 `rollbackPending`，不得创建第二个 child。双方还校验 PID、创建时间、绝对映像、安装前 admission 绑定的 relative path/SHA-256/FileVersion/signer SPKI/provenance、boot ID 和安全 CWD/环境，消息含协议版本、长度、严格序号且只接受一次。nonce 通过不继承给其他进程的受限继承句柄或等效私密通道传递，不进入命令行、环境变量或普通状态文件。120 秒内的 `TrialHealthReceipt` 必须绑定 trial/update ID、版本、nonce、消息序号和检查摘要；协调器独立复核。`RULES_UNAVAILABLE` 的失败关闭可作为规则子系统健康结果，不使应用 binary trial 永久失败；收据缺失、对端不符、重放、崩溃或超时进入 rollbackPending。
- 回滚仅允许 journal 精确上一 LKG 一次。它可绕过普通 SemVer 升序和网络控制面过期，但绝不能绕过已经持久化的 sticky key/manifest/package 或 blocked-version 撤销；回滚前重验本地 manifest、包哈希/签名、MSI 元数据和当前 durable sticky 状态，命中撤销或证据不足直接进入 `recoveryRequired`。`rollbackEligibleUntilUtc` 前已进入 rollingBack 可完成；尚未开始却已过期则进入 recoveryRequired。LKG/备份只有在无 journal、trial、用户迁移或 recovery record 引用且高完整性 GC 事务提交后才能删除。
- machine trial 只验证二进制能读取旧 schema、全机状态及隔离库存一致性；不迁移所有用户 DPAPI/profile 数据。每个 SID 首次启动时使用独立 `UserSchemaMigrationJournal` 延迟迁移并备份；失败只使该 profile 进入 `readOnlyRecovery`，不能自动回滚全机二进制。破坏性用户 schema 迁移不进入自动更新。隔离文件本体在安装、回滚、profile 迁移或卸载时均不得删除。
- 每机 MSI 自动更新仅支持同 SID 拆分令牌管理员；标准用户只查看已验签候选并等待管理员/IT，不触发 over-the-shoulder UAC。app update、full installer 和 product uninstall 的 `recoveryRequired` 均默认只读且不能靠改写 journal/attempt 退出：`reconcile_update_recovery` 接受完整 `InstallRecoverySource`，只在精确目标、LKG 或 verified product absence、无在途 MSI 且 machine/schema/隔离证据全部一致时产生新 resolution；否则只能由原生选择器导入恢复阈值签名的 `RecoverySignedAppRecoveryPackage`，再由通过 `ProductAuthenticodePolicy` 的恢复执行器高完整性处理。文件 grant、assessment、恢复包、`TrustedRecoveryAttempt` 和 `RecoveryResolutionRecord` 必须逐字段绑定同一 source/admission、现有 `MachineInstallAnchor`、installation instance、原生架构和 `expectedPriorRecoverySequence`。可信恢复使用三个耐久事务：T1 在任何副作用前以 anchor revision CAS 接受严格更高 sequence/hash、写 `TrustedRecoveryAttempt(prepared)` 并把 floor 置 `acceptedPending`；T2 在首次恢复副作用前刷盘 `callArmed`，启动后写 `inFlight`，二者崩溃均不得重调；T3 仅在最终状态可证明时，同一事务写 `RecoveryResolutionRecord`、最终 security-state digest、attempt `resolved`、floor `resolved` 和对应 source 终态。对 product uninstall，只有精确 verified absence 才可写 `uninstalledPreserved`。同序号同 payload 在 pending 时返回原 attempt、resolved 时返回原 resolution，均零新副作用；低序号或同序号异 payload 返回 `UPDATE_RECOVERY_REPLAY_BLOCKED`。卸载、重装和后续恢复不得降低 floor；旧 journal、sticky 撤销和高水位不删除、不降低。规则信任损坏仅可由代码签名应用携带的失败关闭 recovery baseline 修复，普通远程响应不得重置。
- `AppRecoveryPackagePayload(schemaVersion=2)`、`UpdateRecoveryAssessment` 和 `RecoveryResolutionRecord` 都按 `source.kind` 生成 closed `oneOf`。`appUpdate/fullInstaller` 分支只允许 `installRecoveryTarget` 与 `recoveredToTarget/recoveredToLkg`；`productUninstall` 分支只允许 `completeProductUninstall`，绑定预期产品身份、固定 removal properties、absence evidence policy、`uninstalledPreserved` 和 anchor lifecycle revision，禁止携带 target package/version 字段。T3 对 app update 追加 `AppUpdateJournal.recoveredExternally`；对 full installer 将 `MachineFullInstallerJournal` 转为 `resolved(completionKind=trustedRecovery)` 但保留原 attempt `recoveryRequired`；对 product uninstall 以 `VerifiedProductAbsenceEvidence` 转 `uninstalledPreserved(completionKind=trustedRecovery)`。三者都不覆盖旧证据。

启动恢复按 journal 主状态执行，先查询 MSI 产品、目标二进制、boot ID 和 Windows Installer 状态，不凭 journal 单独推测结果：

| 启动时状态 | 规范对账 |
|---|---|
| `downloaded` | 候选缺失或传输证据不完整则安全转 `cancelled`；存在时只重新做传输级校验，不自动推进 verified |
| `verified` | 重新验证 package hash、签名和 manifest 绑定；缺失/撤销则转 `cancelled`，一致则等待显式 stage |
| `staged` | 重验受保护 MSI、machine policy、未过期控制面、通道/全局撤销和实际 base；无精确可用 LKG 时保持 staged 并返回 `UPDATE_LKG_UNAVAILABLE`，不得创建 admission |
| `installPrepared` | 要求 journal admission、`activeAdmission=admitted`、实际 floor 等于 `installerAdmissionFloorBeforeCall`、package/base identity 和 `callAttemptCount=0` 精确一致；可继续到 callArmed/installing 或按合法取消协议写 `resolvedWithoutCall`，任一不一致进入 `recoveryRequired` |
| `installing` | 安装调用已经 armed，绝不重跑；实际 MSI return 未耐久时不得合成 `0/3010`。只有独立 Windows Installer transaction、精确目标 binary provenance 和 reboot requirement 均可证明，才写 `reconciledExactTarget` 并进入对应 `rebootPending/trialPending`；任一证据不明、仍是旧版或状态冲突都进入 `recoveryRequired`，不得伪造 return code 或 `rolledBack` |
| `rebootPending` | boot ID 未变化时保持并提示重启；重启后确认精确目标版才进入 `trialPending`，否则进入 `recoveryRequired` |
| `trialPending` | 只创建一个绑定 update/nonce/child 的 trial session；若已有精确 session 则采用，不能并发创建第二个 |
| `trialLaunchArmed` | session/nonce/Job intent 已耐久但 child 尚未被证明已安全恢复；启动对账不得再创建 child，直接进入 `rollbackPending` |
| `trialRunning` | 只接受仍存活的精确 child/管道收据，不重启第二个 child；崩溃、超时、收据不完整/重放或身份变化进入 `rollbackPending` |
| `rollbackPending` | 尚未调用回滚；重验精确 LKG、期限和通道/全局 sticky 撤销后最多一次转 `rollingBack`，不满足则 `recoveryRequired` |
| `rollingBack` | 回滚调用已经 armed，绝不重跑；精确 LKG 存在才转 `rolledBack`，精确目标仍在、结果未知或一次失败均转 `recoveryRequired` |
| `committed` | 磁盘必须精确等于目标版且 health receipt/当前制品 provenance 一致，否则进入 `recoveryRequired` |
| `rolledBack` | 磁盘必须精确等于 journal LKG 且 rollback attempt/evidence 一致，否则进入 `recoveryRequired` |
| `recoveryRequired` | 保持只读；只允许无副作用对账或第 14.1 节阈值签名且代码签名的恢复包。成功追加 resolution 后转 `recoveredExternally`，不能覆盖旧证据 |
| `recoveredExternally` | 终态；重验磁盘状态与 resolution 一致，不一致创建新的 recovery 事件而不重写历史 |
| `cancelled` | 必须无在途 installer/rollback，磁盘仍为 expected base；从 installPrepared 取消的 admission 必须为 `resolvedWithoutCall`，且 floor 精确保持 admission 前值并允许同一 package 重新 admission。任一不符进入 `recoveryRequired` |

另外两类 machine journal 的恢复矩阵固定如下；状态集合不得由实现自行扩展或把 embedded attempt 的状态误当成 journal 状态：

| 恢复来源 | 允许的 journal 状态 | 可信恢复终态 |
|---|---|---|
| `MachineFullInstallerJournal` | 恰好 `admitted`、`callArmed`、`inFlight`、`resolvedWithoutCall`、`resolved`、`recoveryRequired` 六个 state discriminator；`resolved` 再以 `completionKind` 区分安装器调用与可信恢复 | 新 journal resolution 为 `resolved(completionKind=trustedRecovery)`，绑定 `recoveryResolutionRecordId`；embedded 原 attempt 保持 `recoveryRequired`，不得改写为安装器成功 |
| `MachineProductUninstallJournal` | `uninstallPrepared`、`uninstalling`、`rebootPending`、`uninstalledPreserved`、`recoveryRequired`；只有产品、服务和目标二进制缺失证据闭合才可成功 | `uninstalledPreserved(completionKind=trustedRecovery)`，绑定 `VerifiedProductAbsenceEvidence`、anchor lifecycle revision 和 resolution；embedded 原 attempt 保持 `recoveryRequired` |

完整安装、应用内更新和产品卸载的反降级矩阵至少覆盖：当前较新版本上运行旧 MSI/NSIS、同版本不同包、旧 staged 更新、另一管理员先完成升级、ProductCode 置换、同产品族不同历史 UpgradeCode、x64 包尝试在 ARM64 建第二锚、合法/伪造/重放 UpgradeCode 迁移、epoch/manifest 高水位更高、A 通道撤销后切换 B、ARP/裸 `msiexec /x`、以及卸载后保留 installer 与迁移 floor。除 journal 精确 LKG 的受控回滚及经阈值签名对同一锚执行的规范 UpgradeCode 单调迁移外，全部必须在任何产品写入前拒绝；`AppUpdateJournal` v4、`MachineFullInstallerJournal` 和 `MachineProductUninstallJournal` 的每个状态都必须在上述恢复矩阵中恰好出现一次。

必须故障注入下载截断、控制面签名/撤销、staged 后撤销、安装前离线/过期控制面、双管理员竞争、pending reboot、MSI `0/3010/1641`、安装调用中断、换用户重启、trial 管道伪造/重放/child 丢失、120 秒超时、用户 A 更新后用户 B 首次迁移失败、LKG 后续撤销、回滚期限/GC、恢复包错误状态及 key-set 同公钥多 ID。无法证明目标或 LKG 状态时进入 `recoveryRequired`，不宣称自动恢复成功。

<a id="qpn-sec-14-2"></a>
### 14.2 规则发布

1. 在 internal 通道通过规则单元、安全语料和目标应用版本测试。
2. 发布 beta 通道供明确选择的测试用户使用。
3. 完成结果复核后签名发布 stable。
4. 发现问题时由恢复密钥阈值签名撤销清单，停止受影响规则，并以更高序号重新发布已知良好内容；不得激活低于高水位的旧包。

规则发布不依赖上传扫描结果。效果评估使用维护者构建的本地固定数据集。V1 不提供诊断材料上传端点；用户主动保存的脱敏诊断包若要通过产品外渠道交给支持人员，必须由独立支持隐私告知说明接收字段、接收方、保留期和删除方式，不能复用第 5.7 节任何联网端点。

<a id="qpn-sec-14-3"></a>
### 14.3 诊断与事件响应

- 诊断包由用户主动生成、预览并保存到本地，不自动上传。
- 路径默认替换为规则 ID、扩展名和不可逆本地标识；不得包含文件内容。
- 数据损失或危险规则事件立即停止对应规则、发布撤销清单、保留证据并通知受影响版本用户。
- 常规规则发布密钥疑似泄露时停止发布并使用独立恢复密钥撤销；恢复根疑似泄露时不能自签替换，必须发布新的代码签名应用版本来轮换恢复根。

---

