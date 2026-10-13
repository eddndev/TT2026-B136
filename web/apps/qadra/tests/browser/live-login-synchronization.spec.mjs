import { test, expect } from '@playwright/test';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { setup } from './helpers.mjs';

test('live login waits for recovery verification before checking the authenticated page', async ({
  page,
}) => {
  const directory = mkdtempSync(join(tmpdir(), 'tt-live-login-'));
  const previousFixture = process.env.TT_WEB_FIXTURES;
  const account = { email: 'hatz@example.com', password: 'synthetic test password' };
  let login;
  try {
    process.env.TT_WEB_FIXTURES = join(directory, 'fixture.json');
    writeFileSync(process.env.TT_WEB_FIXTURES, JSON.stringify(account));
    ({ login } = await import('../live/helpers.mjs'));
  } finally {
    if (previousFixture === undefined) delete process.env.TT_WEB_FIXTURES;
    else process.env.TT_WEB_FIXTURES = previousFixture;
    rmSync(directory, { recursive: true, force: true });
  }
  await setup(page);
  let attempts = 0;
  await page.route('**/api/v1/auth/mfa/recovery', async (route) => {
    attempts += 1;
    // Model verification that outlasts the default UI assertion budget.
    await delay(6000);
    await route.fallback();
  });
  await login(page, 'synthetic-recovery-code', account);
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  expect(attempts).toBe(1);
});
