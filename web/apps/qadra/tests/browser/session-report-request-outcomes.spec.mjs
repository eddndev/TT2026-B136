import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { form, fillFilters, reports, details } from './case-reports-helpers.mjs';
import {
  reportDraftSetup,
  checkReportRequests,
  holdReportRequest,
  reportPath,
} from './session-report-request-fixtures.mjs';

test.afterEach(async ({ page }) => checkReportRequests(page));
const resume = (page) =>
  page.getByRole('button', { name: 'Retomar solicitud de informe', exact: true });
const retry = (page) =>
  form(page).getByRole('button', { name: 'Reintentar solicitud', exact: true });
async function enter(page) {
  await login(page, false, false);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
}
async function interrupt(page, state, commit) {
  await enter(page);
  await fillFilters(page);
  const gate = holdReportRequest(state, 'POST', reportPath);
  state.reportWrite = { commit, status: 503 };
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  await expect.poll(() => gate.entered).toBe(true);
  const command = structuredClone(state.reportWrites[0]);
  await expire(page, state, await form(page).elementHandle());
  return { gate, command };
}

test('an uncertain report preserves its exact request across two expirations without automatically repeating it', async ({
  page,
}) => {
  const state = await reportDraftSetup(page),
    { gate, command } = await interrupt(page, state, false);
  await enter(page);
  await resume(page).click();
  await expect(retry(page)).toBeEnabled();
  expect(state.reportWrites).toHaveLength(1);
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  await resume(page).click();
  await expect(retry(page)).toBeEnabled();
  expect(state.reportWrites).toHaveLength(1);
  gate.release();
  state.reportWrite = { commit: true };
  await retry(page).click();
  await expect(details(page)).toContainText('En cola');
  expect(state.reportWrites).toEqual([command, command]);
  expect(state.reportRecords.size).toBe(1);
});

test('a durable queued report does not silently reconcile a lost response and exact explicit replay starts no second job', async ({
  page,
}) => {
  const state = await reportDraftSetup(page),
    { gate, command } = await interrupt(page, state, true);
  await enter(page);
  expect(state.reportRecords.size).toBe(1);
  await resume(page).click();
  await expect(retry(page)).toBeEnabled();
  expect(state.reportWrites).toHaveLength(1);
  state.reportWrite = { commit: true };
  await retry(page).click();
  await expect(details(page)).toContainText('En cola');
  expect(state.reportWrites).toEqual([command, command]);
  expect(state.reportRecords.size).toBe(1);
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  await expect(resume(page)).toHaveCount(0);
  await expect(retry(page)).toHaveCount(0);
  gate.release();
  await fillFilters(page, { until: '2026-09-28' });
  state.reportWrite = { commit: true };
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  await expect(details(page)).toContainText('En cola');
  expect(state.reportWrites).toHaveLength(3);
  expect(state.reportWrites[2].operation_id).not.toBe(command.operation_id);
  expect(state.reportRecords.size).toBe(2);
});

test('fresh loss of report permission discards its pending draft instead of issuing the old command', async ({
  page,
}) => {
  const state = await reportDraftSetup(page),
    { gate } = await interrupt(page, state, false);
  await enter(page);
  state.reportRole = 'paralegal';
  await resume(page).click();
  await expect(retry(page)).toHaveCount(0);
  await expect(resume(page)).toHaveCount(0);
  expect(state.reportWrites).toHaveLength(1);
  state.reportRole = null;
  gate.release();
  await navigate(page, 'Inicio');
  await navigate(page, 'Informes');
  await expect(resume(page)).toHaveCount(0);
  await expect(retry(page)).toHaveCount(0);
  expect(state.reportWrites).toHaveLength(1);
});
