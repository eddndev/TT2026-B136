import { expect } from '@playwright/test';
import { setupResourceActivities, clone } from './resource-activities-helpers.mjs';
import { login, navigate, caseId } from './helpers.mjs';
import { overview, otherAdministration } from './case-administration-helpers.mjs';
import { alertRecord, alertInstant } from '../fixtures/alerts.mjs';
import { resourceActor } from '../fixtures/procedural-resource-unit.mjs';
import { activityCheckedAt } from '../fixtures/resource-activity-unit.mjs';
export { navigate, login, caseId };
export const related = (page) =>
  page.getByRole('region', { name: 'Recursos relacionados', exact: true });
export const item = (page, id) => related(page).locator(`[data-association-id="${id}"]`);
export const targetDetail = (page, kind) =>
  page.getByRole('region', {
    name: kind === 'hearing' ? 'Detalle de audiencia' : 'Detalle de plazo',
    exact: true,
  });
export const assocId = (n) => `e0000000-0000-4000-8000-${String(n).padStart(12, '0')}`;

export async function setupRelated(page, kind = 'hearing', options = {}) {
  const activities = await setupResourceActivities(page, options);
  const association = activities.seed(kind),
    source = activities[kind][0],
    current = activities[kind].at(-1);
  const alert = alertRecord(kind === 'hearing' ? 'upcoming' : 'review_required', kind);
  alert.recipient_id = resourceActor.id;
  alert.subject = { kind, case_id: caseId, id: current.id };
  alert.origin = {
    revision: current.revision,
    evidence_digest: current.receipt[kind === 'hearing' ? 'submission_digest' : 'capture_digest'],
  };
  if (kind === 'hearing') {
    alert.kind.activity_at = alertInstant(Date.parse(current.values.scheduled_at) / 1000);
    alert.trigger_at = alertInstant(alert.kind.activity_at.unix_seconds - 86400);
    alert.created_at = clone(alert.trigger_at);
  }
  const observedAlert = alertInstant(
    alert.created_at.unix_seconds + 1,
    alert.created_at.nanosecond,
  );
  const state = {
    activities,
    association,
    source,
    current,
    alert,
    kind,
    calls: [],
    alertCalls: [],
    handle: null,
  };
  state.seed = (count, unlinked = false) => {
    activities.records.clear();
    for (let n = 1; n <= count; n++) {
      const row = clone(association);
      row.id = assocId(n);
      row.receipt.operation_id = assocId(n + 100);
      activities.records.set(row.id, [row]);
    }
    if (unlinked) {
      const before = clone(association),
        row = clone(association);
      before.id = assocId(count + 1);
      row.id = before.id;
      row.revision = 2;
      row.status = 'unlinked';
      row.reason = 'Relacion retirada expresamente';
      row.receipt.action = 'unlink';
      row.receipt.expected_revision = 1;
      row.receipt.previous = { revision: 1, capture_digest: before.receipt.capture_digest };
      row.receipt.operation_id = assocId(199);
      row.receipt.capture_digest = 'd'.repeat(64);
      activities.records.set(row.id, [before, row]);
    }
  };
  state.page = (url) => {
    const status = url.searchParams.get('status') || 'linked';
    const after = url.searchParams.get('after_id'),
      limit = Number(url.searchParams.get('limit') || 20);
    const rows = [...activities.records.values()]
      .map((values) => values.at(-1))
      .filter((row) => (status === 'all' || row.status === status) && (!after || row.id > after))
      .sort((a, b) => a.id.localeCompare(b.id));
    const selected = rows.slice(0, limit),
      more = rows.length > limit;
    return {
      case_id: caseId,
      target: { kind, id: current.id },
      checked_at: clone(activityCheckedAt),
      associations: selected.map(activities.view),
      has_more: more,
      next_after_id: more ? selected.at(-1).id : null,
    };
  };
  await page.route('**/api/v1/case-administrations?*', (route) =>
    route.fulfill({
      json: {
        cases: [
          overview(activities.resources.facts.results.scheduling.admin),
          overview(otherAdministration()),
        ],
        has_more: false,
        next_after_id: null,
      },
    }),
  );
  await page.route('**/api/v1/alerts**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.alertCalls.push({ path: url.pathname, method: request.method() });
    if (request.method() !== 'GET')
      return route.fulfill({ status: 400, json: { error: { code: 'unexpected_mutation' } } });
    const detail = { checked_at: observedAlert, alert: clone(alert) };
    return route.fulfill({
      json: url.pathname.endsWith('/alerts')
        ? { checked_at: observedAlert, alerts: [clone(alert)], has_more: false, next_cursor: null }
        : detail,
    });
  });
  await page.route('**/api/v1/cases/*/*/*/resource-associations**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.calls.push({ path: url.pathname, search: url.search, method: request.method() });
    if (state.handle && (await state.handle(route, url))) return;
    return route.fulfill({ json: state.page(url) });
  });
  return state;
}
export async function openRelatedFromAlert(page, state) {
  await login(page, false, false);
  await navigate(page, 'Alertas');
  await page.getByRole('combobox', { name: 'Lectura', exact: true }).selectOption('unread');
  await page.getByRole('combobox', { name: 'Estado de alerta', exact: true }).selectOption('all');
  await page.getByRole('button', { name: 'Consultar alertas', exact: true }).click();
  await page
    .locator(`[data-alert-id="${state.alert.id}"]`)
    .getByRole('button', {
      name: state.kind === 'hearing' ? 'Abrir audiencia' : 'Abrir plazo',
      exact: true,
    })
    .click();
  await expect(targetDetail(page, state.kind)).toContainText(/consultada exactamente/i);
  await expect(related(page)).toBeVisible();
}
