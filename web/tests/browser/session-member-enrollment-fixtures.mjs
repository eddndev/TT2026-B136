import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { memberRecord, memberPage } from '../fixtures/members.mjs';

export const usersPath = '/api/v1/users';
export const principalPath = '/api/v1/auth/me';
export const initialPassword = 'synthetic-initial-password';
export const enrollmentSecret = 'JBSWY3DPEHPK3PXP';
export const enrollmentCode = 'synthetic-enrollment-code';
export const submittedEmail = 'Later.Member@Example.Test';
const states = new WeakMap();

export function holdEnrollmentRequest(state, method, path) {
  const gate = { method, path, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.enrollmentGates.push(gate);
  return gate;
}

export function armEnrollment(
  state,
  { applied = true, hold = false, email = submittedEmail } = {},
) {
  const plan = { applied, email, entered: false };
  if (hold) {
    plan.promise = new Promise((resolve) => {
      plan.release = resolve;
    });
    state.enrollmentGates.push(plan);
  }
  state.enrollmentWrite = plan;
  return plan;
}

export async function enrollmentSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    enrollmentGates: [],
    enrollmentWrites: [],
    enrollmentWrite: null,
    enrollmentIdentity: null,
    enrollmentRows: [
      memberRecord(1, {
        id: '11111111-1111-4111-8111-111111111111',
        email: 'hatz@example.com',
        role: 'owner',
      }),
    ],
  });
  states.set(page, state);
  const cursors = new Map();
  let sequence = 0;
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      path = url.pathname;
    if (path !== principalPath && path !== usersPath && !path.startsWith(`${usersPath}/`))
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
      route.fulfill({ status, json, headers: { 'Cache-Control': 'no-store' } });
    const failure = (status, code) => reply({ error: { code } }, status);
    const unexpected = () => {
      state.unexpected.push(call);
      return failure(501, 'unimplemented_member_enrollment_request');
    };
    if (
      !state.current ||
      call.headers.authorization !== call.expectedBearer ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    )
      return failure(401, 'invalid_session');
    if (call.method === 'GET' && call.body === null) {
      const gate = state.enrollmentGates.find(
        (value) => !value.entered && value.method === call.method && value.path === path,
      );
      if (gate) {
        gate.entered = true;
        await gate.promise;
      }
      const principal = state.enrollmentIdentity ?? state.current.user;
      if (path === principalPath && !call.search) return reply(structuredClone(principal));
      if (principal.role !== 'owner') return failure(403, 'permission_denied');
      if (path !== usersPath) return unexpected();
      const query = Object.fromEntries(url.searchParams);
      expect([...url.searchParams.keys()]).toHaveLength(Object.keys(query).length);
      expect(
        Object.keys(query).every((key) =>
          ['limit', 'status', 'email_prefix', 'cursor'].includes(key),
        ),
      ).toBe(true);
      expect(query.limit).toBe('20');
      expect(['active', 'all']).toContain(query.status);
      if (query.email_prefix) {
        expect(query.email_prefix).toBe(submittedEmail.toLowerCase());
        expect(query.status).toBe('all');
      }
      const bound = JSON.stringify([query.status, query.email_prefix ?? '']);
      const previous = query.cursor ? cursors.get(query.cursor) : null;
      if (query.cursor) expect(previous?.bound).toBe(bound);
      let rows = state.enrollmentRows
        .filter(
          (row) =>
            (!previous || row.id > previous.id) &&
            (query.status === 'all' || row.active) &&
            (!query.email_prefix || row.email.startsWith(query.email_prefix)),
        )
        .sort((left, right) => left.id.localeCompare(right.id));
      let next = null;
      if (rows.length > 20) {
        rows = rows.slice(0, 20);
        next = `enrollment-page-${++sequence}`;
        cursors.set(next, { bound, id: rows.at(-1).id });
      }
      return reply(memberPage(structuredClone(rows), next));
    }
    const plan = state.enrollmentWrite;
    if (path !== usersPath || call.method !== 'POST' || call.search || !plan) return unexpected();
    expect(call.headers['content-type']).toBe('application/json');
    const input = request.postDataJSON();
    expect(input).toEqual({ email: plan.email, password: initialPassword, role: 'litigator' });
    state.enrollmentWrite = null;
    state.enrollmentWrites.push({ ...call, input: { email: input.email, role: input.role } });
    const row = memberRecord(99, {
      email: input.email.toLowerCase(),
      role: input.role,
      revision: '0',
    });
    if (plan.applied) state.enrollmentRows.push(row);
    plan.entered = true;
    if (plan.promise) await plan.promise;
    if (!plan.applied) return failure(503, 'server_busy');
    return reply(
      {
        user: { id: row.id, email: row.email, role: row.role },
        totp_secret_base32: enrollmentSecret,
        otpauth_uri: 'otpauth://totp/synthetic-member',
        recovery_codes: [enrollmentCode],
      },
      201,
    );
  });
  return state;
}

export function includeSimilarAccounts(state) {
  state.enrollmentRows.push(
    ...Array.from({ length: 21 }, (_, index) =>
      memberRecord(index + 10, {
        email: `${submittedEmail.toLowerCase()}.near${index}`,
        role: 'litigator',
      }),
    ),
  );
}

export async function checkEnrollmentRequests(page) {
  const state = states.get(page);
  state?.enrollmentGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state) expect(state.enrollmentWrite, 'Every enrollment must be explicitly armed').toBeNull();
}
