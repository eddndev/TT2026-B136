import { writeFile } from 'node:fs/promises';
import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { provisionActivityCatalogue } from './case-report-activity-fixtures.mjs';
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

const scenario = fixture.caseReports?.activity;
if (!scenario) throw new Error('Activity report fixtures must be provisioned by web-demo.sh');

function capturedUpload(csv, pdfPath, row, requester, source = scenario, counts = ['1', '0', '0']) {
  const records = csvRecords(csv);
  expect(records.map((value) => value.row_type)).toEqual(['capture', 'litigator', 'case_activity']);
  for (const value of records) {
    expect(value.report_id).toBe(row.id);
    expect(value.snapshot_digest).toBe(row.ready.snapshot_digest);
    expect(value.checked_at).toBe(row.ready.checked_at);
    expect(value.requester_id).toBe(requester.id);
    expect(value.scope).toBe('assigned_cases');
    expect(value.report_kind).toBe('litigator_activity');
    expect(value.period_from).toBe(row.filters.occurred_from);
    expect(value.period_before).toBe(row.filters.occurred_before);
    expect(value.status_filter).toBe('all');
    expect(value.litigator_filter).toBe(source.author.id);
    expect(value.documents_complete).toBe('true');
    expect([
      value.documents_uploaded,
      value.procedural_activities,
      value.deadlines_attended,
    ]).toEqual(counts);
  }
  for (const value of records.slice(1)) {
    expect(value.litigator_id).toBe(source.author.id);
    expect(value.litigator_email).toBe(`'${source.author.email}`);
    if (source === scenario) expect(value.litigator_id).not.toBe(requester.id);
  }
  expect(records[2].case_id).toBe(source.case.id);
  expect(records[2].title).toBe(`'${source.case.title}`);
  expect(records[2].reference).toBe(`'${source.case.reference}`);
  expect(records[2].status).toBe('active');
  const text = pdfText(pdfPath);
  for (const value of [
    row.id,
    row.ready.snapshot_digest,
    requester.id,
    source.author.id,
    source.author.email,
    source.case.id,
    source.case.title,
    source.case.reference,
    row.filters.occurred_from,
    row.filters.occurred_before,
    'Cobertura documental completa',
    `Documentos: ${counts[0]}`,
    `Actuaciones: ${counts[1]}`,
    `Plazos atendidos: ${counts[2]}`,
  ])
    expect(text).toContain(compact(value));
}

// This live path records one original upload and no acts or attended deadlines.
test('real activity report retains original upload author 1/0/0 after reassignment and fresh login', async ({
  page,
}, testInfo) => {
  const errors = [],
    requests = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/api/v1/case-reports' && request.method() === 'POST')
      requests.push(request.postDataJSON());
  });
  expect(scenario.document.version).toBe(1);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await loginAs(page, scenario.reader, 0);
  expect((await enterReports(page)).reports).toEqual([]);
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
  await expect(form(page).getByLabel('Litigante asignado', { exact: true })).not.toContainText(
    scenario.author.email,
  );
  const selected = responseTo(page, '/case-reports/litigators');
  await form(page)
    .getByLabel('Tipo de informe', { exact: true })
    .selectOption('litigator_activity');
  const directory = await selected;
  expect(directory.status()).toBe(200);
  expect(new URL(directory.url()).searchParams.get('report_type')).toBe('litigator_activity');
  await selectMember(page, scenario.author, 'Litigante autor');
  await form(page).getByLabel('Actividad desde (UTC)', { exact: true }).fill(scenario.from);
  await form(page)
    .getByLabel('Actividad hasta (excluida, UTC)', { exact: true })
    .fill(scenario.before);
  await form(page).getByLabel('Estado administrativo', { exact: true }).selectOption('all');
  const created = responseTo(page, '/case-reports', 'POST');
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  const response = await created;
  expect(response.status()).toBe(202);
  const initial = await response.json();
  expect(initial.state).toBe('queued');
  expect(initial.report_type).toBe('litigator_activity');
  expect(initial.scope).toBe('assigned_cases');
  expect(initial.filters).toEqual({
    occurred_from: `${scenario.from}T00:00:00Z`,
    occurred_before: `${scenario.before}T00:00:00Z`,
    status: 'all',
    author_litigator: scenario.author.id,
  });
  const ready = await readyReport(page, initial);
  expect(ready.report_type).toBe(initial.report_type);
  expect(ready.filters).toEqual(initial.filters);
  const original = await downloadPair(
    page,
    testInfo,
    ready,
    scenario.reader,
    'activity-original',
    capturedUpload,
  );
  expect(await openReport(page, ready.id)).toEqual(ready);
  await screenshots(page, testInfo, 'case-report-activity-real');
  const read = responseTo(page, `/case-reports/${ready.id}/notice-read`, 'POST');
  await detail(page)
    .getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true })
    .click();
  const readResponse = await read;
  expect(readResponse.status()).toBe(200);
  const acknowledged = await readResponse.json();
  expect(acknowledged.notice.read_at).toEqual(expect.any(String));
  await expect(detail(page)).toContainText('Aviso le\u00eddo');
  const logout = responseTo(page, '/auth/logout', 'POST');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  expect((await logout).status()).toBe(204);
  await expect(page.getByLabel('Correo electr\u00f3nico', { exact: true })).toBeVisible();
  await loginAs(page, scenario.reader, 1);
  expect((await enterReports(page)).reports).toEqual([acknowledged]);
  expect(await openReport(page, ready.id)).toEqual(acknowledged);
  await expect(detail(page)).toContainText('Aviso le\u00eddo');
  const restored = await downloadPair(
    page,
    testInfo,
    acknowledged,
    scenario.reader,
    'activity-after-login',
    capturedUpload,
  );
  for (const format of ['pdf', 'csv']) expect(restored[format].equals(original[format])).toBe(true);
  expect(requests).toEqual([
    {
      operation_id: initial.operation_id,
      report_type: 'litigator_activity',
      filters: initial.filters,
    },
  ]);
  expect(errors).toEqual([]);
  const receiptPath = testInfo.outputPath('activity-report-receipt.json');
  await writeFile(
    receiptPath,
    JSON.stringify(
      {
        case: scenario.case,
        author: { id: scenario.author.id, email: scenario.author.email },
        reader: { id: scenario.reader.id, email: scenario.reader.email },
        document: scenario.document,
        counts: { documents_uploaded: 1, procedural_activities: 0, deadlines_attended: 0 },
        initial,
        ready,
        acknowledged,
      },
      null,
      2,
    ) + '\n',
  );
  await testInfo.attach('activity-report-receipt', {
    path: receiptPath,
    contentType: 'application/json',
  });
});

test('real activity report captures the complete original activity catalogue as 1/9/1', async ({
  page,
}, testInfo) => {
  const catalogue = await provisionActivityCatalogue(fixture.caseReports.owner);
  expect(catalogue.acts.map((act) => act.values.kind)).toEqual([
    'interposition',
    'admission',
    'inadmissibility',
    'withdrawal',
    'resolution',
  ]);
  expect(catalogue.result.values.agreements).toHaveLength(2);
  await page.goto('/');
  await loginAs(page, catalogue.author, 1);
  expect((await enterReports(page)).reports).toEqual([]);
  await form(page)
    .getByLabel('Tipo de informe', { exact: true })
    .selectOption('litigator_activity');
  await selectMember(page, catalogue.author, 'Litigante autor');
  await form(page).getByLabel('Actividad desde (UTC)', { exact: true }).fill(catalogue.from);
  await form(page)
    .getByLabel('Actividad hasta (excluida, UTC)', { exact: true })
    .fill(catalogue.before);
  await form(page).getByLabel('Estado administrativo', { exact: true }).selectOption('all');
  const created = responseTo(page, '/case-reports', 'POST');
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  const response = await created;
  expect(response.status()).toBe(202);
  const initial = await response.json();
  expect(initial.filters).toEqual({
    occurred_from: `${catalogue.from}T00:00:00Z`,
    occurred_before: `${catalogue.before}T00:00:00Z`,
    status: 'all',
    author_litigator: catalogue.author.id,
  });
  const ready = await readyReport(page, initial);
  await downloadPair(
    page,
    testInfo,
    ready,
    catalogue.author,
    'activity-catalogue',
    (csv, pdfPath, row, requester) =>
      capturedUpload(csv, pdfPath, row, requester, catalogue, ['1', '9', '1']),
  );
  const receiptPath = testInfo.outputPath('activity-catalogue-receipt.json');
  await writeFile(
    receiptPath,
    JSON.stringify(
      {
        case: catalogue.case,
        author: { id: catalogue.author.id, email: catalogue.author.email },
        sources: {
          document: catalogue.document.id,
          resolution: catalogue.resolution.id,
          notification: catalogue.notification.id,
          resource_acts: catalogue.acts.map((act) => ({ id: act.id, kind: act.values.kind })),
          hearing_result: catalogue.result.id,
          measure_decision: catalogue.decision.group.review.command.decision_id,
          deadline: catalogue.deadline.id,
        },
        counts: { documents_uploaded: 1, procedural_activities: 9, deadlines_attended: 1 },
        initial,
        ready,
      },
      null,
      2,
    ) + '\n',
  );
  await testInfo.attach('activity-catalogue-receipt', {
    path: receiptPath,
    contentType: 'application/json',
  });
});
