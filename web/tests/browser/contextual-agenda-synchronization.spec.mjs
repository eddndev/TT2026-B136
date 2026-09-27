import { test, expect } from '@playwright/test';
import { setupCombinedAgenda, openCombinedAgenda, agendaPage } from './combined-agenda-helpers.mjs';
import { queryContextualAgenda } from '../live/resource-activities-contextual-helpers.mjs';
import { notChecked } from '../fixtures/deadline-v2-unit.mjs';

test('contextual agenda waits for its selected query when the initial response arrives late', async ({
  page,
}) => {
  const state = await setupCombinedAgenda(page);
  const historical = { ...state.deadline, operational: notChecked() };
  await page.route(
    `**/api/v1/cases/${historical.case_id}/deadlines/${historical.id}/revisions/1`,
    (route) => route.fulfill({ json: historical }),
  );
  let initial;
  let selected = false;
  state.handle = async (route, url) => {
    if (!initial) {
      initial = { route, url };
      return true;
    }
    selected = true;
    const previous = page.waitForResponse((response) => response.url() === initial.url.href);
    await initial.route.fulfill({ json: agendaPage(initial.url, []) });
    await (await previous).finished();
    await route.fulfill({ json: agendaPage(url, [state.deadlineItem]) });
    return true;
  };
  await openCombinedAgenda(page);
  await queryContextualAgenda(
    page,
    {
      case: { id: historical.case_id },
      date: '2026-01-02',
      dueAt: state.deadlineItem.at,
    },
    historical,
  );
  expect(selected).toBe(true);
  expect(state.calls).toHaveLength(2);
});
