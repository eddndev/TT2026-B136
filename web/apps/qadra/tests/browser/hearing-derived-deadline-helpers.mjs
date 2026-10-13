import { expect } from '@playwright/test';
import { setupResults, openResults, resultPanel } from './hearing-result-helpers.mjs';
import { setupDeadlines } from './deadline-editor-helpers.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  clone,
} from '../fixtures/hearing-derived-deadline-unit.mjs';

export const derivedEditor = (page) =>
  page.getByRole('region', { name: 'Resultado y plazo configurado', exact: true });

export async function setupDerivedDeadline(page, options = {}) {
  const results = await setupResults(page, options);
  const deadlines = await setupDeadlines(page, {
    ...options,
    factState: { results },
    profiles: [derivedReady().deadline.profile],
  });
  const state = {
    results,
    deadlines,
    calls: [],
    submissions: [],
    committed: new Map(),
    handle: null,
  };
  state.prepare = (command) => {
    const ready = derivedReady();
    ready.command = clone(command);
    ready.result = results.prepare(command.result);
    ready.result.actor_id = principal().id;
    ready.deadline.definition = clone(command.deadline.change.definition);
    ready.deadline.tracking = clone(command.deadline.change.tracking);
    return ready;
  };
  state.commit = (ready) => {
    const key = ready.command.result.operation_id;
    const previous = state.committed.get(key);
    if (previous) return clone(previous);
    const record = derivedRecord(ready);
    state.committed.set(key, clone(record));
    results.records.set(record.result.id, [clone(record.result)]);
    deadlines.records.set(record.deadline.id, [clone(record.deadline)]);
    return record;
  };
  results.handle = async (route, call) => {
    if (!call.path.includes('/results/derived-deadline/')) return false;
    state.calls.push(clone(call));
    if (state.handle && (await state.handle(route, call))) return true;
    const preparing = call.path.endsWith('/prepare');
    const command = preparing ? call.body : call.body.command;
    const record = state.committed.get(command.result.operation_id);
    if (preparing) {
      await route.fulfill({
        json: record ? { state: 'replay', record: clone(record) } : state.prepare(command),
      });
    } else {
      state.submissions.push(clone(call.body));
      await route.fulfill({ status: 201, json: record ?? state.commit(state.prepare(command)) });
    }
    return true;
  };
  return state;
}

export async function openDerivedDeadline(page) {
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar resultado y plazo', exact: true })
    .click();
  await expect(derivedEditor(page)).toBeVisible();
}

export async function fillDerivedDeadline(page) {
  const form = derivedEditor(page);
  await form.getByRole('combobox', { name: 'Ocurrencia', exact: true }).selectOption('occurred');
  await form
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('partial');
  await form.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await form.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await form
    .getByLabel('Relato del operador', { exact: true })
    .fill('Resultado con plazo declarado');
  await form
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('operator_note');
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill('Respuesta al resultado');
  await form.getByRole('button', { name: 'Elegir perfil exacto', exact: true }).click();
  await form.getByRole('button', { name: 'Revisiones de Horas declaradas', exact: true }).click();
  await form.getByRole('button', { name: /Consultar perfil revisi\u00f3n 1/ }).click();
  await form.getByRole('button', { name: 'Usar este perfil exacto', exact: true }).click();
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  await form
    .getByRole('button', { name: 'Elegir responsable staff@example.test', exact: true })
    .click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('fixed');
  await form
    .getByRole('combobox', { name: 'Cuando cambie la fuente', exact: true })
    .selectOption('follow');
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

export async function prepareDerivedDeadline(page) {
  const form = derivedEditor(page);
  await form.getByRole('button', { name: 'Preparar resultado y plazo', exact: true }).click();
  await expect(
    form.getByRole('heading', { name: 'Revisa el resultado y el plazo', exact: true }),
  ).toBeVisible();
  await expect(form).toContainText('Sin vencimiento calculado');
  await expect(
    form.getByRole('button', { name: 'Confirmar resultado y plazo', exact: true }),
  ).toBeDisabled();
}

export async function submitDerivedDeadline(page) {
  const form = derivedEditor(page);
  await form
    .getByRole('checkbox', { name: 'Confirmo el resultado y el plazo revisados', exact: true })
    .check();
  await form.getByRole('button', { name: 'Confirmar resultado y plazo', exact: true }).click();
}
