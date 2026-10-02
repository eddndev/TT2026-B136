import { expect } from '@playwright/test';
import { caseRecord } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { casePath } from './session-inactivity-helpers.mjs';
import {
  participantDraftSetup,
  checkParticipantDraftRequests,
  participant,
  participantsPath,
  participantPath,
  fieldLabels,
  holdParticipantRead,
} from './session-participant-drafts-helpers.mjs';

const states = new WeakMap();
export const committedId = '99999999-9999-4999-8999-999999999991';

export async function edgeSetup(page) {
  const state = await participantDraftSetup(page);
  state.heldWrites = [];
  state.closedWrites = [];
  state.edgeGates = [];
  state.releaseWrite = () => {};
  states.set(page, state);
  return state;
}

export async function checkEdges(page) {
  const state = states.get(page);
  state?.releaseWrite();
  state?.edgeGates.forEach((gate) => gate.release());
  await checkParticipantDraftRequests(page);
  for (const call of state?.heldWrites || []) {
    expect(call.method).toBe('POST');
    expect(call.path).toBe(participantsPath);
    expect(call.search).toBe('');
    expect(call.headers.authorization).toBe(call.expectedBearer);
  }
  for (const call of state?.closedWrites || []) {
    expect(call).toMatchObject({ method: 'PUT', path: participantPath, search: '' });
    expect(call.headers.authorization).toBe(call.expectedBearer);
  }
}

export function holdEdgeRead(state, path) {
  const gate = holdParticipantRead(state, path);
  state.edgeGates.push(gate);
  return gate;
}

function captureRequest(state, request) {
  const url = new URL(request.url());
  const call = {
    path: url.pathname,
    method: request.method(),
    body: request.postData(),
    headers: request.headers(),
    search: url.search,
    at: state.now,
    expectedBearer: `Bearer ${state.current?.token}`,
  };
  state.calls.push(call);
  expect(state.current).not.toBeNull();
  expect(call.headers.authorization).toBe(call.expectedBearer);
  expect(state.now).toBeLessThan(Math.min(state.current.absolute, state.current.deadline));
  expect(state.allowed).toBe(true);
  expect(['owner', 'litigator']).toContain(state.current.user.role);
  return call;
}

export async function caseStatusReads(page, state, current) {
  await page.route(
    (url) => url.pathname === casePath,
    async (route) => {
      const call = captureRequest(state, route.request());
      expect(call).toMatchObject({ method: 'GET', body: null, search: '' });
      if (state.gate?.path === casePath) {
        state.gate.entered = true;
        await state.gate.promise;
      }
      return route.fulfill({
        json: administration(caseRecord, current.revision, null, current.status),
        headers: { 'Cache-Control': 'no-store' },
      });
    },
  );
}

export const closeCaseReads = (page, state) =>
  caseStatusReads(page, state, { revision: 2, status: 'closed' });

export async function rejectClosedReplacement(page, state, current) {
  await page.route(
    (url) => url.pathname === participantPath,
    (route) => {
      if (route.request().method() !== 'PUT') return route.fallback();
      const call = captureRequest(state, route.request());
      call.values = route.request().postDataJSON();
      state.closedWrites.push(call);
      current.status = 'closed';
      current.revision = 2;
      return route.fulfill({
        status: 409,
        json: { error: { code: 'case_closed' } },
        headers: { 'Cache-Control': 'no-store' },
      });
    },
  );
}

export async function holdCommittedCreation(page, state) {
  const gate = { entered: false, release: () => {} };
  const pending = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.releaseWrite = gate.release;
  await page.route(
    (url) => url.pathname === participantsPath,
    async (route) => {
      const request = route.request();
      if (request.method() !== 'POST' || gate.entered) return route.fallback();
      const call = captureRequest(state, request);
      call.values = request.postDataJSON();
      state.heldWrites.push(call);
      const created = { ...participant, ...call.values, id: committedId };
      state.records.set(created.id, [created]);
      gate.entered = true;
      await pending;
      return route.fulfill({
        status: 201,
        json: created,
        headers: { 'Cache-Control': 'no-store' },
      });
    },
  );
  return gate;
}

export function populateDirectory(state) {
  for (let index = 1; index <= 51; index++) {
    const row = {
      ...participant,
      id: `44444444-4444-4444-8444-${String(index).padStart(12, '0')}`,
      display_name: `Ficha del directorio ${index}`,
      directory_status: index === 51 ? 'archived' : 'active',
    };
    state.records.set(row.id, [row]);
  }
}

export async function expectReadonly(modal) {
  for (const label of fieldLabels)
    await expect(modal.getByLabel(label, { exact: true })).toBeDisabled();
  await expect(
    modal.getByRole('button', { name: 'Guardar mis cambios', exact: true }),
  ).toBeDisabled();
}

export function expectFreshRecordReads(state, before, resourcePath) {
  const reads = state.calls.slice(before).filter((call) => call.method === 'GET');
  const caseIndex = reads.findIndex((call) => call.path === casePath);
  const resourceIndex = reads.findIndex(
    (call, index) => index > caseIndex && call.path === resourcePath,
  );
  expect(caseIndex).toBeGreaterThanOrEqual(0);
  expect(resourceIndex).toBeGreaterThan(caseIndex);
  for (const call of [reads[caseIndex], reads[resourceIndex]])
    expect(call).toMatchObject({
      body: null,
      search: '',
      headers: { authorization: `Bearer ${state.current.token}` },
    });
}
