import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const targets = {
  x64: 'x86_64-pc-windows-msvc',
  arm64: 'aarch64-pc-windows-msvc',
};

const arch = process.argv[2];
const target = targets[arch];
if (!target) {
  console.error('用法: node scripts/cross-windows.mjs <x64|arm64>');
  process.exit(2);
}

if (process.platform !== 'darwin') {
  console.warn('该命令面向 macOS + cargo-xwin；Windows 请使用 pnpm tauri:build。');
}

function run(command, args, env = process.env) {
  const result = spawnSync(command, args, { stdio: 'inherit', env });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

console.log(`正在构建 Windows ${arch} 前端资源...`);
run('pnpm', ['--filter', 'qingpan', 'build']);

console.log(`正在通过 cargo-xwin 交叉编译 ${target}...`);
run('cargo', [
  'xwin', 'build',
  '--manifest-path', 'desktop/src-tauri/Cargo.toml',
  '--release',
  '--target', target,
]);

const executable = resolve(`desktop/src-tauri/target/${target}/release/qingpan.exe`);
console.log(`Windows 可执行文件已生成: ${executable}`);
console.log('MSI/NSIS 安装器请使用 Windows CI 或 Windows 构建机生成。');
