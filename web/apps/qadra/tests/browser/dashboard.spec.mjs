import { test, expect } from '@playwright/test';
import { navigate } from './helpers.mjs';
import {
  dashboardValue,
  dashboard,
  metric,
  setupDashboard,
  enterDashboard,
} from './dashboard-helpers.mjs';

for (const width of [1440, 390]) {
  test(`dashboard shows complete authorized counts and workload at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupDashboard(page);
    await enterDashboard(page);
    await expect(dashboard(page)).toHaveAttribute('aria-busy', 'false');
    await expect(dashboard(page)).toContainText('Todo el despacho');
    for (const [name, value] of Object.entries({
      active_cases: 9,
      pending_contracts: 4,
      deadlines_overdue: 2,
      deadlines_due_48h: 3,
      deadlines_due_7d: 7,
      deadlines_unresolved: 1,
    }))
      await expect(metric(page, name).locator('strong')).toHaveText(String(value));
    await expect(metric(page, 'deadlines_due_48h')).toHaveClass(/dashboard-urgent/);
    await expect(metric(page, 'deadlines_due_7d')).toContainText('Incluye los de 48 horas');
    await expect(dashboard(page)).toContainText('lawyer@example.test');
    await expect(dashboard(page).locator('time')).toHaveAttribute(
      'datetime',
      state.value.checked_at,
    );
    expect(state.calls).toBe(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath(`dashboard-${width}.png`), fullPage: true });
  });
}

test('litigator sees assigned scope and rejects an office response', async ({ page }) => {
  const state = await setupDashboard(page, 'litigator');
  await enterDashboard(page);
  await expect(dashboard(page)).toContainText('Tus expedientes asignados');
  state.value = dashboardValue();
  await dashboard(page)
    .getByRole('button', { name: 'Actualizar indicadores', exact: true })
    .click();
  await expect(dashboard(page).getByRole('alert')).toBeVisible();
  await expect(dashboard(page).locator('[data-metric]')).toHaveCount(0);
});

test('loading and failed refresh never present zero or retain stale counts', async ({ page }) => {
  const state = await setupDashboard(page);
  let release;
  state.handle = (route) =>
    new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: state.value });
        resolve();
      };
    });
  await enterDashboard(page);
  await expect(dashboard(page)).toHaveAttribute('aria-busy', 'true');
  await expect(dashboard(page).locator('[data-metric]')).toHaveCount(0);
  await release();
  await expect(metric(page, 'active_cases').locator('strong')).toHaveText('9');
  state.handle = (route) =>
    route.fulfill({ status: 503, json: { error: { code: 'unavailable' } } });
  await dashboard(page)
    .getByRole('button', { name: 'Actualizar indicadores', exact: true })
    .click();
  await expect(dashboard(page).getByRole('alert')).toBeVisible();
  await expect(dashboard(page).locator('[data-metric]')).toHaveCount(0);
  state.handle = null;
  state.value = dashboardValue({
    active_cases: 0,
    pending_contracts: 0,
    deadlines_overdue: 0,
    deadlines_due_48h: 0,
    deadlines_due_7d: 0,
    deadlines_unresolved: 0,
    workload: [],
  });
  await dashboard(page)
    .getByRole('button', { name: 'Actualizar indicadores', exact: true })
    .click();
  await expect(metric(page, 'active_cases').locator('strong')).toHaveText('0');
  await expect(dashboard(page)).toContainText('Sin expedientes activos');
  await expect(dashboard(page).getByRole('alert')).toHaveCount(0);
});

test('returning to dashboard discards the departed request and fetches current counts', async ({
  page,
}) => {
  const state = await setupDashboard(page);
  let release;
  state.handle = (route) =>
    new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: dashboardValue() });
        resolve();
      };
    });
  await enterDashboard(page);
  await expect(dashboard(page)).toHaveAttribute('aria-busy', 'true');
  await navigate(page, 'Expedientes');
  state.handle = null;
  state.value = dashboardValue({ active_cases: 12 });
  await navigate(page, 'Inicio');
  await expect(metric(page, 'active_cases').locator('strong')).toHaveText('12');
  await release();
  await expect(metric(page, 'active_cases').locator('strong')).toHaveText('12');
  expect(state.calls).toBe(2);
});

for (const role of ['paralegal', 'client']) {
  test(`dashboard makes no aggregate request for ${role}`, async ({ page }) => {
    const state = await setupDashboard(page, role);
    await enterDashboard(page);
    await expect(dashboard(page)).toHaveCount(0);
    expect(state.calls).toBe(0);
  });
}

test('session revocation removes all dashboard metrics', async ({ page }) => {
  const state = await setupDashboard(page);
  await enterDashboard(page);
  await expect(metric(page, 'active_cases').locator('strong')).toHaveText('9');
  state.handle = (route) =>
    route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
  await dashboard(page)
    .getByRole('button', { name: 'Actualizar indicadores', exact: true })
    .click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(dashboard(page)).toHaveCount(0);
});
