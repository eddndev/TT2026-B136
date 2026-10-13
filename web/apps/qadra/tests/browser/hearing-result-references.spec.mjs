import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultEditor,
  resultDetail,
  fillResult,
  confirmResult,
} from './hearing-result-helpers.mjs';
import { participant } from './participant-helpers.mjs';
import { document } from './helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';

test('no start preserves an explicitly selected archived historical attendee and declared capacity', async ({
  page,
}) => {
  const state = await setupResults(page);
  state.directory.set(participant.id, [
    structuredClone(participant),
    {
      ...participant,
      revision: 2,
      display_name: 'Nombre actual diferente',
      directory_status: 'archived',
    },
  ]);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  const editor = resultEditor(page);
  await editor
    .getByRole('combobox', { name: 'Ocurrencia', exact: true })
    .selectOption('not_started');
  await editor
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('unspecified');
  await editor.getByText('Comparecencias informadas (0/32)', { exact: true }).click();
  await editor.getByRole('button', { name: 'Agregar comparecencia', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await expect(picker).toContainText('Nombre actual diferente');
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${participant.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Consultar ficha revisi\u00f3n 1', exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Informar comparecencia de esta revisi\u00f3n', exact: true })
    .click();
  await editor
    .getByLabel('Calidad en esta sesi\u00f3n', { exact: true })
    .fill('Representante presente');
  await editor
    .getByLabel('Observaci\u00f3n de comparecencia (opcional)', { exact: true })
    .fill('Comparecio sin inicio del acto');
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  await expect(resultDetail(page)).toContainText(participant.display_name);
  await expect(resultDetail(page)).not.toContainText('Nombre actual diferente');
  expect(state.submissions[0].change.values.attendees).toEqual([
    {
      participant_id: participant.id,
      revision: 1,
      capacity: 'Representante presente',
      observation: 'Comparecio sin inicio del acto',
    },
  ]);
  expect(state.submissions[0].change.values.occurrence).toBe('not_started');
});

test('declared agreement identifiers survive reordering and correction under shared source', async ({
  page,
}) => {
  const state = await setupResults(page);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  const editor = resultEditor(page);
  await editor.getByText('Acuerdos declarados (0/16)', { exact: true }).click();
  await editor.getByRole('button', { name: 'Agregar acuerdo declarado', exact: true }).click();
  await editor.getByLabel('Texto del acuerdo 1', { exact: true }).fill('Primero');
  await editor.getByRole('button', { name: 'Agregar acuerdo declarado', exact: true }).click();
  await editor.getByLabel('Texto del acuerdo 2', { exact: true }).fill('Segundo');
  await editor.getByRole('button', { name: 'Subir acuerdo 2', exact: true }).click();
  await editor
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('oral_reference');
  await editor
    .getByLabel('Localizador de la fuente', { exact: true })
    .fill('https://archivo.example/registro, minuto 10');
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  const before = state.submissions[0].change.values.agreements;
  expect(before.map((row) => row.text)).toEqual(['Segundo', 'Primero']);
  await expect(resultDetail(page).getByRole('link')).toHaveCount(0);
  await resultDetail(page)
    .getByRole('button', { name: 'Rectificar registro', exact: true })
    .click();
  await editor.getByText('Acuerdos declarados (2/16)', { exact: true }).click();
  await editor.getByLabel('Texto del acuerdo 1', { exact: true }).fill('Segundo precisado');
  await editor.getByLabel('Motivo', { exact: true }).fill('Precision de relato');
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[1].change.values.agreements[0]).toEqual({
    ...before[0],
    text: 'Segundo precisado',
  });
});

test('selects a historical scheduling revision while a later head stays unchanged', async ({
  page,
}) => {
  const hearing = hearingRecord(),
    state = await setupResults(page);
  state.scheduling.records
    .get(hearing.id)
    .push({ ...structuredClone(hearing), revision: 2, status: 'cancelled' });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  const editor = resultEditor(page);
  await editor
    .getByRole('button', { name: 'Elegir programaci\u00f3n de origen', exact: true })
    .click();
  await editor
    .getByRole('button', { name: `Consultar programaci\u00f3n ${hearing.id}`, exact: true })
    .click();
  await editor
    .getByRole('button', { name: 'Usar programaci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await fillResult(page);
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.anchor_revision).toBe(1);
  expect(state.scheduling.records.get(hearing.id).at(-1).status).toBe('cancelled');
});

test('written source links an explicitly consulted exact documentary version', async ({ page }) => {
  const state = await setupResults(page);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  const editor = resultEditor(page);
  await editor
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('written_record');
  await editor.getByLabel('Localizador de la fuente', { exact: true }).fill('Pagina 2');
  await editor.getByText('Soporte documental (opcional)', { exact: true }).click();
  await editor.getByRole('button', { name: 'Elegir soporte del resultado', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /contrato.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ contrato.pdf/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.values.provenance.support).toEqual({
    document_id: document.id,
    version: 1,
    digest: document.digest,
  });
});
