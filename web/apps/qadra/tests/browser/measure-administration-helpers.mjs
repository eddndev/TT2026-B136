import { expect } from '@playwright/test';
import { setupMeasureDecisions, openMeasures } from './measure-decision-helpers.mjs';
import { measureAdministrationOperation } from '../fixtures/measure-administrations.mjs';
import { inMeasureCase } from '../fixtures/measure-decision-browser.mjs';
import {
  prepareBrowserAdministration,
  commitBrowserAdministration,
  clone,
} from '../fixtures/measure-administration-browser.mjs';

export { openMeasures, clone };
export const administrativeActions = {
  correct: 'Rectificar registro',
  entered_in_error: 'Marcar registro por error',
  replace_entered_in_error: 'Corregir identidad registrada',
};
export const administrationEditor = (page) =>
  page.getByRole('region', { name: 'Rectificacion de medida', exact: true });
export const administrationDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de rectificacion', exact: true });
export const measurePanel = (page) =>
  page.getByRole('region', { name: 'Registros de medidas', exact: true });
const fail = (route, code, status = 409) => route.fulfill({ status, json: { error: { code } } });

export async function setupMeasureAdministrations(page, { role = 'owner', closed = false } = {}) {
  const decisions = await setupMeasureDecisions(page, { role, closed, seeded: true });
  const replacement = inMeasureCase(
    measureAdministrationOperation({ action: 'replace_entered_in_error' }),
    decisions.caseId,
  );
  const state = {
    caseId: decisions.caseId,
    decisions,
    base: `/api/v1/cases/${decisions.caseId}/measure-administrative-operations`,
    actor: clone(decisions.actor),
    context: clone(decisions.context),
    support: clone(decisions.seed.operation.group.review.material.support),
    replacementSubject: clone(replacement.capture.review.replacement.sources.subject),
    records: decisions.records,
    target: clone(decisions.seed.detail),
    operations: new Map(),
    sequence: new Map(),
    calls: [],
    preparations: [],
    submissions: [],
    loseResponse: false,
    unexpected: [],
  };
  state.prepare = (command) => prepareBrowserAdministration(state, command);
  state.commit = (review) => commitBrowserAdministration(state, review);
  await page.route(`**/api/v1/cases/${state.caseId}/subjects**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.calls.push({ path: url.pathname, search: url.search, method: request.method() });
    expect(request.method()).toBe('GET');
    const rows = [decisions.subject, state.replacementSubject];
    const parts = url.pathname.split('/subjects')[1].split('/').filter(Boolean);
    if (!parts.length)
      return route.fulfill({
        json: {
          subjects: rows.map((row) => ({
            case_id: row.case_id,
            id: row.id,
            revision: row.revision,
            kind: row.values.kind,
            display_name: row.values.name.value,
          })),
          has_more: false,
          next_after_id: null,
        },
      });
    const row = rows.find(
      (value) =>
        value.id === parts[0] &&
        (parts.length === 1 || (parts[1] === 'revisions' && value.revision === Number(parts[2]))),
    );
    return row ? route.fulfill({ json: row }) : fail(route, 'subject_not_found', 404);
  });
  await page.route(`**${state.base}**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      search: url.search,
      method: request.method(),
      body: request.postDataJSON(),
    };
    state.calls.push(call);
    const parts = url.pathname.slice(state.base.length).split('/').filter(Boolean);
    if (request.method() === 'GET') {
      if (!parts.length) {
        const after = url.searchParams.get('after_operation_id'),
          limit = Number(url.searchParams.get('limit') || 10);
        const rows = [...state.operations.values()]
          .filter((row) => !after || row.origin.operation_id > after)
          .sort((a, b) => a.origin.operation_id.localeCompare(b.origin.operation_id));
        const items = rows.slice(0, limit),
          more = rows.length > limit;
        return route.fulfill({
          json: {
            case_id: state.caseId,
            items,
            has_more: more,
            next_after_operation_id: more ? items.at(-1).origin.operation_id : null,
          },
        });
      }
      const value = state.operations.get(parts[0]);
      return value
        ? route.fulfill({ json: value })
        : fail(route, 'measure_administrative_not_found', 404);
    }
    if (role === 'paralegal') return fail(route, 'permission_denied', 403);
    if (closed) return fail(route, 'case_closed');
    if (request.method() !== 'POST' || !['prepare', 'submit'].includes(parts[0]) || url.search) {
      state.unexpected.push(call);
      return fail(route, 'invalid_request', 400);
    }
    const command = parts[0] === 'prepare' ? call.body : call.body.command;
    expect(command.case_id).toBe(state.caseId);
    const replay = state.operations.get(command.operation_id);
    if (!replay) {
      expect(command.context).toEqual(state.context.expectation);
      expect(command.target).toEqual(state.records.get(command.target.id).at(-1).reference);
    }
    if (command.action.kind === 'replace_entered_in_error')
      expect(command.action.subject).toEqual({
        id: state.replacementSubject.id,
        revision: state.replacementSubject.revision,
        values_digest: state.replacementSubject.values_digest,
      });
    const prepared = replay?.capture.review ?? state.prepare(command);
    if (parts[0] === 'prepare') {
      state.preparations.push(clone(prepared));
      return route.fulfill({ json: prepared });
    }
    expect(call.body).toEqual({
      command: prepared.command,
      expected_submission_digest: prepared.submission_digest,
      expected_review_digest: prepared.review_digest,
    });
    state.submissions.push(clone(call.body));
    const value = state.commit(prepared);
    if (state.loseResponse) {
      state.loseResponse = false;
      return route.abort('failed');
    }
    return route.fulfill({ status: 201, json: value });
  });
  return state;
}

export async function openAdministration(page, state, action) {
  await openMeasures(page);
  await measurePanel(page)
    .getByRole('button', { name: `Consultar medida ${state.target.reference.id}`, exact: true })
    .click();
  await measurePanel(page)
    .getByRole('button', { name: administrativeActions[action], exact: true })
    .click();
  await expect(administrationEditor(page)).toBeVisible();
}
export async function fillAdministration(page, action) {
  const editor = administrationEditor(page);
  await editor
    .getByLabel(/Motivo de rectificaci[o\u00f3]n/, { exact: true })
    .fill('Rectificacion administrativa declarada en soporte');
  if (action === 'correct') {
    await editor
      .getByLabel('Condiciones', { exact: true })
      .fill('Texto de condiciones rectificado expresamente');
    await editor
      .getByLabel(/Declaraci[o\u00f3]n de vigencia/, { exact: true })
      .fill('Vigencia textual rectificada sin completar el horario');
    await editor
      .getByLabel(/Texto de supervisi[o\u00f3]n/, { exact: true })
      .fill('Texto de supervision rectificado sin nueva ficha');
  } else if (action === 'replace_entered_in_error') {
    await editor.getByRole('button', { name: 'Elegir sujeto', exact: true }).click();
    const picker = editor.getByRole('region', { name: 'Elegir identidad existente', exact: true });
    await picker
      .getByRole('button', {
        name: 'Consultar identidad: Persona sustituta declarada',
        exact: true,
      })
      .click();
    await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  }
}
export async function prepareAdministration(page) {
  const editor = administrationEditor(page);
  await editor.getByRole('button', { name: 'Revisar rectificacion', exact: true }).click();
  const submit = editor.getByRole('button', { name: 'Confirmar rectificacion', exact: true });
  await expect(submit).toBeDisabled();
  await editor
    .getByRole('checkbox', {
      name: 'Reconozco la rectificacion y su alcance administrativo',
      exact: true,
    })
    .check();
  await expect(submit).toBeEnabled();
}
export const submitAdministration = (page) =>
  administrationEditor(page)
    .getByRole('button', { name: 'Confirmar rectificacion', exact: true })
    .click();
