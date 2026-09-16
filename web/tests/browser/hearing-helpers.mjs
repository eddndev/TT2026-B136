import { expect } from '@playwright/test';
import { setup, login, navigate, caseId, caseRecord } from './helpers.mjs';
import { administration, overview, profile } from './case-administration-helpers.mjs';
import {
  hearingContext,
  hearingPrepared,
  hearingRecord,
  hearingRow,
} from '../fixtures/hearings.mjs';
import { participant } from './participant-helpers.mjs';
export { caseId, participant };
export async function setupHearings(
  page,
  { role = 'owner', records = [], stage = 'investigation', closed = false } = {},
) {
  const requests = await setup(page, role);
  const context = {
    ...structuredClone(hearingContext),
    stage,
    administrative_status: closed ? 'closed' : 'active',
  };
  const admin = administration(caseRecord, 1, profile, context.administrative_status);
  const state = {
    context,
    admin,
    calls: [],
    submissions: [],
    records: new Map(records.map((row) => [row.id, [structuredClone(row)]])),
    participants: [structuredClone(participant)],
    denied: false,
    pageSize: 20,
  };
  await page.route('**/api/v1/case-administrations?*', (route) =>
    route.fulfill({ json: { cases: [overview(admin)], has_more: false, next_after_id: null } }),
  );
  await page.route(`**/api/v1/cases/${caseId}/administration`, (route) =>
    route.fulfill({ json: admin }),
  );
  await page.route(`**/api/v1/cases/${caseId}/participants**`, (route) => {
    const url = new URL(route.request().url()),
      rest = url.pathname.split('/participants')[1].split('/').filter(Boolean);
    if (state.denied)
      return route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
    if (!rest.length)
      return route.fulfill({
        json: {
          participants: state.participants.filter((row) => row.directory_status === 'active'),
          has_more: false,
          next_after_id: null,
        },
      });
    const row = state.participants.find((value) => value.id === rest[0]);
    return row
      ? route.fulfill({ json: row })
      : route.fulfill({ status: 404, json: { error: { code: 'participant_not_found' } } });
  });
  const failure = (route, code, status = 409) =>
    route.fulfill({ status, json: { error: { code } } });
  state.commit = (prepared) => {
    const row = hearingRecord(prepared),
      previous = state.records.get(row.id)?.at(-1);
    row.participants = row.values.participants.map((ref) => {
      const source = state.participants.find((item) => item.id === ref.participant_id);
      return {
        id: ref.participant_id,
        revision: ref.revision,
        profile: source.profile ? 'typed' : 'manual',
        display_name: source.display_name,
        procedural_role: source.procedural_role,
        kind: source.profile?.kind || null,
        subject: source.subject
          ? {
              id: source.subject.id,
              revision: source.subject.revision,
              values_digest: source.subject.values_digest,
            }
          : null,
        values_digest: source.values_digest,
        directory_status: source.directory_status,
      };
    });
    if (row.values.conviction_basis)
      row.support = {
        ...row.values.conviction_basis.support,
        name: 'contrato.pdf',
        format: 'pdf',
        policy: 'pdf_docx_v1',
      };
    row.scheduling_context =
      prepared.command.change.action === 'cancel'
        ? previous.scheduling_context
        : {
            administration_revision: state.context.case_revision,
            administration_digest: state.context.case_values_digest,
            stage_revision: state.context.stage_revision,
            stage: state.context.stage,
            stage_digest: state.context.stage_values_digest,
          };
    if (previous && prepared.command.change.action === 'cancel')
      row.participants = previous.participants;
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    return row;
  };
  await page.route('**/api/v1/**hearings**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    state.calls.push({
      path,
      search: url.search,
      method: request.method(),
      body: request.postDataJSON(),
    });
    if (state.denied || role === 'client') return failure(route, 'permission_denied', 403);
    if (request.method() !== 'GET' && role === 'paralegal')
      return failure(route, 'permission_denied', 403);
    if (state.handle && (await state.handle(route, state.calls.at(-1)))) return;
    if (path === '/api/v1/hearings') {
      const from = Date.parse(url.searchParams.get('from')),
        until = Date.parse(url.searchParams.get('until'));
      let rows = [...state.records.values()]
        .map((history) => history.at(-1))
        .filter(
          (row) =>
            Date.parse(row.values.scheduled_at) >= from &&
            Date.parse(row.values.scheduled_at) < until,
        );
      const status = url.searchParams.get('status') || 'scheduled';
      rows = rows.filter((row) => status === 'all' || row.status === status);
      rows.sort(
        (a, b) =>
          Date.parse(a.values.scheduled_at) - Date.parse(b.values.scheduled_at) ||
          a.id.localeCompare(b.id),
      );
      if (url.searchParams.has('after_id')) {
        const at = Date.parse(url.searchParams.get('after_time')),
          id = url.searchParams.get('after_id');
        rows = rows.filter(
          (row) =>
            Date.parse(row.values.scheduled_at) > at ||
            (Date.parse(row.values.scheduled_at) === at && row.id > id),
        );
      }
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      const last = rows.at(-1);
      return route.fulfill({
        json: {
          hearings: rows.map((row) => ({
            ...hearingRow(row),
            case_status: state.context.administrative_status,
          })),
          has_more: more,
          next_after: more
            ? {
                at: new Date(last.values.scheduled_at).toISOString().replace('.000Z', 'Z'),
                id: last.id,
              }
            : null,
        },
      });
    }
    const parts = path.split('/hearings')[1].split('/').filter(Boolean);
    if (parts[0] === 'context') return route.fulfill({ json: state.context });
    if (request.method() !== 'GET') {
      if (state.context.administrative_status === 'closed') return failure(route, 'case_closed');
      const command =
        parts[0] === 'prepare' ? request.postDataJSON() : request.postDataJSON().command;
      const current = state.records.get(command.hearing_id)?.at(-1);
      if ((current?.revision || 0) !== command.change.expected_revision)
        return failure(route, 'hearing_revision_conflict');
      const prepared = hearingPrepared(command);
      if (command.change.action === 'cancel') prepared.values = structuredClone(current.values);
      if (parts[0] === 'prepare') return route.fulfill({ json: prepared });
      state.submissions.push(command);
      return route.fulfill({ status: 201, json: state.commit(prepared) });
    }
    if (!parts.length) {
      const status = url.searchParams.get('status') || 'all',
        after = url.searchParams.get('after_id');
      let rows = [...state.records.values()]
        .map((history) => history.at(-1))
        .filter((row) => (status === 'all' || row.status === status) && (!after || row.id > after));
      rows.sort((a, b) => a.id.localeCompare(b.id));
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: {
          hearings: rows.map(hearingRow),
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        },
      });
    }
    const history = state.records.get(parts[0]);
    if (!history) return failure(route, 'hearing_not_found', 404);
    if (parts[1] === 'history') {
      let rows = [...history]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.has('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: {
          revisions: rows,
          has_more: more,
          next_before_revision: more ? rows.at(-1).revision : null,
        },
      });
    }
    const row =
      parts[1] === 'revisions'
        ? history.find((item) => item.revision === Number(parts[2]))
        : history.at(-1);
    return row ? route.fulfill({ json: row }) : failure(route, 'hearing_not_found', 404);
  });
  return { state, requests };
}
export async function openHearings(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Audiencias del expediente', exact: true }),
  ).toBeVisible();
}
export const hearingEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de audiencia', exact: true });
export async function fillHearing(page) {
  const editor = hearingEditor(page);
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-10-01');
  await editor.getByLabel('Hora', { exact: true }).fill('09:02:03');
  await editor.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await editor.getByLabel('Sede o conexi\u00f3n', { exact: true }).fill('Sala privada declarada');
}
export async function confirmHearing(page) {
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
}
