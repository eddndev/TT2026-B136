import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
export { accountAction } from './hearing-result-helpers.mjs';

export const accounts = fixture.documentContent;
const media = new URL(
  '../../../crates/infrastructure/tests/fixtures/media-admission/',
  import.meta.url,
);
export const png = readFileSync(new URL('tiny.png', media));
export const wav = readFileSync(new URL('tiny.wav', media));
// Synthetic one-pixel GIF89a, deliberately outside the admitted format list.
export const gif = Buffer.from(
  'R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7',
  'base64',
);
export const values = {
  document_type: 'Imagen',
  classification: 'Reservado',
  tags: ['admission, exact'],
};
export const metadataCard = (page) =>
  page.getByRole('region', { name: 'Clasificaci\u00f3n actual del documento', exact: true });
export const history = (page) =>
  page.getByRole('region', { name: 'Historial de versiones', exact: true });

export function responseTo(page, path, method = 'POST') {
  return page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `/api/v1${path}` &&
      response.request().method() === method,
  );
}
export async function openDocuments(page, record) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByLabel('Buscar por t\u00edtulo', { exact: true }).fill(record.title);
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  const listed = responseTo(page, '/case-administrations', 'GET');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  expect((await listed).status()).toBe(200);
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(record.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await expect(page.locator('.document-list')).toHaveAttribute('aria-busy', 'false');
}
export async function fillMetadata(modal) {
  await modal
    .getByLabel('Tipo de documento (opcional)', { exact: true })
    .fill(values.document_type);
  await modal
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill(values.classification);
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill(values.tags[0]);
}
export async function retainedFile(input, name, bytes) {
  expect(
    await input.evaluate(async (element) => ({
      name: element.files[0].name,
      bytes: [...new Uint8Array(await element.files[0].arrayBuffer())],
    })),
  ).toEqual({ name, bytes: [...bytes] });
}
export async function downloadExact(page, row, expected) {
  const event = page.waitForEvent('download');
  const received = responseTo(
    page,
    `/cases/${row.case_id}/documents/${row.id}/versions/${row.version}/content`,
    'GET',
  );
  await page.getByRole('button', { name: 'Descargar archivo', exact: true }).click();
  const response = await received;
  expect(response.status()).toBe(200);
  expect(response.headers()['x-document-version']).toBe(String(row.version));
  expect(response.headers()['x-document-digest']).toBe(row.digest);
  expect(response.headers()['cache-control']).toBe('no-store');
  expect(response.headers()['x-content-type-options']).toBe('nosniff');
  const download = await event;
  const chunks = [];
  for await (const chunk of await download.createReadStream()) chunks.push(chunk);
  const bytes = Buffer.concat(chunks);
  expect(bytes).toEqual(expected);
  expect(createHash('sha256').update(bytes).digest('hex')).toBe(row.digest);
  const extension = row.name.lastIndexOf('.');
  expect(download.suggestedFilename()).toBe(
    `${row.name.slice(0, extension)}-v${row.version}${row.name.slice(extension)}`,
  );
}
