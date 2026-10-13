import { test, expect } from '@playwright/test';
import { navigate } from './helpers.mjs';
import { signInOther } from './session-inactivity-helpers.mjs';
import { absoluteSession } from '../fixtures/session.mjs';
import {
  setupPrecautionaryAgenda,
  openPrecautionaryAgenda,
  precautionaryCard,
  precautionaryDetail,
  setupPrecautionaryAlerts,
  openAlerts,
  openPrecautionaryAlert,
  filterAlerts,
  holdPrecautionaryDetail,
  rendered,
} from './precautionary-hearing-read-helpers.mjs';

for (const source of ['Agenda', 'Alertas'])
  test(`${source} clears the prior precautionary detail and case rows after ${source === 'Agenda' ? 403 : 404}`, async ({
    page,
  }) => {
    const agenda = source === 'Agenda';
    const state = agenda
      ? await setupPrecautionaryAgenda(page)
      : await setupPrecautionaryAlerts(page);
    const open = () =>
      agenda ? precautionaryCard(page, state).click() : openPrecautionaryAlert(page, state);
    if (agenda) await openPrecautionaryAgenda(page, state);
    else await openAlerts(page);
    await open();
    await expect(precautionaryDetail(page)).toBeVisible();
    if (agenda) state.accessStatus = 403;
    else state.detailStatus = 404;
    await open();
    await expect(
      page
        .getByRole('region', {
          name: agenda ? 'Agenda combinada' : 'Mis alertas',
          exact: true,
        })
        .getByRole('alert'),
    ).toBeVisible();
    await expect(precautionaryDetail(page)).toHaveCount(0);
    await expect(page.locator(agenda ? '[data-agenda-kind]' : '[data-alert-id]')).toHaveCount(0);
    await expect(
      page.getByText(state.selected.capture.review.resolved_values.scheduling_basis.statement, {
        exact: true,
      }),
    ).toHaveCount(0);
    expect(state.detailCalls).toHaveLength(agenda ? 1 : 2);
    expect(state.headCalls).toEqual([]);
    expect(state.unwanted).toEqual([]);
  });

test('a late precautionary alert origin cannot reopen after applying alert filters', async ({
  page,
}) => {
  const state = await setupPrecautionaryAlerts(page);
  const gate = holdPrecautionaryDetail(state);
  try {
    await openAlerts(page);
    await openPrecautionaryAlert(page, state);
    await expect.poll(() => gate.entered).toBe(true);
    await filterAlerts(page, 'unread', 'all');
    await expect.poll(() => state.calls.at(-1)?.search).toContain('state=all');
    gate.release();
    await expect.poll(() => gate.completed).toBe(true);
    await rendered(page);
    await expect(precautionaryDetail(page)).toHaveCount(0);
    await expect(
      page.getByText(state.selected.capture.review.resolved_values.scheduling_basis.statement, {
        exact: true,
      }),
    ).toHaveCount(0);
    expect(state.detailCalls).toHaveLength(1);
  } finally {
    gate.release();
  }
});

test('a precautionary Agenda reply from the previous principal cannot reopen in a new session', async ({
  page,
}) => {
  const state = await setupPrecautionaryAgenda(page);
  const gate = holdPrecautionaryDetail(state);
  try {
    await openPrecautionaryAgenda(page, state);
    await precautionaryCard(page, state).click();
    await expect.poll(() => gate.entered).toBe(true);
    const oldAuthorization = state.detailCalls[0].headers().authorization;
    await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    const other = {
      id: '22222222-2222-4222-8222-222222222222',
      email: 'other@example.test',
      role: 'owner',
    };
    await page.route(/\/api\/v1\/auth\/(mfa\/|me)/, async (route) =>
      route.fulfill({
        json: route.request().url().endsWith('/me')
          ? other
          : absoluteSession(other, 'precautionary-other-token'),
      }),
    );
    await signInOther(page);
    state.rows = [];
    await navigate(page, 'Agenda');
    await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toBeVisible();
    gate.release();
    await expect.poll(() => gate.completed).toBe(true);
    await rendered(page);
    await expect(precautionaryDetail(page)).toHaveCount(0);
    await expect(page.locator('[data-agenda-kind]')).toHaveCount(0);
    await expect(
      page.getByText(state.selected.capture.review.resolved_values.scheduling_basis.statement, {
        exact: true,
      }),
    ).toHaveCount(0);
    expect(oldAuthorization).toBe('Bearer deadline-token');
    expect(state.detailCalls).toHaveLength(1);
    expect(state.unwanted).toEqual([]);
  } finally {
    gate.release();
  }
});
