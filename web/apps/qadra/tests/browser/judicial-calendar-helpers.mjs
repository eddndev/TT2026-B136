import { expect } from '@playwright/test';
import { setup, login } from './helpers.mjs';
import {
  calendarPrepared,
  calendarRecord,
  calendarOverview,
  calendarHistoryRow,
  calendarDayRows,
} from '../fixtures/judicial-calendars.mjs';
export async function setupCalendars(page, { role = 'owner', records = [calendarRecord()] } = {}) {
  const common = await setup(page, role);
  const state = {
    calls: [],
    submissions: [],
    records: new Map(),
    denied: false,
    listLimit: 20,
    historyLimit: 10,
  };
  for (const row of records)
    state.records.set(row.id, [...(state.records.get(row.id) || []), structuredClone(row)]);
  state.prepare = (command) => {
    const base = state.records.get(command.calendar_id)?.at(-1),
      p = calendarPrepared(command, base);
    p.initial_scope = structuredClone(base?.values.scope || p.values.scope);
    return p;
  };
  state.commit = (prepared) => {
    const row = calendarRecord(prepared);
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    state.submissions.push(structuredClone(prepared.command));
    return row;
  };
  await page.route('**/api/v1/judicial-calendars**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      rest = url.pathname.split('/judicial-calendars')[1].split('/').filter(Boolean);
    state.calls.push({ method: request.method(), path: url.pathname, search: url.search });
    const failure = (code, status) => route.fulfill({ status, json: { error: { code } } });
    if (state.denied || role === 'client' || (request.method() !== 'GET' && role !== 'owner'))
      return failure('permission_denied', 403);
    if (request.method() !== 'GET') {
      const payload = request.postDataJSON(),
        command = rest[0] === 'prepare' ? payload : payload.command,
        base = state.records.get(command.calendar_id)?.at(-1);
      if ((base?.revision || 0) !== command.change.expected_revision)
        return failure('judicial_calendar_revision_conflict', 409);
      if (base?.status === 'retired') return failure('judicial_calendar_retired', 409);
      const prepared = state.prepare(command);
      return route.fulfill({
        status: rest[0] === 'prepare' ? 200 : 201,
        json: rest[0] === 'prepare' ? prepared : state.commit(prepared),
      });
    }
    if (!rest.length) {
      const q = url.searchParams,
        status = q.get('status') || 'published',
        limit = Math.min(Number(q.get('limit') || 20), state.listLimit),
        after = q.get('after_id');
      let rows = [...state.records.values()]
        .map((rs) => rs.at(-1))
        .sort((a, b) => a.id.localeCompare(b.id));
      rows = rows.filter(
        (r) =>
          (status === 'all' || r.status === status) &&
          (!q.get('jurisdiction') || r.values.scope.jurisdiction === q.get('jurisdiction')) &&
          (!q.get('entity_code') || r.values.scope.entity_codes.includes(q.get('entity_code'))) &&
          (!after || r.id > after),
      );
      const has_more = rows.length > limit;
      rows = rows.slice(0, limit);
      return route.fulfill({
        json: {
          calendars: rows.map(calendarOverview),
          has_more,
          next_after_id: has_more ? rows.at(-1).id : null,
        },
      });
    }
    const history = state.records.get(rest[0]);
    if (!history) return failure('judicial_calendar_not_found', 404);
    if (rest[1] === 'history') {
      const before = Number(url.searchParams.get('before_revision') || Infinity),
        limit = Math.min(Number(url.searchParams.get('limit') || 10), state.historyLimit),
        rows = [...history].reverse().filter((r) => r.revision < before),
        page = rows.slice(0, limit),
        has_more = rows.length > limit;
      return route.fulfill({
        json: {
          revisions: page.map(calendarHistoryRow),
          has_more,
          next_before_revision: has_more ? page.at(-1).revision : null,
        },
      });
    }
    const row =
      rest[1] === 'revisions'
        ? history.find((r) => r.revision === Number(rest[2]))
        : history.at(-1);
    if (!row) return failure('judicial_calendar_not_found', 404);
    if (rest[3] === 'days')
      return route.fulfill({
        json: {
          calendar_id: row.id,
          revision: row.revision,
          values_digest: row.values_digest,
          days: calendarDayRows(
            row.values,
            url.searchParams.get('from'),
            url.searchParams.get('through'),
          ),
        },
      });
    return route.fulfill({ json: row });
  });
  return { state, common };
}
export const calendarPanel = (page) =>
  page.getByRole('region', { name: 'Cat\u00e1logo de calendarios', exact: true });
export const calendarEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de calendario', exact: true });
export const calendarDetail = (page) =>
  page.getByRole('region', { name: 'Detalle del calendario', exact: true });
export async function openCalendars(page) {
  await login(page, false, false);
  await page.evaluate(() => {
    location.hash = 'judicial-calendars';
  });
  await expect(
    page.getByRole('heading', { name: 'Calendarios jurisdiccionales', exact: true }),
  ).toBeVisible();
}
export async function fillCalendar(page) {
  const editor = calendarEditor(page);
  await editor
    .getByLabel('T\u00edtulo del calendario', { exact: true })
    .fill('Calendario declarado');
  await editor.getByRole('combobox', { name: 'Fuero', exact: true }).selectOption('local');
  await editor.getByLabel('01 Aguascalientes', { exact: true }).check();
  for (const label of ['Autoridad', '\u00d3rgano', 'Territorio'])
    await editor.getByLabel(label, { exact: true }).fill(`${label} declarado`);
  await editor
    .getByLabel('Uso declarado', { exact: true })
    .fill('Clasificacion civil declarada para consulta');
  await editor.getByLabel('Cobertura desde', { exact: true }).fill('2000-02-01');
  await editor.getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-31');
  const weekdays = [
    'Lunes',
    'Martes',
    'Mi\u00e9rcoles',
    'Jueves',
    'Viernes',
    'S\u00e1bado',
    'Domingo',
  ];
  for (const day of weekdays) {
    const rule = editor.getByRole('group', { name: `Regla de ${day}`, exact: true });
    await rule
      .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
      .selectOption('unresolved');
    await rule.getByLabel('Explicaci\u00f3n', { exact: true }).fill('Requiere fuente aplicable');
  }
}
export async function confirmCalendar(page) {
  const editor = calendarEditor(page);
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await expect(editor).toHaveCount(0);
}
