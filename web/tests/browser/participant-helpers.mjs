import { expect } from '@playwright/test';
import { setup, login, caseId } from './helpers.mjs';
export const participantId = '11111111-1111-4111-8111-111111111111';
export const otherParticipantId = '22222222-2222-4222-8222-222222222222';
export const participant = {
  case_id: caseId,
  id: participantId,
  revision: 1,
  display_name: 'Ana Mu\u00f1oz',
  procedural_role: 'Defensa',
  organization: 'Despacho',
  legal_status: null,
  directory_status: 'active',
  values_digest: 'a'.repeat(64),
  changed_at: '2026-09-14T20:00:00Z',
  changed_by: { id: '33333333-3333-4333-8333-333333333333', email: 'autor@example.com' },
};
export const directory = (page) =>
  page.getByRole('region', { name: 'Directorio del expediente', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Datos del participante', exact: true });
export async function openParticipant(page, name = participant.display_name) {
  await directory(page)
    .getByRole('button', { name: `Abrir ${name}`, exact: true })
    .click();
  await expect(detail(page).getByRole('heading', { name, exact: true })).toBeVisible();
}
export async function participantSetup(page, role = 'owner', initial = [participant]) {
  const requests = await setup(page, role);
  const calls = [];
  const records = new Map(initial.map((row) => [row.id, [structuredClone(row)]]));
  const state = { calls, records, requests };
  await page.route('**/api/v1/cases/*/participants**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const path = url.pathname,
      body = request.postDataJSON();
    calls.push({ path, search: url.search, method: request.method(), body });
    if (!path.startsWith(`/api/v1/cases/${caseId}/participants`))
      return route.fulfill({ status: 404 });
    if (role === 'client') return route.fulfill({ status: 403 });
    if (request.method() !== 'GET' && role === 'paralegal') return route.fulfill({ status: 403 });
    const parts = path.split('/participants')[1].split('/').filter(Boolean);
    if (!parts.length) {
      if (request.method() === 'POST') {
        const row = { ...participant, ...body, id: otherParticipantId };
        records.set(row.id, [row]);
        return route.fulfill({ status: 201, json: row });
      }
      const status = url.searchParams.get('status') || 'active';
      let rows = [...records.values()]
        .map((history) => history.at(-1))
        .filter(
          (row) =>
            (status === 'all' || row.directory_status === status) &&
            row.display_name.includes(url.searchParams.get('name') || '') &&
            (!url.searchParams.get('procedural_role') ||
              row.procedural_role === url.searchParams.get('procedural_role')) &&
            (!url.searchParams.get('after_id') || row.id > url.searchParams.get('after_id')),
        );
      rows.sort((a, b) => a.id.localeCompare(b.id));
      const limit = Number(url.searchParams.get('limit') || 50),
        more = rows.length > limit;
      rows = rows.slice(0, limit);
      return route.fulfill({
        json: { participants: rows, has_more: more, next_after_id: more ? rows.at(-1).id : null },
      });
    }
    const history = records.get(parts[0]);
    if (!history) return route.fulfill({ status: 404 });
    const current = history.at(-1);
    if (parts[1] === 'history') {
      let rows = [...history]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.get('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const limit = Number(url.searchParams.get('limit') || 50),
        more = rows.length > limit;
      rows = rows.slice(0, limit);
      return route.fulfill({
        json: {
          revisions: rows,
          has_more: more,
          next_before_revision: more ? rows.at(-1).revision : null,
        },
      });
    }
    if (request.method() === 'PUT') {
      if (body.expected_revision !== current.revision)
        return route.fulfill({
          status: 409,
          json: { error: { code: 'participant_revision_conflict' } },
        });
      const { expected_revision, ...changes } = body;
      const next = { ...current, ...changes, revision: current.revision + 1 };
      history.push(next);
      return route.fulfill({ json: next });
    }
    return route.fulfill({ json: current });
  });
  await login(page);
  if (role !== 'client')
    await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  return state;
}
