import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
export { accountAction } from './hearing-result-helpers.mjs';
export const accounts = fixture.proceduralResources;
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de recurso', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle de recurso', exact: true });
export const collection = (page) =>
  page.getByRole('region', { name: 'Recursos registrados', exact: true });
export const responseTo = (page, path, method = 'GET') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === path && response.request().method() === method,
  );
export async function openCase(page, record) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByLabel('Buscar por t\u00edtulo', { exact: true }).fill(record.title);
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  const pending = responseTo(page, '/api/v1/case-administrations');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  expect((await pending).status()).toBe(200);
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(record.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}
export async function openResource(page, scenario) {
  await openCase(page, scenario.case);
  await page.getByRole('link', { name: 'Recursos', exact: true }).click();
  await expect(collection(page)).toHaveAttribute('aria-busy', 'false');
  const pending = responseTo(
    page,
    `/api/v1/cases/${scenario.case.id}/procedural-resources/${scenario.resource.id}`,
  );
  await collection(page)
    .getByRole('button', {
      name: `Consultar recurso ${scenario.resource.values.title}`,
      exact: true,
    })
    .click();
  const response = await pending;
  expect(response.status()).toBe(200);
  await expect(detail(page)).toBeVisible();
  return response.json();
}
export async function prepare(page, scenario, expected = 200) {
  const pending = responseTo(
    page,
    `/api/v1/cases/${scenario.case.id}/procedural-resources/prepare`,
    'POST',
  );
  await editor(page).getByRole('button', { name: 'Preparar registro', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(expected);
  expect(response.headers()['cache-control']).toBe('no-store');
  return response.json();
}
export async function submit(page, scenario, prepared) {
  const change = prepared.command.change;
  const path = `/api/v1/cases/${scenario.case.id}/procedural-resources/${prepared.command.resource_id}/acts${change.action === 'correct_act' ? `/${change.act_id}` : ''}`;
  const pending = responseTo(page, path, change.action === 'correct_act' ? 'PUT' : 'POST');
  await editor(page).getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(201);
  expect(response.headers()['cache-control']).toBe('no-store');
  await expect(editor(page)).toHaveCount(0);
  return response.json();
}
export async function exact(page, scenario, revision) {
  const button = detail(page).getByRole('button', {
    name: 'Ver historial de recurso',
    exact: true,
  });
  if (await button.isVisible()) await button.click();
  const history = page.getByRole('region', { name: 'Historial de recurso', exact: true });
  await expect(history).toHaveAttribute('aria-busy', 'false');
  const pending = responseTo(
    page,
    `/api/v1/cases/${scenario.case.id}/procedural-resources/${scenario.resource.id}/revisions/${revision}`,
  );
  await history
    .getByRole('button', { name: `Consultar recurso revision ${revision}`, exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(200);
  await expect(detail(page)).toContainText('Consultada exactamente');
  return response.json();
}
export async function chooseSupport(page, scenario) {
  const form = editor(page);
  await form.getByRole('button', { name: 'Elegir soporte: acto 1', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker
    .getByRole('button', {
      name: `${scenario.currentSupport.name} / versi\u00f3n actual 2`,
      exact: true,
    })
    .click();
  await picker
    .getByRole('button', {
      name: `Versi\u00f3n 1 / ${scenario.support.name} / Hist\u00f3rica`,
      exact: true,
    })
    .click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await form
    .getByLabel('Localizador documental: acto 1', { exact: true })
    .fill('Pagina 1 historica');
}
