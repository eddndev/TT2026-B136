import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import {
  setupCombinedAgenda,
  openCombinedAgenda,
  queryAgenda,
} from './combined-agenda-helpers.mjs';
import {
  resourceHearingCreation,
  resourceHearingAgendaItem,
} from '../fixtures/resource-hearing-unit.mjs';

export const ownDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia de recurso', exact: true });
export const ownCard = (page, state) =>
  page.getByRole('button', {
    name: `Consultar audiencia de recurso ${state.creation.hearing.id}`,
    exact: true,
  });

export async function setupResourceHearingAgenda(page, { closed = false } = {}) {
  const state = await setupCombinedAgenda(page);
  const original = resourceHearingCreation({ withAct: true });
  const creation = JSON.parse(
    JSON.stringify(original)
      .replaceAll(original.hearing.case_id, caseId)
      .replaceAll(original.hearing.id, state.deadline.id),
  );
  Object.assign(state, {
    creation,
    own: resourceHearingAgendaItem(creation, closed ? 'closed' : 'active'),
    detailCalls: [],
    accessCalls: [],
    order: [],
    denied: false,
    onExact: null,
  });
  state.rows.push(state.own);
  state.exactPath =
    `/api/v1/cases/${caseId}/procedural-resources/${creation.hearing.resource_id}` +
    `/activities/resource-hearings/${creation.hearing.id}/revisions/1`;
  await page.route(`**/api/v1/cases/${caseId}/administration`, async (route) => {
    state.accessCalls.push(route.request().url());
    state.order.push('administration');
    await route.fulfill(
      state.denied
        ? { status: 403, json: { error: { code: 'permission_denied' } } }
        : { json: administration(undefined, 2, null, closed ? 'closed' : 'active') },
    );
  });
  await page.route(`**${state.exactPath}`, async (route) => {
    state.detailCalls.push(route.request());
    state.order.push('exact');
    if (state.onExact) return state.onExact(route);
    await route.fulfill({ json: state.creation });
  });
  return state;
}

export async function openResourceHearingAgenda(page, state) {
  await openCombinedAgenda(page);
  await queryAgenda(page);
  await expect(ownCard(page, state)).toBeVisible();
}

export async function rendered(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}
