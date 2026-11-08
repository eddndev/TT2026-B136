import { test, expect } from '@playwright/test';
import { navigate, signOut } from './helpers.mjs';
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
