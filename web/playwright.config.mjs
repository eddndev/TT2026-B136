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
export default defineConfig({
  testDir: './tests/browser',
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true },
  webServer: {
    command: `npm run dev -- --port ${port}`,
    env: { ASTRO_DEV_BACKGROUND: '1' },
    timeout: 120000,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
  },
});
