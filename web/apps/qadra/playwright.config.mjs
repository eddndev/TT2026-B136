import { createServer } from 'node:net';
import { defineConfig } from '@playwright/test';

async function unusedPort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

const port = process.env.TT_WEB_MOCK_PORT
  ? Number(process.env.TT_WEB_MOCK_PORT)
  : await unusedPort();
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error('Invalid mock browser port');
process.env.TT_WEB_MOCK_PORT = String(port);
// CI serves the build it already made; see docs/adr/0088-ci-time-budget.md.
const preview = process.env.TT_WEB_MOCK_SERVER === 'preview';
export default defineConfig({
  testDir: './tests/browser',
  // Module diagnostics name dev-server sources; CI runs them in a dev step.
  testIgnore: preview ? ['**/frontend-entry-diagnostics.spec.mjs'] : [],
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true },
  webServer: {
    command: preview
      ? `npm run preview -- --port ${port} --ignore-lock`
      : `npm run dev -- --port ${port} --ignore-lock`,
    // Both flags keep Astro in the foreground under an agent-detected shell.
    env: { ASTRO_DEV_BACKGROUND: '1', ASTRO_PREVIEW_BACKGROUND: '1' },
    timeout: 120000,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
  },
});
