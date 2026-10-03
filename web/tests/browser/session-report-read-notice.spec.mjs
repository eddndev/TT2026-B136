import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { reports, details, openReport } from './case-reports-helpers.mjs';
import {
  readNoticeSetup,
  checkNoticeRequests,
  seedReport,
  holdNoticeRequest,
  reportsPath,
  reportDetailPath,
  reportId,
  expectFreshRead,
  releaseNoticeResponse,
} from './session-read-notice-fixtures.mjs';

test.afterEach(async ({ page }) => checkNoticeRequests(page));
const mark = (page) =>
  details(page).getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true });
const saving = (page) =>
  details(page).getByRole('button', { name: 'Guardando lectura...', exact: true });
const choose = (page) =>
  reports(page).getByRole('button', { name: `Consultar informe ${reportId}`, exact: true });
const readPath = `${reportDetailPath}/notice-read`;

async function pendingRead(page, state, commit) {
  await login(page, false, false);
  seedReport(state);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
  await openReport(page);
  const sent = holdNoticeRequest(state, 'POST', readPath);
  state.nextRead = { path: readPath, commit, ...(commit ? {} : { status: 503 }) };
  await mark(page).click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await details(page).elementHandle());
  return sent;
}

async function freshList(page, state) {
  await login(page, false, false);
  const fresh = holdNoticeRequest(state, 'GET', reportsPath);
  await navigate(page, 'Informes');
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(details(page)).toHaveCount(0);
  await expect(choose(page)).toHaveCount(0);
  expectFreshRead(fresh, state);
  expect(state.noticeWrites).toHaveLength(1);
  fresh.release();
  await expect(choose(page)).toBeEnabled();
}

test('an uncommitted report notice requires fresh exact detail after MFA and a new explicit acknowledgment ignores the old failure', async ({
  page,
}) => {
  const state = await readNoticeSetup(page);
  const sent = await pendingRead(page, state, false);
  await freshList(page, state);
  const detail = holdNoticeRequest(state, 'GET', reportDetailPath);
  await choose(page).click();
  await expect.poll(() => detail.entered).toBe(true);
  await expect(details(page)).toHaveAttribute('aria-busy', 'true');
  await expect(mark(page)).toHaveCount(0);
  expectFreshRead(detail, state);
  expect(state.noticeWrites).toHaveLength(1);
  detail.release();
  await expect(details(page)).toContainText(reportId);
  await expect(details(page).getByText('Aviso sin leer', { exact: true })).toBeVisible();
  await expect(mark(page)).toBeEnabled();
  const current = holdNoticeRequest(state, 'POST', readPath);
  state.nextRead = { path: readPath };
  await mark(page).click();
  await expect.poll(() => current.entered).toBe(true);
  await releaseNoticeResponse(page, sent, state);
  await expect(saving(page)).toBeDisabled();
  await expect(details(page)).toContainText(reportId);
  await expect(reports(page).getByRole('alert')).toHaveCount(0);
  expect(state.noticeWrites.map((call) => ({ path: call.path, command: call.command }))).toEqual([
    { path: readPath, command: {} },
    { path: readPath, command: {} },
  ]);
  expectFreshRead(current, state);
  current.release();
  await expect(details(page).getByText('Aviso le\u00eddo', { exact: true })).toBeVisible();
  expect(state.noticeWrites).toHaveLength(2);
});

test('a committed notice cannot restore an exact report denied after MFA when its old success arrives', async ({
  page,
}) => {
  const state = await readNoticeSetup(page);
  const sent = await pendingRead(page, state, true);
  await freshList(page, state);
  state.noticeDenials.set(reportDetailPath, { status: 403, code: 'case_report_access_revoked' });
  const detail = holdNoticeRequest(state, 'GET', reportDetailPath);
  await choose(page).click();
  await expect.poll(() => detail.entered).toBe(true);
  await expect(mark(page)).toHaveCount(0);
  await expect(details(page).getByRole('button', { name: /^Descargar (PDF|CSV)$/ })).toHaveCount(0);
  expectFreshRead(detail, state);
  detail.release();
  await expect(reports(page).getByRole('alert').first()).toContainText('Ya no tienes acceso');
  await expect(details(page)).toHaveCount(0);
  await releaseNoticeResponse(page, sent, state);
  await expect(details(page)).toHaveCount(0);
  await expect(reports(page)).not.toContainText(reportId);
  await expect(reports(page)).not.toContainText('ab'.repeat(32));
  await expect(reports(page).getByRole('button', { name: /^Descargar (PDF|CSV)$/ })).toHaveCount(0);
  await expect(reports(page).getByRole('alert').first()).toContainText('Ya no tienes acceso');
  expect(state.noticeWrites).toHaveLength(1);
  expect(state.noticeWrites[0].command).toEqual({});
  expect(state.noticeWrites[0].headers.authorization).toBe(
    `Bearer ${state.grants[0].access_token}`,
  );
});
