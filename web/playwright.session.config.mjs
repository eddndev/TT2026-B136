import { defineConfig } from '@playwright/test';
const port = Number(process.env.TT_WEB_PORT);
if (!port || !process.env.TT_WEB_FIXTURES || !process.env.API_PROXY_TARGET)
  throw new Error('An isolated session stack is required.');
export default defineConfig({
  testDir: './tests/session-live',
  testMatch: 'reentry.spec.mjs',
  workers: 1,
  timeout: 90000,
  outputDir: process.env.TT_IDLE_BROWSER_OUTPUT || './test-results-session',
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true, actionTimeout: 5000 },
  webServer: {
    command: `npm run dev -- --port ${port} --ignore-lock`,
    env: { ASTRO_DEV_BACKGROUND: '1', TT_LIVE_PARTICIPANT_PRIVATE_KEY: '' },
    timeout: 120000,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
  },
});
