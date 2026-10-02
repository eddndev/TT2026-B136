import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';

const accounts = fixture.caseReports;
if (!accounts) throw new Error('Case report fixtures must be provisioned by web-demo.sh');
const form = (page) => page.getByRole('region', { name: 'Solicitar informe', exact: true });
const detail = (page) => page.getByRole('region', { name: 'Detalle de informe', exact: true });
const reports = (page) =>
  page.getByRole('region', { name: 'Informes de expedientes', exact: true });
const compact = (text) => text.replaceAll(/\s+/gu, '');
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const responseTo = (page, path, method = 'GET', format) =>
  page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.pathname === `/api/v1${path}` &&
      response.request().method() === method &&
      (format === undefined || url.searchParams.get('format') === format)
    );
  });

async function enterReports(page) {
  const response = responseTo(page, '/case-reports');
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name: 'Informes', exact: true })
    .click();
  expect((await response).status()).toBe(200);
  await expect(reports(page)).toBeVisible();
  return (await response).json();
}

async function selectMember(page) {
  const select = form(page).getByLabel('Litigante asignado', { exact: true });
  for (let index = 0; index < 20; index += 1) {
    await expect(select).toBeEnabled();
    if (await select.getByRole('option', { name: accounts.litigator.email, exact: true }).count()) {
      await select.selectOption(accounts.litigator.id);
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

async function openReport(page, id) {
  const loaded = responseTo(page, `/case-reports/${id}`);
  await page.getByRole('button', { name: `Consultar informe ${id}`, exact: true }).click();
  expect((await loaded).status()).toBe(200);
  await expect(detail(page)).toHaveAttribute('aria-busy', 'false');
  return (await loaded).json();
}

async function readyReport(page, initial) {
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

function capturedIdentities(csv, pdfPath, row, account) {
  const records = JSON.parse(
    execFileSync(
      'python3',
      [
        '-c',
        'import csv,io,json,sys; print(json.dumps(list(csv.DictReader(io.StringIO(sys.stdin.read(), newline="")))))',
      ],
      { input: csv, encoding: 'utf8', timeout: 5000 },
    ),
  );
  expect(records).toHaveLength(4);
  for (const record of records) {
    expect(record.report_id).toBe(row.id);
    expect(record.snapshot_digest).toBe(row.ready.snapshot_digest);
    expect(record.requester_id).toBe(account.id);
    expect(record.checked_at).toBe(row.ready.checked_at);
    expect(record.scope).toBe(row.scope);
    expect(record.created_from).toBe(row.filters.created_from);
    expect(record.created_before).toBe(row.filters.created_before);
    expect(record.status_filter).toBe('all');
    expect(record.assigned_litigator_filter).toBe(accounts.litigator.id);
  }
  expect(records[0].row_type).toBe('capture');
  const captured = records.filter((value) => value.row_type === 'case');
  expect(captured.map((value) => value.case_id)).toEqual(
    accounts.cases.map((value) => value.case_id).sort(),
  );
  for (const expected of accounts.cases) {
    const current = captured.find((value) => value.case_id === expected.case_id);
    expect(current.title).toBe(`'${expected.title}`);
    expect(current.reference).toBe(`'${expected.reference}`);
    expect(current.status).toBe(expected.administrative_status);
    expect(current.administration_revision).toBe(String(expected.revision || ''));
    expect(current.administration_digest).toBe(expected.values_digest || '');
    expect(current.assigned_litigators.startsWith("'")).toBe(true);
    expect(JSON.parse(current.assigned_litigators.slice(1))).toEqual([
      { user_id: accounts.litigator.id, email: accounts.litigator.email },
    ]);
  }
  const workload = records.find((value) => value.row_type === 'workload');
  expect(workload.litigator_id).toBe(accounts.litigator.id);
  expect(workload.litigator_email).toBe(`'${accounts.litigator.email}`);
  for (const summary of [records[0], workload])
    expect([summary.active_cases, summary.closed_cases, summary.total_cases]).toEqual([
      '1',
      '1',
      '2',
    ]);
  const text = compact(
    execFileSync('pdftotext', ['-enc', 'UTF-8', '-layout', pdfPath, '-'], {
      encoding: 'utf8',
      timeout: 10000,
    }),
  );
  for (const value of [row.id, row.ready.snapshot_digest, account.id, accounts.litigator.id])
    expect(text).toContain(value);
  for (const expected of accounts.cases) {
    expect(text.split(expected.case_id)).toHaveLength(2);
    expect(text).toContain(compact(expected.title));
    expect(text).toContain(compact(expected.reference));
  }
}

async function downloadPair(page, testInfo, row, account, suffix) {
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
  capturedIdentities(bytes.csv, paths.pdf, row, account);
  return bytes;
}

async function screenshots(page, testInfo) {
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
      path: testInfo.outputPath(`case-reports-real-${width}.png`),
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
}

for (const role of ['owner', 'litigator']) {
  test(`real ${role} reports retain exact downloads and acknowledged notices after fresh login`, async ({
    page,
  }, testInfo) => {
    const errors = [],
      requests = [],
      account = accounts[role];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('request', (request) => {
      if (new URL(request.url()).pathname === '/api/v1/case-reports' && request.method() === 'POST')
        requests.push(request.postDataJSON());
    });
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto('/');
    await loginAs(page, account, 0);
    expect((await enterReports(page)).reports).toEqual([]);
    await expect(reports(page)).toContainText(
      role === 'owner' ? 'Todo el despacho' : 'Tus expedientes asignados',
    );
    await form(page).getByLabel('Creaci\u00f3n desde (UTC)', { exact: true }).fill(accounts.from);
    await form(page)
      .getByLabel('Creaci\u00f3n hasta (excluida, UTC)', { exact: true })
      .fill(accounts.before);
    await form(page).getByLabel('Estado administrativo', { exact: true }).selectOption('all');
    await selectMember(page);
    const created = responseTo(page, '/case-reports', 'POST');
    await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
    expect((await created).status()).toBe(202);
    const initial = await (await created).json();
    expect(initial.state).toBe('queued');
    await expect(detail(page)).toContainText(initial.id);
    expect(initial.scope).toBe(role === 'owner' ? 'office' : 'assigned_cases');
    expect(initial.filters).toEqual({
      created_from: `${accounts.from}T00:00:00Z`,
      created_before: `${accounts.before}T00:00:00Z`,
      status: 'all',
      assigned_litigator: accounts.litigator.id,
    });
    const ready = await readyReport(page, initial);
    const original = await downloadPair(page, testInfo, ready, account, 'original');
    await expect(detail(page)).toContainText('Aviso sin leer');
    await screenshots(page, testInfo);
    await page
      .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
      .getByRole('button', { name: 'Inicio', exact: true })
      .click();
    await expect(
      page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
    ).toBeVisible();
    expect((await enterReports(page)).reports).toEqual([ready]);
    expect(await openReport(page, ready.id)).toEqual(ready);
    const read = responseTo(page, `/case-reports/${ready.id}/notice-read`, 'POST');
    await detail(page)
      .getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true })
      .click();
    expect((await read).status()).toBe(200);
    const acknowledged = await (await read).json();
    expect(acknowledged.notice.read_at).toEqual(expect.any(String));
    await expect(detail(page)).toContainText('Aviso le\u00eddo');
    const logout = responseTo(page, '/auth/logout', 'POST');
    await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    expect((await logout).status()).toBe(204);
    await expect(page.getByLabel('Correo electr\u00f3nico', { exact: true })).toBeVisible();
    await loginAs(page, account, 1);
    expect((await enterReports(page)).reports).toEqual([acknowledged]);
    expect(await openReport(page, ready.id)).toEqual(acknowledged);
    await expect(detail(page)).toContainText('Aviso le\u00eddo');
    const restored = await downloadPair(page, testInfo, acknowledged, account, 'after-login');
    for (const format of ['pdf', 'csv'])
      expect(restored[format].equals(original[format])).toBe(true);
    expect(requests).toHaveLength(1);
    expect(requests[0].operation_id).toBe(initial.operation_id);
    expect(errors).toEqual([]);
  });
}
