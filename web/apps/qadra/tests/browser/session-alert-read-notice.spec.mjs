import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { alertCard } from './alerts-helpers.mjs';
import {
  readNoticeSetup,
  checkNoticeRequests,
  seedAlert,
  holdNoticeRequest,
  alertsPath,
  alertOtherId,
  expectFreshRead,
  releaseNoticeResponse,
} from './session-read-notice-fixtures.mjs';

test.afterEach(async ({ page }) => checkNoticeRequests(page));
const mark = (card) => card.getByRole('button', { name: 'Marcar como le\u00edda', exact: true });
const saving = (card) => card.getByRole('button', { name: 'Guardando lectura...', exact: true });

test('a committed alert read is queried after MFA and its late receipt cannot complete another alert intention', async ({
  page,
}) => {
  const state = await readNoticeSetup(page);
  await login(page, false, false);
  const first = seedAlert(state),
    second = seedAlert(state, alertOtherId);
  await navigate(page, 'Alertas');
  const firstCard = alertCard(page, first),
    secondCard = alertCard(page, second);
  const firstPath = `${alertsPath}/${first.id}/read`;
  const sent = holdNoticeRequest(state, 'POST', firstPath);
  state.nextRead = { path: firstPath };
  await mark(firstCard).click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await firstCard.elementHandle());
  await login(page, false, false);
  const fresh = holdNoticeRequest(state, 'GET', alertsPath);
  await navigate(page, 'Alertas');
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(page.locator('.alert-card')).toHaveCount(0);
  expectFreshRead(fresh, state);
  expect(state.noticeWrites).toHaveLength(1);
  fresh.release();
  await expect(firstCard.getByText('Le\u00edda', { exact: true })).toBeVisible();
  await expect(mark(firstCard)).toHaveCount(0);
  await expect(mark(secondCard)).toBeEnabled();
  const secondPath = `${alertsPath}/${second.id}/read`;
  const current = holdNoticeRequest(state, 'POST', secondPath);
  state.nextRead = { path: secondPath };
  await mark(secondCard).click();
  await expect.poll(() => current.entered).toBe(true);
  await releaseNoticeResponse(page, sent, state);
  await expect(saving(secondCard)).toBeDisabled();
  await expect(secondCard.getByText('Sin leer', { exact: true })).toBeVisible();
  await expect(secondCard.getByRole('alert')).toHaveCount(0);
  await expect(
    page.getByRole('status').filter({ hasText: 'Estado de lectura actualizado.' }),
  ).toHaveCount(0);
  expect(state.noticeWrites.map((call) => call.path)).toEqual([firstPath, secondPath]);
  expect(state.noticeWrites[1].command.operation_id).not.toBe(
    state.noticeWrites[0].command.operation_id,
  );
  expectFreshRead(current, state);
  current.release();
  await expect(secondCard.getByText('Le\u00edda', { exact: true })).toBeVisible();
  expect(state.noticeWrites).toHaveLength(2);
});

test('an uncommitted alert read is never replayed for another account and its late failure cannot poison that account card', async ({
  page,
}) => {
  const state = await readNoticeSetup(page);
  await login(page, false, false);
  const first = seedAlert(state),
    firstActor = state.current.user.id;
  await navigate(page, 'Alertas');
  const sent = holdNoticeRequest(state, 'POST', `${alertsPath}/${first.id}/read`);
  state.nextRead = { path: sent.path, commit: false, status: 503 };
  await mark(alertCard(page, first)).click();
  await expect.poll(() => sent.entered).toBe(true);
  await expire(page, state, await alertCard(page, first).elementHandle());
  await signInOther(page);
  const own = seedAlert(state, alertOtherId);
  const fresh = holdNoticeRequest(state, 'GET', alertsPath);
  await navigate(page, 'Alertas');
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(page.locator('.alert-card')).toHaveCount(0);
  expectFreshRead(fresh, state);
  expect(state.noticeWrites).toHaveLength(1);
  fresh.release();
  await expect(alertCard(page, first)).toHaveCount(0);
  await expect(mark(alertCard(page, own))).toBeEnabled();
  const current = holdNoticeRequest(state, 'POST', `${alertsPath}/${own.id}/read`);
  state.nextRead = { path: current.path };
  await mark(alertCard(page, own)).click();
  await expect.poll(() => current.entered).toBe(true);
  await releaseNoticeResponse(page, sent, state);
  await expect(saving(alertCard(page, own))).toBeDisabled();
  await expect(
    page.getByRole('region', { name: 'Mis alertas', exact: true }).getByRole('alert'),
  ).toHaveCount(0);
  await expect(alertCard(page, first)).toHaveCount(0);
  expect(state.noticeWrites.map((call) => call.actor)).toEqual([firstActor, state.current.user.id]);
  expect(state.noticeWrites.map((call) => call.path)).toEqual([sent.path, current.path]);
  expect(state.noticeWrites[1].command.operation_id).not.toBe(
    state.noticeWrites[0].command.operation_id,
  );
  expectFreshRead(current, state);
  current.release();
  await expect(alertCard(page, own).getByText('Le\u00edda', { exact: true })).toBeVisible();
  expect(first.read_at).toBeNull();
  expect(state.noticeWrites).toHaveLength(2);
});
