import { expect } from '@playwright/test';
import { caseRecord, navigate } from './helpers.mjs';
import { administration, overview, profile } from './case-administration-helpers.mjs';
import { sessionSetup, checkSessionRequests, casePath } from './session-inactivity-helpers.mjs';

export const titleLabel = 'T\u00edtulo del expediente';
export const offenseLabel = 'Nueva descripci\u00f3n de delito';
export const rawTitle = '  Expediente todavia incompleto  ';
export const rawReference = '  REF- pendiente  ';
export const rawNuc = '  NUC- sin terminar  ';
export const pendingOffense = '  descripcion sin confirmar  ';
export const caseIndexPath = '/api/v1/case-administrations';
const states = new WeakMap();

export async function caseDraftSetup(page) {
  const state = await sessionSetup(page);
  state.record = administration(caseRecord, 1, structuredClone(profile));
  state.confirmedWrites = [];
  state.allowReplacement = false;
  states.set(page, state);
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    if (![casePath, caseIndexPath].includes(url.pathname)) return route.fallback();
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
    if (call.method === 'GET' && call.body === null) {
      if (state.gate?.path === call.path) {
        state.gate.entered = true;
        await state.gate.promise;
      }
      if (!state.allowed) return reply({ error: { code: 'case_not_found' } }, 404);
      return reply(
        call.path === casePath
          ? state.record
          : {
              cases: [overview(state.record)],
              has_more: false,
              next_after_id: null,
            },
      );
    }
    if (call.path === casePath && call.method === 'PUT' && state.allowReplacement) {
      state.allowReplacement = false;
      const body = request.postDataJSON();
      state.confirmedWrites.push({ ...call, values: body });
      if (body.expected_revision !== state.record.administration.revision)
        return reply({ error: { code: 'case_revision_conflict' } }, 409);
      state.record = administration(
        { ...caseRecord, ...body },
        body.expected_revision + 1,
        body.profile,
      );
      return reply(state.record);
    }
    state.writes.push(call);
    return reply({ error: { code: 'unexpected_case_draft_request' } }, 501);
  });
  return state;
}

export async function checkCaseDraftRequests(page) {
  await checkSessionRequests(page);
  const state = states.get(page);
  if (state)
    expect(state.allowReplacement, 'An armed replacement must be submitted once').toBe(false);
}

export function holdRead(state, path) {
  const gate = { path, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  return gate;
}

export const editor = (page) => page.getByRole('region', { name: 'Formulario del expediente' });

export async function createEditor(page) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await expect(editor(page)).toBeVisible();
  return editor(page);
}

export async function fillPartialCase(form) {
  await form.getByLabel(titleLabel, { exact: true }).fill(rawTitle);
  await form.getByLabel('Referencia interna', { exact: true }).fill(rawReference);
  await form.getByLabel('NUC', { exact: true }).fill(rawNuc);
  await form.getByLabel(offenseLabel, { exact: true }).fill(pendingOffense);
  await expect(form.getByRole('button', { name: /^Quitar descripci/ })).toHaveCount(0);
  return form.elementHandle();
}

export async function expectBlankCase(form) {
  for (const label of [titleLabel, 'Referencia interna', 'NUC', offenseLabel])
    await expect(form.getByLabel(label, { exact: true })).toHaveValue('');
  await expect(form.getByRole('button', { name: /^Quitar descripci/ })).toHaveCount(0);
}

export async function openCaseSummary(page, title = caseRecord.title) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: new RegExp(title) }).click();
  await expect(
    page.getByRole('heading', {
      name: 'Resumen del expediente',
      exact: true,
    }),
  ).toBeVisible();
}
