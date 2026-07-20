# 清盘套件

Windows 磁盘清理客户端及其授权管理系统。项目按运行边界拆分为三个独立目录：

- `desktop/`：Tauri 2 + React 客户端，包含安全扫描与清理、空间分析工作台、文件发现、应用管理、恢复审计和卡密激活。
- `web/`：React 卡密管理后台，包含管理员登录、授权概览、批量生成和设备状态界面。
- `server/`：Express + SQLite 授权服务，处理卡密、设备激活、会话校验、撤销和审计。

## 本地运行

```bash
pnpm install
pnpm dev
pnpm dev:server
pnpm dev:web
pnpm dev:desktop
pnpm dev:desktop:preview
```

`pnpm dev` 会同时启动 Tauri 桌面窗口、管理后台和授权服务。开发端口固定为：桌面资源服务 `127.0.0.1:1420`、管理后台 `127.0.0.1:3000`、授权 API `127.0.0.1:8787`。`pnpm dev:desktop` 单独启动 Tauri 应用，`pnpm dev:desktop:preview` 只启动浏览器预览。首次启动服务端会创建管理员 `admin@qingpan.local`，密码来自 `ADMIN_PASSWORD`；开发默认值为 `change-me-now`，生产环境必须覆盖。

## 安全约束

- 服务端仅保存卡密 HMAC，不保存或再次展示完整卡密。
- 完整卡密只在批量生成响应中返回一次。
- 客户端清理命令只接受最近一次扫描产生的条目 ID；执行阶段只处理扫描快照中的文件，并重新校验规则根目录、父目录链、规范路径、文件大小与修改时间。
- 扫描后新增或变化的文件、符号链接、Windows Junction/重解析点、越界路径和锁定文件一律跳过；清理内核不使用递归目录删除。
- Temp 仅纳入至少 72 小时未修改的文件；浏览器和应用规则只进入明确的 Cache/Code Cache/GPUCache/cache2 叶子目录。相关进程运行中或状态无法确认时，扫描与执行均安全跳过。
- 大文件、目录树和重复文件使用独立的只读任务。重复文件按大小、头尾采样 SHA-256、完整 SHA-256 分阶段确认，并排除硬链接、NTFS 额外数据流、云占位、网络盘和重解析点。
- 应用卸载只调用最近一次注册表快照中的正常 `UninstallString`，不接受前端命令行、不经过命令解释器，也不直接删除安装目录。
- 清理与卸载结果追加到本机 `%LOCALAPPDATA%/Qingpan/audit.jsonl`，实际释放空间和隔离占用分别统计；损坏的历史行会跳过，不影响后续操作。
- 管理员会话有效期 8 小时，设备会话有效期 7 天；客户端会定期重新校验。
- `SERVER_SECRET`、`LICENSE_PEPPER` 和 `ADMIN_PASSWORD` 必须通过环境变量注入，不得提交到仓库。

## 验证

```bash
pnpm build
pnpm test
pnpm tauri:build
cd desktop/src-tauri && cargo test
```

`pnpm build` 只构建三个 workspace 的前端与服务端产物；`pnpm tauri:build` 会从根目录调用 desktop workspace 的 Tauri CLI，生成当前目标平台的安装包。Windows 的 MSI/NSIS 包应在 Windows x64 或 ARM64 环境及对应 CI 任务中执行。

Windows 安装包由 CI 分别构建 `x86_64-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`。当前 Windows 构建通过 Tauri 命令读取真实磁盘、严格白名单缓存、已安装应用、目录占用、大文件、重复文件和本地操作历史；长时间只读扫描支持协作式取消和资源上限。样例数据仅在浏览器预览模式展示，原生构建不会把样例结果伪装成真实扫描。恢复中心目前只展示真实审计记录；由于产品尚未开放用户文件删除，原生构建不会制造没有真实隔离内容的“可恢复”记录。
