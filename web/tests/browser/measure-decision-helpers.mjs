import { expect } from '@playwright/test';
import { setupPrecautionaryScheduling } from './precautionary-hearing-scheduling-helpers.mjs';
import { login, navigate, caseId, document } from './helpers.mjs';
import { measureRecord } from '../fixtures/measure-records.mjs';
import {
  inMeasureCase,
  seedBrowserMeasure,
  prepareBrowserDecision,
  commitBrowserDecision,
  clone,
} from '../fixtures/measure-decision-browser.mjs';

export { clone };
export const decisionEditor = (page) =>
  page.getByRole('region', { name: 'Decision cautelar', exact: true });
export const decisionPanel = (page) =>
  page.getByRole('region', { name: 'Medidas cautelares', exact: true });
export const decisionDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de decision cautelar', exact: true });
const fail = (route, code, status = 409) => route.fulfill({ status, json: { error: { code } } });
const paged = (items, id, url) => {
  const after = url.searchParams.get('after_id'),
    limit = Number(url.searchParams.get('limit') || 10);
  const all = items
    .filter((row) => !after || id(row) > after)
    .sort((a, b) => id(a).localeCompare(id(b)));
  const selected = all.slice(0, limit),
    more = all.length > limit;
  return {
    case_id: caseId,
    items: selected,
    has_more: more,
    next_after_id: more ? id(selected.at(-1)) : null,
  };
};

export async function setupMeasureDecisions(
  page,
  { role = 'owner', closed = false, seeded = false } = {},
) {
  const scheduling = await setupPrecautionaryScheduling(page, { role, closed });
  const state = {
    caseId,
    base: `/api/v1/cases/${caseId}/measure-decisions`,
    scheduling,
    actor: clone(scheduling.actor),
    context: clone(scheduling.context),
    support: clone(document),
    subject: inMeasureCase(measureRecord().record.capture.result.sources.subject, caseId),
    records: new Map(),
    operations: new Map(),
    sequence: new Map(),
    calls: [],
    preparations: [],
    submissions: [],
    unexpected: [],
    loseResponse: false,
  };
  state.prepare = (command) => prepareBrowserDecision(state, command);
  state.commit = (prepared) => commitBrowserDecision(state, prepared);
  if (seeded) state.seed = seedBrowserMeasure(state);
  await page.route(`**/api/v1/cases/${caseId}/subjects**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.calls.push({ path: url.pathname, search: url.search, method: request.method() });
    expect(request.method()).toBe('GET');
    const parts = url.pathname.split('/subjects')[1].split('/').filter(Boolean);
    if (!parts.length)
      return route.fulfill({
        json: {
          subjects: [
            {
              case_id: caseId,
              id: state.subject.id,
              revision: state.subject.revision,
              kind: state.subject.values.kind,
              display_name: state.subject.values.name.value,
            },
          ],
          has_more: false,
          next_after_id: null,
        },
      });
    if (
      parts[0] === state.subject.id &&
      (parts.length === 1 ||
        (parts[1] === 'revisions' && Number(parts[2]) === state.subject.revision))
    )
      return route.fulfill({ json: state.subject });
    return fail(route, 'subject_not_found', 404);
  });
  await page.route(`**/api/v1/cases/${caseId}/measures**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    state.calls.push({ path: url.pathname, search: url.search, method: request.method() });
    expect(request.method()).toBe('GET');
    const parts = url.pathname.split('/measures')[1].split('/').filter(Boolean);
    if (!parts.length)
      return route.fulfill({
        json: paged(
          [...state.records.values()].map((rows) => rows.at(-1)),
          (row) => row.reference.id,
          url,
        ),
      });
    const rows = state.records.get(parts[0]);
    const selected =
      parts.length === 1
        ? rows?.at(-1)
        : parts[1] === 'revisions'
          ? rows?.find(
              (row) =>
                row.reference.revision === Number(parts[2]) &&
                row.reference.capture_digest === url.searchParams.get('capture_digest'),
            )
          : null;
    return selected
      ? route.fulfill({ json: selected })
      : fail(route, 'measure_record_not_found', 404);
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
      if (!parts.length)
        return route.fulfill({
          json: paged([...state.operations.values()], (row) => row.origin.decision_id, url),
        });
      const found =
        parts[0] === 'operations'
          ? state.operations.get(parts[1])
          : [...state.operations.values()].find((row) => row.origin.decision_id === parts[0]);
      return found
        ? route.fulfill({ json: found })
        : fail(route, 'measure_decision_not_found', 404);
    }
    if (role === 'paralegal') return fail(route, 'permission_denied', 403);
    if (closed) return fail(route, 'case_closed');
    if (request.method() !== 'POST' || !['prepare', 'submit'].includes(parts[0]) || url.search) {
      state.unexpected.push(call);
      return fail(route, 'invalid_request', 400);
    }
    const command = parts[0] === 'prepare' ? call.body : call.body.command;
    expect(command.case_id).toBe(caseId);
    expect(command.anchor).toEqual(state.expectedAnchor ?? null);
    expect(command.context).toEqual(state.context.expectation);
    expect(command.values.support).toEqual({
      document_id: document.id,
      version: document.version,
      digest: document.digest,
    });
    const replay = state.operations.get(command.operation_id);
    for (const effect of command.outcome.effects ?? []) {
      const previous =
        effect.action === 'substitute'
          ? effect.predecessors
          : effect.previous
            ? [effect.previous]
            : [];
      if (!replay)
        for (const reference of previous)
          expect(reference).toEqual(state.records.get(reference.id).at(-1).reference);
      const proposals =
        effect.action === 'substitute'
          ? effect.successors
          : effect.action === 'impose'
            ? [effect.proposal]
            : [];
      for (const proposal of proposals)
        expect(proposal.values.subject).toEqual({
          id: state.subject.id,
          revision: state.subject.revision,
          values_digest: state.subject.values_digest,
        });
    }
    const prepared = replay
      ? { family: replay.family, review: replay.group.review }
      : state.prepare(command);
    if (parts[0] === 'prepare') {
      state.preparations.push(clone(prepared));
      return route.fulfill({ json: prepared });
    }
    expect(call.body).toEqual({
      command: prepared.review.command,
      expected_submission_digest: prepared.review.submission_digest,
      expected_review_digest: prepared.review.review_digest,
    });
    state.submissions.push(clone(call.body));
    const result = state.commit(prepared);
    if (state.loseResponse) {
      state.loseResponse = false;
      return route.abort('failed');
    }
    return route.fulfill({ status: 201, json: result });
  });
  return state;
}

export async function openMeasures(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Medidas cautelares', exact: true }).click();
  await expect(decisionPanel(page)).toBeVisible();
}
export async function newDecision(page) {
  await page.getByRole('button', { name: 'Registrar decision cautelar', exact: true }).click();
  await expect(decisionEditor(page)).toBeVisible();
}
export async function fillDecisionCommon(page) {
  const editor = decisionEditor(page);
  await editor.getByLabel('Autoridad', { exact: true }).fill('Juzgado declarado en soporte');
  await editor
    .getByLabel(/Justificaci[o\u00f3]n/, { exact: true })
    .fill('Decision declarada en documento seleccionado');
  await editor
    .getByLabel(/Precisi[o\u00f3]n de decisi[o\u00f3]n/, { exact: true })
    .selectOption('unknown');
  await editor
    .getByLabel(/Motivo de tiempo desconocido de decisi[o\u00f3]n/, { exact: true })
    .fill('No consta el momento de decision');
  await editor.getByLabel('Localizador', { exact: true }).fill('Pagina 2');
  await editor.getByRole('button', { name: 'Elegir soporte', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi[o\u00f3]n 1 \/ contrato.pdf/ }).click();
  await picker.getByRole('button', { name: /Usar esta versi[o\u00f3]n/ }).click();
}
export async function fillImposition(page) {
  const editor = decisionEditor(page),
    effect = editor.getByRole('group', { name: 'Efecto 1', exact: true });
  await editor.getByRole('combobox', { name: 'Resultado', exact: true }).selectOption('changes');
  await effect.getByLabel(/Acci[o\u00f3]n de medida 1/, { exact: true }).selectOption('impose');
  await effect.getByRole('button', { name: 'Elegir sujeto', exact: true }).click();
  const picker = effect.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: 'Consultar identidad: Persona declarada', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await effect
    .getByRole('combobox', { name: 'Clase de medida', exact: true })
    .selectOption('periodic_appearance');
  await effect
    .getByLabel('Condiciones', { exact: true })
    .fill('Presentarse cada viernes segun soporte');
  await effect
    .getByLabel(/Precisi[o\u00f3]n de inicio de vigencia/, { exact: true })
    .selectOption('unknown');
  await effect
    .getByLabel('Motivo de tiempo desconocido de inicio de vigencia', { exact: true })
    .fill('No consta inicio de vigencia');
  await effect
    .getByLabel(/Declaraci[o\u00f3]n de vigencia/, { exact: true })
    .fill('Vigencia declarada sin termino conocido');
  await effect
    .getByRole('combobox', { name: /Supervisi[o\u00f3]n/, exact: true })
    .selectOption('unknown');
  await effect
    .getByLabel(/Motivo de supervisi[o\u00f3]n desconocida/, { exact: true })
    .fill('No consta autoridad supervisora');
}
export async function prepareDecision(page) {
  const editor = decisionEditor(page);
  await editor.getByRole('button', { name: 'Revisar decision', exact: true }).click();
  const submit = editor.getByRole('button', { name: 'Confirmar decision', exact: true });
  await expect(submit).toBeDisabled();
  await editor
    .getByRole('checkbox', {
      name: 'Reconozco la decision y las fuentes seleccionadas',
      exact: true,
    })
    .check();
  await expect(submit).toBeEnabled();
}
export const submitDecision = (page) =>
  decisionEditor(page).getByRole('button', { name: 'Confirmar decision', exact: true }).click();
