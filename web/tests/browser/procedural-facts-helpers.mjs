import { prepareFact } from '../fixtures/procedural-facts-sources.mjs';
import { expect } from '@playwright/test';
import { setupResults } from './hearing-result-helpers.mjs';
import { login, navigate } from './helpers.mjs';
import {
  factCaseId,
  factKey,
  factRecord,
  factRow,
  factHistoryRow,
} from '../fixtures/procedural-facts.mjs';
export const failFact = (route, code, status = 409) =>
  route.fulfill({ status, json: { error: { code } } });
export async function setupFacts(page, { facts = [], ...options } = {}) {
  const results = await setupResults(page, options);
  const state = {
    results,
    calls: [],
    submissions: [],
    records: new Map(),
    pageSize: 20,
    historySize: 10,
  };
  for (const row of facts)
    state.records.set(factKey(row), [
      ...(state.records.get(factKey(row)) || []),
      structuredClone(row),
    ]);
  state.prepare = (command) => prepareFact(state, command);
  state.commit = (prepared) => {
    const row = factRecord(prepared),
      key = factKey(row);
    state.records.set(key, [...(state.records.get(key) || []), row]);
    return row;
  };
  await page.route('**/api/v1/cases/*/resolutions**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const match = /\/cases\/([^/]+)\/resolutions(?:\/([^/]+)\/notifications)?(.*)/.exec(
      url.pathname,
    );
    const family = match[2] ? 'notification' : 'resolution',
      parent = match[2];
    const parts = match[3].split('/').filter(Boolean);
    const call = {
      path: url.pathname,
      search: url.search,
      method: request.method(),
      body: request.postDataJSON(),
      family,
      parent,
    };
    state.calls.push(call);
    if (options.role === 'client' || results.scheduling.denied)
      return failFact(route, 'permission_denied', 403);
    if (state.handle && (await state.handle(route, call))) return;
    if (match[1] !== factCaseId) return failFact(route, 'case_not_found', 404);
    if (call.method !== 'GET') {
      if (options.role === 'paralegal') return failFact(route, 'permission_denied', 403);
      if (results.scheduling.context.administrative_status === 'closed')
        return failFact(route, 'case_closed');
      const command = parts[0] === 'prepare' ? call.body : call.body.command;
      if (
        command.family !== family ||
        command.resolution_id !== parent ||
        (parts[0] && parts[0] !== 'prepare' && parts[0] !== command.id)
      )
        return failFact(route, 'procedural_fact_command_mismatch', 400);
      const current = state.records.get(factKey(command))?.at(-1);
      if ((current?.revision || 0) !== command.change.expected_revision)
        return failFact(route, 'procedural_fact_revision_conflict');
      if (current?.status === 'withdrawn')
        return failFact(route, 'procedural_fact_already_withdrawn');
      const prepared = state.prepare(command);
      if (parts[0] === 'prepare') return route.fulfill({ json: prepared });
      state.submissions.push(command);
      if (call.body.expected_submission_digest !== prepared.submission_digest)
        return failFact(route, 'procedural_fact_submission_mismatch');
      return route.fulfill({ status: 201, json: state.commit(prepared) });
    }
    if (!parts.length) {
      const status = url.searchParams.get('status') || 'all',
        after = url.searchParams.get('after_id');
      let rows = [...state.records.values()]
        .map((items) => items.at(-1))
        .filter(
          (row) =>
            row.family === family &&
            row.resolution_id === parent &&
            (status === 'all' || row.status === status) &&
            (!after || row.id > after),
        );
      rows.sort((a, b) => a.id.localeCompare(b.id));
      const more = rows.length > state.pageSize;
      rows = rows.slice(0, state.pageSize);
      return route.fulfill({
        json: {
          [family === 'resolution' ? 'resolutions' : 'notifications']: rows.map(factRow),
          has_more: more,
          next_after_id: more ? rows.at(-1).id : null,
        },
      });
    }
    const history = state.records.get(factKey({ family, resolution_id: parent, id: parts[0] }));
    if (!history) return failFact(route, 'procedural_fact_not_found', 404);
    if (parts[1] === 'history') {
      let rows = [...history]
        .reverse()
        .filter(
          (row) =>
            !url.searchParams.has('before_revision') ||
            row.revision < Number(url.searchParams.get('before_revision')),
        );
      const more = rows.length > state.historySize;
      rows = rows.slice(0, state.historySize);
      return route.fulfill({
        json: {
          revisions: rows.map(factHistoryRow),
          has_more: more,
          next_before_revision: more ? rows.at(-1).revision : null,
        },
      });
    }
    const row =
      parts[1] === 'revisions'
        ? history.find((item) => item.revision === Number(parts[2]))
        : history.at(-1);
    return row ? route.fulfill({ json: row }) : failFact(route, 'procedural_fact_not_found', 404);
  });
  return state;
}
export const factNoun = (family = 'resolution') =>
  family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
export const factEditor = (page, family) =>
  page.getByRole('region', { name: `Formulario de ${factNoun(family)}`, exact: true });
export const factDetail = (page, family) =>
  page.getByRole('region', { name: `Detalle de ${factNoun(family)}`, exact: true });
export const factList = (page, family = 'resolution') =>
  page.getByRole('region', {
    name: family === 'resolution' ? 'Resoluciones registradas' : 'Notificaciones registradas',
    exact: true,
  });
export async function openFacts(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Resoluciones', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resoluciones y notificaciones', exact: true }),
  ).toBeVisible();
}
export async function fillResolution(page, summary = 'Declaracion de resolucion capturada') {
  const editor = factEditor(page);
  await editor
    .getByRole('combobox', { name: 'Clase de resoluci\u00f3n', exact: true })
    .selectOption('order');
  await editor.getByRole('combobox', { name: 'Emisor', exact: true }).selectOption('unknown');
  await editor
    .getByLabel('Motivo: Emisor', { exact: true })
    .fill('El emisor no consta en la fuente');
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true })
    .selectOption('unknown');
  await editor.getByLabel('Resumen de la resoluci\u00f3n', { exact: true }).fill(summary);
  await editor
    .getByRole('combobox', { name: 'Procedencia de la resoluci\u00f3n', exact: true })
    .selectOption('operator_note');
  await editor
    .getByLabel('Nota: Procedencia de la resoluci\u00f3n', { exact: true })
    .fill('Declaracion informada sin soporte');
}
export async function confirmFact(page, family) {
  const editor = factEditor(page, family);
  await editor.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
}
export async function fillNotification(page, summary = 'Declaracion de notificacion capturada') {
  const editor = factEditor(page, 'notification');
  for (const [label, value] of [
    ['Car\u00e1cter de notificaci\u00f3n', 'personal'],
    ['Medio de notificaci\u00f3n', 'in_person'],
    ['Contexto de notificaci\u00f3n', 'outside_hearing'],
    ['Resultado declarado', 'attempted'],
  ])
    await editor.getByRole('combobox', { name: label, exact: true }).selectOption(value);
  await editor
    .getByRole('combobox', { name: 'Precisi\u00f3n de pr\u00e1ctica', exact: true })
    .selectOption('unknown');
  for (const label of ['Destinatario declarado', 'Receptor material']) {
    await editor.getByRole('combobox', { name: label, exact: true }).selectOption('unknown');
    await editor
      .getByLabel(`Motivo: ${label}`, { exact: true })
      .fill('Persona no identificada en la fuente');
  }
  await editor
    .getByRole('combobox', { name: 'Representaci\u00f3n declarada', exact: true })
    .selectOption('not_recorded');
  await editor
    .getByLabel('Motivo: Representaci\u00f3n declarada', { exact: true })
    .fill('No se declaro representacion');
  await editor.getByLabel('Resumen de la notificaci\u00f3n', { exact: true }).fill(summary);
  await editor
    .getByRole('combobox', { name: 'Procedencia de la notificaci\u00f3n', exact: true })
    .selectOption('operator_note');
  await editor
    .getByLabel('Nota: Procedencia de la notificaci\u00f3n', { exact: true })
    .fill('Declaracion sin efectos inferidos');
}
