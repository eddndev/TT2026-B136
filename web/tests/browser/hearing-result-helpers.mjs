import { expect } from '@playwright/test';
import { setupHearings, openHearings } from './hearing-helpers.mjs';
import { hearingRecord, hearingId, hearingCaseId } from '../fixtures/hearings.mjs';
import {
  resultAnchor,
  resultPrepared,
  resultRecord,
  resultRow,
  resultHistoryRow,
} from '../fixtures/hearing-results.mjs';
export const failResult = (route, code, status = 409) =>
  route.fulfill({ status, json: { error: { code } } });
export async function setupResults(
  page,
  { results = [], hearings = [hearingRecord()], ...options } = {},
) {
  const { state: scheduling } = await setupHearings(page, { ...options, records: hearings });
  const state = {
    scheduling,
    calls: [],
    submissions: [],
    records: new Map(),
    pageSize: 20,
    historySize: 10,
  };
  for (const row of results)
    state.records.set(row.id, [...(state.records.get(row.id) || []), structuredClone(row)]);
  state.directory = new Map(scheduling.participants.map((row) => [row.id, [structuredClone(row)]]));
  await page.route(`**/api/v1/cases/${hearingCaseId}/participants**`, async (route) => {
    const url = new URL(route.request().url()),
      rest = url.pathname.split('/participants')[1].split('/').filter(Boolean);
    if (scheduling.denied) return failResult(route, 'permission_denied', 403);
    if (!rest.length)
      return route.fulfill({
        json: {
          participants: [...state.directory.values()]
            .map((rows) => rows.at(-1))
            .filter((row) => row.display_name.includes(url.searchParams.get('name') || '')),
          has_more: false,
          next_after_id: null,
        },
      });
    const history = state.directory.get(rest[0]);
    if (!history) return failResult(route, 'participant_not_found', 404);
    if (rest[1] === 'history')
      return route.fulfill({
        json: { revisions: [...history].reverse(), has_more: false, next_before_revision: null },
      });
    const row =
      rest[1] === 'revisions'
        ? history.find((value) => value.revision === Number(rest[2]))
        : history.at(-1);
    return row ? route.fulfill({ json: row }) : failResult(route, 'participant_not_found', 404);
  });
  state.prepare = (command) => {
    const base = state.records.get(command.result_id)?.at(-1),
      change = command.change;
    const source = scheduling.records
      .get(command.hearing_id)
      ?.find((row) => row.revision === change.anchor_revision);
    let continuation = base?.continuation || null;
    if (change.action === 'record' && change.continuation) {
      const previous = state.records
        .get(change.continuation.result_id)
        ?.find((row) => row.revision === change.continuation.revision);
      continuation = {
        hearing_id: previous.hearing_id,
        result_id: previous.id,
        revision: previous.revision,
        values_digest: previous.values_digest,
        submission_digest: previous.receipt.submission_digest,
        status: previous.status,
      };
    }
    const prepared = resultPrepared(
      command,
      base?.anchor || resultAnchor(source),
      base,
      continuation,
    );
    if (change.action !== 'withdraw') {
      prepared.attendees = prepared.values.attendees.map((item) => {
        const person = state.directory
          .get(item.participant_id)
          .find((row) => row.revision === item.revision);
        return {
          ...structuredClone(person),
          profile: person.profile ? 'typed' : 'manual',
          kind: person.profile?.kind || null,
          subject_digest: person.subject?.values_digest || null,
          capacity: item.capacity,
          observation: item.observation,
        };
      });
      const support = prepared.values.provenance.support;
      prepared.support = support
        ? { ...support, name: 'contrato.pdf', format: 'pdf', policy: 'pdf_docx_v1' }
        : null;
    }
    return prepared;
  };
  state.commit = (prepared) => {
    const row = resultRecord(prepared);
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    return row;
  };
  await page.route('**/api/v1/cases/*/hearings/*/results**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    const match = /\/hearings\/([^/]+)\/results(.*)/.exec(path),
      hearing = match[1];
    const parts = match[2].split('/').filter(Boolean);
    const call = {
      path,
      search: url.search,
      method: request.method(),
      body: request.postDataJSON(),
    };
    state.calls.push(call);
    if (scheduling.denied || options.role === 'client')
      return failResult(route, 'permission_denied', 403);
    if (state.handle && (await state.handle(route, call))) return;
    if (request.method() !== 'GET') {
      if (options.role === 'paralegal') return failResult(route, 'permission_denied', 403);
      if (scheduling.context.administrative_status === 'closed')
        return failResult(route, 'case_closed');
      const command = parts[0] === 'prepare' ? call.body : call.body.command;
      const current = state.records.get(command.result_id)?.at(-1);
      if ((current?.revision || 0) !== command.change.expected_revision)
        return failResult(route, 'hearing_result_revision_conflict');
      if (current?.status === 'withdrawn')
        return failResult(route, 'hearing_result_already_withdrawn');
      const prepared = state.prepare(command);
      if (parts[0] === 'prepare') return route.fulfill({ json: prepared });
      state.submissions.push(command);
      return route.fulfill({ status: 201, json: state.commit(prepared) });
    }
    if (!parts.length) {
      const status = url.searchParams.get('status') || 'all',
        after = url.searchParams.get('after_id');
      let rows = [...state.records.values()]
        .map((history) => history.at(-1))
        .filter(
          (row) =>
            row.hearing_id === hearing &&
            (status === 'all' || row.status === status) &&
            (!after || row.id > after),
        );
      rows.sort((a, b) => a.id.localeCompare(b.id));
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: {
          results: rows.map(resultRow),
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        },
      });
    }
    const history = state.records.get(parts[0]);
    if (!history || history[0].hearing_id !== hearing)
      return failResult(route, 'hearing_result_not_found', 404);
    if (parts[1] === 'history') {
      let rows = [...history]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.has('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const more = rows.length > state.historySize;
      rows = rows.slice(0, state.historySize);
      return route.fulfill({
        json: {
          revisions: rows.map(resultHistoryRow),
          has_more: more,
          next_before_revision: more ? rows.at(-1).revision : null,
        },
      });
    }
    const row =
      parts[1] === 'revisions'
        ? history.find((row) => row.revision === Number(parts[2]))
        : history.at(-1);
    return row ? route.fulfill({ json: row }) : failResult(route, 'hearing_result_not_found', 404);
  });
  return state;
}
export const resultPanel = (page) =>
  page.getByRole('region', { name: 'Sesiones y resultados declarados', exact: true });
export const resultEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de sesi\u00f3n o acto', exact: true });
export const resultDetail = (page) =>
  page.getByRole('region', { name: 'Detalle del resultado declarado', exact: true });
export async function openResults(page, id = hearingId) {
  await openHearings(page);
  await page.getByRole('button', { name: `Consultar audiencia ${id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Ver sesiones y resultados', exact: true }).click();
  await expect(resultPanel(page)).toBeVisible();
}
export async function fillResult(page, summary = 'Sesion parcial comunicada') {
  const editor = resultEditor(page);
  await editor.getByRole('combobox', { name: 'Ocurrencia', exact: true }).selectOption('occurred');
  await editor
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('partial');
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await editor.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await editor.getByLabel('Relato del operador', { exact: true }).fill(summary);
  await editor
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('operator_note');
}
export async function confirmResult(page) {
  const editor = resultEditor(page);
  await editor.getByRole('button', { name: 'Revisar resultado', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar resultado', exact: true }).click();
}
