import { test, expect } from '@playwright/test';
import { login, setup } from './helpers.mjs';
import { dashboard, metric, setupDashboard, enterDashboard } from './dashboard-helpers.mjs';

const heading = (page) => page.getByRole('region', { name: 'Tu cuenta y tus pendientes' });

for (const width of [1440, 390]) {
  test(`the overview heading greets the account and leads with three counts at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    await setupDashboard(page);
    await enterDashboard(page);
    const card = heading(page);
    await expect(card).toContainText('Hatziel Ortega');
    await expect(card).not.toContainText('hatz@example.com');
    await expect(card).toContainText('Administrador');
    await expect(card).toContainText('Todo el despacho');
    // The leading counts live in the heading and are not repeated in the grid.
    for (const [key, value] of [
      ['active_cases', '9'],
      ['deadlines_overdue', '2'],
      ['deadlines_due_48h', '3'],
    ]) {
      await expect(card.locator(`[data-metric="${key}"] strong`)).toHaveText(value);
      await expect(metric(page, key)).toHaveCount(1);
    }
    await expect(dashboard(page).locator('[data-metric]')).toHaveCount(6);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await card.getByRole('button', { name: 'Ver expedientes', exact: true }).click();
    await expect(page).toHaveURL(/#cases$/);
  });
}

test('a paralegal sees the greeting without office counts', async ({ page }) => {
  await setup(page, 'paralegal');
  await login(page, false, false);
  const card = heading(page);
  await expect(card).toContainText('Hatziel Ortega');
  await expect(card).not.toContainText('hatz@example.com');
  await expect(card).toContainText('Asistente legal');
  await expect(card.locator('[data-metric]')).toHaveCount(0);
});

for (const width of [1440, 390]) {
  test(`the account menu leads with the name and keeps the email secondary at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    await setup(page, 'litigator');
    await login(page, false, false);
    const trigger = page.getByRole('button', { name: 'Tu cuenta', exact: true });
    await expect(trigger).toContainText('HO');
    if (width >= 1024) await expect(trigger).toContainText('Hatziel Ortega');
    await expect(trigger).not.toContainText('hatz@example.com');
    await trigger.click();
    const panel = page.locator('#shell-account');
    const lines = panel.locator('p');
    await expect(lines).toHaveText(['Hatziel Ortega', 'hatz@example.com', 'Litigante']);
    await expect(lines.nth(0)).toHaveAttribute('title', 'Hatziel Ortega');
    await expect(lines.nth(1)).toHaveClass(/text-xs/);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}
