import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
export { accountAction } from './hearing-result-helpers.mjs';
export const accounts = fixture.members;
if (!accounts) throw new Error('Member lifecycle fixtures must be provisioned by web-demo.sh');
export const directory = (page) =>
  page.getByRole('region', { name: 'Directorio de cuentas', exact: true });
export const editor = (page) => page.getByRole('region', { name: 'Acceso de cuenta', exact: true });
export const assignments = (page) =>
  page.getByRole('region', { name: 'Asignaciones', exact: true });
export const card = (page, id) => directory(page).locator(`[data-user-id="${id}"]`);
export const responseTo = (page, path, method = 'GET') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `/api/v1${path}` &&
      response.request().method() === method,
  );

export async function findAccount(page, account, status = 'all') {
  await navigate(page, 'Equipo');
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
  await directory(page).getByLabel('Buscar por correo', { exact: true }).fill(account.email);
  await directory(page)
    .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
    .selectOption(status);
  const pending = responseTo(page, '/users');
  await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  const row = (await response.json()).items.find((item) => item.id === account.id);
  expect(row).toBeTruthy();
  await expect(card(page, account.id)).toBeVisible();
  return row;
}

export async function changeAccess(page, account, role, active) {
  const reading = responseTo(page, `/users/${account.id}`);
  await directory(page)
    .getByRole('button', { name: `Administrar acceso de ${account.email}`, exact: true })
    .click();
  const read = await reading;
  expect(read.status()).toBe(200);
  const current = await read.json();
  await expect(editor(page)).toHaveAttribute('aria-busy', 'false');
  await editor(page)
    .getByRole('combobox', { name: 'Rol de la cuenta', exact: true })
    .selectOption(role);
  await editor(page)
    .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
    .selectOption(active ? 'active' : 'inactive');
  await editor(page).getByRole('button', { name: 'Revisar acceso', exact: true }).click();
  const saved = responseTo(page, `/users/${account.id}/access`, 'PUT');
  await editor(page)
    .getByRole('button', { name: 'Confirmar cambio de acceso', exact: true })
    .click();
  const response = await saved;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  expect(response.request().postDataJSON()).toEqual({
    expected_revision: current.revision,
    role,
    active,
  });
  const result = await response.json();
  expect(result).toEqual({
    ...current,
    role,
    active,
    revision: String(BigInt(current.revision) + 1n),
  });
  return result;
}

export async function openCase(page, record, staff = true) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  if (staff) {
    await page.getByLabel('Buscar por t\u00edtulo', { exact: true }).fill(record.title);
    await page
      .getByRole('combobox', { name: 'Estado administrativo', exact: true })
      .selectOption('all');
    const pending = responseTo(page, '/case-administrations');
    await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
    expect((await pending).status()).toBe(200);
    await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  }
  await page.getByRole('button', { name: new RegExp(record.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}

export async function searchAssignments(page, email, selection = 'assigned') {
  await assignments(page)
    .getByRole('combobox', { name: 'Selecci\u00f3n de cuentas', exact: true })
    .selectOption(selection);
  await assignments(page).getByLabel('Buscar por correo', { exact: true }).fill(email);
  const pending = page.waitForResponse(
    (response) =>
      /^\/api\/v1\/cases\/[^/]+\/members$/.test(new URL(response.url()).pathname) &&
      response.request().method() === 'GET',
  );
  await assignments(page).getByRole('button', { name: 'Buscar asignaciones', exact: true }).click();
  expect((await pending).status()).toBe(200);
  await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
}

export async function capture(page, testInfo, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}
