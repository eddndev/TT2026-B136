import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import {
  setupCombinedAgenda,
  openCombinedAgenda,
  queryAgenda,
  agendaPage,
} from './combined-agenda-helpers.mjs';
import {
  setupAlerts,
  openAlerts,
  alertCard,
  filterAlerts,
  alertPageFor,
} from './alerts-helpers.mjs';
import {
  precautionaryCaseId,
  precautionaryHearingId,
  precautionaryHearingOperation,
  precautionaryHearingOverview,
  precautionaryHearingAlert,
} from '../fixtures/precautionary-hearing-unit.mjs';

export { queryAgenda, openAlerts, alertCard, filterAlerts };
export const precautionaryDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia cautelar', exact: true });
export const precautionaryCard = (page, state) =>
  page.getByRole('button', {
    name: `Consultar audiencia cautelar ${state.selected.capture.review.command.hearing_id}`,
    exact: true,
  });
export const openPrecautionaryAlert = (page, state) =>
  alertCard(page, state.own)
    .getByRole('button', { name: 'Abrir audiencia cautelar', exact: true })
    .click();
export const closePrecautionaryDetail = (page) =>
  precautionaryDetail(page)
    .getByRole('button', { name: 'Cerrar detalle de audiencia', exact: true })
    .click();
export const rendered = (page) =>
  page.evaluate(
    () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))),
  );

function operation(revision, id) {
  return JSON.parse(
    JSON.stringify(precautionaryHearingOperation({ revision }))
      .replaceAll(precautionaryCaseId, caseId)
      .replaceAll(precautionaryHearingId, id),
  );
}

async function installReads(page, state, revision, closed) {
  Object.assign(state, {
    selected: operation(revision, state.deadline.id),
    head: operation(3, state.deadline.id),
    order: [],
    detailCalls: [],
    headCalls: [],
    unwanted: [],
    accessStatus: 200,
    detailStatus: 200,
    onExact: null,
  });
  state.basePath = `/api/v1/cases/${caseId}/precautionary-hearings/${state.deadline.id}`;
  state.exactPath = `${state.basePath}/revisions/${revision}`;
  await page.route(`**/api/v1/cases/${caseId}/administration`, async (route) => {
    state.order.push('administration');
    await route.fulfill(
      state.accessStatus === 200
        ? { json: administration(undefined, 2, null, closed ? 'closed' : 'active') }
        : { status: state.accessStatus, json: { error: { code: 'permission_denied' } } },
    );
  });
  await page.route(`**${state.basePath}**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    if (request.method() !== 'GET' || url.search) {
      state.unwanted.push(request.url());
      return route.fulfill({ status: 400, json: { error: { code: 'invalid_request' } } });
    }
    if (url.pathname === state.exactPath) {
      state.order.push('exact');
      state.detailCalls.push(request);
      if (state.onExact) return state.onExact(route);
      return route.fulfill(
        state.detailStatus === 200
          ? { json: state.selected }
          : {
              status: state.detailStatus,
              json: { error: { code: 'precautionary_hearing_not_found' } },
            },
      );
    }
    if (url.pathname === state.basePath) {
      state.headCalls.push(request);
      return route.fulfill({ json: state.head });
    }
    state.unwanted.push(request.url());
    return route.fulfill({
      status: 404,
      json: { error: { code: 'precautionary_hearing_not_found' } },
    });
  });
  page.on('request', (request) => {
    const path = new URL(request.url()).pathname;
    if (
      ['hearings', 'deadlines'].some((family) =>
        path.startsWith(`/api/v1/cases/${caseId}/${family}/${state.deadline.id}`),
      )
    )
      state.unwanted.push(path);
  });
}

export async function setupPrecautionaryAgenda(page, { closed = false } = {}) {
  const state = await setupCombinedAgenda(page);
  await installReads(page, state, 3, closed);
  const row = precautionaryHearingOverview(state.selected);
  state.own = {
    kind: 'precautionary_hearing',
    at: { unix_seconds: Date.parse(row.scheduled_at) / 1000, nanosecond: 0, offset_seconds: 0 },
    case_title: 'Defensa inicial',
    case_reference: 'NUC-123',
    case_status: closed ? 'closed' : 'active',
    precautionary_hearing: row,
  };
  state.rows.push(state.own);
  state.handle = async (route, url) => {
    const from = Date.parse(url.searchParams.get('from')) / 1000,
      until = Date.parse(url.searchParams.get('until')) / 1000,
      kind = url.searchParams.get('kind') || 'all',
      status = url.searchParams.get('hearing_status') || 'scheduled';
    const rows = state.rows.filter(
      (item) =>
        item.at.unix_seconds >= from &&
        item.at.unix_seconds < until &&
        (kind === 'all' || item.kind === kind) &&
        (item.kind === 'deadline' || status === 'all' || item[item.kind].status === status),
    );
    await route.fulfill({ json: agendaPage(url, rows) });
    return true;
  };
  return state;
}

export async function openPrecautionaryAgenda(page, state) {
  await openCombinedAgenda(page);
  await page
    .getByRole('combobox', { name: 'Estado de audiencia', exact: true })
    .selectOption('all');
  await queryAgenda(page, 'week', '2026-01-02');
  await expect(precautionaryCard(page, state)).toBeVisible();
}

export async function setupPrecautionaryAlerts(page, { closed = false } = {}) {
  const state = await setupAlerts(page);
  await installReads(page, state, 2, closed);
  state.own = precautionaryHearingAlert(state.selected);
  state.own.recipient_id = state.rows[0].recipient_id;
  const ordinary = state.rows.find((row) => row.subject.kind === 'hearing');
  ordinary.subject.id = state.own.subject.id;
  const deadline = state.rows.find(
    (row) => row.subject.kind === 'deadline' && row.kind.kind === 'upcoming',
  );
  state.rows = [state.own, ordinary, deadline].sort(
    (a, b) =>
      b.created_at.unix_seconds - a.created_at.unix_seconds ||
      b.created_at.nanosecond - a.created_at.nanosecond ||
      b.id.localeCompare(a.id),
  );
  const checked = {
    unix_seconds: Date.parse(state.head.capture.recorded_at) / 1000 + 1,
    nanosecond: 0,
    offset_seconds: 0,
  };
  state.handle = async (route, url) => {
    if (url.pathname.endsWith(`/alerts/${state.own.id}`)) {
      state.order.push('alert');
      await route.fulfill({ json: { checked_at: checked, alert: state.own } });
      return true;
    }
    if (url.pathname === '/api/v1/alerts') {
      const rows = state.rows.filter(
        (row) =>
          (url.searchParams.get('read') !== 'unread' || row.read_at === null) &&
          (url.searchParams.get('state') !== 'active' || row.state.kind === 'active'),
      );
      await route.fulfill({ json: { ...alertPageFor(rows), checked_at: checked } });
      return true;
    }
    return false;
  };
  return state;
}

export function holdPrecautionaryDetail(state) {
  const gate = { entered: false, completed: false };
  const promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.onExact = async (route) => {
    gate.entered = true;
    await promise;
    await route.fulfill({ json: state.selected });
    gate.completed = true;
  };
  return gate;
}
