# 清盘套件

Windows 磁盘清理客户端及其授权管理系统。项目按运行边界拆分为三个独立目录：

- `desktop/`：Tauri 2 + React 客户端，包含安全扫描、清理、空间分析、应用与启动项管理和卡密激活。
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
- 客户端清理命令只接受最近一次扫描产生的条目 ID，并在执行前重新校验规范路径。
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

Windows 安装包由 CI 分别构建 `x86_64-pc-windows-msvc` 和 `aarch64-pc-windows-msvc`。空间分析与管理界面的样例数据用于非 Windows 预览；Windows 构建通过 Tauri 命令读取真实磁盘、缓存目录和注册表。
