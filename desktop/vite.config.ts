import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: Number(process.env.VITE_DEV_PORT || 5173),
    strictPort: Boolean(process.env.VITE_DEV_PORT),
    host: '127.0.0.1',
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: { target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13' },
});
