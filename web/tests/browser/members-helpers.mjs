import { expect } from '@playwright/test';
import { setup, login, navigate, caseId, otherCaseId, caseRecord } from './helpers.mjs';
import { administration, overview, otherAdministration } from './case-administration-helpers.mjs';
import {
  memberRecord,
  memberActor,
  memberId,
  memberPage,
  caseMemberPage,
} from '../fixtures/members.mjs';

export const directory = (page) =>
  page.getByRole('region', { name: 'Directorio de cuentas', exact: true });
export const editor = (page) => page.getByRole('region', { name: 'Acceso de cuenta', exact: true });
export const assignments = (page) =>
  page.getByRole('region', { name: 'Asignaciones', exact: true });
export const memberCard = (page, id) => directory(page).locator(`[data-user-id="${id}"]`);
const principal = ({ id, email, role }) => ({ id, email, role });
const failure = (route, status, code) => route.fulfill({ status, json: { error: { code } } });

export async function setupMembers(page, { role = 'owner', rows, closed = false } = {}) {
  const baseCalls = await setup(page, role);
  const actor = { ...memberActor, role };
  const state = {
    actor,
    baseCalls,
    rows: rows || [
      actor,
      memberRecord(2),
      memberRecord(3, { role: 'owner', email: 'second-owner@example.test' }),
      memberRecord(4, { active: false }),
      memberRecord(5, { role: 'client' }),
      memberRecord(6),
    ],
    calls: [],
    handle: null,
    memberships: new Map([
      [caseId, new Set([memberId(2), memberId(4)])],
      [otherCaseId, new Set([memberId(6)])],
    ]),
    revoked: false,
  };
  const cursors = new Map();
  let cursorSequence = 0;
  const listing = (route, url, scope) => {
    const query = Object.fromEntries(url.searchParams),
      limit = Number(query.limit || 50);
    const bound = JSON.stringify([
      scope,
      query.status || 'active',
      query.selection || 'assigned',
      query.role || '',
      query.email_prefix || '',
    ]);
    const previous = query.cursor ? cursors.get(query.cursor) : null;
    if (query.cursor && (!previous || previous.bound !== bound))
      return failure(route, 400, 'invalid_input');
    const assigned = scope ? state.memberships.get(scope) : null;
    let items = state.rows.filter(
      (row) =>
        (!previous || row.id > previous.id) &&
        (!query.role || row.role === query.role) &&
        (!query.email_prefix || row.email.startsWith(query.email_prefix)),
    );
    if (scope)
      items = items.filter((row) =>
        query.selection === 'available'
          ? row.active && !assigned.has(row.id)
          : assigned.has(row.id),
      );
    else
      items = items.filter(
        (row) => query.status === 'all' || row.active === (query.status !== 'inactive'),
      );
    items.sort((a, b) => (a.id < b.id ? -1 : 1));
    const more = items.length > limit;
    items = items.slice(0, limit);
    let next = null;
    if (more) {
      next = `member-fixture:${++cursorSequence}`;
      cursors.set(next, { bound, id: items.at(-1).id });
    }
    return route.fulfill({
      json: scope
        ? caseMemberPage(
            items.map((row) => ({
              ...row,
              assigned_at: assigned.has(row.id) ? '2026-09-19T10:00:00.123456789Z' : null,
            })),
            next,
            scope,
          )
        : memberPage(items, next),
    });
  };
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    if (/\/auth\/mfa\//.test(path))
      return route.fulfill({
        json: {
          access_token: 'member-session',
          user: principal(state.actor),
          expires_in_seconds: 86400,
        },
      });
    if (path.endsWith('/auth/me'))
      return state.revoked
        ? failure(route, 401, 'invalid_session')
        : route.fulfill({ json: principal(state.actor) });
    if (closed && path === `/api/v1/cases/${caseId}/administration`)
      return route.fulfill({ json: administration(caseRecord, 1, null, 'closed') });
    if (closed && path === '/api/v1/case-administrations')
      return route.fulfill({
        json: {
          cases: [
            overview(administration(caseRecord, 1, null, 'closed')),
            overview(otherAdministration()),
          ],
          has_more: false,
          next_after_id: null,
        },
      });
    if (
      !/^\/api\/v1\/users(?:\/|$)/.test(path) &&
      !/^\/api\/v1\/cases\/[^/]+\/members(?:\/|$)/.test(path)
    )
      return route.fallback();
    const call = {
      path,
      method: request.method(),
      query: Object.fromEntries(url.searchParams),
      body: request.postData() ? request.postDataJSON() : null,
    };
    state.calls.push(call);
    if (state.handle && (await state.handle(route, call))) return;
    if (state.actor.role !== 'owner') return failure(route, 403, 'permission_denied');
    if (path === '/api/v1/users' && call.method === 'GET') return listing(route, url, null);
    if (path === '/api/v1/users' && call.method === 'POST') {
      const row = memberRecord(99, { email: call.body.email, role: call.body.role, revision: '0' });
      state.rows.push(row);
      return route.fulfill({
        status: 201,
        json: {
          user: principal(row),
          totp_secret_base32: 'TESTSECRET',
          otpauth_uri: 'otpauth://totp/test',
          recovery_codes: ['recovery-one', 'recovery-two'],
        },
      });
    }
    const userMatch = /^\/api\/v1\/users\/([^/]+)(\/access)?$/.exec(path);
    if (userMatch) {
      const row = state.rows.find((item) => item.id === userMatch[1]);
      if (!row) return failure(route, 404, 'user_not_found');
      if (!userMatch[2]) return route.fulfill({ json: row });
      const input = call.body;
      if (input.expected_revision !== row.revision)
        return failure(route, 409, 'user_revision_conflict');
      if (
        row.role === 'owner' &&
        row.active &&
        (input.role !== 'owner' || !input.active) &&
        state.rows.filter((item) => item.role === 'owner' && item.active).length === 1
      )
        return failure(route, 409, 'last_active_owner');
      if (row.role !== input.role || row.active !== input.active) {
        Object.assign(row, {
          role: input.role,
          active: input.active,
          revision: String(BigInt(row.revision) + 1n),
        });
        if (row.id === actor.id) {
          state.actor = row;
          state.revoked = true;
        }
      }
      return route.fulfill({ json: row });
    }
    const scope = /^\/api\/v1\/cases\/([^/]+)\/members(?:\/([^/]+))?$/.exec(path);
    if (!scope || !state.memberships.has(scope[1])) return failure(route, 404, 'case_not_found');
    if (!scope[2]) return listing(route, url, scope[1]);
    const assigned = state.memberships.get(scope[1]);
    if (call.method === 'PUT') {
      if (!state.rows.some((row) => row.id === scope[2] && row.active))
        return failure(route, 404, 'user_not_found');
      assigned.add(scope[2]);
    } else if (call.method === 'DELETE') assigned.delete(scope[2]);
    else return failure(route, 405, 'method_not_allowed');
    return route.fulfill({ status: 204 });
  });
  return state;
}

export async function enterDirectory(page) {
  await login(page, false, false);
  await navigate(page, 'Equipo');
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
}
export async function openAccess(page, row) {
  await directory(page)
    .getByRole('button', { name: `Administrar acceso de ${row.email}`, exact: true })
    .click();
  await expect(editor(page)).toBeVisible();
  await expect(editor(page)).toHaveAttribute('aria-busy', 'false');
}
export async function reviewAccess(page, role, active) {
  await editor(page)
    .getByRole('combobox', { name: 'Rol de la cuenta', exact: true })
    .selectOption(role);
  await editor(page)
    .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
    .selectOption(active ? 'active' : 'inactive');
  await editor(page).getByRole('button', { name: 'Revisar acceso', exact: true }).click();
}
export async function openAssignments(page, title = 'Defensa inicial') {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Asignaciones', exact: true }).click();
  await expect(assignments(page)).toHaveAttribute('aria-busy', 'false');
}
