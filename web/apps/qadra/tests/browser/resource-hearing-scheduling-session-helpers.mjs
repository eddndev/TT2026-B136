import { expect } from '@playwright/test';
import { login, navigate, caseId } from './helpers.mjs';
import { resourceActor } from '../fixtures/procedural-resource-unit.mjs';
import { resourceDetail } from './procedural-resources-helpers.mjs';
import {
  setupResourceHearing,
  hearingEditor,
  activityPanel,
} from './resource-hearing-scheduling-helpers.mjs';

export { login };
export const administrationPath = `/api/v1/cases/${caseId}/administration`;
export async function setupHearingSession(page) {
  const epoch = Date.UTC(2026, 9, 3, 18);
  await page.clock.install({ time: new Date(epoch) });
  await page.clock.pauseAt(new Date(epoch));
  const state = await setupResourceHearing(page);
  Object.assign(state, {
    now: epoch,
    current: null,
    nextUser: null,
    grants: 0,
    gate: null,
    requests: [],
  });
  const idle = 40000;
  const metadata = () => ({
    user: state.current.user,
    policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: 40 },
    server_now_unix_ms: state.now,
    absolute_expires_at_unix_ms: state.current.absolute,
    idle_expires_at_unix_ms: state.current.deadline,
  });
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname;
    state.requests.push({
      path,
      method: request.method(),
      authorization: request.headers().authorization,
    });
    if (path === '/api/v1/auth/login') {
      const other = request.postDataJSON().email === 'other@example.test';
      state.nextUser = other
        ? { id: '22222222-2222-4222-8222-222222222222', email: 'other@example.test', role: 'owner' }
        : { ...resourceActor, role: 'owner' };
      return route.fulfill({
        json: { challenge_token: 'hearing-challenge', expires_in_seconds: 300 },
      });
    }
    if (path.startsWith('/api/v1/auth/mfa/')) {
      state.current = {
        user: state.nextUser,
        token: `hearing-session-${++state.grants}`,
        absolute: state.now + 86400000,
        deadline: state.now + idle,
      };
      return route.fulfill({
        json: {
          ...metadata(),
          access_token: state.current.token,
          token_type: 'Bearer',
          expires_in_seconds: 40,
        },
      });
    }
    if (
      !state.current ||
      request.headers().authorization !== `Bearer ${state.current.token}` ||
      state.now >= state.current.deadline
    )
      return route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
    if (path === '/api/v1/auth/logout') {
      state.current = null;
      return route.fulfill({ status: 204 });
    }
    if (path === '/api/v1/auth/me') return route.fulfill({ json: state.current.user });
    if (['/api/v1/auth/session', '/api/v1/auth/activity'].includes(path)) {
      if (path.endsWith('/activity'))
        state.current.deadline = Math.min(state.current.absolute, state.now + idle);
      return route.fulfill({ json: metadata() });
    }
    if (state.gate?.path === path) {
      state.gate.entered = true;
      await state.gate.promise;
    }
    return route.fallback();
  });
  state.advance = async (milliseconds) => {
    state.now += milliseconds;
    await page.clock.runFor(milliseconds);
  };
  return state;
}
export function holdHearingAuthority(state) {
  const gate = { path: administrationPath, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  return gate;
}
export async function openHearingScope(page, state) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Recursos', exact: true }).click();
  await page
    .getByRole('button', {
      name: `Consultar recurso ${state.activities.resource.values.title}`,
      exact: true,
    })
    .click();
  await resourceDetail(page)
    .getByRole('button', { name: 'Ver historial de recurso', exact: true })
    .click();
  await page.getByRole('button', { name: 'Consultar recurso revision 1', exact: true }).click();
  await expect(
    activityPanel(page).getByRole('button', { name: 'Crear audiencia de recurso', exact: true }),
  ).toBeEnabled();
}
export async function resumeHearing(page) {
  await activityPanel(page)
    .getByRole('button', { name: 'Retomar borrador de audiencia', exact: true })
    .click();
  await expect(hearingEditor(page)).toBeVisible();
}
