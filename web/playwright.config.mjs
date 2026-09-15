import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/browser',
  use: { baseURL: 'http://127.0.0.1:4321', headless: true },
  webServer: {
    command: 'npm run dev -- --port 4321',
    env: { ASTRO_DEV_BACKGROUND: '1' },
    timeout: 120000,
    url: 'http://127.0.0.1:4321',
    reuseExistingServer: !process.env.CI,
  },
});
