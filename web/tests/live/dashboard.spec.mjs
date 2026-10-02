import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';

const accounts = fixture.dashboard;
if (!accounts) throw new Error('Dashboard fixtures must be provisioned by web-demo.sh');
const panel = (page) => page.getByRole('region', { name: 'Indicadores operativos', exact: true });
const metric = (page, key) => panel(page).locator(`[data-metric="${key}"] strong`);
const responseTo = (page, path, method = 'GET') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `/api/v1${path}` &&
      response.request().method() === method,
  );

async function enter(page, account) {
  await page.goto('/');
  const session = responseTo(page, '/auth/mfa/recovery', 'POST');
  const summary = responseTo(page, '/dashboard');
  await loginAs(page, account, 0);
  const response = await summary;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  await expect(panel(page)).toHaveAttribute('aria-busy', 'false');
  return { session: await (await session).json(), summary: await response.json() };
}
async function capture(page, testInfo, width) {
  await page.setViewportSize({ width, height: 1000 });
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({
    path: testInfo.outputPath(`dashboard-real-${width}.png`),
    fullPage: true,
  });
}

test('real dashboard separates office totals from assignments and removes revoked case data', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  const context = await browser.newContext({
    baseURL: testInfo.project.use.baseURL,
    viewport: { width: 390, height: 1000 },
  });
  try {
    const owner = await enter(page, accounts.owner);
    expect(owner.summary.scope).toBe('office');
    expect(owner.summary.active_cases).toBeGreaterThanOrEqual(2);
    expect(owner.summary.pending_contracts).toBeGreaterThanOrEqual(2);
    expect(owner.summary.workload.find((row) => row.user_id === accounts.litigator.id)).toEqual({
      user_id: accounts.litigator.id,
      email: accounts.litigator.email,
      active_cases: 1,
    });
    await expect(metric(page, 'active_cases')).toHaveText(String(owner.summary.active_cases));
    await expect(metric(page, 'pending_contracts')).toHaveText(
      String(owner.summary.pending_contracts),
    );
    await expect(panel(page)).toContainText('Todo el despacho');
    await capture(page, testInfo, 1440);
    await capture(page, testInfo, 390);

    const assignedPage = await context.newPage();
    assignedPage.on('pageerror', (error) => errors.push(error.message));
    const assigned = await enter(assignedPage, accounts.litigator);
    expect(assigned.summary.scope).toBe('assigned_cases');
    expect(assigned.summary.active_cases).toBe(1);
    expect(assigned.summary.pending_contracts).toBe(1);
    expect(assigned.summary.workload).toEqual([
      {
        user_id: accounts.litigator.id,
        email: accounts.litigator.email,
        active_cases: 1,
      },
    ]);
    for (const key of [
      'deadlines_overdue',
      'deadlines_due_48h',
      'deadlines_due_7d',
      'deadlines_unresolved',
    ])
      expect(assigned.summary[key]).toBe(0);
    await expect(panel(assignedPage)).toContainText('Tus expedientes asignados');
    await expect(metric(assignedPage, 'active_cases')).toHaveText('1');
    await expect(metric(assignedPage, 'pending_contracts')).toHaveText('1');

    const removed = await page.request.delete(
      `${process.env.API_PROXY_TARGET}/api/v1/cases/${accounts.visibleCase.id}/members/${accounts.litigator.id}`,
      {
        headers: { Authorization: `Bearer ${owner.session.access_token}` },
      },
    );
    expect(removed.status()).toBe(204);
    const refreshing = responseTo(assignedPage, '/dashboard');
    await panel(assignedPage)
      .getByRole('button', { name: 'Actualizar indicadores', exact: true })
      .click();
    const refreshed = await refreshing;
    expect(refreshed.status()).toBe(200);
    const empty = await refreshed.json();
    expect(empty.scope).toBe('assigned_cases');
    expect(empty.workload).toEqual([]);
    for (const key of [
      'active_cases',
      'pending_contracts',
      'deadlines_overdue',
      'deadlines_due_48h',
      'deadlines_due_7d',
      'deadlines_unresolved',
    ]) {
      expect(empty[key]).toBe(0);
      await expect(metric(assignedPage, key)).toHaveText('0');
    }
    await expect(panel(assignedPage)).not.toContainText(accounts.litigator.email);
    expect(errors).toEqual([]);
  } finally {
    await context.close();
  }
});
