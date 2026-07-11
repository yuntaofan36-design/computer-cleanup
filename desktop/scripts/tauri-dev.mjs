import net from 'node:net';
import { spawn } from 'node:child_process';

function availablePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

const port = await availablePort();
if (!port) throw new Error('无法获取可用端口');

const config = JSON.stringify({
  build: { devUrl: `http://127.0.0.1:${port}` },
});

console.log(`清盘开发服务器将使用端口 ${port}`);

const child = spawn(
  'pnpm',
  ['tauri', 'dev', '--config', config],
  {
    cwd: new URL('..', import.meta.url),
    stdio: 'inherit',
    env: { ...process.env, VITE_DEV_PORT: String(port) },
  },
);

child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});
