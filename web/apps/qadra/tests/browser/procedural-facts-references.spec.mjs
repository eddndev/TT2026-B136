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
