# 清盘设计文档：需求、规则与支持矩阵

> 文档集：[Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)
> 规范章节：第 9-12 章
> 状态与版本继承自主索引；本文件不单独构成批准对象。

[上一篇：运行时 API](03-runtime-api.md#qpn-sec-8) · [主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>) · [下一篇：测试与发布](05-test-release.md#qpn-sec-13)

---

<a id="qpn-sec-9"></a>
## 9. 模块需求与验收

以下是 V1 规范性需求。验收摘要均按 Given/When/Then 表达。标记“P0（条件）”的需求不属于 M1 默认交付；只有发布构建启用该能力时才视为该构建的 P0，并必须满足 GATE-003/GATE-008。能力关闭时入口、命令和发布声明必须同时不存在。

| 需求 ID | 优先级 | 风险 | 需求 | 验收摘要 |
|---|---|---|---|---|
| SAFE-001 | P0 | 全部 | 执行前重新验证路径、父目录链和全部可观察身份字段 | Given 任一已记录身份字段在扫描后变化，When 执行计划，Then 跳过并返回对应稳定码 |
| SAFE-002 | P0 | 全部 | 仅绝对打开已验证卷 GUID 根；配置根的全部中间组件在扫描、执行和重启重建时均以父句柄相对打开且不得跟随重解析点 | Given Junction/挂载点在扫描前已存在、扫描后被替换或持久根在重启后重建，When 逐级打开，Then 在该组件停止，不进入目标且不影响外部文件 |
| SAFE-003 | P0 | R1-R4 | 复检和写动作使用同一已验证句柄 | Given 路径在打开后被并发替换，When 执行动作，Then 只作用于已复检句柄或失败关闭 |
| SAFE-004 | P0 | R1-R4 | 根/父链句柄固定到提交，目标使用后端句柄相对定位 | Given 父目录或仓库根被并发重命名/替换，When 删除、移动或恢复，Then 动作不越过已固定根 |
| SAFE-005 | P0 | R1/R2/R4 | R1 文件删除、R2 已验证容器后的源删除和 R4 容器删除均在调用前耐久记录 `callPrepared`，可能调用后绝不重调且仅验证目录项消失才成功 | Given 在任一删除 WAL 边界崩溃、祖先目录并发移动或双执行器竞争，When 执行或启动对账，Then 删除 API 最多调用一次；只有同一固定父链下的 `removedVerified` 可报告永久移除 |
| RULE-001 | P0 | R1-R4 | 规则包必须验签、校验 schema、通道和单调序列 | Given 被篡改、跨通道或低序号包，When 激活，Then 拒绝并保持当前有效包 |
| RULE-002 | P0 | R1-R4 | 有效签名规则仍不得越过应用内能力包络 | Given 签名有效但扩大根/动作或弱化保护，When 能力校验，Then 返回 `RULE_CAPABILITY_VIOLATION` |
| RULE-003 | P0 | R1-R4 | 规则业务回滚使用更高序号重发已知良好内容 | Given 当前包需撤回，When 发布修复，Then 高水位不下降且旧包不能被重放 |
| RULE-004 | P0 | R1-R4 | 签名 index 必须绑定恢复签名的 key authorization、revocation 和规则包哈希/大小/序号 | Given 控制面任一文档被替换或同序号异内容，When 请求更新，Then 在下载或激活前拒绝并保留原信任状态 |
| RULE-005 | P0 | R1-R4 | 恢复签名撤销先于候选激活且单调 sticky，命中 active 时关闭规则清理 | Given 已接受撤销命中 active key/包，When 扫描或重启，Then 不生成新规则候选且普通文档不能解除撤销 |
| RULE-006 | P0 | R1-R4 | 规则 matcher 使用封闭判别联合、确定深度/大小写/组合语义及绝对文档/数组/字符串上限 | Given 空数组、路径组件转义、大小写目录、超限文档或越深匹配，When 解析规则，Then 整包拒绝且 active 包不变 |
| SCAN-001 | P0 | R0-R2 | 扫描受资源上限和取消控制 | Given 超过文件上限或收到取消，When 扫描，Then 返回准确的 `limitReached/cancelled` |
| SCAN-002 | P1 | R0-R2 | 普通权限任务协调器同时最多接受 3 个只读扫描，task ID 后端唯一且超限请求不排队、不创建耐久 task | Given 已有 3 个运行中的只读扫描，When 并发提交第 4 个扫描及重复 request，Then 只有前三个唯一 task 存在，第 4 个无副作用返回 `LIMIT_EXCEEDED`，取消或终态释放 slot 后才可接受新 task |
| EXCL-001 | P0 | R0-R2 | 排除 entry/policy 只能由原生选择器和后端摘要创建，使用前复核卷/File ID | Given 排除根被替换、撤销或 UI 伪造 ID，When 扫描或保存策略，Then 返回稳定策略错误且不得扫描原排除位置以外的新对象 |
| CLEAN-001 | P0 | R1 | 默认清理只包含允许列表中的可重建缓存 | Given 未知目录或登录数据，When 默认扫描，Then 不产生可选候选 |
| CLEAN-002 | P0 | R1-R4 | 预览显示依据、影响、风险、动作和恢复能力 | Given 任一候选，When 打开详情，Then 五类信息均可见且与计划一致 |
| CLEAN-003 | P0 | R1-R4 | 计划为单次、30 分钟有效且不可变 | Given 过期或已执行计划，When 再次调用，Then 返回 `STALE_PLAN` |
| CLEAN-004 | P0 | R2 | 通用 Temp、日志和崩溃材料不冒充可重建缓存 | Given 仅能证明位于通用 Temp/日志目录，When 分类，Then 至少为 R2 且不得自动化 |
| REC-001 | P0 | R2 | 隔离只使用 QPC1 `prepared→sourceDigestPrepared→containerPrepared→copying→copied→containerVerified→containerCommitted→sourceDeletePrepared→sourceRemovedVerified→committed`；源文件不 rename、不改 owner/DACL | Given 在任一 copy/tag/flush/容器发布/quota/源删除边界崩溃或 content guard break，When 执行或重启对账，Then 容器完整验证和提交前零源删除；提交后删除不明转 `sourceRetained/recoveryRequired`，至少一份可验证内容保留且不伪报成功 |
| REC-002 | P0 | R2 | V1 用户可见动作固定为“导出隔离副本”：只复制到后端在用户授权父目录下新建的本地目录，不还原或覆盖原位置，成功后仍保留隔离源 | Given 用户查看、确认或完成导出，或目标已存在、为重解析路径、哈希不符，When UI 渲染或后端执行导出，Then 按钮、确认和结果均使用“导出副本”并明确“不会还原原位置、不会自动释放隔离占用”；后端不覆盖任何内容且始终保留隔离源 |
| REC-003 | P0 | R2/R4 | 到期只标记，清除使用独立 R4 计划 | Given 记录到期或配额已满，When 后台运行，Then 不自动清除；只有确认后的 R4 计划可清除 |
| REC-004 | P0 | R2 | 恢复和异常救援使用操作级授权快照与分阶段 copy WAL | Given 在 prepared、temp create/copy/verify/publish 任一刷盘点崩溃或临时对象被替换，When 新实例对账，Then 只续作身份完全一致的 copy；否则保留源并进入受限状态 |
| REC-005 | P0 | R2/R4 | 批量恢复/救援/清除在首个文件副作用前整批 CAS | Given 1~500 项中任一 state、journal sequence、身份或确认大小变化，When 认领批次，Then 零个对象被复制/删除且不得部分消费其他记录 |
| REC-006 | P0 | R2/R4 | 每卷 quota reservation、charge、record 和仓库对象在同一账本协议转换，未知核算失败关闭新隔离 | Given 任一 reserve/convert/release 刷盘点崩溃、孤立对象或账本不一致，When 对账，Then 不超卖或按超时释放；列表、诊断和救援仍可用 |
| REC-007 | P0 | R2 | QPC1 v1 固定 AES-256-GCM、DPAPI CurrentUser DEK 包装、唯一 nonce、逐字节格式和独立全量解密验证 | Given golden/negative vector、篡改 header/manifest/chunk/tag、nonce/index 重复、密钥丢失、长度溢出/尾随或复制期间源变化，When writer/reader/恢复工具验证，Then 只有同一规范字节与全部 AEAD/hash/identity 证据通过才允许提交容器；任一失败保留源且零源删除 |
| PLAN-001 | P0 | R1-R4 | 计划 source/items/confirmation/lifecycle/category summary 使用严格联合且全部摘要从同一 item 集派生 | Given 来源混合、非法生命周期/确认或分类缺项，When 建立、读取或认领计划，Then 返回 `OPERATION_STATE_INVALID` 且零 item 开始 |
| RESULT-001 | P0 | R1-R4 | item ref/action/adapter、code、outcome、phase、disposition、进程证据、counts、空间核算和 operation 终态使用封闭联合并只从耐久 item rows 与 terminal cause 派生 | Given 跨领域错误码、非 Win32 退出码、当前 item unknown 与后续未开始混合、取消竞争或篡改 counts/status，When schema 校验或终态提交，Then 非法组合拒绝；当前 unknown 与后续 unprocessed 使用不同稳定码且聚合顺序唯一 |
| R4-001 | P0 | R4 | 每种不可逆动作使用独立发布开关、摘要、确认和验收，不复用授权 | Given 仅隔离清除通过门禁，When 尝试永久删除原文件，Then 原文件删除入口仍不可用 |
| R4-DELETE-001 | P0（条件） | R4 | 启用原文件永久删除的构建必须提供独立高级入口、仅接受新的大文件/用户选择快照、快照复检、二次确认和发布证据 | Given capability 未启用、source 是 cleanupRules/远程规则或未通过专项门禁，When 请求 `permanentDeleteOriginal`，Then schema/后端拒绝且 UI 无入口 |
| STORAGE-001 | P0 | R0 | 空间分析不写入扫描树 | Given 任意本地支持卷，When 分析，Then 目录内容、时间和属性不被修改 |
| LARGE-001 | P0 | R0/R2 | 大文件扫描为 R0，M1 写处置只提供用户逐项选择后的 R2 隔离 | Given 用户选择身份稳定的大文件，When 建 M1 计划，Then 默认动作是隔离且不会生成原文件永久删除 item |
| DUP-001 | P0 | R0 | “已确认重复”必须经过完整 SHA-256、稳定句柄逐字节比较并排除硬链接别名 | Given 同内容、哈希碰撞测试替身和同一文件硬链接，When 扫描，Then 只把逐字节相同的独立文件标为已确认重复 |
| DUP-002 | P0 | R0 | V1 重复文件功能不产生写计划 | Given 任一重复组，When 查看结果，Then 无整理、隔离或删除命令可调用 |
| APP-001 | P1 | R0/R3 | 应用枚举为 R0，启动卸载统一按 R3 确认 | Given 用户请求卸载，When 启动受支持接口，Then 使用快照 ID 且显示外部变更风险 |
| APP-002 | P0 | R3 | M1 卸载目标只允许最终执行 token 用受支持 API 复检后的 singleton-context MSI identity 或当前用户 AppX identity；Win32 EXE 仅归 APP-004 条件能力 | Given 同 ProductCode 多 context、枚举权限不足/集合变化、伪造 identity/路径/协议/解释器/EXE，或 capability 关闭时提交 Win32 目标，When 请求卸载，Then 多 context 返回 `UNINSTALL_TARGET_AMBIGUOUS`、其他非法目标返回 `UNINSTALL_TARGET_INVALID`，且零外部调用 |
| APP-003 | P0 | R3 | per-user 目标使用用户 journal，machine MSI/HKLM 目标使用受保护 machine journal、全局 mutex 和唯一资源索引；调用结果、待重启和每个附着 operation 只从同一 resolved machine attempt 投影 | Given 两个管理员 SID、别名目标、MSI `0/3010/1641/失败`、AppX HRESULT/restart、启动/完成/观测/attachment 任一耐久边界崩溃或证据错配，When 执行、附着、生成结果或重启，Then 同一 machine 资源最多调用一次；3010/未知保持锁且不重调，每个 owner 只见其本地 operation 投影，无完整证据时返回 `UNINSTALL_REBOOT_REQUIRED/UNINSTALL_RECOVERY_REQUIRED` |
| APP-004 | P0（条件） | R3 | 所有通用 Win32 EXE 卸载均按构建关闭或开放；普通权限分支只接受固定 `currentUserOnly` 快照和 sealed invocation，提权分支另须受保护 HKLM、受信任固定 EXE、编译参数模板及固定安全 CWD/环境/DLL/依赖上下文 | Given capability 关闭、目标不满足普通权限约束，或提权目标命中 HKCU/可写 HKLM、任意签名、未知模板、参数插槽、用户可写 CWD/PATH/DLL 或依赖变化，When 请求 Win32 卸载，Then 零进程启动并返回 `UNINSTALL_TARGET_INVALID` |
| STARTUP-001 | P1 | R0 | V1 启动项仅分页枚举和展示来源，不提供启用、禁用或删除命令 | Given 任一启动项，When 查看或尝试写入，Then 可读取脱敏信息且目标 API 不存在写入口 |
| PART-001 | P1 | R0 | 分区页面保持只读 | Given 用户需要写操作，When 点击管理，Then 只打开 Windows 磁盘管理 |
| AUDIT-001 | P0 | 全部 | 审计最小化、可追加并容忍坏行；诊断导出只消费原生 `CREATE_NEW` 目标 grant | Given 历史中有截断行或 WebView 伪造/重放/跨会话保存 grant，When 读取或导出，Then 跳过坏行并继续显示有效记录；无合法 grant 时零文件创建且绝不自动上传 |
| NET-001 | P0 | 全部 | 生产 WebView 禁止联网，Rust 客户端执行端点/字段白名单 | Given 请求含路径、未知字段或未复检重定向，When 发送前校验，Then 阻止请求 |
| IPC-001 | P0 | R3 | 提权 IPC 校验同 SID、会话、进程、签名、序号和 nonce | Given 跨账户 UAC、伪造对端或重放消息，When 握手，Then 终止且不读取/执行计划 |
| IPC-002 | P0 | R3 | 高完整性摘要、确认和系统调用使用同一不可变 execution bundle | Given helper 验证后篡改 plan、AppSnapshot、sealed invocation 或用户仓库页，When 确认并执行，Then 展示 A 绝不执行 B；bundle 丢失时消费计划且无系统调用 |
| UPDATE-001 | P0 | 全部 | 应用更新、完整安装和本产品卸载必须验签/验票并分别写 `AppUpdateJournal`、`MachineFullInstallerJournal`、`MachineProductUninstallJournal`；完整安装的 attempt ID 与 journal sequence 不可变，更新保留精确上一 LKG，三类来源都可进入统一可信恢复 | Given 任一来源在 admission、安装/卸载、迁移或试运行中断，When 启动对账，Then `InstallRecoverySource` 必须以精确 source ID + admission ID + journal sequence（完整安装再加 installer attempt ID）唯一定位历史事实，证明目标/LKG/已卸载状态或进入 `recoveryRequired`，不得因缺少 AppUpdateJournal 而丢失恢复路径 |
| UPDATE-002 | P0 | 全部 | 更新 manifest 使用单调序列、恢复密钥撤销和编译期固定下载端点 | Given 旧 manifest、同序号异内容或签名新 origin，When 检查更新，Then 拒绝且不下载/安装 |
| UPDATE-003 | P0 | 全部 | 应用 release key 授权、撤销和 epoch 迁移只能由恢复阈值签名并事务持久化 | Given 签名不足、迁移链断裂或 sticky 撤销缩小，When 检查更新，Then 拒绝且高水位不回退 |
| UPDATE-004 | P0 | 全部 | MSI `0/3010/1641`、pending reboot、nonce 试运行和 120 秒超时使用第 14.1 节确定状态 | Given 每种返回码或试运行故障，When 安装/启动对账，Then 状态和用户文案与矩阵一致且不提前 committed |
| UPDATE-005 | P0 | 全部 | 安装、产品卸载或回滚结果未知时不得重复 MSI/NSIS 调用或伪造返回码；`MachineFullInstallerJournal` 六态与 attempt/anchor/ticket 同事务 | Given 任一 full journal/attempt 刷盘边界崩溃，或调用已 armed 但 `0/3010/1641/失败` 证据未耐久，When 重启，Then 只续作同一 ID/sequence 的对账；仅凭独立 transaction/target/reboot 证据生成 reconciled evidence，证据不足进入 `recoveryRequired`，任何来源均不重调 |
| UPDATE-006 | P0 | 全部 | 应用通道/信任/journal 为 machine scope，安装使用全局 mutex、base CAS 和反降级 floor | Given 双管理员竞争、旧 staged/完整 MSI/NSIS 或另一会话先升级，When 安装，Then 最多一个提交且低/同 build 在写入前拒绝 |
| UPDATE-007 | P0 | 全部 | normal install 与 LKG 在调用前重验当前 sticky 撤销 | Given staged 或 LKG 后来命中 key/manifest/package/version 撤销，When apply/rollback，Then 不调用 MSI；normal 离线且控制面过期同样停止 |
| UPDATE-008 | P0 | 全部 | trial IPC 绑定精确 medium linked-token child/nonce/序号，用户 schema 按 SID 延迟迁移 | Given child 继承高完整性/换账户 token、被伪造/重放/丢失或用户 B 迁移失败，When 对账，Then child 不启动或 binary 不提前 committed；单 profile 只读恢复且不回滚全机已验证 binary |
| UPDATE-009 | P0 | 全部 | 任一 `InstallRecoverySource` 的 `recoveryRequired` 只能由精确对账或原生文件 grant 选取、阈值签名、代码签名且 sequence 单调的恢复包退出；卸载恢复必须是 `completeProductUninstall` 并提交 absence evidence、`uninstalledPreserved` 和 anchor revision | Given app update/full installer/product uninstall 的 journal/磁盘不一致、伪造/过期/跨会话文件 grant、旧/同序号异 payload，或安装分支携带 absence 字段/卸载分支携带 target package 字段，When 恢复，Then closed oneOf 在副作用前拒绝错配；只有绑定同一 source/admission/观测摘要的可信包可追加 resolution，卸载成功仍保留原 recoveryRequired attempt 并原子推进 anchor/journal/floor |
| UPDATE-010 | P0 | 全部 | 所有 MSI 只可经 ARP 指向的签名 bootstrapper和用途绑定 admission ticket；admission 不推进 floor，只有 `callArmed` 与唯一外部调用意图在同一事务推进单调 floor | Given 裸 msiexec、缺失/过期/错用途 ticket、零调用取消/过期、任一刷盘点崩溃、同/低 build 或并发安装/卸载，When 执行，Then 零调用 resolution 保持 admission 前 floor 并允许同包重新准入；armed 后最多一个精确调用且 floor 永不回退 |
| UPDATE-011 | P0 | 全部 | package/version 撤销跨通道且绑定不可重置的 machine anchor，journal v4 每个状态与 trust/磁盘确定性对账 | Given A 通道接受撤销后切换 B、卸载重装，或任一 journal 状态与磁盘不一致，When apply/rollback/启动，Then 被撤销制品零调用，未知状态只读且新 instance ID 不能绕过历史 |
| UPDATE-012 | P0 | 全部 | `MachineInstallAnchor` 仅按产品族全机唯一并跨卸载重装保留，原生机器架构是不可变属性；任何 anchor 读写前用 `IsWow64Process2`/MSI metadata 拒绝非原生包；UpgradeCode 只经同锚阈值签名单调迁移 | Given x64 包运行于 ARM64、历史包携带不同 UpgradeCode、anchor 缺失/重复/ACL owner 错误、迁移签名/序号/CAS 无效或产品状态冲突，When 首装、重装、迁移、更新或卸载，Then 非原生包返回 `UPDATE_PLATFORM_MISMATCH`，其他锚错误返回 `UPDATE_MACHINE_ANCHOR_INVALID`，且不创建第二锚、不调用 MSI |
| UPDATE-013 | P0 | 全部 | manifest/admission 在安装前绑定目标 binary hash/path/FileVersion/signer SPKI，所有高完整性制品执行固定 Authenticode policy | Given 同名证书、未知撤销、package 内 binary 变化、安装后自绑定或 signer 轮换未授权，When stage/apply，Then 返回 `UPDATE_SIGNER_POLICY_INVALID/UPDATE_PACKAGE_INVALID` 且零安装调用 |
| AUTO-001 | P0 | R0/R1 | 任务计划只包含固定 R0 策略或绑定完整摘要的预批准 R1 | Given 策略实质变化或作业含 R2-R4，When 保存或运行，Then 批准失效或拒绝作业 |
| AUTO-002 | P0 | R0/R1 | 定时根和 R1 批准只能由后端经原生确认签发不透明 ID | Given WebView 伪造路径、策略哈希或绑定字段，When 创建作业，Then 拒绝且不产生 Windows 任务 |
| AUTO-003 | P0 | R1 | grant/policy/job 撤销与下一 item 的 prepared CAS 线性化 | Given 撤销发生在触发后、claim 前后或 item 间，When 执行，Then 只有先提交 prepared 的当前 item 可完成，撤销后没有新 item 开始 |
| AUTO-004 | P0 | R1 | grant 永久一对一绑定 job、完整 schedule/principal/run limit；runner 证明批准的 Task Scheduler trigger instance 后才原子认领唯一 ordinal | Given 手动启动 runner、on-demand Run、触发/主体/binding 变化或两个 runner 竞争最后一次，When 触发，Then 最多一个经证明的 scheduled claim 成功，其他零扫描和零 item |
| AUTO-005 | P0 | R0/R1 | Task Scheduler upsert/delete 使用耐久 WAL、精确幂等重放和启动命名空间对账 | Given 每个 journal 刷盘点崩溃、孤立/篡改 Task 或同/异 payload 重放，When 启动，Then 未提交定义先禁用且不执行，精确请求只产生一个 mutation result |
| AUTO-006 | P0 | R0/R1 | 批准创建可耐久幂等找回；create/update CAS 严格分支；DST/时钟回拨按 occurrence key 去重 | Given 批准响应丢失、jobId/revision 只给一个、跳时/重复小时或系统时钟回拨，When 创建/触发，Then 只得到原 grant/job 且每个日历 occurrence 最多一个 run claim |
| LICENSE-001 | P0 | 全部 | 生产构建不得包含固定离线口令 | Given 发布候选包，When 静态扫描和激活测试，Then 找不到旁路且令牌不在 Web Storage |
| LICENSE-002 | P0 | 全部 | 激活/刷新使用原生输入、发送前 WAL、固定字段、CNG PoP、系统凭据 slot/CAS、双截止时间、长期精确结果/effect fence 和只读对账；activation v3 WAL 耐久绑定普通 PoP/预注册强保护双密钥、完整规范请求 slot、activation proof profile 与服务端 key ID，refresh fence 终态持久化旧凭据 disposition/expiry | Given activation proof 缺失、错 key/alg/typ/domain/purpose/method/path/body/ID/SPKI、`iat` 越窗、`jti` 重放、强保护 key 代签、WebView 字段渗漏、跨 state 非 fresh 字段、双密钥或请求材料错配、v2 非终态迁移及任一 WAL/resolution 边界崩溃，When 激活/校验/刷新/启动对账，Then deadline 前只可从同一 slot 续作逐字节原 body 且每次使用新的有效 proof；任一绑定不符、closed schema 失败或 v2 证据不足均零席位/凭据副作用且不返回旧 token；terminal WAL 清理后仍能唯一判定凭据处置，不重复轮换/占席且未安全存储前不报告成功 |
| LICENSE-003 | P0 | 全部 | 停用先经原生一次性 grant、独立强保护停用授权 key、一次性服务端 challenge 和 Windows Hello/高保护 CNG 用户验证；普通设备 key 只做 activation proof、validate、refresh、reconcile，永不签停用声明 | Given 伪造/过期/跨会话 grant、同 SID 进程直接普通 key 签名/HTTP、challenge 重放/过期/错 key、用户验证取消、在线/离线响应丢失或任一 WAL/resolution/key 边界崩溃，When 停用、对账或终态重建，Then 非法授权零 `/deactivate`；服务端最多撤销一次，retained 凭据与日常 PoP key 原子恢复并销毁未使用停用 key，destroyed 分支准确显示席位状态且不猜测 |
| SVC-001 | P0 | 全部 | 授权服务 `QPN-SVC-LIC-001` 必须有 staging/production `ServiceDeliveryRecord` 和服务端 contract/幂等/日志删除/E2E 证据 | Given owner、设计、制品、origin、迁移、key ceremony、保留删除或任一运行证据缺失/失败，或 `designDoc.id/serviceDesignId/serviceMilestoneId/domain` 不是同一授权分支，When 评估 M1-08/M1-SVC-01 或发布，Then schema 拒绝错配记录或 readiness 为 blocked/failed，且 M1 不得退出 |
| SVC-002 | P0 | R1-R4 | 规则控制面 `QPN-SVC-RULE-001` 必须有 staging/production `ServiceDeliveryRecord` 和服务端签名/撤销/回滚/隐私证据 | Given origin 未固定、服务端 schema 可接受未知字段、撤销/幂等/回滚/日志删除证据缺失，或四个服务判别字段不是同一规则分支，When 评估 M1-02/M1-SVC-02 或发布，Then schema/readiness 不得通过，规则远程更新保持关闭且 M1 不得退出 |
| SVC-003 | P0 | 全部 | 应用更新控制面 `QPN-SVC-UPDATE-001` 必须有 staging/production `ServiceDeliveryRecord` 和服务端 manifest/撤销/包分发/隐私 E2E 证据 | Given artifact/provenance、固定 origin、撤销/epoch migration、包哈希分发、回滚/日志删除证据缺失，或四个服务判别字段不是同一更新分支，When 评估 M1-10/M1-SVC-03 或发布，Then schema/readiness 不得通过，应用更新保持关闭且 M1 不得退出 |
| API-001 | P0 | 全部 | 目标命令使用版本化 envelope、后端签发且用途绑定的瞬时/耐久 ID、成对 request-response schema 和第 8.2 节硬上限 | Given 路径/URL、跨会话/错用途/已消费 grant ID、与请求 discriminator 不匹配的成功响应或超限请求，When 调用命令或校验响应，Then 请求无副作用地被拒绝或响应 schema 失败；不得接受跨用途 grant |
| API-002 | P0 | R2/R4 | 副作用命令使用 command-specific 耐久幂等联合、唯一 ID 数组与整批事务 | Given 五种 command 的同 key 同/异 payload、pending/响应前崩溃或并发重试，When 重放，Then 同 payload 只附着或返回该 command 的原类型结果，异 payload 拒绝且不产生第二个 mutation |
| API-003 | P0 | 全部 | closed schema、规范标量和领域 canonicalization 在 RFC 8785 前执行，集合拒绝重复后排序 | Given 非规范 UUID/整数/时间、重复 record/rule ID 或顺序数组被重排，When 解析或计算摘要，Then 非规范输入拒绝且所有合法消费者产生相同摘要 |
| API-004 | P0 | 全部 | `FormalContractRegistry` 必须恰好登记 `CONTRACT-001~008` 的权威根、真实版本字段/值、三个具名制品摘要及排除自摘要字段的 v2 canonical registry digest | Given Contract ID 缺失/重复/多出、根/版本漂移、任一 Rust/TypeScript/JSON Schema 制品摘要被独立篡改或交换、tuple 顺序/profile/registry digest 变化，When 复算 pinned bytes 与 RFC 8785 canonical payload，Then 任一不等即构建失败且 M1-11 不得通过 |
| PAGE-001 | P1 | 全部 | snapshot 与 append 分页使用固定 fence、排序、TTL 和明确完成语义 | Given 读取期间追加/删除对象或 cursor 过期，When 翻页，Then 有效链无重复遗漏；过期返回稳定码并从新 snapshot 重读 |
| PAGE-002 | P0 | 全部 | cursor 绑定 API/对象/owner/session/filter/page size，并执行版本 pinning、链/行/字节和页大小上限 | Given 跨域 cursor、cursor flood、四种 append 完成组合或超过 pin 配额，When 翻页，Then 不越权/漂移/驱逐有效链，超限失败关闭 |
| PLAT-001 | P0 | 全部 | V1 仅在第 12.1 节声明的平台和文件系统上开放对应能力 | Given ReFS、网络盘或不支持架构，When 请求写操作/安装，Then 保持只读或明确拒绝 |
| PLAT-002 | P0 | 全部 | `PlatformTupleRegistry v3` 必须由 3 个冻结 Windows build、2 个原生架构、3 个数据卷 profile 和 2 个发起权限 profile 生成精确 36 项笛卡尔积，并在 canonical payload 中以 `RELEASE_TEST_OBLIGATIONS` 闭合登记当前 252 个发布测试 ID、唯一 capability 与唯一 `hostIndependent` 或 registry profile | Given tuple/profile/test obligation 缺失、重复、多出、字段与 key 分解不符、已知 test ID 的 capability/profile 被改变、义务/registry digest 变化或任一 required tuple 无 passed run，When 校验发布候选，Then GATE-003 和 M1-11 失败且不得由 definition 自报值、抽样、pairwise 或自由字符串缩小范围 |
| PERF-001 | P1 | R0 | 性能按固定机器、数据集和轮次报告 P50/P95 与资源 | Given 批准基准环境，When 运行第 12.2 节轮次，Then 原始数据可复现且未批准回归阻止发布 |
| REL-001 | P0 | R1-R4 | 状态仓库、写操作和更新在崩溃后先对账再开放新动作 | Given 任一耐久边界崩溃，When 重启，Then 不重复执行且未知结果进入 `recoveryRequired` |
| RELEASE-001 | P0 | 全部 | `ReleaseGateManifest v4` 是 M1-11 唯一机器发布证据根，必须绑定已通过的 M0 三文件、受外部组织信任根认证的治理策略、M0 到最终 build input 的源码派生、实际 `ReleaseCapabilityManifest`、SBOM/`DependencySecurityReport v2`，以及同一 release/artifact/source/build/CI 的 test/trace、平台登记、正式契约、文档批准和六份服务记录；gate 外 `SignedReleaseAttestation v1` 再签署 gate 与 artifact | Given M0 verification/派生链缺失或非 passed、审批不可认证或同一自然人重复角色、测试义务或能力摘要与实际发布配置不同、SBOM/findings/dispositions 缺失或未处置高危依赖、required tuple 非 passed、文档非 `Approved`，或跨构建交换任一工件，When 从 bundle 根解析并复算 19 个固定顶层文件、六服务记录、全部传递证据、canonical digest、可信签名与外层 provenance，Then 缺失、额外、错路径/摘要/build、M1 capability 降级或 P1 waived 均阻止发布 |
| UI-001 | P0 | 全部 | 结果页必须分别显示“原位置移除量”“隔离占用”“回收估算”“可用空间变化（观测）”“结果未知”和失败项；R2 明示“隔离不等于释放磁盘空间” | Given R1/R2/R4、并发系统 I/O、观测 complete/partial/unavailable/notApplicable 或部分成功，When 渲染完成结果，Then 按第 5.5、7.8 节显示各自 basis/观测状态；不得以候选大小、原位置移除量或隔离占用冒充精确释放量，缺测不得按零，卷级观测不得宣称由本操作精确归因 |
| UI-002 | P1 | 全部 | 核心流程在 Windows 125%、150%、200% 缩放下保持内容可读、命令可达且文字、按钮、表格不重叠或截断 | Given 三个强制缩放级别、最长本地化文本和最小支持窗口，When 遍历扫描、预览、确认、进度、结果、隔离和设置页，Then 不出现遮挡/截断/不可滚动命令，截图差异证据通过 |
| UI-003 | P1 | 全部 | 核心流程支持仅键盘操作、可见焦点、合理顺序、屏幕阅读器名称/角色/状态和动态结果通知 | Given 不使用鼠标并开启 Windows 屏幕阅读器，When 完成扫描到结果及恢复/错误流程，Then 所有命令可达、焦点不丢失且名称、风险、状态和错误可感知 |

<a id="qpn-sec-9-1"></a>
### 9.1 缓存清理

- `%LocalAppData%\Temp` 是跨应用通用暂存区，不能仅凭“超过 72 小时”证明可重建；V1 仅将 72 小时以前、通过全部排除和进程/安装状态检查的普通文件作为 R2 隔离候选。`%Windows%\Temp` 不进入 V1 默认文件规则。
- 浏览器仅纳入明确的 `Cache`、`Code Cache`、`GPUCache`、Firefox `cache2` 等可重建叶子目录。
- 浏览器 Cookie、历史、表单、下载记录、密码和登录状态不属于 V1 默认清理。
- 应用缓存按产品、版本、配置根和进程保护建模；不能把目录名包含“cache”作为唯一证据。
- 日志和崩溃材料可能含诊断价值且不能重建，默认至少为 R2；不得因应用可继续运行而归为 R1。
- 微信/聊天软件必须区分可重建缓存与聊天数据库、图片、视频、文件和语音。后者至少为 R2；永久删除时为 R4。
- 运行中的 SQLite/LevelDB 或无法判断所属进程状态的规则不生成清理候选。

<a id="qpn-sec-9-2"></a>
### 9.2 空间分析、大文件与重复文件

- 空间分析、大文件和重复文件使用独立只读任务，任务 ID 必须唯一。
- 同时最多运行 3 个只读任务；超过上限按 SCAN-002 返回 `LIMIT_EXCEEDED`，不创建或排队隐藏 task。
- 大文件“闲置”只能作为低置信度提示。NTFS Last Access 可能关闭或延迟，不得据此自动删除。
- 大文件发现始终为 R0；M1 写动作只提供用户逐项选择后的 R2 隔离。`permanentDeleteOriginal` 不属于 M1 默认范围；仅当具体发布构建显式启用 R4-DELETE-001、通过 GATE-008 且后端 capability 开关有效时，才可从新的快照创建独立 R4 计划和二次确认，不能把现有 R2 计划改动作后执行。
- 重复识别采用：逻辑大小 → 头尾采样 SHA-256 → 完整 SHA-256 → 对哈希相同项用稳定只读句柄逐字节比较。
- 完整 SHA-256 只表示高置信匹配证据；只有逐字节比较完成且身份在比较前后稳定，UI 才标为“已确认重复”。V1 只展示重复组、硬链接差异和保留建议，不创建整理、隔离或删除计划。
- ACL、ADS、EFS、稀疏、压缩、云占位或硬链接语义不同的对象必须提示或排除。

<a id="qpn-sec-9-3"></a>
### 9.3 应用、启动项与分区

- 应用枚举可读取 HKCU/HKLM 的 32/64 位卸载视图，但必须隐藏系统组件和不可安全启动的卸载项。
- V1 最低交付必须开放并通过门禁的卸载适配器是 MSI 与 AppX/MSIX；安全快照无法建立的条目隐藏。所有通用 Win32 EXE 适配器，无论是否需要提升，均属于 APP-004 条件能力；只有通过独立参数、注册项身份、外部进程和崩溃门禁，且提权分支另通过签名与高完整性上下文门禁后，才可按构建整体或按编译适配器子集开放。未开放项必须从 UI 隐藏并在发布说明列明，不能用“仅支持枚举”冒充 V1 卸载覆盖。
- 应用枚举为 R0；任何卸载启动统一走 `create_uninstall_plan → authorize_plan → execute_plan → get_uninstall_operation`。`create_uninstall_plan` 只把已复检 `AppSnapshot` 封装为一个不可变 `appUninstall` item，不启动程序；风险固定 R3，是否进入 `readyForElevation` 由后端计算的 `requiresElevation` 决定。
- MSI 调用仍只使用 Windows Installer 受支持 API，但 ProductCode 本身不能精确指定 install context。枚举、建计划以及调用紧前必须由最终执行 token 使用 `MsiEnumProductsExW + MsiGetProductInfoExW` 覆盖 machine/userManaged/userUnmanaged 和 broker 可见的全部用户；同一规范 ProductCode 的匹配集合必须恰好一项，且与 snapshot 的 context/SID/版本/local-package registration identity 相同。多 context、枚举权限不足、集合变化或 API 无法精确约束所确认实例时返回 `UNINSTALL_TARGET_AMBIGUOUS`，零 MSI 调用；不得依赖当前 token“通常会选择正确 context”。调用后再次全量枚举并把 context-set digest 写入 observation。AppX/MSIX 仅通过当前用户 PackageFullName 调用 PackageManager deployment API；provisioned/all-users package 不属于 V1 并从 UI 隐藏。
- APP-004 开放时，Win32 卸载仅接受快照中的规范化本地绝对 `.exe`。后端保存卷/File ID、大小、FILETIME、SHA-256、固定参数、文件位置保护级别和 Authenticode 链/叶证书 SPKI 哈希；不经过 ShellExecute 协议、`cmd.exe`、PowerShell、`rundll32`、脚本解释器或环境变量展开。APP-004 关闭时，枚举结果不得携带可建计划的 Win32 capability，`create_uninstall_plan` 必须在创建任何计划、锁或 attempt 前拒绝。
- 启动前以拒绝共享写入/删除的方式打开 EXE 并保持句柄，完成身份、哈希和签名复检，再用精确 `lpApplicationName` 与 `CREATE_SUSPENDED` 调用 `CreateProcessW`。进程创建返回后查询子进程实际映像并与快照核对；一致只表示可继续，主线程此时仍必须保持 suspended，原文件句柄也继续保留。只有完成下文固定顺序中的 token/image 复核、禁止 breakaway 的 Job 归属、child+Job+launch evidence 刷盘后，才可调用唯一一次 `ResumeThread`，随后按 attempt 记录释放原文件句柄。无法保持句柄、映像不一致或任一前置证据不完整时终止/保持 suspended 子进程并进入规定的 notStarted 或 recoveryRequired 分支，绝不提前执行卸载代码。
- 当前用户、无需提升的卸载器可将“无签名/未知发布者”作为高风险展示，但不得谎称已验证发布者。任何通过提权助手启动的 Win32 卸载器必须同时满足：来源为受保护 HKLM 32/64 位卸载注册项；注册键 DACL/owner 与 last-write identity 证明普通用户不可写；EXE 位于普通用户不可写目录；具有受信任 Authenticode 链且叶证书 SPKI 哈希与快照一致；`elevationPolicy.kind=compiledElevatedAdapter` 且 template ID/受保护产品元数据命中应用内编译表。“任意有效签名”、仅位于 Program Files 或仅来自 HKLM 均不足以通过。
- 提权 Win32 的参数只能由编译进应用的产品专用模板从已验证注册值生成，模板规定完整 token 序列、允许的可选开关和零个任意字符串插槽；生成结果与 `sealedInvocation.fixedArguments`、template ID 和产品元数据摘要一起进入 planHash，高完整性 helper 重新生成并逐 token 比较。HKCU、普通用户可写 HKLM 注册项、user-writable EXE、未知模板或需要拼接任意参数的目标只能按明确 `currentUserOnly` 在普通权限下执行；如果其卸载实际需要提升，则隐藏/拒绝为 `UNINSTALL_TARGET_INVALID`，不得临时改走 UAC。
- 条件性提权 Win32 还必须把 `SealedElevatedProcessContext` 纳入 snapshot/planHash：CWD 只能是已固定且普通用户不可写的 EXE 目录或已验证 System32；环境从零构造，只含表列系统字段，PATH 只含已验证 System32/Windows，TEMP/TMP 指向本次创建且普通用户不可写的目录；helper 自身先采用安全 DLL 搜索策略，child 通过扩展启动属性启用 prefer-System32、禁止 remote/low-label image load。应用目录依赖逐个固定身份并验证普通用户不可替换；每个 compiled adapter 必须通过产品特定 DLL planting/side-loading 测试。任一 CWD、环境、mitigation、依赖或测试证据缺失时 APP-004 capability 保持关闭，不能以主 EXE 已签名为由放行。
- 高完整性确认页不得复用普通权限快照的展示名称。helper 必须用已封装的 MSI ProductCode、AppX package identity 或 Win32 受保护注册 identity 重新枚举名称、发布者、scope 和目标版本，并把该 identity digest 写入确认收据；重枚举缺失、变化或无法确定时拒绝执行。
- 相对路径、UNC、URL/协议、未闭合或歧义参数、扫描后替换的可执行文件，以及需要清盘代为拼接提权参数的卸载项均拒绝。清盘不直接删除安装目录。
- 标准用户只允许执行计划时已确认无需提升的当前用户卸载；per-machine MSI 或预检判断需要管理员权限时，标准用户返回 `ELEVATION_SAME_USER_REQUIRED`，不触发凭据式 UAC。拆分令牌管理员的系统级卸载由第 6.4 节助手只从受保护 `PlanItem` 读取适配器和固定参数。若普通执行阶段意外收到 `ERROR_ELEVATION_REQUIRED`，当前 operation 失败并消费计划；不得在同一计划内升级权限，必须重新枚举、建计划和确认。
- 每个卸载目标先生成 `CanonicalUninstallResourceIdentity`。MSI ProductCode 只能来自上述 singleton context evidence，解析为 GUID 后用 `StringFromGUID2` 的大写花括号形式，并加入 install context；machine context 禁止 SID，per-user context 必须加入目标 SID 摘要。AppX 调用封装 OS 返回的 PackageFullName，但锁身份固定为 PackageFamilyName + 当前用户 SID 摘要，使版本变化不能绕锁。Win32 锁身份只含 hive、32/64 view 和从已打开注册键解析的 canonical key-address digest，明确排除 last-write、values、EXE 和 snapshot 等可变字段。统一摘要为 `SHA256(ASCII("qingpan.uninstall-resource.v1\0") || UTF8(RFC8785(validated identity)))`。per-user MSI/AppX/currentUserOnly Win32 在用户 store 建唯一 lock；machine MSI 和受保护 HKLM elevated Win32 必须由高完整性 broker 在 `SYSTEM/Administrators` 保护的 machine store 中，以同一 resource digest 的部分唯一索引和受保护全局 mutex 原子 claim。不同 TokenUser SID 看到已有 machine attempt 时不能创建本地替代 lock 或第二次调用；其本地 plan/operation 只有在 broker 原子追加 owner 绑定的 `MachineUninstallAttachment` 后才可附着。attachment 固定 owner SID、本地 operation/plan/item/snapshot/plan hash、machine attempt ID、附着时 sequence/digest；查询按当前 owner 过滤，不暴露发起管理员 SID、源 operation ID 或其他 attachment。
- 调用 Windows API/创建外部进程前，operation item 的 `pending→prepared`、与 `sealedInvocation.adapter` 同分支的 `UninstallAttempt(state=launchPrepared)`、active target lock 和发起 operation 的首个 attachment 必须在所属 user/machine journal 的同一事务/CAS 提交；machine 分支还必须持有 mutex。attempt/launch evidence 的 adapter、`requiresElevation`、coordination scope、context set 和 resource digest 任一不一致都以 `OPERATION_STATE_INVALID` 在 OS 调用前失败。锁冲突只可原子附着到现有 attempt；其为 rebootPending 时 attachment 标为 rebootPending 并返回 `UNINSTALL_REBOOT_REQUIRED`，结果不明时标为 recoveryRequired 并返回 `UNINSTALL_RECOVERY_REQUIRED`。已证明 API/CreateProcess 在产生任何副作用前失败时可写 `notStarted` 并释放锁；调用边界崩溃时绝不再次启动。Win32 仍严格执行 Job 创建 → `CREATE_SUSPENDED` → token/image 复核 → 禁止 breakaway 的 Job 归属 → child/Job/launch evidence 刷盘 → 唯一 `ResumeThread` 的顺序。
- MSI 返回后先耐久记录真实 `0/3010/1641/其他`、调用前 boot ID 和 API evidence；不得只保存不透明 digest。`0` 才能直接进入资源/context 复检；`3010` 进入 `rebootPending` 并保持 lock，boot ID 未变化时任何 apply 只返回待重启，变化后才以全新 singleton-context observation 解析 removed/notRemoved；`1641` 因重启边界不可控进入 `recoveryRequired`；其他失败码即使 target 仍 present 也不能自动释放 lock 或重调。AppX `started` 只表示异步操作已建立；`completed` 必须保存 HRESULT、ExtendedErrorCode、ActivityId、restart disposition 和 result digest。success/no-restart 或 restart 已由 boot ID 变化满足后才可资源复检；failed/outcomeUnknown 保持 recoveryRequired。
- `launched/started` 只是非终态进度。known removed/notRemoved 只有三套合法证据：MSI 为 `0` 或 3010 后已证明重启，并有调用前后 singleton-context set；AppX 为同一 deployment 的 completed success/no-restart 或 rebootSatisfied；Win32 为已耐久 launch、`processState=exited` 且禁止 breakaway 的同一受控 Job 树 drained。三者再分别要求精确 target `absent` 才 removed、`present` 才 notRemoved；unknown 不能代替。`KnownRemovedEvidence/KnownNotRemovedEvidence` 的 source attempt sequence/digest、launch、observation、adapter、coordination、resource identity 与 `observedAtUtc` 必须逐字段对应同一 resolved attempt。per-user item/view 的 operation/plan/item/snapshot 直接对应该 attempt；machine item/view 则必须对应当前 owner 的唯一 `MachineUninstallAttachment`，并在同一终态事务把 resolved attempt sequence/digest 投影到其本地 operation/plan/item，禁止要求附着者复用源 attempt 的 operation ID。MSI/AppX 返回值不得放入 `processExitCode`；Win32 顶层与 observation exit code 必须相等。
- unknown/restart-required attempt 的 machine/user target lock 分别保持 `recoveryRequired/rebootPending`。只有同一受保护 journal 的只读对账明确满足上述 typed evidence，或另行设计的签名人工恢复流程，才可释放；“目标仍存在”本身不能解锁。清盘不为外部卸载承诺备份或恢复，UI 不得把 launched、started、返回码、HRESULT 或超时显示为已卸载。
- 创建计划前应用快照缺失或摘要变化返回 `APP_SNAPSHOT_NOT_FOUND/APP_SNAPSHOT_STALE`；计划认领后目标 EXE、注册资源或签名变化仍按执行时复检返回 `UNINSTALL_TARGET_INVALID`。任何情况都不得按旧快照自动重建并继续。
- V1 启动项只读。启动项写支持当前未规划，也不属于 V1.x/V2 路线图；未来如需禁用、恢复或删除，必须先批准独立决策/设计文档，覆盖 HKCU/HKLM、启动文件夹、任务计划、服务、UWP、备份恢复和提权边界，不能直接启用现有占位命令。
- 分区新建、删除、格式化、压缩和扩展均不由清盘执行。

<a id="qpn-sec-9-4"></a>
### 9.4 自动任务

- 自动任务默认关闭。
- 使用 Windows 任务计划程序，运行身份为当前用户，不保存管理员密码。产品只管理固定命名空间下由安装密钥派生名称的任务；action 固定为签名 scheduled runner 和单个不透明 job ID，不接受 UI 提交的可执行文件、参数、工作目录或 XML。
- R0 作业只引用后端持久化的 `analysisPolicyId + canonicalPolicyDigest`。持久策略由原生目录选择/确认产生，规范路径使用 DPAPI 保护；每次运行重新打开并验证卷 GUID 和根 File ID，变化时禁用作业而不是扫描新位置。
- `R1AutomationPolicy.rules` 和派生的 `ApprovalGrant.bindings` 使用集合语义，均为 1 至 256 项；`ruleId` 大小写敏感且唯一，重复项在查库、确认和摘要前拒绝，不得静默去重或 last-wins。后端验证后按 `ruleId` 的精确 UTF-8 字节排序，grant bindings 只能从已批准 policy revision 派生，UI 不能提交。
- R1 自动批准必须由 `create_automation_approval` 显示原生摘要后签发 `approvalGrantId`。同一状态仓库事务生成 grant、唯一 job ID 和永久 `ApprovalGrantJobBinding`；数据库对 grant ID 建主键、对 job ID 建唯一索引。grant 绑定 policy/revision/digest、全部规则、trigger、Windows time-zone ID、task principal、1 至 180 次运行上限和 `authorizedScheduleDigest`，最长有效 180 天并可随时撤销。删除 job 会终态化 binding 并撤销 grant，不释放唯一关系或清零运行次数；只有同一 journal 的崩溃修复可重建同一 job，用户重新创建或改变触发器、时区、主体、上限及任一规则时必须重新原生批准。
- R1 `upsert_scheduled_job` 的严格 schema 只允许 `kind/idempotencyKey/approvalGrantId`；后端从 binding 解析 job 和全部定义。R0 upsert 可提交受限 trigger/time-zone/policy ID。upsert/delete 均先查耐久 idempotency record；同 key 同 payload 返回原完整 mutation result，同 key 异 payload 在检查 revision/grant 或调用 Task Scheduler 前拒绝。
- `canonicalPolicyDigest` 覆盖能力包络、根选择、匹配器、最小年龄、进程保护、排除、风险、动作、恢复、权限、平台和资源上限；任一实质字段变化均使自动批准失效，不只检查风险是否升高。
- `authorizedScheduleDigest` 只覆盖 `ScheduledJobDefinitionMaterial`：policy/grant/job、trigger、Windows 时区、principal、运行上限、固定 runner 版本和固定任务设置；明确排除 job/grant revision、enabled、runsStarted、状态、时间戳和所有 digest 字段。`windowsTaskDefinitionDigest` 对 Task Scheduler API 重读并规范化后的 folder/name、唯一 action、签名 runner 身份、参数形状、principal/logon/run-level、trigger/时区模式、并发/错过运行设置和 ACL 计算；禁止直接哈希原始 XML。创建、批准、注册、启动对账和 runner 使用同一投影与测试向量。
- 作业 upsert 按第 8.3 节 journal 执行：注册时始终 disabled，验证后才应用期望的 enabled 状态并重读，匹配后提交 `committed`；R1 新批准 job 的期望值固定为 enabled，R0 可保存 disabled 定义。主应用和 runner 在开放任何自动命令前都枚举固定命名空间；未知、缺失、篡改、owner/principal 不符、无法禁用或无法精确续作的任务使自动子系统失败关闭，不覆盖或执行未知 Task。删除先持久化 intent、确认 disabled，再删除和写 tombstone。
- Task 定义固定 `AllowStartOnDemand=false`。每次 R1 触发在任何扫描、计划或 item 前，runner 必须通过 Task Scheduler COM 与 Operational evidence 证明：精确 registered task 存在 active instance GUID；该 instance 由批准的 time trigger 或 `StartWhenAvailable` 产生；当前 runner PID/创建时间要么与目标 build 事件中的 action PID 精确对应，要么能以受保护进程句柄证明是对应 `IRunningTask.EnginePID` 启动的精确 action descendant。`EnginePID` 不等同于 runner PID，禁止把二者直接比较；两种 correlation 的 build-specific 行为必须在第 13.3 节 VM fixture 验证，无法取得任一证据时失败关闭。当前系统定义摘要还必须匹配，且 instance GUID/计划 occurrence 尚未消费；直接启动 runner、调用 on-demand Run、伪造 job ID 或无法读取触发原因均返回 `JOB_TRIGGER_ATTESTATION_INVALID`。随后单事务 CAS `attestation + committed job + active grant + revision/digest/binding + runsStarted < maximumRuns`，递增唯一的 `ApprovalGrant.usageRevision/runsStarted` 并创建唯一 `ScheduledRunClaim(jobId, runOrdinal, instanceGuid)`；`ScheduledJob.runsStarted` 只是该计数器投影。两个 runner 竞争最后一次运行或同一 instance 重放时最多一个成功；达到上限同事务把 grant 置为 `runLimitReached` 并逻辑禁用 job，之后零 item。
- 作业触发及每个下一 R1 item 的 prepared CAS 都重新验证 SID、期限、grant/job/policy revision、当前包撤销、全部 binding 和能力包络；否则不生成新写 item并在本地记录稳定原因。R1 自动任务仍执行快照复检；失败项保留并进入本地审计。

---

<a id="qpn-sec-10"></a>
## 10. 清理规则规范与 V1 目录

<a id="qpn-sec-10-1"></a>
### 10.1 规则能力限制

本节是 RULE-001~006 的规范来源。

**应用内能力包络是最高权限边界。** 每个 `CapabilityEnvelope` 随签名应用发布，固定根发现器、允许叶子模板、匹配器集合、最小年龄下限、强制进程保护/排除、允许的风险-动作-恢复组合、自动化上限和资源上限。远程 `RuleManifest` 必须引用已知 `envelopeId`，并且只能：

- 选择包络允许的根发现器和叶子，或继续收窄相对路径；不能声明新的本地路径。
- 提高最小年龄、增加进程保护/排除、降低文件数/结果数/深度/时长上限；`RuleManifest.resourceLimits` 每项必须为正数且不得高于应用内 `CapabilityEnvelope.resourceCeiling`，不能省略后解释为无限制。
- 选择包络允许且不更危险的风险、动作、恢复与权限组合；不能自行把内容标为 R1 或开放自动化。
- 使用枚举匹配器；相对部分不得包含 `..`、绝对/设备/UNC 路径、通配盘符、环境变量展开、NTFS 流语法或重解析跳转。

三种 matcher 的语义封闭如下，不能由实现或远程包扩展：

- `allFilesInLeafRecursive`：从每个已解析叶子向下递归，只产生普通文件候选，不产生目录候选。
- `rootFileNames`：只检查叶子的直接普通文件子项，深度严格为 1；`names` 内是 OR。
- `descendantDirectoryNames`：只对深度 1 至 `resourceLimits.maximumDepth` 的目录 basename 应用条件，命中目录后递归产生其下普通文件；嵌套命中按卷 GUID + File ID 去重。`names` 内是 OR，`names/prefix/suffix` 之间是 AND；省略字段不参与，但至少一个条件必须存在。
- 多个 `rootSelections` 是 OR；matcher、最小年龄、进程保护、排除类、根授权、平台和全部资源限制之间是 AND，任一失败都不生成候选。目录本身、重解析目标和超深对象永不成为隐式候选。
- matcher 数组必须非空。名称、前缀、后缀、resolver/template/rule ID 均为非空字符串；路径组件字段必须是单个合法 Windows 组件，拒绝 `/`、`\`、冒号、NUL、通配符、`.`、`..`、尾随点/空格和保留设备名。不得自动做 Unicode 规范化或 locale case-fold。
- 名称比较服从父目录的实际大小写语义：普通目录使用 Windows ordinal ignore-case，启用 case-sensitive 标志的目录使用 ordinal exact；无法确定时不匹配。prefix/suffix 使用同一比较器，不能混用另一套折叠规则。

控制面在分配大型集合前执行绝对限制：index、key authorization、revocation 各不超过 256 KiB，规则包不超过 8 MiB；每包最多 256 条规则，每规则最多 16 个根选择、256 个 matcher 名称、32 个 build 范围、64 个进程保护、16 个排除类和 64 个验收 ID；其他未单列数组最多 256 项。任一字符串最多 512 UTF-8 字节，包内全部字符串最多 4 MiB，JSON 嵌套最多 32 层；规则自身的 `maximumDepth` 还不得超过应用全局 128。所有数组拒绝重复项，所有计数必须为正整数；超限、未知字段、空集合或重复值返回 `RULE_SCHEMA_INVALID/LIMIT_EXCEEDED`，不得截断、降采样或忽略。

签名有效但越过包络与签名无效同样拒绝。远程规则不得声明用户选择根、Windows/Program Files/ProgramData 根、其他用户配置文件，也不支持脚本、正则替换、命令执行、动态下载子规则或任意自定义删除动作。每条规则声明发现证据、最小年龄、进程保护、风险、动作、权限、排除、恢复、支持系统和验收 ID。

规则包规范如下：

1. 规则 index、key authorization、revocation 和包分别使用第 8.3 节媒体类型；UTF-8 JSON 拒绝重复键，payload 按 RFC 8785 规范化。`payloadSha256` 与签名覆盖相同规范化字节，recovery signer ID 必须唯一且达到应用内阈值。
2. `request_rule_update` 只接受耐久的规则 channel setting ID。Rust 从固定 `/index`、`/key-authorization`、`/revocations` 获取控制面，先验证恢复阈值签名，再验证 key 授权和撤销高水位，随后验证 index release 签名；只有 index 绑定的 `packagePayloadSha256 + packageSizeBytes` 可映射到固定包路径。
3. stable、beta、internal 使用独立发布密钥和独立高水位。客户端按通道事务持久化授权、撤销、index、包四类最高序号及同序号 payload 哈希；更低序号拒绝，同序号异哈希、同 key ID 异公钥、sticky 撤销集合缩小或 `minimumAcceptedPackageSequence` 下降均视为安全事件。
4. 常规发布密钥只能由离线恢复根阈值授权；撤销 release key、index 或包哈希和提高最低包序号同样必须由恢复阈值签名。撤销一经接受即并入单调 sticky 集合，最低包序号只取历史最大值；普通 key authorization 或更高普通 index 不得解除。active key/index/包命中或包序号低于 floor 均关闭规则清理。发布密钥不能授权自身替代者；恢复根变更只能随代码签名应用更新完成。
5. 业务回滚不降低序号，而是把已知良好规则内容以更高包序号和 index 序号重新签发。被撤销或低于高水位的旧包不能因 active 包损坏而静默复活。
6. 包下载后依次执行格式/大小、通道/兼容性、哈希/包签名、schema、能力包络和固定安全语料。全部通过后先刷盘包文件，再在单个状态仓库事务中提交 active 包哈希、四类高水位、sticky 撤销和激活事件；另一个槽只作诊断与更高序号重发来源。
7. active 包切换按第 7.6 节只使引用该包的未认领计划失效；已认领操作固定包哈希，紧急撤销在 item 边界停止。包文件在所有引用释放前不得删除。R1 自动批准绑定 `ruleId + packageHash + ruleVersion + canonicalPolicyDigest`；任何实质策略变化都要求重新批准。
8. stable 生产构建绝不允许选择 internal。stable 与 beta 之间切换必须由 Rust 原生页确认；规则通道切换会撤销旧 setting、使全部规则候选计划和相关 grant 失效并使用目标通道独立信任状态，不迁移 active 包、高水位或撤销集合。

至少验收：签名有效但越包络、资源上限扩大、低序号、同序号异内容、跨通道 index/包、未知或同 ID 异公钥、授权/撤销签名不足、撤销集合缩小、已撤销 key/index/包、损坏 active 槽、激活与执行并发、通道切换，以及以更高序号恢复已知良好内容。

V1 的单调高水位用于阻止网络重放、候选包降级和普通状态损坏，不承诺抵御已经能以同一 Windows SID 任意替换/还原用户数据目录的恶意进程；Windows 用户态 ACL 无法按同 SID 进程隔离。此限制不放宽签名/能力包络，也不允许提权助手信任用户态状态：助手仍按第 6.4 节独立验证完整计划和应用内能力。高水位状态一旦丢失或冲突只能通过签名应用修复/显式恢复流程重建，不得自动接受当前磁盘上的最低或最高包。

<a id="qpn-sec-10-2"></a>
### 10.2 初始规则目录

| 规则类别 | 发现依据 / 允许根 | 匹配器 | 最小年龄 | 进程保护 | 风险 / 动作 | 权限 | 排除项 | 恢复 | 系统 | 验收 |
|---|---|---|---|---|---|---|---|---|---|---|
| 用户通用 Temp | `LocalAppData\Temp` 内置精确锚点 | 普通文件递归 | 72 小时 | 安装/更新/待重启状态不明时停用 | R2 / 隔离 | 用户 | 重解析、锁定、身份变化、EFS、云占位、安装回滚材料 | 7 天隔离 | Win10/11 | CLEAN-004、REC-001、SAFE-001 |
| Chromium 缓存 | 浏览器已发现配置根下明确 Cache 叶子目录 | 普通文件递归 | 无 | 对应浏览器必须关闭 | R1 / 永久删除 | 用户 | Cookie、History、Login Data、扩展数据 | 浏览器可重建 | Win10/11 | CLEAN-001、SAFE-001 |
| Firefox 缓存 | 配置根下 `cache2` | 普通文件递归 | 无 | Firefox 必须关闭 | R1 / 永久删除 | 用户 | cookies.sqlite、places.sqlite、logins.json | 浏览器可重建 | Win10/11 | CLEAN-001、SAFE-001 |
| VS Code/Discord/Figma 缓存 | Local/Roaming 下已知产品目录的 Cache 叶子 | 普通文件递归 | 无 | 对应应用必须关闭 | R1 / 永久删除 | 用户 | 工作区、配置、插件和项目文件 | 应用可重建 | Win10/11 | CLEAN-001、SAFE-001 |
| 微信可重建缓存 | 已发现微信配置根下经版本验证的明确缓存叶子 | 规则列举目录 | 无 | 微信必须关闭 | R1 / 永久删除 | 用户 | 日志、崩溃材料、消息数据库、图片、视频、文件、语音 | 应用可重建 | Win10/11 | CLEAN-001、SAFE-001 |
| 应用日志/崩溃材料 | 应用内置精确日志或崩溃叶子 | 规则列举目录 | 7 天 | 对应应用必须关闭 | R2 / 隔离 | 用户 | 活跃诊断会话、身份不稳、EFS、云占位 | 7 天隔离 | Win10/11 | CLEAN-004、REC-001 |
| 用户选择的大文件 | 原生选择器签发的本地分析根 | 最近快照中的单文件 | 无 | 视文件类型 | R0 扫描；R2 隔离 | 用户 | 受保护、系统目录、额外数据流、云占位 | R2 隔离 | Win10/11 | LARGE-001、SAFE-001 |
| 条件：原文件永久删除 | 仅引用新的、仍有效的 R2 用户选择候选，不接受远程规则扩大根 | 精确 `PlanItem` 单文件 | 无 | 沿用并加强来源项保护 | R4 / 永久删除 | 用户 | M1 默认关闭；全部 R2 排除及身份复检 | 无 | Win10/11 | R4-001、R4-DELETE-001、SAFE-001~005 |

Windows 更新残留、组件仓库、驱动仓库、系统还原点和注册表不属于 V1 文件规则。它们只能在后续版本通过专用系统 API 适配器实现。

<a id="qpn-sec-10-3"></a>
### 10.3 用户自定义规则

V1 只提供“排除规则”和“用户选择的只读分析根”。任意自定义删除规则不进入 V1。

后续如开放自定义清理规则，必须：

- 仅保存在本机，不从远程同步。
- 默认 R2、不可自动化并要求逐项预览。
- 拒绝 Windows、Program Files、ProgramData 根、其他用户目录、设备路径和网络路径。
- 使用与签名规则相同的复检、隔离和审计流程。

---

<a id="qpn-sec-11"></a>
## 11. 高风险能力设计边界

本章只记录第 15.3、15.4 节非承诺候选方向的安全上限，不是 V1 目标、确定的 V1.x/V2 阶段、排期或现有隐藏功能。对应方向在另立并批准含稳定需求/里程碑 ID、威胁模型、支持矩阵和专项门禁的设计前，不得进入发布构建、UI、命令面、feature flag、官网或发布说明；以下“需满足”均表示未来获批方案的准入条件。

<a id="qpn-sec-11-1"></a>
### 11.1 注册表

若未来注册表候选方案获批，其最初可交付阶段也仅允许诊断和导出，不修改注册表；再后续的有限修复还需满足：

- 每类规则有明确产品/组件证据；“路径不存在”本身不足以判定冗余。
- 区分 HKCU/HKU、HKLM、32/64 位视图、COM、MSI、AppX、服务和文件关联。
- 修改前逐项导出，记录键、值类型和原始数据；恢复不得覆盖用户后续修改。
- 默认不处理启动关键、服务、驱动、网络、Shell、Windows Installer 和安全软件键。
- 每条修复规则通过干净安装、升级、卸载、回滚和多语言系统测试。

<a id="qpn-sec-11-2"></a>
### 11.2 Windows 组件与驱动

- 不直接删除 `WinSxS`、`SoftwareDistribution`、`DriverStore` 或 VSS 数据。
- 组件维护仅调用 DISM/Servicing Stack 支持的操作并验证返回状态。
- 驱动管理仅使用 SetupAPI/PnPUtil 等支持接口。
- 当前、启动、存储、显示基本驱动、网络和回滚所需驱动默认保护。
- 操作前建立对应系统恢复信息；完成后验证重启、设备管理器、Windows Update、DISM 和 SFC。

<a id="qpn-sec-11-3"></a>
### 11.3 安全擦除

- 普通删除、隔离、HDD 尽力覆写和设备级擦除是不同能力，不得混用名称。
- HDD 单文件覆写只能描述为尽力降低常规恢复概率，不保证覆盖坏扇区、备份、VSS、缓存或其他副本。
- SSD、U 盘和闪存受 FTL、磨损均衡、TRIM 和预留块影响，不提供单文件多次覆写保证。
- 全盘加密路径只称“条件性密码学擦除指导”：敏感数据写入前必须已完整加密，流程需枚举并撤销全部保护器、托管/恢复副本和 clear key，处理内存、休眠及备份副本，并验证卷不能再解锁。删除单个 BitLocker 保护器不等于销毁 VMK/FVEK，也不构成高保证结论。
- 不能满足上述前置条件时，只能使用设备明确支持、能验证完成状态的 ATA/NVMe sanitize；否则不提供高保证擦除声明。两类流程均为 R4 独立能力。
- 只有未来独立 R4 擦除方案获批并通过专项门禁后，才可设计高级入口，并要求外接电源、目标设备二次识别、输入确认和不可恢复警告；在此之前发布构建中不得存在公开或隐藏入口。

---

<a id="qpn-sec-12"></a>
## 12. 非功能要求与支持矩阵

<a id="qpn-sec-12-1"></a>
### 12.1 平台与介质

本节展开 PLAT-001~002。

| 平台/介质 | V1 支持级别 | 说明 |
|---|---|---|
| Windows 11 24H2 build 26100 x64/ARM64 | 完整目标 | 主要发布矩阵；分别验证原生架构包 |
| Windows 11 25H2 build 26200 x64/ARM64 | 完整目标 | 主要发布矩阵；分别验证原生架构包 |
| Windows 10 22H2 build 19045 x64/ARM64 | 兼容目标 | 独立验证 WebView2、安装器和 API 降级；不改变微软生命周期事实 |
| Win7/Win8/32 位 | 不支持 | 安装器应明确拒绝 |
| NTFS 本地固定盘 | 完整目标 | 支持快照复检、隔离和哈希 |
| ReFS | 只读分析目标 | 写操作需独立验证后开放 |
| FAT32/exFAT/移动闪存 | 只读分析目标 | 不进入默认缓存清理和隔离 |
| 网络盘/UNC | 不支持 | 不扫描、不哈希、不删除 |
| 云占位文件 | 排除 | 不触发召回，不把云端删除当本地清理 |
| Storage Spaces | 条件支持 | 作为本地挂载卷只读分析；写操作需通过专项测试 |

磁盘碎片不是垃圾。清盘不实现碎片整理；SSD/HDD 优化由 Windows Optimize Drives 或设备管理工具负责。

每个发布候选冻结上述精确 build 及最新累计更新号并写入发布矩阵。高于矩阵的未来 Windows build 初次运行只开放 R0 与恢复/诊断；R1-R4 后端 capability 必须失败关闭，直到该 build/架构/文件系统组合完成门禁并由新发布配置明确加入。不得用“Windows 11”宽泛匹配自动开放写能力。

机器权威范围不是上表自然语言或自由 selector，而是 `PlatformTupleRegistry v3`：固定 3 个 build × 2 个原生架构 × `ntfsSsd/ntfsHdd/refsSsdReadOnly` 3 个数据卷 profile × `standardUser/splitTokenAdminMedium` 2 个发起权限 profile，共 36 项。注册表从四个编译期维度生成精确 key map，JSON Schema 要求 36 个属性且 `additionalProperties=false`；CI 复算笛卡尔积、key/value 分解、冻结 UBR、`RELEASE_TEST_OBLIGATIONS` 的 252 个唯一 ID/capability/coverage、两个 canonical digest 和 `all36=36/ntfsWrite24=24/refsReadOnly12=12` 三个闭合覆盖 profile。系统卷仍按 Windows 支持方式安装，数据卷 profile 用于扫描/写能力测试；ReFS profile 的 `maximumRisk=R0`，不得因安装流程在 NTFS 系统卷通过而开放 ReFS 写动作。

<a id="qpn-sec-12-2"></a>
### 12.2 性能基准

本节展开 PERF-001。

参考环境至少包含：

- 4 核 CPU、8 GB 内存、512 GB SSD。
- 4 核 CPU、16 GB 内存、1 TB HDD。
- 10 万文件和 100 万文件两套固定数据集，包含小文件、大文件、重复文件和排除对象。
- 每个机器/数据集/任务组合先运行 1 次不计入结果的预热，再运行 10 次暖缓存轮次；另执行 5 次重启后的冷启动轮次。记录数据集版本、磁盘剩余空间、电源模式、杀毒软件状态和应用构建号。

V1 门槛：

- 峰值 RSS 不超过 512 MB。
- 应用空闲 10 分钟后的 CPU P95 低于 1%，且无持续磁盘读写。
- 用户取消后，除正在阻塞的单次 OS 调用外，任务 P95 在 2 秒内停止继续枚举。
- 同一参考环境相对上一批准版本的扫描时间、峰值内存或读取字节回归超过 20% 时阻止发布，除非评审记录接受原因。
- 分别报告暖/冷轮次的 P50/P95 扫描时间、CPU、峰值内存、读取字节、写入字节、跳过项和是否达到资源上限；保留每轮原始数据，不与未固定版本和数据集的竞品比较速度。

<a id="qpn-sec-12-3"></a>
### 12.3 可靠性与可用性

本节展开 REL-001 与 UI-001~003。

- 所有长任务可取消并显示阶段、当前对象和已处理数量。
- 任何部分失败均保留逐项原因，不显示笼统“全部成功”。
- 125%、150%、200% Windows 缩放下文字、按钮和表格不得重叠。
- 核心流程支持键盘操作、可见焦点和屏幕阅读器名称。
- UI 文案不得将“候选大小”“原位置移除量”“回收估算”“可用空间变化（观测）”混为一谈。
- 关闭应用后不得保留常驻清理进程；任务计划由 Windows 调度一次性启动。

<a id="qpn-sec-12-4"></a>
### 12.4 发布安全

本节展开 RELEASE-001。

- MSI/NSIS 安装包、应用更新和提权执行器使用受信任代码签名证书。
- 生产 Tauri/WebView CSP 将 `connect-src` 设为 `'none'` 并限制导航/远程资源；第 5.7 节固定 origin 只存在于 Rust 类型化客户端策略，不以宽泛 `https:` 或通配域名实现。
- 发布包不包含开发管理员密码、离线卡密、测试端点或空签名配置。
- `ReleaseCapabilityManifest v2` 必须从构建和打包实际消费的受签名发布配置生成，而不是由测试进程自报；`RELEASE_CAPABILITY_POLICY` 固定 16 个 M1 必交能力只能 enabled，仅 `genericWin32Uninstall/permanentOriginalFileDelete` 可按判别联合 enabled 或 disabled。发布 artifact 内嵌该 manifest canonical digest，构建 provenance 把 capability manifest 作为 material，发布根再同时绑定最终 artifact SHA-256 与 manifest digest，避免双向摘要环。每个 run 和 trace 必须引用同一 digest。
- 每个发布 artifact 生成 CycloneDX JSON 1.6 SBOM、canonical JSONL dependency findings/dispositions 与闭合 `DependencySecurityReport v2(result=passed)`；报告绑定 artifact/source/build/CI、SBOM 原始摘要、扫描器/漏洞库快照、两份原始记录和可复算 root/count，`unresolvedCriticalOrHighCount` 必须由 join 结果得到 0。SBOM、原始记录或报告缺失、跨构建替换、摘要/计数不符、orphan/过期 disposition、`unknown` severity 或仍有未处置 critical/high finding 时不得发布。
- 依赖构建生成 SBOM，安全公告和高危依赖在发布前完成处置。
- 规则签名私钥离线保存；构建系统只获得受控签名结果，不保存根私钥。

---

