import { expect } from '@playwright/test';
import { setupHearings, openHearings } from './hearing-helpers.mjs';
import { caseId, document } from './helpers.mjs';
import { absoluteSession } from '../fixtures/session.mjs';
import {
  precautionaryBrowserRecord,
  preparePrecautionaryBrowser,
  commitPrecautionaryBrowser,
  clone,
} from '../fixtures/precautionary-hearing-browser.mjs';

export { openHearings, clone };
export const precautionaryEditor = (page) =>
  page.getByRole('region', { name: 'Convocatoria cautelar', exact: true });
export const precautionaryPanel = (page) =>
  page.getByRole('region', { name: 'Audiencias cautelares', exact: true });
export const precautionaryDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia cautelar', exact: true });
const fail = (route, code, status = 409) => route.fulfill({ status, json: { error: { code } } });

export async function setupPrecautionaryScheduling(
  page,
  { role = 'owner', closed = false, seeded = false } = {},
) {
  const hearings = await setupHearings(page, { role, closed });
  const example = precautionaryBrowserRecord(caseId);
  const state = {
    hearings,
    base: `/api/v1/cases/${caseId}/precautionary-hearings`,
    context: clone(example.capture.review.observed_context),
    actor: clone(example.capture.review.actor),
    support: clone(document),
    calls: [],
    submissions: [],
    preparations: [],
    expectedReviewTargets: [],
    records: new Map(),
    operations: new Map(),
    loseResponse: false,
    unexpected: [],
  };
  state.prepare = (command) => preparePrecautionaryBrowser(state, command);
  state.commit = (review) => commitPrecautionaryBrowser(state, review);
  if (seeded) {
    const command = clone(example.capture.review.command);
    command.change.values.scheduling_basis.support = {
      document_id: document.id,
      version: document.version,
      digest: document.digest,
    };
    state.seed = state.commit(state.prepare(command));
  }
  if (closed) {
    Object.assign(state.context.administration, {
      revision: 2,
      administrative_status: 'closed',
      changed_at: '2026-01-06T12:00:00Z',
      values_digest: 'c'.repeat(64),
    });
    state.context.context_digest = 'd'.repeat(64);
    state.context.expectation = {
      administration_revision: 2,
      stage_revision: 1,
      context_digest: state.context.context_digest,
    };
    hearings.state.admin.administration.revision = 2;
  }
  await page.route(/\/api\/v1\/auth\/(mfa\/|me)/, async (route) => {
    const user = { ...state.actor, role };
    return route.fulfill({
      json: route.request().url().endsWith('/me')
        ? user
        : absoluteSession(user, 'precautionary-scheduling-token'),
    });
  });
  await page.route(`**/api/v1/cases/${caseId}/precautionary-context`, async (route) => {
    state.calls.push({
      path: new URL(route.request().url()).pathname,
      method: route.request().method(),
    });
    await route.fulfill({ json: state.context });
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
        const after = url.searchParams.get('after_id'),
          limit = Number(url.searchParams.get('limit') || 10);
        const all = [...state.records.values()]
          .map((rows) => rows.at(-1))
          .filter((row) => !after || row.capture.review.command.hearing_id > after)
          .sort((a, b) =>
            a.capture.review.command.hearing_id.localeCompare(b.capture.review.command.hearing_id),
          );
        const items = all.slice(0, limit),
          more = all.length > limit;
        return route.fulfill({
          json: {
            case_id: caseId,
            items,
            has_more: more,
            next_after_id: more ? items.at(-1).capture.review.command.hearing_id : null,
          },
        });
      }
      const rows = state.records.get(parts[0]);
      const value =
        parts[0] === 'operations'
          ? state.operations.get(parts[1])
          : parts[1] === 'revisions'
            ? rows?.find((row) => row.capture.review.result_revision === Number(parts[2]))
            : rows?.at(-1);
      return value
        ? route.fulfill({ json: value })
        : fail(route, 'precautionary_hearing_not_found', 404);
    }
    if (role === 'paralegal') return fail(route, 'permission_denied', 403);
    if (closed) return fail(route, 'case_closed');
    if (request.method() !== 'POST' || !['prepare', 'submit'].includes(parts[0]) || url.search) {
      state.unexpected.push(call);
      return fail(route, 'invalid_request', 400);
    }
    const command = parts[0] === 'prepare' ? call.body : call.body.command;
    expect(command.case_id).toBe(caseId);
    const replay = state.operations.get(command.operation_id),
      previous = state.records.get(command.hearing_id)?.at(-1);
    if (!replay && command.change.action !== 'schedule') {
      expect(command.change.expected_revision).toBe(previous.capture.review.result_revision);
      expect(command.change.expected_capture_digest).toBe(previous.capture.capture_digest);
    }
    if (!replay && command.change.action !== 'cancel') {
      expect(command.change.context).toEqual(state.context.expectation);
      expect(command.change.values.participants).toEqual([]);
      expect(command.change.values.review_targets).toEqual(state.expectedReviewTargets);
      expect(command.change.values.scheduling_basis.support).toEqual({
        document_id: document.id,
        version: document.version,
        digest: document.digest,
      });
    }
    const review = replay?.capture.review ?? state.prepare(command);
    expect(review.command).toEqual(command);
    if (parts[0] === 'prepare') {
      state.preparations.push(clone(review));
      return route.fulfill({ json: review });
    }
    if (call.body.expected_submission_digest !== review.submission_digest)
      return fail(route, 'precautionary_hearing_submission_mismatch');
    if (call.body.expected_review_digest !== review.review_digest)
      return fail(route, 'precautionary_hearing_review_mismatch');
    state.submissions.push(clone(call.body));
    const result = state.commit(review);
    if (state.loseResponse) {
      state.loseResponse = false;
      return route.abort('failed');
    }
    return route.fulfill({ status: 201, json: result });
  });
  return state;
}

export async function openPrecautionaryForm(page) {
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia cautelar', exact: true }).click();
  await expect(precautionaryEditor(page)).toBeVisible();
}
export async function fillPrecautionaryForm(page) {
  const editor = precautionaryEditor(page);
  await editor.getByLabel(/Prop[o\u00f3]sito/).selectOption('imposition');
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-10-10');
  await editor.getByLabel('Hora', { exact: true }).fill('09:02:03');
  await editor.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await editor.getByLabel('Sede o enlace', { exact: true }).fill('Sala cautelar declarada');
  await editor.getByLabel('Nota', { exact: true }).fill('Convocatoria con soporte exacto');
  await editor.getByLabel(/Base de se[n\u00f1]alamiento/).fill('Senalamiento comunicado');
  await editor.getByLabel('Localizador', { exact: true }).fill('Pagina 2');
  await editor.getByRole('button', { name: 'Elegir soporte', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi[o\u00f3]n 1 \/ contrato.pdf/ }).click();
  await picker.getByRole('button', { name: /Usar esta versi[o\u00f3]n/ }).click();
}
export async function preparePrecautionary(page) {
  const editor = precautionaryEditor(page);
  await editor.getByRole('button', { name: 'Revisar convocatoria', exact: true }).click();
  const submit = editor.getByRole('button', { name: 'Confirmar convocatoria', exact: true });
  await expect(submit).toBeDisabled();
  await editor
    .getByRole('checkbox', {
      name: 'Reconozco la convocatoria y las fuentes seleccionadas',
      exact: true,
    })
    .check();
  await expect(submit).toBeEnabled();
}
export const submitPrecautionary = (page) =>
  precautionaryEditor(page)
    .getByRole('button', { name: 'Confirmar convocatoria', exact: true })
    .click();
