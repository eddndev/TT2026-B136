import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

const proxy = { '/api': { target: process.env.API_PROXY_TARGET || 'http://127.0.0.1:3000' } };

export default defineConfig({
  integrations: [svelte()],
  vite: { server: { proxy }, preview: { proxy } },
});
