import { test, expect } from '@playwright/test';
import { expire } from './session-inactivity-helpers.mjs';
import { openResults, resultPanel, resultDetail } from './hearing-result-helpers.mjs';
import {
  setupDerivedDeadline,
  openDerivedDeadline,
  fillDerivedDeadline,
  prepareDerivedDeadline,
  submitDerivedDeadline,
  derivedEditor,
} from './hearing-derived-deadline-helpers.mjs';
import { principal, clone } from '../fixtures/hearing-derived-deadline-unit.mjs';
import { hearingCaseId, hearingId } from '../fixtures/hearings.mjs';

const epoch = Date.UTC(2026, 9, 2, 18);
const endpoint = `/api/v1/cases/${hearingCaseId}/hearings/${hearingId}/results/derived-deadline`;

async function installShortSession(page) {
  const state = { now: epoch + 1000, current: null, grants: [] };
  const metadata = () => ({
    user: clone(state.current.user),
    policy: { absolute_ttl_seconds: 60, idle_ttl_seconds: null },
    server_now_unix_ms: state.now,
    absolute_expires_at_unix_ms: state.current.deadline,
    idle_expires_at_unix_ms: null,
  });
  await page.route('**/api/v1/auth/**', async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname;
    if (path.endsWith('/login')) return route.fallback();
    if (/\/mfa\/(totp|recovery)$/.test(path)) {
      state.current = {
        user: principal(),
        token: `compound-session-${state.grants.length + 1}`,
        deadline: state.now + 60000,
      };
      const grant = {
        ...metadata(),
        access_token: state.current.token,
        token_type: 'Bearer',
        expires_in_seconds: 60,
      };
      state.grants.push(clone(grant));
      return route.fulfill({ json: grant });
    }
    if (
      !state.current ||
      state.now >= state.current.deadline ||
      request.headers().authorization !== `Bearer ${state.current.token}`
    )
      return route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
    if (path.endsWith('/me')) return route.fulfill({ json: state.current.user });
    if (path.endsWith('/session') || path.endsWith('/activity'))
      return route.fulfill({ json: metadata() });
    return route.fallback();
  });
  state.advance = async (milliseconds) => {
    state.now += milliseconds;
    await page.clock.runFor(milliseconds);
  };
  return state;
}

test('same-user reentry reconciles the preserved uncertain compound attempt before current sources', async ({
  page,
}) => {
  await page.clock.install({ time: new Date(epoch) });
  await page.clock.pauseAt(new Date(epoch + 1000));
  const state = await setupDerivedDeadline(page);
  const session = await installShortSession(page);
  const readsBeforeReplay = [],
    requests = [];
  let recovering = false,
    replayed = false;
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname;
    requests.push({
      path,
      method: request.method(),
      authorization: request.headers().authorization,
    });
    if (
      recovering &&
      request.method() === 'GET' &&
      !path.startsWith('/api/v1/auth/') &&
      !path.endsWith('/administration')
    ) {
      readsBeforeReplay.push(path);
      return route.fulfill({ status: 503, json: { error: { code: 'service_busy' } } });
    }
    return route.fallback();
  });
  state.handle = async (route, call) => {
    if (call.path.endsWith('/submit')) {
      state.submissions.push(clone(call.body));
      state.commit(state.prepare(call.body.command));
      await route.abort('failed');
      return true;
    }
    if (recovering && call.path.endsWith('/prepare')) {
      const record = state.committed.get(call.body.result.operation_id);
      expect(record, 'Recovery must query the original joint operation').toBeTruthy();
      expect(call.body).toEqual(state.submissions[0].command);
      replayed = true;
      recovering = false;
    }
    return false;
  };

  await openDerivedDeadline(page);
  await fillDerivedDeadline(page);
  await prepareDerivedDeadline(page);
  await submitDerivedDeadline(page);
  let form = derivedEditor(page);
  await expect(
    form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  const submitted = clone(state.submissions[0]);
  const original = clone(state.committed.get(submitted.command.result.operation_id));
  const originalToken = session.current.token;
  await expire(page, session, await form.elementHandle());

  await openResults(page);
  expect(session.grants).toHaveLength(2);
  expect(session.grants[1].user).toEqual(session.grants[0].user);
  expect(session.current.token).not.toBe(originalToken);
  const beforeRecovery = requests.length;
  recovering = true;
  await resultPanel(page)
    .getByRole('button', { name: 'Recuperar resultado y plazo', exact: true })
    .click();
  form = derivedEditor(page);
  await expect(
    form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(form).toContainText('Resultado con plazo declarado');
  await expect(form).toContainText('Respuesta al resultado');
  await expect(
    form.getByRole('heading', { name: 'Revisa el resultado y el plazo', exact: true }),
  ).toHaveCount(0);
  await expect(
    form.getByRole('checkbox', { name: 'Confirmo el resultado y el plazo revisados', exact: true }),
  ).toHaveCount(0);
  await expect(
    form.getByRole('button', { name: 'Confirmar resultado y plazo', exact: true }),
  ).toHaveCount(0);
  await expect(
    form.getByRole('button', { name: 'Reintentar el mismo envio', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.map((call) => call.path)).toEqual([
    `${endpoint}/prepare`,
    `${endpoint}/submit`,
  ]);
  expect(state.submissions).toEqual([submitted]);
  expect(readsBeforeReplay).toEqual([]);

  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form).toHaveCount(0);
  await expect(resultDetail(page)).toContainText('Resultado con plazo declarado');
  expect(replayed).toBe(true);
  expect(readsBeforeReplay).toEqual([]);
  const recoveryCalls = requests.slice(beforeRecovery);
  expect(recoveryCalls[0].path).toBe(`/api/v1/cases/${hearingCaseId}/administration`);
  expect(recoveryCalls.find((call) => call.method === 'POST')?.path).toBe(`${endpoint}/prepare`);
  expect(recoveryCalls.find((call) => call.path === `${endpoint}/prepare`)?.authorization).toBe(
    `Bearer ${session.current.token}`,
  );
  expect(state.calls.map((call) => call.path)).toEqual([
    `${endpoint}/prepare`,
    `${endpoint}/submit`,
    `${endpoint}/prepare`,
  ]);
  expect(state.calls[2].body).toEqual(submitted.command);
  expect(state.submissions).toEqual([submitted]);
  expect(state.committed.size).toBe(1);
  expect(state.committed.get(submitted.command.result.operation_id)).toEqual(original);
  expect(state.results.records.size).toBe(1);
  expect(state.deadlines.records.size).toBe(1);
  expect(state.results.submissions).toHaveLength(0);
  expect(state.deadlines.submissions).toHaveLength(0);
  await expect(
    resultPanel(page).getByRole('button', { name: 'Recuperar resultado y plazo', exact: true }),
  ).toHaveCount(0);
});
