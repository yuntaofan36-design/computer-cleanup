import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const serverPort = Number(process.env.SERVER_DEV_PORT || 8787);

export default defineConfig({
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: Number(process.env.WEB_DEV_PORT || 1421),
    strictPort: Boolean(process.env.WEB_DEV_PORT),
    proxy: { '/api': `http://127.0.0.1:${serverPort}` },
  },
});
