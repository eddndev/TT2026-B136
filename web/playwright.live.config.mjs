import { defineConfig } from '@playwright/test';
import { currentLivePlan } from '../scripts/web-live-plan.mjs';

const port = Number(process.env.TT_WEB_PORT || 4322);
if (!process.env.TT_WEB_FIXTURES || !process.env.API_PROXY_TARGET) {
  throw new Error('Run bash scripts/web-demo.sh from the repository root.');
}

export default defineConfig({
  testDir: './tests/live',
  testMatch: currentLivePlan().files.map((file) => `**/${file}`),
  testIgnore:
    process.env.TT_DEADLINE_REEVALUATION_ACCEPTANCE === '1'
      ? []
      : ['**/deadline-reevaluation.spec.mjs'],
  outputDir: './test-results-live',
  workers: 1,
  timeout: 60000,
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true },
  webServer: {
    command: `npm run dev -- --port ${port} --ignore-lock`,
    env: {
      ASTRO_DEV_BACKGROUND: '1',
      TT_LIVE_PARTICIPANT_PRIVATE_KEY: '',
      TT_LIVE_OWNER_CERTIFICATE_PRIVATE_KEY: '',
    },
    timeout: 120000,
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
  },
});
