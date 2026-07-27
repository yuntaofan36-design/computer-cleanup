# 清盘设计文档集

入口与状态以仓库根目录的[主索引](<../../Windows磁盘清理工具（聚合主流工具精华）开发设计文档.md>)为准。

1. [产品范围与现状基线](01-product-scope.md#qpn-sec-1)
2. [安全、恢复与目标架构](02-safety-architecture.md#qpn-sec-5)
3. [运行时 API 与数据契约](03-runtime-api.md#qpn-sec-8)
4. [需求、规则与支持矩阵](04-requirements.md#qpn-sec-9)
5. [测试、发布门禁与运维](05-test-release.md#qpn-sec-13)
6. [路线图与退出条件](06-roadmap.md#qpn-sec-15)
7. [附录](07-appendices.md#qpn-app-a)

开发工作树事实另放在 `implementation/`，不与目标契约混写：

- [2026-07-25 实验性隔离与副本导出](implementation/2026-07-25-quarantine-preview.md)

`contracts/` 与 `release-contracts/` 是第 8.3 节和第 13.6 节的规范类型片段，按文件名前缀顺序读取和校验。
