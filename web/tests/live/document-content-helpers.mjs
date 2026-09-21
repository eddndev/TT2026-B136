import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
export { accountAction } from './hearing-result-helpers.mjs';

export const accounts = fixture.documentContent;
if (!accounts) throw new Error('Document content fixtures must be provisioned by web-demo.sh');
export const inbox = (page) =>
  page.getByRole('region', { name: 'Incidentes de integridad', exact: true });
export const card = (page, row) => inbox(page).locator(`[data-incident-id="${row.id}"]`);
export const downloadButton = (page) =>
  page.getByRole('button', { name: 'Descargar archivo', exact: true });
export const contentPath = (scenario) =>
  `/cases/${scenario.case.id}/documents/${scenario.first.id}/versions/1/content`;
export const responseTo = (page, path) =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `/api/v1${path}` &&
      response.request().method() === 'GET',
  );

export async function openHistoricalContent(page, scenario) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByLabel('Buscar por t\u00edtulo', { exact: true }).fill(scenario.case.title);
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  const listed = responseTo(page, '/case-administrations');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  expect((await listed).status()).toBe(200);
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(scenario.case.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  if (scenario.closed) await expect(page.locator('.case-summary')).toContainText('Cerrado');
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page
    .locator('.reference-panel')
    .getByLabel('Identificador del documento')
    .fill(scenario.first.id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: scenario.current.name, exact: true }),
  ).toBeVisible();
  await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
  const history = page.getByRole('region', { name: 'Historial de versiones', exact: true });
  await expect(history).toHaveAttribute('aria-busy', 'false');
  const exact = responseTo(page, contentPath(scenario).replace('/content', ''));
  await history
    .getByRole('button')
    .filter({
      has: page.getByText(`Versi\u00f3n 1 / ${scenario.first.name}`, { exact: true }),
    })
    .click();
  const response = await exact;
  expect(response.status()).toBe(200);
  expect(await response.json()).toEqual(scenario.first);
  await expect(page.getByRole('heading', { name: scenario.first.name, exact: true })).toBeVisible();
  await expect(
    page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
  ).toBeVisible();
}

export async function downloadExact(page, scenario) {
  const completed = page.waitForEvent('download');
  const received = responseTo(page, contentPath(scenario));
  await downloadButton(page).click();
  const response = await received;
  expect(response.status()).toBe(200);
  const headers = response.headers();
  expect(headers['content-type']).toBe('application/octet-stream');
  expect(headers['content-disposition']).toMatch(/^attachment;/);
  expect(headers['cache-control']).toBe('no-store');
  expect(headers['x-content-type-options']).toBe('nosniff');
  expect(headers['x-case-id']).toBe(scenario.case.id);
  expect(headers['x-document-id']).toBe(scenario.first.id);
  expect(headers['x-document-version']).toBe('1');
  expect(headers['x-document-digest']).toBe(scenario.first.digest);
  const downloaded = await completed;
  const chunks = [];
  for await (const chunk of await downloaded.createReadStream()) chunks.push(chunk);
  const bytes = Buffer.concat(chunks);
  expect(bytes).toEqual(Buffer.from(scenario.payload_base64, 'base64'));
  expect(createHash('sha256').update(bytes).digest('hex')).toBe(scenario.first.digest);
  expect(downloaded.suggestedFilename()).toBe(scenario.first.name.replace('.bin', '-v1.bin'));
  await expect(page.getByText('Descarga del archivo iniciada.', { exact: true })).toBeVisible();
}

export async function findIncident(page, row) {
  for (let pageNumber = 0; pageNumber < 20; pageNumber++) {
    await expect(inbox(page)).toHaveAttribute('aria-busy', 'false');
    await expect(inbox(page).getByRole('alert')).toHaveCount(0);
    if (await card(page, row).count()) return;
    const more = inbox(page).getByRole('button', {
      name: 'Cargar m\u00e1s incidentes',
      exact: true,
    });
    if (!(await more.count())) break;
    await more.click();
  }
  await expect(card(page, row)).toBeVisible();
}

export async function capture(page, testInfo, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}
