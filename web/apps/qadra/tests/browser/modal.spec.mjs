import { test, expect } from '@playwright/test';
import { setup, login, selectCase } from './helpers.mjs';

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

async function openUpload(page, width) {
  await page.setViewportSize({ width, height: 900 });
  await setup(page);
  await login(page, false, false);
  await selectCase(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

test('a modal is centered on desktop and closes with Escape', async ({ page }) => {
  const modal = await openUpload(page, 1440);
  // Wait for the entry animation to settle before measuring.
  await expect(modal).toHaveCSS('opacity', '1');
  const box = await modal.boundingBox();
  expect(Math.abs(box.x + box.width / 2 - 720)).toBeLessThan(2);
  expect(Math.abs(box.y + box.height / 2 - 450)).toBeLessThan(2);
  expect(box.width).toBeLessThanOrEqual(576);
  const actions = modal.locator('.dialog-actions');
  await expect(actions.getByRole('button', { name: 'Cancelar', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(modal).toBeHidden();
});

test('a modal sits at the bottom of a phone screen and stacks its actions', async ({ page }) => {
  const modal = await openUpload(page, 390);
  await expect(modal).toHaveCSS('opacity', '1');
  const box = await modal.boundingBox();
  expect(Math.round(box.x)).toBe(16);
  expect(Math.round(box.width)).toBe(390 - 32);
  expect(Math.round(box.y + box.height)).toBe(900 - 16);
  const cancel = await modal.getByRole('button', { name: 'Cancelar', exact: true }).boundingBox();
  const submit = await modal
    .getByRole('button', { name: 'Cargar documento', exact: true })
    .boundingBox();
  // Stacked full width, main action on top.
  expect(Math.round(cancel.width)).toBe(Math.round(submit.width));
  expect(submit.y).toBeLessThan(cancel.y);
});
