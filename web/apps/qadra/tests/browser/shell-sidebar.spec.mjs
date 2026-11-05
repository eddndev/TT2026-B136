import { test, expect } from '@playwright/test';
import { setup, login, selectCase } from './helpers.mjs';

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

test('the desktop sidebar collapses to icons and keeps keyboard focus on its toggle', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  const sidebar = page.locator('.desktop-sidebar');
  const nav = sidebar.getByRole('navigation');
  const collapse = sidebar.getByRole('button', { name: 'Contraer men\u00fa', exact: true });
  await expect(collapse).toHaveAttribute('aria-expanded', 'true');
  const wide = (await sidebar.boundingBox()).width;
  await collapse.focus();
  await page.keyboard.press('Enter');
  const expand = sidebar.getByRole('button', { name: 'Expandir men\u00fa', exact: true });
  await expect(expand).toBeFocused();
  await expect(expand).toHaveAttribute('aria-expanded', 'false');
  await expect.poll(async () => (await sidebar.boundingBox()).width).toBeLessThan(wide / 2);
  const home = nav.getByRole('button', { name: 'Inicio', exact: true });
  await expect(home).toBeVisible();
  await expect(home).toHaveAttribute('title', 'Inicio');
  // The label stays in the accessible name but is visually hidden.
  await expect(nav.getByText('Inicio', { exact: true })).toHaveClass(/sr-only/);
  const main = page.locator('#main-content');
  expect((await main.boundingBox()).x).toBeLessThan(wide);
  await page.keyboard.press('Enter');
  await expect(collapse).toBeFocused();
  await expect(nav.getByText('Inicio', { exact: true })).not.toHaveClass(/sr-only/);
});

test('a recent case is a link named by its visible title in both sidebar states', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  await selectCase(page);
  const sidebar = page.locator('.desktop-sidebar');
  const recent = sidebar.getByRole('link', { name: 'Defensa inicial', exact: true });
  await expect(recent).toBeVisible();
  await sidebar.getByRole('button', { name: 'Contraer men\u00fa', exact: true }).click();
  await expect(recent).toBeVisible();
  await expect(sidebar.getByText('Defensa inicial', { exact: true })).toBeHidden();
  await recent.click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
});
