import { defineConfig } from '@playwright/test';

const port = Number(process.env.TT_WEB_PORT || 4322);
if (!process.env.TT_WEB_FIXTURES || !process.env.API_PROXY_TARGET) {
  throw new Error('Run bash scripts/web-demo.sh from the repository root.');
}

export default defineConfig({
  testDir: './tests/live',
  outputDir: './test-results-live',
  workers: 1,
  timeout: 60000,
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true },
  webServer: {
    command: `npm run dev -- --port ${port}`,
    env: { ASTRO_DEV_BACKGROUND: '1' },
    timeout: 120000,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
  },
});
