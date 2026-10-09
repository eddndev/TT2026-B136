import { test, expect } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { reportDraftSetup, checkReportRequests } from './session-report-request-fixtures.mjs';
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
  updateReport,
  reports,
  details,
  form,
} from './case-reports-helpers.mjs';

const type = (page) => form(page).getByLabel('Tipo de informe', { exact: true });
const generate = (page) => form(page).getByRole('button', { name: 'Generar informe', exact: true });
const activityFrom = (page) => form(page).getByLabel('Actividad desde (UTC)', { exact: true });
const activityBefore = (page) =>
  form(page).getByLabel('Actividad hasta (excluida, UTC)', { exact: true });
async function activityFilters(page) {
  await type(page).selectOption('litigator_activity');
  await activityFrom(page).fill('2026-09-01');
  await activityBefore(page).fill('2026-10-01');
  await form(page).getByLabel('Estado administrativo', { exact: true }).selectOption('active');
  await form(page).getByLabel('Litigante autor', { exact: true }).selectOption(lawyerId);
}
function accepted(command, scope = 'office') {
  return pending({
    scope,
    operation_id: command.operation_id,
    filters: command.filters,
    ...(command.report_type ? { report_type: command.report_type } : {}),
  });
}

for (const [role, width] of [
  ['owner', 1440],
  ['litigator', 390],
]) {
  test(`activity reports preserve period author progress and both downloads for ${role} at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupReports(page, role);
    state.handle = async (route, url) => {
      if (url.pathname !== '/api/v1/case-reports' || route.request().method() !== 'POST')
        return false;
      const command = route.request().postDataJSON();
      state.requests.push(command);
      const value = accepted(command, state.picker.scope);
      state.records.set(reportId, value);
      await route.fulfill({ status: 202, json: value });
      return true;
    };
    await enterReports(page);
    await expect(type(page)).toHaveValue('case_state');
    await activityFilters(page);
    await expect(form(page).getByLabel('Litigante asignado', { exact: true })).toHaveCount(0);
    await generate(page).click();
    await expect(details(page)).toContainText('En cola');
    expect(state.requests).toHaveLength(1);
    expect(state.requests[0]).toEqual({
      operation_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
      report_type: 'litigator_activity',
      filters: {
        occurred_from: '2026-09-01T00:00:00Z',
        occurred_before: '2026-10-01T00:00:00Z',
        status: 'active',
        author_litigator: lawyerId,
      },
    });
    await expect(details(page)).toContainText('Actividad por litigante');
    await expect(details(page)).toContainText('Litigante autor');
    await expect(details(page)).toContainText(lawyerId);
    await expect(reports(page)).toContainText('Actividad registrada del 2026-09-01');
    await expect(details(page).getByRole('button', { name: /^Descargar/ })).toHaveCount(0);
    state.records.set(reportId, {
      ...state.records.get(reportId),
      state: 'processing',
      phase: 'capturing',
    });
    await updateReport(page);
    await expect(details(page)).toContainText('Capturando actividad');
    await expect(details(page).getByRole('progressbar')).toHaveCount(0);
    state.records.set(
      reportId,
      report({
        scope: state.picker.scope,
        operation_id: state.requests[0].operation_id,
        report_type: 'litigator_activity',
        filters: state.requests[0].filters,
      }),
    );
    await updateReport(page);
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
    expect(state.acknowledgments).toEqual([]);
    await details(page)
      .getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true })
      .click();
    await expect(details(page)).toContainText('Aviso le\u00eddo');
    expect(state.acknowledgments).toEqual([reportId]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}

test('changing the report type preserves an uncertain activity operation until explicit retry', async ({
  page,
}) => {
  const state = await setupReports(page);
  const attempted = [];
  state.handle = async (route, url) => {
    if (url.pathname !== '/api/v1/case-reports' || route.request().method() !== 'POST')
      return false;
    const command = route.request().postDataJSON();
    attempted.push(command);
    if (attempted.length === 1) {
      await route.abort('failed');
      return true;
    }
    const value = accepted(command);
    state.records.set(reportId, value);
    await route.fulfill({ status: 202, json: value });
    return true;
  };
  await enterReports(page);
  await activityFilters(page);
  await generate(page).click();
  await expect(form(page).getByRole('alert')).toBeVisible();
  expect(attempted).toHaveLength(1);
  await type(page).selectOption('case_state');
  await fillFilters(page, { assigned: lawyerId, status: 'active' });
  await expect(form(page).locator('.report-retry')).toContainText('Actividad por litigante');
  expect(attempted).toHaveLength(1);
  await form(page).getByRole('button', { name: 'Reintentar solicitud', exact: true }).click();
  await expect(details(page)).toContainText('En cola');
  expect(attempted[1]).toEqual(attempted[0]);
  await expect(details(page)).toContainText('Actividad por litigante');
  await generate(page).click();
  await expect(details(page)).toContainText('Estado y carga de expedientes');
  expect(attempted).toHaveLength(3);
  expect(attempted[2].operation_id).not.toBe(attempted[0].operation_id);
  expect(attempted[2]).not.toHaveProperty('report_type');
  expect(attempted[2].filters).toEqual({
    created_from: '2026-09-01T00:00:00Z',
    created_before: '2026-10-01T00:00:00Z',
    status: 'active',
    assigned_litigator: lawyerId,
  });
});

test.describe('activity report draft reentry', () => {
  test.afterEach(async ({ page }) => checkReportRequests(page));
  test('the same account restores its incomplete activity period without submitting', async ({
    page,
  }) => {
    const state = await reportDraftSetup(page);
    await enterReports(page);
    await activityFilters(page);
    await activityBefore(page).fill('');
    await expire(page, state, await form(page).elementHandle());
    await login(page, false, false);
    await navigate(page, 'Informes');
    await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
    await expect(type(page)).toHaveValue('case_state');
    await form(page)
      .getByRole('button', { name: 'Retomar solicitud de informe', exact: true })
      .click();
    await expect(type(page)).toHaveValue('litigator_activity');
    await expect(activityFrom(page)).toHaveValue('2026-09-01');
    await expect(activityBefore(page)).toHaveValue('');
    await expect(form(page).getByLabel('Litigante autor', { exact: true })).toHaveValue(lawyerId);
    expect(state.reportWrites).toEqual([]);
  });
});
