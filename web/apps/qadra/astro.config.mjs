import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import { pinnedRootsSource } from './src/lib/evidence/pinned-roots-schema.mjs';

const proxy = { '/api': { target: process.env.API_PROXY_TARGET || 'http://127.0.0.1:3000' } };

// The evidence validator trusts only the roots pinned in this file, which a
// build for another environment replaces by naming an absolute path in
// QADRA_PINNED_ROOTS. See docs/adr/0085-in-browser-evidence-validation.md.
const pinnedRoots = pinnedRootsSource(
  process.env.QADRA_PINNED_ROOTS,
  fileURLToPath(new URL('./src/lib/evidence/pinned-roots.json', import.meta.url)),
  (file) => readFileSync(file, 'utf8'),
);

export default defineConfig({
  publicDir: '../../packages/ui/public',
  integrations: [svelte()],
  devToolbar: { enabled: false },
  vite: {
    resolve: { alias: { '@qadra/pinned-roots': pinnedRoots } },
    server: { proxy },
    preview: { proxy },
  },
});
