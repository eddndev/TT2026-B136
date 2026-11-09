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
    // A row shows the display name and falls back to the email without one.
    state.value = dashboardValue({
      workload: [
        { ...state.value.workload[0], name: 'Laura Mendoza' },
        {
          user_id: '22222222-2222-4222-8222-222222222222',
          email: 'second@example.test',
          active_cases: 3,
        },
      ],
    });
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
    await expect(metric(page, 'pending_contracts')).toContainText('Contratos en firma');
    await expect(metric(page, 'deadlines_overdue')).toHaveClass(/dashboard-urgent/);
    await expect(metric(page, 'deadlines_overdue').locator('strong')).toHaveClass(/text-danger/);
    await expect(metric(page, 'deadlines_due_48h')).toHaveClass(/dashboard-urgent/);
    await expect(metric(page, 'deadlines_due_48h')).toHaveText('3 en menos de 48 horas');
    await expect(dashboard(page)).not.toContainText('Indicadores operativos');
    const workload = dashboard(page).getByRole('region', { name: 'Carga por litigante' });
    await expect(workload.getByRole('listitem')).toHaveText([
      /^LM\s*Laura Mendoza\s*6 expedientes\s*$/,
      /^SE\s*second@example\.test\s*3 expedientes\s*$/,
    ]);
    await expect(workload).not.toContainText('lawyer@example.test');
    const cutoff = dashboard(page).locator('p', { hasText: 'Consulta del' });
    await expect(cutoff.locator('time')).toHaveAttribute('datetime', state.value.checked_at);
    await expect(cutoff).not.toContainText('..');
    // Phones show the figures two per row.
    const top = (name) =>
      metric(page, name).evaluate((item) => Math.round(item.getBoundingClientRect().top));
    if (width < 640) expect(await top('active_cases')).toBe(await top('deadlines_overdue'));
    expect(state.calls).toBe(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath(`dashboard-${width}.png`), fullPage: true });
    // A narrow desktop beside the sidebar keeps every column inside the page.
    if (width < 640) return;
    await page.setViewportSize({ width: 1024, height: 1000 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
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

for (const role of ['paralegal', 'client']) {
  test(`dashboard makes no aggregate request for ${role}`, async ({ page }) => {
    const state = await setupDashboard(page, role);
    await enterDashboard(page);
    await expect(dashboard(page)).toHaveCount(0);
    expect(state.calls).toBe(0);
  });
}
