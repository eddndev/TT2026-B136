import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  factList,
  fillResolution,
  fillNotification,
  confirmFact,
} from './procedural-facts-helpers.mjs';
import { factRecord, factKey } from '../fixtures/procedural-facts.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { participant } from './participant-helpers.mjs';
import { document } from './helpers.mjs';

async function openNotice(page, parent) {
  await openFacts(page);
  await factList(page)
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${parent.id}`, exact: true })
    .click();
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }).click();
  await fillNotification(page);
}
async function support(editor, label, version) {
  await editor.getByRole('button', { name: `Elegir soporte: ${label}`, exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: new RegExp(`Versi\\u00f3n ${version} /`) }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await editor
    .getByLabel(`Localizador documental: ${label}`, { exact: true })
    .fill(`Pagina ${version}`);
}

test('explicit hearing selection retains the withdrawn exact result and the zero agreement UUID', async ({
  page,
}) => {
  const hearing = hearingRecord(),
    row = resultRecord();
  row.status = 'withdrawn';
  row.revision = 2;
  row.reason = 'Retiro administrativo';
  row.receipt = { ...row.receipt, action: 'withdraw', expected_revision: 1 };
  row.values.agreements = [
    { id: '00000000-0000-0000-0000-000000000000', text: 'Acuerdo exacto declarado' },
  ];
  const state = await setupFacts(page, { hearings: [hearing], results: [row] });
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  const editor = factEditor(page),
    label = 'Procedencia de la resoluci\u00f3n';
  await editor.getByRole('combobox', { name: label, exact: true }).selectOption('hearing_result');
  await editor.getByRole('button', { name: `Elegir resultado: ${label}`, exact: true }).click();
  const picker = editor.getByRole('region', {
    name: 'Elegir resultado hist\u00f3rico',
    exact: true,
  });
  expect(state.results.calls).toHaveLength(0);
  await picker
    .getByRole('button', { name: `Consultar resultados de audiencia ${hearing.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: `Consultar revisiones de resultado ${row.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Consultar resultado revisi\u00f3n 2', exact: true })
    .click();
  await picker
    .getByRole('combobox', { name: 'Acuerdo de origen', exact: true })
    .selectOption(row.values.agreements[0].id);
  await picker.getByRole('button', { name: 'Vincular este resultado exacto', exact: true }).click();
  await editor
    .getByLabel(`Localizador en el resultado: ${label}`, { exact: true })
    .fill('Acuerdo declarado');
  await confirmFact(page);
  await expect(editor).toHaveCount(0);
  const command = state.submissions[0],
    saved = state.records.get(factKey(command))[0];
  expect(command.change.values.provenance.reference).toEqual({
    hearing_id: hearing.id,
    result_id: row.id,
    revision: 2,
    agreement_id: row.values.agreements[0].id,
  });
  expect(saved.sources.hearing_results[0].status).toBe('withdrawn');
  expect(saved.sources.hearing_results[0].agreement).toEqual(row.values.agreements[0]);
  expect(saved.sources.direct_supports).toEqual([]);
});

test('notification selects an older parent revision explicitly while preserving the same parent UUID', async ({
  page,
}) => {
  const parent = factRecord(),
    later = { ...structuredClone(parent), revision: 2, status: 'withdrawn' };
  later.receipt = { ...later.receipt, action: 'withdraw', expected_revision: 1 };
  later.reason = 'Retiro administrativo';
  const state = await setupFacts(page, { facts: [parent, later] });
  await openNotice(page, later);
  const editor = factEditor(page, 'notification');
  await editor
    .getByRole('button', { name: 'Elegir revisi\u00f3n de la resoluci\u00f3n', exact: true })
    .click();
  const picker = editor.getByRole('region', {
    name: 'Elegir revisi\u00f3n de la resoluci\u00f3n',
    exact: true,
  });
  await picker
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n', exact: true })
    .click();
  await confirmFact(page, 'notification');
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].resolution_id).toBe(parent.id);
  expect(state.submissions[0].change.values.resolution).toEqual({ id: parent.id, revision: 1 });
});

test('two direct support versions and an archived exact fiche remain separate from declared representation', async ({
  page,
}) => {
  const parent = factRecord(),
    state = await setupFacts(page, { facts: [parent] });
  const archived = { ...structuredClone(participant), revision: 2, directory_status: 'archived' };
  state.results.directory.set(participant.id, [participant, archived]);
  await page.route('**/documents/*/versions**', (route) => {
    const path = new URL(route.request().url()).pathname;
    const old = { ...document, version: 1 },
      current = { ...document, version: 2, digest: 'b'.repeat(64) };
    return route.fulfill({
      json: path.endsWith('/versions')
        ? {
            versions: [current, old],
            has_more: false,
            next_before_version: null,
            first_available_version: 1,
          }
        : path.endsWith('/1')
          ? old
          : current,
    });
  });
  await openNotice(page, parent);
  const editor = factEditor(page, 'notification');
  await editor
    .getByRole('combobox', { name: 'Destinatario declarado', exact: true })
    .selectOption('participant');
  await editor
    .getByRole('button', { name: 'Elegir ficha: Destinatario declarado', exact: true })
    .click();
  const picker = editor.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${participant.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Consultar ficha revisi\u00f3n 2', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
  const label = 'Procedencia de la notificaci\u00f3n';
  await editor
    .getByRole('combobox', { name: label, exact: true })
    .selectOption('external_reference');
  await editor
    .getByLabel(`Referencia externa: ${label}`, { exact: true })
    .fill('Constancia informada');
  await support(editor, label, 1);
  await editor
    .getByRole('combobox', { name: 'Representaci\u00f3n declarada', exact: true })
    .selectOption('declared');
  for (const person of ['Persona representada', 'Persona representante']) {
    await editor.getByRole('combobox', { name: person, exact: true }).selectOption('unlinked');
    await editor.getByLabel(`Nombre declarado: ${person}`, { exact: true }).fill(person);
    await editor
      .getByLabel(`Descripci\u00f3n: ${person}`, { exact: true })
      .fill('Identidad declarada sin inferencia');
  }
  await editor
    .getByLabel('Alcance de la representaci\u00f3n', { exact: true })
    .fill('Alcance expresamente declarado');
  const representation = 'Procedencia de la representaci\u00f3n';
  await editor
    .getByRole('combobox', { name: representation, exact: true })
    .selectOption('external_reference');
  await editor
    .getByLabel(`Referencia externa: ${representation}`, { exact: true })
    .fill('Referencia de facultades declaradas');
  await support(editor, representation, 2);
  await confirmFact(page, 'notification');
  await expect(editor).toHaveCount(0);
  const command = state.submissions[0],
    saved = state.records.get(factKey(command))[0];
  expect(command.change.values.intended_recipient.value).toEqual({
    kind: 'participant',
    id: participant.id,
    revision: 2,
  });
  expect(saved.sources.participants[0].directory_status).toBe('archived');
  expect(saved.sources.direct_supports.map((row) => row.version)).toEqual([1, 2]);
  expect(command.change.values.representation.represented.kind).toBe('unlinked');
  await expect(factDetail(page, 'notification')).toContainText('Alcance expresamente declarado');
});
