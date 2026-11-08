import { test, expect } from '@playwright/test';
import { login, setup } from './helpers.mjs';
import { dashboard, setupDashboard, enterDashboard } from './dashboard-helpers.mjs';

const heading = (page) => page.getByRole('region', { name: 'Tu cuenta y tus pendientes' });

for (const width of [1440, 390]) {
  test(`the overview heading names the day and holds every count at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.clock.install({ time: new Date('2026-11-08T18:00:00Z') });
    await setupDashboard(page);
    await enterDashboard(page);
    const card = heading(page);
    await expect(card.locator('p').first()).toHaveText('Domingo, 8 de noviembre');
    await expect(card).toContainText('Hatziel Ortega \u00b7 Administrador \u00b7 Todo el despacho');
    await expect(card).not.toContainText('hatz@example.com');
    await expect(card).not.toContainText('Hola de nuevo');
    for (const [key, value] of [
      ['active_cases', '9'],
      ['deadlines_overdue', '2'],
      ['deadlines_due_7d', '7'],
      ['deadlines_due_48h', '3'],
      ['deadlines_unresolved', '1'],
      ['pending_contracts', '4'],
    ])
      await expect(card.locator(`[data-metric="${key}"] strong`)).toHaveText(value);
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
  await expect(page.getByRole('region', { name: 'Accesos', exact: true })).toContainText(
    'Pendientes de sello',
  );
  await expect(page.getByRole('heading', { name: 'Tu siguiente paso' })).toBeVisible();
  await expect(page.getByText('Tu trabajo, en un solo lugar')).toHaveCount(0);
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
    await expect(lines.nth(1)).toHaveClass(/text-caption/);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}
