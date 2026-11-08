import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { navigate, login, expectLanding } from './helpers.mjs';
import {
  artifactBytes,
  captureDigest,
  lawyerId,
  pending,
  report,
  reportId,
} from './case-reports-fixtures.mjs';
import {
  setupReports,
  enterReports,
  fillFilters,
  openReport,
  updateReport,
  reports,
  details,
  form,
} from './case-reports-helpers.mjs';

for (const [role, width] of [
  ['owner', 1440],
  ['litigator', 390],
]) {
  test(`reports request current cases with authorized filters for ${role} at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupReports(page, role);
    await enterReports(page);
    await expect(reports(page)).toContainText('Estado actual');
    await expect(reports(page)).toContainText('creados');
    await expect(reports(page)).toContainText(
      role === 'owner' ? 'Todo el despacho' : 'Tus expedientes asignados',
    );
    await expect(form(page).getByLabel('Litigante asignado', { exact: true })).toContainText(
      'lawyer@example.test',
    );
    await expect(form(page).getByRole('textbox', { name: /UUID|Identificador/i })).toHaveCount(0);
    await fillFilters(page, { status: 'active', assigned: lawyerId });
    await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
    await expect(details(page)).toContainText('En cola');
    expect(state.requests).toHaveLength(1);
    expect(state.requests[0]).toEqual({
      operation_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
      filters: {
        created_from: '2026-09-01T00:00:00Z',
        created_before: '2026-10-01T00:00:00Z',
        status: 'active',
        assigned_litigator: lawyerId,
      },
    });
    await expect(
      details(page).getByRole('button', { name: 'Descargar PDF', exact: true }),
    ).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath(`reports-${width}.png`), fullPage: true });
  });
}

for (const role of ['client', 'paralegal']) {
  test(`reports deny ${role} navigation and a forced hash without private requests`, async ({
    page,
  }) => {
    const state = await setupReports(page, role);
    await login(page, false, false);
    await expect(
      page.getByRole('navigation').getByRole('button', { name: 'Informes', exact: true }),
    ).toHaveCount(0);
    await page.evaluate(() => {
      location.hash = 'reports';
    });
    await expectLanding(page, role);
    await expect(reports(page)).toHaveCount(0);
    expect(state.calls).toHaveLength(0);
  });
}

test('report progress reflects durable phases and a terminal failure without fabricated percentages', async ({
  page,
}) => {
  const state = await setupReports(page);
  state.records.set(reportId, pending());
  await enterReports(page);
  await openReport(page);
  await expect(details(page)).toContainText('En cola');
  for (const [change, label] of [
    [{ state: 'processing', phase: 'capturing' }, 'Capturando expedientes'],
    [{ state: 'processing', phase: 'rendering' }, 'Generando PDF y CSV'],
    [
      { state: 'retry_waiting', phase: 'rendering', retry_at: '2026-09-27T12:02:00Z' },
      'Reintento pendiente',
    ],
    [
      {
        state: 'failed',
        failure: 'render_failed',
        updated_at: '2026-09-27T12:03:00Z',
        notice: { kind: 'failed', created_at: '2026-09-27T12:03:00Z', read_at: null },
      },
      'No se pudo generar el informe',
    ],
  ]) {
    state.records.set(reportId, pending(change));
    await updateReport(page);
    await expect(details(page)).toContainText(label);
    await expect(details(page).getByRole('progressbar')).toHaveCount(0);
    await expect(details(page)).not.toContainText('%');
    await expect(details(page).getByRole('button', { name: /^Descargar/ })).toHaveCount(0);
  }
  expect(state.requests).toHaveLength(0);
  expect(state.acknowledgments).toHaveLength(0);
});

test('both exact report downloads retain one capture and only an explicit action reads the notice', async ({
  page,
}) => {
  const state = await setupReports(page);
  await enterReports(page);
  await openReport(page);
  await expect(details(page)).toContainText(captureDigest);
  await expect(details(page)).toContainText('Aviso sin leer');
  for (const format of ['pdf', 'csv']) {
    const event = page.waitForEvent('download');
    await details(page)
      .getByRole('button', { name: `Descargar ${format.toUpperCase()}`, exact: true })
      .click();
    const download = await event;
    expect(download.suggestedFilename()).toBe(`report-${reportId}.${format}`);
    expect(await readFile(await download.path())).toEqual(artifactBytes[format]);
  }
  expect(state.downloads).toEqual([
    { id: reportId, format: 'pdf' },
    { id: reportId, format: 'csv' },
  ]);
  expect(state.acknowledgments).toHaveLength(0);
  await details(page)
    .getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true })
    .click();
  await expect(details(page)).toContainText('Aviso le\u00eddo');
  expect(state.acknowledgments).toEqual([reportId]);
  await navigate(page, 'Inicio');
  await navigate(page, 'Informes');
  await openReport(page);
  await expect(details(page)).toContainText('Aviso le\u00eddo');
  expect(state.acknowledgments).toHaveLength(1);
  expect(state.records.get(reportId)).toEqual(
    report({
      updated_at: '2026-09-27T12:01:00Z',
      notice: {
        kind: 'ready',
        created_at: '2026-09-27T12:00:00Z',
        read_at: '2026-09-27T12:01:00Z',
      },
    }),
  );
});
