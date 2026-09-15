import { expect } from '@playwright/test';

export async function navigate(page, name) {
  const menu = page.getByRole('button', { name: 'Abrir men\u00fa', exact: true });
  if (await menu.isVisible()) await menu.click();
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name, exact: true })
    .click();
}
export async function openCase(page, record) {
  await navigate(page, 'Expedientes');
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await expect(page.getByRole('alert')).toHaveCount(0);
  const response = page.waitForResponse((result) => {
    const url = new URL(result.url());
    return (
      url.pathname === '/api/v1/case-administrations' && url.searchParams.get('status') === 'all'
    );
  });
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  expect((await response).status()).toBe(200);
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(record.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}
export async function saveAdministration(page, record, button, revision) {
  const response = page.waitForResponse(
    (result) =>
      result.url().endsWith(`/api/v1/cases/${record.id}/administration`) &&
      result.request().method() === 'PUT',
  );
  await page.getByRole('button', { name: button, exact: true }).click();
  const result = await response;
  expect(result.status()).toBe(200);
  const saved = await result.json();
  expect(saved.id).toBe(record.id);
  expect(saved.administration.revision).toBe(revision);
  await expect(page.locator('.case-editor')).toHaveCount(0);
  await expect(page.locator('.case-summary')).toContainText(`Revisi\u00f3n ${revision} /`);
  return saved;
}
