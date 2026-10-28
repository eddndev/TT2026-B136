// The evidence validation panel with the Web Worker replaced by a stub that
// answers the scenario written in the chosen file.
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { id } from './helpers.mjs';
import { versionSetup, append } from './version-helpers.mjs';

const stub = readFileSync(new URL('./evidence-worker-stub.js', import.meta.url), 'utf8');
const pinnedFile =
  process.env.QADRA_PINNED_ROOTS ||
  new URL('../../src/lib/evidence/pinned-roots.json', import.meta.url);
const pinnedLabel = JSON.parse(readFileSync(pinnedFile, 'utf8'))[0].label;
const MARKER = 'evidence-bytes-marker';

async function stubWorker(page) {
  const loads = [];
  await page.route('**/evidence-verify-worker.mjs*', (route) => {
    loads.push(route.request().url());
    return route.fulfill({ contentType: 'text/javascript', body: stub });
  });
  return loads;
}

function scenario(overall, delay = 0) {
  return {
    name: 'evidencia.zip',
    mimeType: 'application/zip',
    buffer: Buffer.from(JSON.stringify({ overall, delay, marker: MARKER })),
  };
}

async function openPanel(page) {
  await page.getByRole('tab', { name: 'Evidencia', exact: true }).click();
  const panel = page.locator('.evidence-validation');
  await expect(
    panel.getByRole('heading', { name: 'Validar un paquete de evidencia' }),
  ).toBeVisible();
  return panel;
}

async function validate(panel, file) {
  await panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true }).setInputFiles(file);
  await panel.getByRole('button', { name: 'Validar paquete', exact: true }).click();
}

test('the four overall results are shown in plain language without uploading the file', async ({
  page,
}) => {
  const sent = [];
  page.on('request', (request) =>
    sent.push({ url: request.url(), method: request.method(), body: request.postData() || '' }),
  );
  const loads = await stubWorker(page);
  const { calls } = await versionSetup(page);
  const panel = await openPanel(page);
  const expected = [
    ['checked', 'Completo', 'Firma correcta'],
    ['indeterminate', 'Parcial', 'Falta la lista de revocaci\u00f3n que cubre esa fecha.'],
    ['failed', 'Fallido', 'Firma incorrecta'],
    ['not-supported', 'No compatible', 'no es el paquete que export\u00f3 Qadra'],
  ];
  const unaltered = 'no significa por s\u00ed solo que el documento se haya modificado';
  for (const [overall, label, detail] of expected) {
    await validate(panel, scenario(overall));
    const result = panel.locator('.evidence-result');
    await expect(result.locator('.evidence-result-overall')).toHaveText(label);
    await expect(result).toContainText(detail);
    if (overall === 'indeterminate') await expect(result).toContainText(unaltered);
    else await expect(result).not.toContainText(unaltered);
  }
  await validate(panel, scenario('checked'));
  const result = panel.locator('.evidence-result');
  await expect(result).toContainText('Cumple el perfil estricto');
  await expect(result).toContainText('Ra\u00edz fija en Qadra');
  await expect(result).toContainText(pinnedLabel);
  await expect(result).toContainText('1.3.6.1.4.1.13762.3');
  await expect(result).toContainText('2026-10-28 00:00:00 UTC');
  await expect(result).toContainText('Sello de tiempo verificado');
  await expect(result).toContainText('Para paquetes hist\u00f3ricos este resultado puede diferir');
  await expect(result.getByRole('heading', { name: 'Estado actual (informativo)' })).toBeVisible();
  await expect(result).toContainText('no cambia el resultado');
  await expect(result).toContainText(
    'Las ra\u00edces fijas de esta versi\u00f3n de Qadra coinciden con las que usa el servidor.',
  );
  expect(loads.length).toBeGreaterThan(0);
  for (const request of sent) {
    expect(request.body).not.toContain(MARKER);
    if (request.url.includes('/api/')) expect(request.method).not.toBe('PUT');
  }
  const trustReads = calls.filter((call) => call.path === '/api/v1/evidence/trust');
  expect(trustReads.length).toBe(5);
  for (const call of trustReads) {
    expect(call.method).toBe('GET');
    expect(call.search).toBe('');
    expect(call.body).toBeNull();
  }
});

test('the stages are shown and cancel ends the validation without a result', async ({ page }) => {
  await stubWorker(page);
  await versionSetup(page);
  const panel = await openPanel(page);
  await validate(panel, scenario('checked', 60000));
  await expect(panel.getByText('Revisando la evidencia', { exact: false })).toBeVisible();
  await expect(panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true })).toBeDisabled();
  await panel.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(panel.getByRole('status')).toContainText('Validaci\u00f3n cancelada');
  await expect(panel.locator('.evidence-result')).toHaveCount(0);
  await expect(panel.getByRole('alert')).toHaveCount(0);
  await validate(panel, scenario('failed'));
  await expect(panel.locator('.evidence-result-overall')).toHaveText('Fallido');
});

test('an engine failure and a file over the limit are explained', async ({ page }) => {
  await stubWorker(page);
  await versionSetup(page);
  const panel = await openPanel(page);
  await validate(panel, scenario('error'));
  await expect(panel.getByRole('alert')).toContainText('no pudo terminar la revisi\u00f3n');
  await panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true }).setInputFiles({
    name: 'grande.zip',
    mimeType: 'application/zip',
    buffer: Buffer.alloc(26 * 1024 * 1024 + 1),
  });
  await expect(panel.getByRole('alert')).toContainText('26 MiB');
  await expect(panel.getByRole('button', { name: 'Validar paquete', exact: true })).toBeDisabled();
});

test('the result of another version is discarded when the version changes', async ({ page }) => {
  await stubWorker(page);
  await versionSetup(page);
  await append(page);
  await expect(page.getByText('Versi\u00f3n actual: 2', { exact: true })).toBeVisible();
  let panel = await openPanel(page);
  await validate(panel, scenario('checked', 6000));
  await expect(panel.getByText('Revisando la evidencia', { exact: false })).toBeVisible();
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect(
    page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
  ).toBeVisible();
  panel = await openPanel(page);
  await expect(panel.getByText('Revisando la evidencia', { exact: false })).toHaveCount(0, {
    timeout: 3000,
  });
  await expect(panel.getByRole('button', { name: 'Cancelar', exact: true })).toHaveCount(0);
  await page.waitForTimeout(4000);
  await expect(panel.locator('.evidence-result')).toHaveCount(0);
});

test('the report is downloaded as JSON without the file bytes', async ({ page }) => {
  await stubWorker(page);
  await versionSetup(page);
  const panel = await openPanel(page);
  await validate(panel, scenario('checked'));
  await expect(panel.locator('.evidence-result-overall')).toHaveText('Completo');
  const downloading = page.waitForEvent('download');
  await panel.getByRole('button', { name: 'Descargar informe JSON', exact: true }).click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe(`validacion-evidencia-${id}-v1.json`);
  const text = readFileSync(await download.path(), 'utf8');
  expect(text).not.toContain(MARKER);
  const value = JSON.parse(text);
  expect(value.format).toBe('qadra-evidence-validation/1');
  expect(value.document).toEqual({ id, version: 1 });
  expect(value.file.name).toBe('evidencia.zip');
  expect(value.file.sha256).toMatch(/^[0-9a-f]{64}$/);
  expect(value.engine).toEqual({
    name: 'evidence-verify',
    version: 'stub',
    source_digest: 'stub-digest',
  });
  expect(value.overall).toBe('checked');
  expect(value.policy.clock).toBe('client');
  expect(value.anchors.pinned[0].label).toBe(pinnedLabel);
  expect(value.anchors.route['server-seal'].root_fingerprint).toMatch(/^[0-9a-f]{64}$/);
  expect(value.components).toHaveLength(8);
});

test('an unavailable trust route only changes the trust notes', async ({ page }) => {
  await stubWorker(page);
  await versionSetup(page);
  await page.route('**/api/v1/evidence/trust', (route) =>
    route.fulfill({ status: 503, json: { error: { code: 'internal_error' } } }),
  );
  const panel = await openPanel(page);
  await validate(panel, scenario('checked'));
  await expect(panel.locator('.evidence-result-overall')).toHaveText('Completo');
  await expect(panel.locator('.evidence-result')).toContainText(
    'No se pudo consultar la confianza actual del servidor',
  );
});
