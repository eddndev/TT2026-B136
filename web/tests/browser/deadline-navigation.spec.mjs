import { test, expect } from '@playwright/test';
import { setupFacts } from './procedural-facts-helpers.mjs';
import { factCaseId } from '../fixtures/procedural-facts.mjs';
import { login, navigate } from './helpers.mjs';

test.setTimeout(15000);
for (const role of ['owner', 'litigator', 'paralegal'])
  test(`deadline collection has an authorized empty state for ${role}`, async ({ page }) => {
    await setupFacts(page, { role });
    const calls = [];
    await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
      calls.push(route.request().method());
      await route.fulfill({
        json: { case_id: factCaseId, deadlines: [], has_more: false, next_after_id: null },
      });
    });
    await login(page, false, false);
    await navigate(page, 'Expedientes');
    await page.getByRole('button', { name: /Defensa inicial/ }).click();
    await page.getByRole('link', { name: 'Plazos', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Plazos del expediente', exact: true }),
    ).toBeVisible();
    await expect(page.getByText('No hay plazos en esta consulta.', { exact: true })).toBeVisible();
    const register = page.getByRole('button', { name: 'Registrar plazo', exact: true });
    if (role === 'paralegal') await expect(register).toHaveCount(0);
    else await expect(register).toBeEnabled();
    expect(calls).toEqual(['GET']);
  });

test('Client cannot open deadlines by navigation or forced hash and sends no private request', async ({
  page,
}) => {
  await setupFacts(page, { role: 'client' });
  const calls = [];
  await page.route('**/api/v1/cases/*/deadlines**', (route) => {
    calls.push(route.request().url());
    return route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Plazos', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'deadlines';
  });
  await expect(page.getByRole('button', { name: 'Inicio', exact: true }).first()).toHaveAttribute(
    'aria-current',
    'page',
  );
  expect(calls).toHaveLength(0);
});
