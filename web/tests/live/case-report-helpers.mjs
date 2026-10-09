import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { expect } from '@playwright/test';

export const form = (page) => page.getByRole('region', { name: 'Solicitar informe', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle de informe', exact: true });
export const reports = (page) =>
  page.getByRole('region', { name: 'Informes de expedientes', exact: true });
export const compact = (text) => text.replaceAll(/\s+/gu, '');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const responseTo = (page, path, method = 'GET', format) =>
  page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.pathname === `/api/v1${path}` &&
      response.request().method() === method &&
      (format === undefined || url.searchParams.get('format') === format)
    );
  });

export async function enterReports(page) {
  const response = responseTo(page, '/case-reports');
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name: 'Informes', exact: true })
    .click();
  expect((await response).status()).toBe(200);
  await expect(reports(page)).toBeVisible();
  return (await response).json();
}

export async function selectMember(page, account, label = 'Litigante asignado') {
  const select = form(page).getByLabel(label, { exact: true });
  for (let index = 0; index < 20; index += 1) {
    await expect(select).toBeEnabled();
    if (await select.getByRole('option', { name: account.email, exact: true }).count()) {
      await select.selectOption(account.id);
      return;
    }
    const loaded = responseTo(page, '/case-reports/litigators');
    await form(page)
      .getByRole('button', { name: 'Cargar m\u00e1s litigantes', exact: true })
      .click();
    expect((await loaded).status()).toBe(200);
  }
  throw new Error('Report fixture member exceeded the selector page budget');
}

export async function openReport(page, id) {
  const loaded = responseTo(page, `/case-reports/${id}`);
  await page.getByRole('button', { name: `Consultar informe ${id}`, exact: true }).click();
  expect((await loaded).status()).toBe(200);
  await expect(detail(page)).toHaveAttribute('aria-busy', 'false');
  return (await loaded).json();
}

export async function readyReport(page, initial) {
  let ready;
  await expect
    .poll(
      async () => {
        const loaded = responseTo(page, `/case-reports/${initial.id}`);
        await detail(page).getByRole('button', { name: 'Actualizar informe', exact: true }).click();
        expect((await loaded).status()).toBe(200);
        ready = await (await loaded).json();
        expect(ready.failure).toBeNull();
        expect(['queued', 'processing', 'ready']).toContain(ready.state);
        await expect(detail(page)).not.toContainText(/\b\d+\s*%/u);
        return ready.state;
      },
      { timeout: 15000, intervals: [250, 500, 1000] },
    )
    .toBe('ready');
  expect(ready.id).toBe(initial.id);
  expect(ready.operation_id).toBe(initial.operation_id);
  expect(ready.notice).toEqual({ kind: 'ready', created_at: expect.any(String), read_at: null });
  expect(ready.ready.artifacts.map((row) => row.format).sort()).toEqual(['csv', 'pdf']);
  await expect(detail(page)).toContainText('Disponible');
  await expect(detail(page)).toContainText('Aviso sin leer');
  return ready;
}

export function csvRecords(csv) {
  return JSON.parse(
    execFileSync(
      'python3',
      [
        '-c',
        'import csv,io,json,sys; print(json.dumps(list(csv.DictReader(io.StringIO(sys.stdin.read(), newline="")))))',
      ],
      { input: csv, encoding: 'utf8', timeout: 5000 },
    ),
  );
}

export function pdfText(path) {
  return compact(
    execFileSync('pdftotext', ['-enc', 'UTF-8', '-layout', path, '-'], {
      encoding: 'utf8',
      timeout: 10000,
    }),
  );
}

export async function downloadPair(page, testInfo, row, account, suffix, verify) {
  const bytes = {},
    paths = {};
  for (const format of ['pdf', 'csv']) {
    const response = responseTo(page, `/case-reports/${row.id}/download`, 'GET', format);
    const event = page.waitForEvent('download');
    await detail(page)
      .getByRole('button', { name: `Descargar ${format.toUpperCase()}`, exact: true })
      .click();
    const download = await event,
      received = await response;
    expect(received.status()).toBe(200);
    expect(await download.failure()).toBeNull();
    expect(download.suggestedFilename()).toBe(`report-${row.id}.${format}`);
    paths[format] = testInfo.outputPath(`case-report-${suffix}.${format}`);
    await download.saveAs(paths[format]);
    bytes[format] = await readFile(paths[format]);
    // Compare against an independent authenticated HTTP download, not DevTools body retention.
    const direct = await page.request.fetch(received.request());
    expect(direct.status()).toBe(200);
    const httpBytes = await direct.body();
    await direct.dispose();
    expect(bytes[format].length).toBe(httpBytes.length);
    expect(bytes[format].equals(httpBytes)).toBe(true);
    const metadata = row.ready.artifacts.find((value) => value.format === format);
    expect(bytes[format].length).toBe(metadata.bytes);
    expect(hash(bytes[format])).toBe(metadata.digest);
    const headers = received.headers();
    expect(headers['cache-control']).toBe('no-store');
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['content-length']).toBe(String(metadata.bytes));
    expect(headers['content-type']).toBe(
      format === 'pdf' ? 'application/pdf' : 'text/csv; charset=utf-8',
    );
    expect(headers['x-report-id']).toBe(row.id);
    expect(headers['x-report-digest']).toBe(metadata.digest);
    expect(headers['x-report-snapshot-digest']).toBe(row.ready.snapshot_digest);
  }
  expect(bytes.pdf.subarray(0, 5).toString()).toBe('%PDF-');
  verify(bytes.csv, paths.pdf, row, account);
  return bytes;
}

export async function screenshots(page, testInfo, prefix = 'case-reports-real') {
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({
      path: testInfo.outputPath(`${prefix}-${width}.png`),
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
}
