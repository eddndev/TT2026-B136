import { setupAlerts, openAlerts, alertCard, filterAlerts } from './alerts-helpers.mjs';
import { caseId } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { resourceHearingCreation } from '../fixtures/resource-hearing-unit.mjs';
import { resourceHearingAlert } from '../fixtures/resource-hearing-alert.mjs';

export { openAlerts, alertCard, filterAlerts };
export const ownDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia de recurso', exact: true });
export const openOwn = (page, state) =>
  alertCard(page, state.own)
    .getByRole('button', { name: 'Abrir audiencia de recurso', exact: true })
    .click();

export async function setupOwnAlerts(page, { closed = false } = {}) {
  const state = await setupAlerts(page);
  const original = resourceHearingCreation({ withAct: true });
  const creation = JSON.parse(
    JSON.stringify(original)
      .replaceAll(original.hearing.case_id, caseId)
      .replaceAll(original.hearing.id, state.deadline.id),
  );
  const own = resourceHearingAlert(creation);
  const ordinary = state.rows.find((row) => row.subject.kind === 'hearing');
  const deadline = state.rows.find(
    (row) => row.subject.kind === 'deadline' && row.kind.kind === 'upcoming',
  );
  ordinary.subject.id = own.subject.id;
  own.recipient_id = ordinary.recipient_id;
  state.rows = [ordinary, deadline, own].sort(
    (a, b) =>
      b.created_at.unix_seconds - a.created_at.unix_seconds ||
      b.created_at.nanosecond - a.created_at.nanosecond ||
      b.id.localeCompare(a.id),
  );
  Object.assign(state, {
    creation,
    own,
    order: [],
    ownCalls: [],
    unwanted: [],
    denied: false,
    onExact: null,
  });
  const base = `/api/v1/cases/${caseId}/procedural-resources/${own.subject.resource_id}`;
  state.exactPath = `${base}/activities/resource-hearings/${own.subject.id}/revisions/1`;
  state.handle = async (_route, url) => {
    if (url.pathname.endsWith(`/alerts/${own.id}`)) state.order.push('alert');
    return false;
  };
  await page.route(`**/api/v1/cases/${caseId}/administration`, async (route) => {
    state.order.push('administration');
    await route.fulfill(
      state.denied
        ? { status: 403, json: { error: { code: 'permission_denied' } } }
        : { json: administration(undefined, 2, null, closed ? 'closed' : 'active') },
    );
  });
  await page.route(`**${base}/activities**`, async (route) => {
    if (new URL(route.request().url()).pathname === state.exactPath) {
      state.order.push('exact');
      state.ownCalls.push(route.request());
      if (state.onExact) return state.onExact(route);
      return route.fulfill({ json: state.creation });
    }
    state.unwanted.push(route.request().url());
    await route.fulfill({ status: 404, json: { error: { code: 'resource_activity_not_found' } } });
  });
  page.on('request', (request) => {
    const path = new URL(request.url()).pathname;
    if (
      ['hearings', 'deadlines'].some((kind) =>
        path.startsWith(`/api/v1/cases/${caseId}/${kind}/${own.subject.id}`),
      )
    )
      state.unwanted.push(path);
  });
  return state;
}
