import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

const proxy = {
  '/api': { target: process.env.CA_API_PROXY_TARGET || 'http://127.0.0.1:18089' },
};

export default defineConfig({
  server: { port: 18088 },
  publicDir: '../../packages/ui/public',
  integrations: [svelte()],
  devToolbar: { enabled: false },
  vite: { server: { proxy }, preview: { proxy } },
});
