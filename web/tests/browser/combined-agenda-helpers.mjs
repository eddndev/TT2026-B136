import { expect } from '@playwright/test';
import { setupDeadlines } from './deadline-editor-helpers.mjs';
import { setupHearings } from './hearing-helpers.mjs';
import { login, navigate, caseId } from './helpers.mjs';
import { hearingRecord, hearingRow } from '../fixtures/hearings.mjs';
import { ids, timedPrepared, v2Record, summary } from '../fixtures/deadline-v2-unit.mjs';

const clone = (value) => structuredClone(value);
export const checkedAt = { unix_seconds: 1767225600, nanosecond: 123456789, offset_seconds: 0 };
export const inCase = (value) => JSON.parse(JSON.stringify(value).replaceAll(ids(1), caseId));

export function agendaExamples() {
  const deadline = inCase(v2Record(timedPrepared()));
  deadline.definition.title = 'Vencimiento combinado';
  deadline.operational = {
    freshness: 'current',
    checked_at: clone(checkedAt),
    changed_dependencies: [],
    due_at: clone(deadline.calculation.result.due_at),
  };
  const hearing = hearingRecord();
  hearing.id = deadline.id;
  hearing.values.scheduled_at = '2026-01-02T00:00:00Z';
  const hearingItem = {
    kind: 'hearing',
    at: { unix_seconds: 1767312000, nanosecond: 0, offset_seconds: 0 },
    hearing: hearingRow(hearing),
  };
  const deadlineItem = {
    kind: 'deadline',
    at: clone(deadline.operational.due_at),
    case_title: 'Defensa inicial',
    case_reference: 'NUC-123',
    case_status: 'active',
    deadline: summary(deadline),
  };
  return { hearing, deadline, hearingItem, deadlineItem };
}

export function agendaPage(url, items, complete = true, next = null) {
  return {
    from: url.searchParams.get('from'),
    until: url.searchParams.get('until'),
    kind: url.searchParams.get('kind') || 'all',
    hearing_status: url.searchParams.get('hearing_status') || 'scheduled',
    checked_at: clone(checkedAt),
    items: clone(items),
    complete,
    next_cursor: next,
  };
}

export function cursorFor(url, seconds, nanos, rank, id) {
  return [
    'a1',
    Date.parse(url.searchParams.get('from')) / 1000,
    Date.parse(url.searchParams.get('until')) / 1000,
    url.searchParams.get('kind') || 'all',
    url.searchParams.get('hearing_status') || 'scheduled',
    seconds,
    nanos,
    rank,
    id,
  ].join(':');
}

export async function setupCombinedAgenda(page) {
  const examples = agendaExamples();
  const deadlines = await setupDeadlines(page, { deadlines: [examples.deadline] });
  deadlines.facts.results.scheduling.records.set(examples.hearing.id, [clone(examples.hearing)]);
  const state = {
    ...examples,
    deadlines,
    calls: [],
    rows: [examples.hearingItem, examples.deadlineItem],
  };
  await page.route('**/api/v1/agenda?*', async (route) => {
    const url = new URL(route.request().url());
    state.calls.push(url);
    if (state.handle && (await state.handle(route, url))) return;
    const from = Date.parse(url.searchParams.get('from')) / 1000;
    const until = Date.parse(url.searchParams.get('until')) / 1000;
    const kind = url.searchParams.get('kind') || 'all';
    const status = url.searchParams.get('hearing_status') || 'scheduled';
    const rows = state.rows.filter(
      (item) =>
        item.at.unix_seconds >= from &&
        item.at.unix_seconds < until &&
        (kind === 'all' || item.kind === kind) &&
        (item.kind !== 'hearing' || status === 'all' || item.hearing.status === status),
    );
    await route.fulfill({ json: agendaPage(url, rows) });
  });
  return state;
}

export async function openCombinedAgenda(page) {
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toBeVisible();
}

export async function queryAgenda(page, view = 'day', date = '2026-01-02') {
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption(view);
  await page.getByLabel('Fecha de referencia', { exact: true }).fill(date);
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
}

export const activity = (page, kind, id) =>
  page.getByRole('button', {
    name: `Consultar ${kind === 'hearing' ? 'audiencia' : 'plazo'} ${id}`,
    exact: true,
  });

export async function setupHearingAgenda(page, records) {
  const result = await setupHearings(page, { records });
  const agenda = { calls: [] };
  await page.route('**/api/v1/agenda?*', async (route) => {
    const url = new URL(route.request().url());
    agenda.calls.push(url);
    if (agenda.handle && (await agenda.handle(route, url))) return;
    const from = Date.parse(url.searchParams.get('from')) / 1000;
    const until = Date.parse(url.searchParams.get('until')) / 1000;
    const status = url.searchParams.get('hearing_status') || 'scheduled';
    const cursor = url.searchParams.get('cursor')?.split(':');
    let rows = [...result.state.records.values()]
      .map((history) => history.at(-1))
      .map((row) => ({
        kind: 'hearing',
        hearing: hearingRow(row),
        at: {
          unix_seconds: Date.parse(row.values.scheduled_at) / 1000,
          nanosecond: 0,
          offset_seconds: 0,
        },
      }))
      .filter(
        (item) =>
          item.at.unix_seconds >= from &&
          item.at.unix_seconds < until &&
          (status === 'all' || item.hearing.status === status) &&
          (!cursor ||
            item.at.unix_seconds > Number(cursor[5]) ||
            (item.at.unix_seconds === Number(cursor[5]) && item.hearing.id > cursor[8])),
      )
      .sort(
        (a, b) =>
          a.at.unix_seconds - b.at.unix_seconds ||
          (a.hearing.id < b.hearing.id ? -1 : a.hearing.id > b.hearing.id ? 1 : 0),
      );
    if (url.searchParams.get('kind') === 'deadline') rows = [];
    const complete = rows.length <= result.state.pageSize;
    rows = rows.slice(0, result.state.pageSize);
    const last = rows.at(-1);
    const next = complete ? null : cursorFor(url, last.at.unix_seconds, 0, 0, last.hearing.id);
    await route.fulfill({ json: agendaPage(url, rows, complete, next) });
  });
  return { ...result, agenda };
}

export async function queryHearingAgenda(page, from = '2026-10-01', until = '2026-10-03') {
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('custom');
  await page.getByLabel('Desde (incluido)', { exact: true }).fill(from);
  await page.getByLabel('Hasta (excluido)', { exact: true }).fill(until);
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('+00:00');
  await page
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('hearing');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
}
