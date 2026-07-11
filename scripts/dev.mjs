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

const ports = [];
while (ports.length < 3) {
  const port = await availablePort();
  if (!ports.includes(port)) ports.push(port);
}
const [desktopPort, webPort, serverPort] = ports;

if (!desktopPort || !webPort || !serverPort) {
  throw new Error('无法为三端分配可用端口');
}

console.log('清盘开发环境');
console.log(`  桌面前端: http://127.0.0.1:${desktopPort}`);
console.log(`  管理后台: http://127.0.0.1:${webPort}`);
console.log(`  授权服务: http://127.0.0.1:${serverPort}`);

const child = spawn(
  'pnpm',
  ['--parallel', '--filter', 'qingpan', '--filter', 'qingpan-web', '--filter', 'qingpan-server', 'run', 'dev'],
  {
    stdio: 'inherit',
    env: {
      ...process.env,
      VITE_DEV_PORT: String(desktopPort),
      WEB_DEV_PORT: String(webPort),
      SERVER_DEV_PORT: String(serverPort),
      PORT: String(serverPort),
      VITE_LICENSE_API: `http://127.0.0.1:${serverPort}`,
      WEB_ORIGIN: `http://127.0.0.1:${webPort}`,
    },
  },
);

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => child.kill(signal));
}

child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});
