import { expect } from '@playwright/test';
import { navigate } from './helpers.mjs';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { memberRecord, memberPage } from '../fixtures/members.mjs';
import { directory, editor, openAccess, reviewAccess } from './members-helpers.mjs';

export { directory, editor, openAccess, reviewAccess };
export const usersPath = '/api/v1/users';
export const mePath = '/api/v1/auth/me';
export const memberPath = (row) => `${usersPath}/${row.id}`;
export const actorId = '11111111-1111-4111-8111-111111111111';
const states = new WeakMap();

export function holdMemberRead(state, path) {
  const gate = { path, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.memberGates.push(gate);
  return gate;
}

export function armMemberWrite(state, id, { hold = false } = {}) {
  const plan = { id, entered: false };
  if (hold) {
    plan.promise = new Promise((resolve) => {
      plan.release = resolve;
    });
    state.memberGates.push(plan);
  }
  state.memberWrite = plan;
  return plan;
}

export async function memberDraftSetup(page) {
  const state = await sessionSetup(page);
  state.actor = memberRecord(1, {
    id: actorId,
    email: 'hatz@example.com',
    role: 'owner',
    revision: '4',
  });
  state.target = memberRecord(2);
  state.rows = [
    state.actor,
    state.target,
    memberRecord(3, {
      id: '22222222-2222-4222-8222-222222222222',
      email: 'other@example.test',
      role: 'owner',
    }),
  ];
  state.memberWrites = [];
  state.memberGates = [];
  state.memberWrite = null;
  state.memberDenied = false;
  states.set(page, state);
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    if (path !== mePath && path !== usersPath && !path.startsWith(`${usersPath}/`))
      return route.fallback();
    const call = {
      path,
      method: request.method(),
      body: request.postData(),
      search: url.search,
      headers: request.headers(),
      at: state.now,
      expectedBearer: `Bearer ${state.current?.token}`,
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
      call.headers.authorization !== call.expectedBearer ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    ) {
      state.unexpected.push(call);
      return reply({ error: { code: 'invalid_session' } }, 401);
    }
    if (path !== mePath && (state.memberDenied || state.current.user.role !== 'owner'))
      return reply({ error: { code: 'permission_denied' } }, 403);
    if (call.method === 'GET' && call.body === null) {
      const gate = state.memberGates.find((value) => value.path === path && !value.entered);
      if (gate) {
        gate.entered = true;
        await gate.promise;
      }
      if (path === mePath && !call.search) return reply(structuredClone(state.current.user));
      if (path === usersPath) {
        const query = Object.fromEntries(url.searchParams);
        expect([...url.searchParams.keys()]).toHaveLength(Object.keys(query).length);
        expect(query).toEqual({ limit: '20', status: query.status });
        expect(['active', 'all']).toContain(query.status);
        return reply(
          memberPage(
            state.rows
              .filter((row) => query.status === 'all' || row.active)
              .map((row) => structuredClone(row))
              .sort((a, b) => a.id.localeCompare(b.id)),
          ),
        );
      }
      const row = state.rows.find((value) => memberPath(value) === path);
      if (row && !call.search) return reply(structuredClone(row));
    }
    const plan = state.memberWrite;
    if (
      plan &&
      call.method === 'PUT' &&
      path === `${usersPath}/${plan.id}/access` &&
      !call.search
    ) {
      state.memberWrite = null;
      const input = request.postDataJSON();
      expect(Object.keys(input).sort()).toEqual(['active', 'expected_revision', 'role']);
      expect(input.expected_revision).toMatch(/^(0|[1-9][0-9]*)$/);
      expect(['owner', 'litigator', 'paralegal', 'client']).toContain(input.role);
      expect(typeof input.active).toBe('boolean');
      const row = state.rows.find((value) => value.id === plan.id);
      expect(row).toBeDefined();
      expect(input.expected_revision).toBe(row.revision);
      state.memberWrites.push({ ...call, input });
      if (row.role !== input.role || row.active !== input.active)
        Object.assign(row, {
          role: input.role,
          active: input.active,
          revision: String(BigInt(row.revision) + 1n),
        });
      const confirmed = structuredClone(row);
      plan.entered = true;
      if (plan.promise) await plan.promise;
      return reply(confirmed);
    }
    state.unexpected.push(call);
    return reply({ error: { code: 'unimplemented_member_draft_request' } }, 501);
  });
  return state;
}

export async function checkMemberDraftRequests(page) {
  const state = states.get(page);
  state?.memberGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  for (const call of state?.memberWrites || []) {
    expect(call.method).toBe('PUT');
    expect(call.search).toBe('');
    expect(call.headers.authorization).toBe(call.expectedBearer);
    expect(call.headers['content-type']).toBe('application/json');
  }
}

export async function enterMembers(page) {
  await navigate(page, 'Equipo');
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
  await directory(page)
    .getByRole('combobox', { name: 'Estado de la cuenta', exact: true })
    .selectOption('all');
  await directory(page).getByRole('button', { name: 'Buscar cuentas', exact: true }).click();
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
}

export async function accessValues(page, role, active) {
  await expect(
    editor(page).getByRole('combobox', { name: 'Rol de la cuenta', exact: true }),
  ).toHaveValue(role);
  await expect(
    editor(page).getByRole('combobox', { name: 'Estado de la cuenta', exact: true }),
  ).toHaveValue(active ? 'active' : 'inactive');
}

export const reviewButton = (page) =>
  editor(page).getByRole('button', { name: 'Revisar acceso', exact: true });
export const confirmButton = (page) =>
  editor(page).getByRole('button', { name: 'Confirmar cambio de acceso', exact: true });
export const consultButton = (page) =>
  editor(page).getByRole('button', { name: 'Consultar cuenta actual', exact: true });
export const adoptButton = (page) =>
  editor(page).getByRole('button', {
    name: 'Usar revisi\u00f3n actual y conservar cambios',
    exact: true,
  });
export const closeAccess = (page) =>
  editor(page).getByRole('button', { name: 'Cerrar acceso de cuenta', exact: true }).click();

export async function prepareAccessDraft(page, state, role = 'litigator', active = false) {
  await enterMembers(page);
  await openAccess(page, state.target);
  await reviewAccess(page, role, active);
  return editor(page).elementHandle();
}

export async function assertNoAccessCommand(page, state, count = 0) {
  await expect(confirmButton(page)).toHaveCount(0);
  expect(state.memberWrites).toHaveLength(count);
  expect(state.calls.filter((call) => call.method === 'POST' && call.path === usersPath)).toEqual(
    [],
  );
}
