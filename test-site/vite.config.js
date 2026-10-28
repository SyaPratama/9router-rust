import { defineConfig } from 'vite';

const target = new URL(process.env.ROUTER_URL ?? 'http://127.0.0.1:20129');
if (!['127.0.0.1', 'localhost', '[::1]'].includes(target.hostname)) {
  throw new Error('ROUTER_URL must point to a loopback host.');
}

export default defineConfig({
  server: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
    proxy: { '/api': { target: target.origin, changeOrigin: true } },
  },
});
