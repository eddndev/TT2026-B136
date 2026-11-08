import { test, expect } from '@playwright/test';
import { setup, login, navigate } from './helpers.mjs';

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

async function openFirstCase(page) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}

test('desktop case sections are links that mark the current one', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  await openFirstCase(page);
  const nav = page.getByRole('navigation', { name: 'Secciones del expediente', exact: true });
  await expect(nav.getByRole('link', { name: 'Resumen', exact: true })).toHaveAttribute(
    'aria-current',
    'page',
  );
  await expect(page.getByRole('combobox', { name: 'Secciones del expediente' })).toBeHidden();
  await nav.getByRole('link', { name: 'Documentos', exact: true }).click();
  await expect(nav.getByRole('link', { name: 'Documentos', exact: true })).toHaveAttribute(
    'aria-current',
    'page',
  );
  await expect(nav.getByRole('link', { name: 'Resumen', exact: true })).not.toHaveAttribute(
    'aria-current',
  );
});

test('on a phone the case sections become a select that switches the view', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await login(page, false, false);
  await openFirstCase(page);
  await expect(page.getByRole('link', { name: 'Documentos', exact: true })).toBeHidden();
  const sections = page.getByRole('combobox', { name: 'Secciones del expediente' });
  await expect(sections).toHaveValue('case-summary');
  await sections.selectOption('documents');
  await expect(page).toHaveURL(/#documents$/);
  await expect(sections).toHaveValue('documents');
});

test('the logbook reuses the same section control on a phone', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await login(page, false, false);
  await navigate(page, 'Bit\u00e1cora');
  const sections = page.getByRole('combobox', { name: /^Secciones de la bit/ });
  await expect(sections).toHaveValue('audit');
  await sections.selectOption('integrity-incidents');
  await expect(page).toHaveURL(/#integrity-incidents$/);
  await expect(sections).toHaveValue('integrity-incidents');
});
