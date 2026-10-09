import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import {
  form,
  detail,
  reports,
  compact,
  responseTo,
  enterReports,
  selectMember,
  openReport,
  readyReport,
  csvRecords,
  pdfText,
  downloadPair,
  screenshots,
} from './case-report-helpers.mjs';

const accounts = fixture.caseReports;
if (!accounts) throw new Error('Case report fixtures must be provisioned by web-demo.sh');

function capturedIdentities(csv, pdfPath, row, account) {
  const records = csvRecords(csv);
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
  const text = pdfText(pdfPath);
  for (const value of [row.id, row.ready.snapshot_digest, account.id, accounts.litigator.id])
    expect(text).toContain(value);
  for (const expected of accounts.cases) {
    expect(text.split(expected.case_id)).toHaveLength(2);
    expect(text).toContain(compact(expected.title));
    expect(text).toContain(compact(expected.reference));
  }
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
    await selectMember(page, accounts.litigator);
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
    const original = await downloadPair(
      page,
      testInfo,
      ready,
      account,
      'original',
      capturedIdentities,
    );
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
    const restored = await downloadPair(
      page,
      testInfo,
      acknowledged,
      account,
      'after-login',
      capturedIdentities,
    );
    for (const format of ['pdf', 'csv'])
      expect(restored[format].equals(original[format])).toBe(true);
    expect(requests).toHaveLength(1);
    expect(requests[0].operation_id).toBe(initial.operation_id);
    expect(errors).toEqual([]);
  });
}
