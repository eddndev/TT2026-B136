import { expect } from '@playwright/test';
import { setupDeadlines } from './deadline-editor-helpers.mjs';
import { login, navigate, caseId } from './helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { ids, v2Record, timedPrepared } from '../fixtures/deadline-v2-unit.mjs';
import {
  alertRecord,
  alertPreferences,
  alertInstant,
  alertCheckedAt,
  alertCursor,
  clone,
} from '../fixtures/alerts.mjs';

const actorId = ids(4);
const byCreated = (a, b) =>
  b.created_at.unix_seconds - a.created_at.unix_seconds ||
  b.created_at.nanosecond - a.created_at.nanosecond ||
  (a.id < b.id ? 1 : a.id > b.id ? -1 : 0);

function examples() {
  const deadline = JSON.parse(JSON.stringify(v2Record(timedPrepared())).replaceAll(ids(1), caseId));
  const hearing = hearingRecord();
  hearing.values.scheduled_at = '2026-01-02T01:00:00Z';
  const rows = [
    'upcoming',
    'upcoming',
    'review_required',
    'overdue_unattended',
    'due_changed_soon',
  ].map((kind, index) => {
    const row = alertRecord(kind, index === 0 ? 'hearing' : 'deadline');
    row.id = `40000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`;
    row.occurrence_id = `50000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`;
    row.recipient_id = actorId;
    row.subject.case_id = caseId;
    row.subject.id = index === 0 ? hearing.id : index === 1 ? deadline.id : ids(100 + index);
    row.origin.revision = 1;
    row.created_at.unix_seconds += index;
    if (index === 0) {
      row.kind.activity_at.nanosecond = 0;
      row.trigger_at.nanosecond = 0;
      row.created_at.nanosecond = 0;
    }
    if (index === 1) {
      const due = { ...deadline.calculation.result.due_at, offset_seconds: 0 };
      row.kind.activity_at = due;
      row.trigger_at = { ...due, unix_seconds: due.unix_seconds - 86400 };
      row.created_at = clone(row.trigger_at);
    }
    return row;
  });
  return { deadline, hearing, rows: rows.sort(byCreated) };
}

export function alertPageFor(rows, has_more = false, next_cursor = null) {
  return { checked_at: clone(alertCheckedAt), alerts: clone(rows), has_more, next_cursor };
}

export async function setupAlerts(page, { role = 'owner' } = {}) {
  const data = examples();
  const deadlines = await setupDeadlines(page, { deadlines: [data.deadline], role });
  deadlines.facts.results.scheduling.records.set(data.hearing.id, [clone(data.hearing)]);
  const preferences = alertPreferences().preferences;
  preferences.user_id = actorId;
  const state = {
    ...data,
    deadlines,
    preferences,
    calls: [],
    pageSize: 20,
    preferenceConflict: false,
    loseReadResponse: false,
  };
  await page.route(/\/api\/v1\/(alerts(?:[/?]|$)|alert-preferences(?:\?|$))/, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const body = request.postDataJSON(),
      method = request.method();
    state.calls.push({ path: url.pathname, search: url.search, method, body });
    if (role === 'client')
      return route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
    if (state.handle && (await state.handle(route, url))) return;
    if (url.pathname.endsWith('/alert-preferences')) {
      if (method === 'PUT') {
        if (state.preferenceConflict) {
          state.preferenceConflict = false;
          state.preferences.revision = 1;
          state.preferences.values.hearing_upcoming.lead_hours = [24];
          state.preferences.updated_at = alertInstant(1767230000);
          state.preferences.receipt = { operation_id: ids(88), expected_revision: 0 };
          return route.fulfill({
            status: 409,
            json: { error: { code: 'alert_revision_conflict' } },
          });
        }
        if (body.expected_revision !== state.preferences.revision)
          return route.fulfill({
            status: 409,
            json: { error: { code: 'alert_revision_conflict' } },
          });
        state.preferences = {
          ...state.preferences,
          revision: body.expected_revision + 1,
          values: clone(body.values),
          updated_at: alertInstant(1767230100),
          receipt: { operation_id: body.operation_id, expected_revision: body.expected_revision },
        };
      }
      return route.fulfill({ json: { preferences: state.preferences } });
    }
    const tail = url.pathname.split('/alerts')[1].split('/').filter(Boolean);
    if (!tail.length) {
      const read = url.searchParams.get('read') || 'all',
        filter = url.searchParams.get('state') || 'active';
      const cursor = url.searchParams.get('cursor')?.split(':');
      let rows = state.rows.filter(
        (row) =>
          (read !== 'unread' || row.read_at === null) &&
          (filter !== 'active' || row.state.kind === 'active') &&
          (!cursor ||
            row.created_at.unix_seconds < Number(cursor[1]) ||
            (row.created_at.unix_seconds === Number(cursor[1]) &&
              (row.created_at.nanosecond < Number(cursor[2]) ||
                (row.created_at.nanosecond === Number(cursor[2]) && row.id < cursor[3])))),
      );
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: alertPageFor(rows, more, more ? alertCursor(rows.at(-1), read, filter) : null),
      });
    }
    const row = state.rows.find((item) => item.id === tail[0]);
    if (!row) return route.fulfill({ status: 404, json: { error: { code: 'alert_not_found' } } });
    const detail = { checked_at: clone(alertCheckedAt), alert: row };
    if (tail[1] === 'read') {
      row.read_at ??= alertInstant(1767230200, 123);
      if (state.loseReadResponse) {
        state.loseReadResponse = false;
        return route.fulfill({ status: 503, json: { error: { code: 'service_unavailable' } } });
      }
      return route.fulfill({ json: { operation_id: body.operation_id, ...detail } });
    }
    return route.fulfill({ json: detail });
  });
  return state;
}

export async function openAlerts(page) {
  await login(page, false, false);
  await navigate(page, 'Alertas');
  await expect(page.getByRole('region', { name: 'Mis alertas', exact: true })).toBeVisible();
}
export const alertCard = (page, row) => page.locator(`[data-alert-id="${row.id}"]`);
export async function filterAlerts(page, read = 'all', state = 'active') {
  await page.getByRole('combobox', { name: 'Lectura', exact: true }).selectOption(read);
  await page.getByRole('combobox', { name: 'Estado de alerta', exact: true }).selectOption(state);
  await page.getByRole('button', { name: 'Consultar alertas', exact: true }).click();
}
