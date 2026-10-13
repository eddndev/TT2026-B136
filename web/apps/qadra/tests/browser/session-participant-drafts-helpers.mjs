import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { participant, participantId, otherParticipantId } from './participant-helpers.mjs';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';

export { participant, participantId };
export const participantsPath = `/api/v1/cases/${caseId}/participants`;
export const participantPath = `${participantsPath}/${participantId}`;
export const fieldLabels = [
  'Nombre del participante',
  'Rol en el expediente',
  'Organizaci\u00f3n (opcional)',
  'Situaci\u00f3n jur\u00eddica registrada (opcional)',
];
export const rawFields = [
  '  Ficha manual todavia parcial  ',
  '   ',
  '  Organizacion por confirmar  ',
  '  Situacion sin terminar  ',
];
const states = new WeakMap();

export async function participantDraftSetup(page) {
  const state = await sessionSetup(page);
  state.records = new Map([[participantId, [structuredClone(participant)]]]);
  state.confirmedWrites = [];
  state.allowWrite = null;
  states.set(page, state);
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    if (!/^\/api\/v1\/cases\/[^/]+\/participants(?:\/|$)/.test(url.pathname))
      return route.fallback();
    const call = {
      path: url.pathname,
      method: request.method(),
      body: request.postData(),
      headers: request.headers(),
      search: url.search,
      at: state.now,
    };
    state.calls.push(call);
    const reply = (json, status = 200) =>
      route.fulfill({
        status,
        json,
        headers: { 'Cache-Control': 'no-store' },
      });
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    )
      return reply({ error: { code: 'invalid_session' } }, 401);
    if (!call.path.startsWith(participantsPath) || !state.allowed)
      return reply({ error: { code: 'case_not_found' } }, 404);
    if (!['owner', 'litigator', 'paralegal'].includes(state.current.user.role))
      return reply({ error: { code: 'permission_denied' } }, 403);
    const parts = call.path.slice(participantsPath.length).split('/').filter(Boolean);
    const history = state.records.get(parts[0]);
    if (
      call.method === 'GET' &&
      call.body === null &&
      parts.length <= 2 &&
      (parts.length < 2 || parts[1] === 'history')
    ) {
      if (state.gate?.path === call.path) {
        state.gate.entered = true;
        await state.gate.promise;
      }
      if (!parts.length) {
        const status = url.searchParams.get('status') || 'active';
        let rows = [...state.records.values()]
          .map((records) => records.at(-1))
          .filter(
            (row) =>
              (status === 'all' || row.directory_status === status) &&
              row.display_name.includes(url.searchParams.get('name') || '') &&
              (!url.searchParams.get('procedural_role') ||
                row.procedural_role === url.searchParams.get('procedural_role')) &&
              (!url.searchParams.get('after_id') || row.id > url.searchParams.get('after_id')) &&
              url.searchParams.get('profile') !== 'typed',
          );
        rows.sort((a, b) => a.id.localeCompare(b.id));
        const limit = Number(url.searchParams.get('limit') || 50),
          more = rows.length > limit;
        rows = rows.slice(0, limit);
        return reply({
          participants: rows,
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        });
      }
      if (!history) return reply({ error: { code: 'participant_not_found' } }, 404);
      if (parts.length === 1) return reply(history.at(-1));
      return reply({
        revisions: [...history].reverse(),
        has_more: false,
        next_before_revision: null,
      });
    }
    const armed = state.allowWrite;
    if (armed?.method === call.method && armed.path === call.path && !call.search) {
      state.allowWrite = null;
      if (!['owner', 'litigator'].includes(state.current.user.role))
        return reply({ error: { code: 'permission_denied' } }, 403);
      const values = request.postDataJSON();
      state.confirmedWrites.push({ ...call, values });
      if (call.method === 'POST' && !parts.length) {
        const created = { ...participant, ...values, id: otherParticipantId };
        state.records.set(created.id, [created]);
        return reply(created, 201);
      }
      if (call.method === 'PUT' && parts.length === 1 && history) {
        if (values.expected_revision !== history.at(-1).revision)
          return reply({ error: { code: 'participant_revision_conflict' } }, 409);
        const { expected_revision, ...changes } = values;
        const updated = { ...history.at(-1), ...changes, revision: expected_revision + 1 };
        history.push(updated);
        return reply(updated);
      }
    }
    state.writes.push(call);
    return reply({ error: { code: 'unexpected_participant_draft_request' } }, 501);
  });
  return state;
}

export async function checkParticipantDraftRequests(page) {
  await checkSessionRequests(page);
  const state = states.get(page);
  if (state) expect(state.allowWrite, 'An armed manual write must be submitted once').toBeNull();
}

export async function openParticipants(page) {
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Participantes', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Aplicar filtros', exact: true })).toBeEnabled();
}

export async function createManual(page) {
  await page.getByRole('button', { name: 'Registrar ficha pendiente', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Agregar participante', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

export async function editManual(page, name = participant.display_name) {
  await page.getByRole('button', { name: `Abrir ${name}`, exact: true }).click();
  const detail = page.getByRole('region', { name: 'Datos del participante', exact: true });
  await expect(detail.getByRole('heading', { name, exact: true })).toBeVisible();
  await detail.getByRole('button', { name: 'Editar participante', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar participante', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

export async function fillManual(modal, values = rawFields) {
  for (let index = 0; index < fieldLabels.length; index++)
    await modal.getByLabel(fieldLabels[index], { exact: true }).fill(values[index]);
  return modal.elementHandle();
}

export async function expectManual(modal, values = rawFields) {
  for (let index = 0; index < fieldLabels.length; index++)
    await expect(modal.getByLabel(fieldLabels[index], { exact: true })).toHaveValue(values[index]);
}

export function holdParticipantRead(state, path) {
  const gate = { path, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  return gate;
}
