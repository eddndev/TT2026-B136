import { expect } from '@playwright/test';
import { setupFacts } from './procedural-facts-helpers.mjs';
import { login, navigate, caseId } from './helpers.mjs';
import {
  prepared,
  detail,
  summary,
  history,
  profile,
  id,
  hash,
} from '../fixtures/deadline-unit.mjs';
export { caseId } from './helpers.mjs';
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de plazo', exact: true });
export const deadlineError = (route, code, status = 409) =>
  route.fulfill({ status, json: { error: { code } } });
export function deadlineFixture(action = 'register') {
  const value = prepared(action);
  value.case_id = caseId;
  value.definition.input.selection.case_id = caseId;
  value.calculation.profile.scope.case_id = caseId;
  value.calculation.profile.href = `/api/v1/cases/${caseId}/deadline-profiles/${id(2)}/revisions/1`;
  value.calculation.material.case_id = caseId;
  if (value.command.change.definition)
    value.command.change.definition.input.selection.case_id = caseId;
  return value;
}
export async function setupDeadlines(
  page,
  { deadlines = [], profiles = [profile(caseId)], ...options } = {},
) {
  const facts = await setupFacts(page, options);
  const state = {
    facts,
    calls: [],
    submissions: [],
    records: new Map(),
    profiles,
    responsibles: [{ id: id(4), email: 'staff@example.test', role: 'owner' }],
    pageSize: 20,
    historySize: 10,
  };
  for (const row of deadlines)
    state.records.set(row.id, [...(state.records.get(row.id) || []), structuredClone(row)]);
  state.prepare = (command) => {
    const value = deadlineFixture(command.change.action),
      current = state.records.get(command.deadline_id)?.at(-1);
    value.command = structuredClone(command);
    value.result_revision = command.change.expected_revision + 1;
    value.definition = structuredClone(command.change.definition || current.definition);
    value.attention = structuredClone(
      command.change.attention || current?.attention || { status: 'pending' },
    );
    value.responsible =
      state.responsibles.find((row) => row.id === value.definition.responsible_id) ||
      current?.responsible;
    if (current && !command.change.definition)
      value.calculation = structuredClone(current.calculation);
    return value;
  };
  state.commit = (value) => {
    const row = detail(value);
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    return row;
  };
  await page.route(/\/api\/v1\/auth\/(mfa\/|me)/, async (route) => {
    const user = { id: id(4), email: 'staff@example.test', role: options.role || 'owner' };
    if (route.request().url().endsWith('/me')) return route.fulfill({ json: user });
    return route.fulfill({
      json: { access_token: 'deadline-token', user, expires_in_seconds: 86400 },
    });
  });
  await page.route('**/api/v1/**/deadline-profiles**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const collection = url.pathname.includes('/cases/')
      ? { kind: 'case', case_id: caseId }
      : { kind: 'global' };
    const suffix = url.pathname.split('/deadline-profiles')[1].split('/').filter(Boolean);
    state.calls.push({ method: request.method(), path: url.pathname, search: url.search });
    const rows = state.profiles.filter(
      (row) => collection.kind === 'case' || row.scope.kind === 'global',
    );
    if (!suffix.length)
      return route.fulfill({
        json: {
          collection,
          profiles: rows.map((row) => ({
            id: row.id,
            revision: row.revision,
            status: row.status,
            algorithm: row.algorithm,
            definition_digest: row.definition_digest,
            title: row.definition.title,
            scope: row.scope,
          })),
          has_more: false,
          next_after_id: null,
        },
      });
    const row = rows.find((item) => item.id === suffix[0]);
    if (!row) return deadlineError(route, 'deadline_profile_not_found', 404);
    if (suffix[1] === 'history') {
      const { collection: unused, definition: ignored, ...header } = row;
      return route.fulfill({
        json: { collection, revisions: [header], has_more: false, next_before_revision: null },
      });
    }
    return route.fulfill({ json: { ...row, collection } });
  });
  await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      parts = url.pathname.split('/deadlines')[1].split('/').filter(Boolean);
    const call = {
      method: request.method(),
      path: url.pathname,
      search: url.search,
      body: request.postDataJSON(),
    };
    state.calls.push(call);
    if (options.role === 'client' || facts.results.scheduling.denied)
      return deadlineError(route, 'permission_denied', 403);
    if (state.handle && (await state.handle(route, call))) return;
    if (parts[0] === 'responsibles') {
      const after = url.searchParams.get('after_id'),
        limit = Number(url.searchParams.get('limit') || 20);
      let rows = state.responsibles.filter((row) => !after || row.id > after);
      const more = rows.length > limit;
      rows = rows.slice(0, limit);
      return route.fulfill({
        json: {
          case_id: caseId,
          responsibles: rows,
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        },
      });
    }
    if (call.method !== 'GET') {
      if (options.role === 'paralegal') return deadlineError(route, 'permission_denied', 403);
      if (facts.results.scheduling.context.administrative_status === 'closed')
        return deadlineError(route, 'case_closed');
      const command = parts[0] === 'prepare' ? call.body : call.body.command;
      const current = state.records.get(command.deadline_id)?.at(-1);
      if ((current?.revision || 0) !== command.change.expected_revision)
        return deadlineError(route, 'deadline_revision_conflict');
      const value = state.prepare(command);
      if (parts[0] === 'prepare') return route.fulfill({ json: value });
      state.submissions.push(command);
      return route.fulfill({ status: 201, json: state.commit(value) });
    }
    if (!parts.length) {
      const after = url.searchParams.get('after_id'),
        status = url.searchParams.get('status') || 'active';
      let rows = [...state.records.values()]
        .map((items) => items.at(-1))
        .filter((row) => (!after || row.id > after) && (status === 'all' || row.status === status))
        .sort((a, b) => a.id.localeCompare(b.id));
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: {
          case_id: caseId,
          deadlines: rows.map(summary),
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        },
      });
    }
    const rows = state.records.get(parts[0]);
    if (!rows) return deadlineError(route, 'deadline_not_found', 404);
    if (parts[1] === 'history') {
      const before = Number(url.searchParams.get('before_revision') || 4294967296);
      const eligible = rows.filter((row) => row.revision < before).toReversed(),
        more = eligible.length > state.historySize;
      const page = eligible.slice(0, state.historySize);
      return route.fulfill({
        json: {
          case_id: caseId,
          id: parts[0],
          revisions: page.map(history),
          has_more: more,
          next_before_revision: more ? page.at(-1).revision : null,
        },
      });
    }
    const row =
      parts[1] === 'revisions'
        ? rows.find((item) => item.revision === Number(parts[2]))
        : rows.at(-1);
    return row ? route.fulfill({ json: row }) : deadlineError(route, 'deadline_not_found', 404);
  });
  return state;
}
export async function openDeadlines(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Plazos', exact: true }).click();
}
export async function fillDeadline(page, title = 'Respuesta con fuente pendiente') {
  const form = editor(page);
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill(title);
  await form.getByRole('button', { name: 'Elegir perfil exacto', exact: true }).click();
  await form.getByRole('button', { name: 'Revisiones de Horas declaradas', exact: true }).click();
  await form.getByRole('button', { name: /Consultar perfil revisi\u00f3n 1/ }).click();
  await form.getByRole('button', { name: 'Usar este perfil exacto', exact: true }).click();
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  await form
    .getByRole('button', { name: 'Elegir responsable staff@example.test', exact: true })
    .click();
  await form.getByRole('combobox', { name: 'Tipo de fuente', exact: true }).selectOption('unknown');
  await form
    .getByLabel('Motivo de fuente no identificada', { exact: true })
    .fill('Falta la fuente de inicio');
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill('Supuesto revisado por la operadora');
  await form.getByLabel('Localizador de aplicabilidad', { exact: true }).fill('Acto, pagina 1');
  await form
    .getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true })
    .selectOption('yes');
  await form
    .getByRole('combobox', { name: 'Existe una incidencia sin resolver', exact: true })
    .selectOption('no');
  await form
    .getByRole('combobox', { name: 'Se cumple la condicion 1', exact: true })
    .selectOption('yes');
  await form.getByLabel('Localizador de condici\u00f3n 1', { exact: true }).fill('Acto, pagina 2');
}
export async function confirmDeadline(page) {
  const form = editor(page);
  await form.getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(form.getByRole('button', { name: 'Confirmar plazo', exact: true })).toBeDisabled();
  await form.getByRole('checkbox', { name: /Revise las declaraciones/ }).check();
  await form.getByRole('button', { name: 'Confirmar plazo', exact: true }).click();
}
