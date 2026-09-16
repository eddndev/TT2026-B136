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
import { hearingRecord, hearingId } from '../fixtures/hearings.mjs';
import { resultRecord, resultPrepared, resultCommand } from '../fixtures/hearing-results.mjs';

test('records multiple sessions separately from revisions without default event or attendance', async ({
  page,
}) => {
  const state = await setupResults(page);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  const editor = resultEditor(page);
  await expect(editor.getByRole('combobox', { name: 'Ocurrencia', exact: true })).toHaveValue('');
  await expect(
    editor.getByRole('combobox', { name: 'Alcance declarado', exact: true }),
  ).toHaveValue('');
  await expect(editor.getByLabel('Fecha', { exact: true })).toHaveValue('');
  await fillResult(page);
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  await expect(resultDetail(page)).toContainText('2026-09-01 / sin hora / UTC-06:00');
  expect(state.submissions[0].change.values.attendees).toEqual([]);
  expect(state.submissions[0].change.anchor_revision).toBe(1);
  await resultDetail(page)
    .getByRole('button', { name: 'Rectificar registro', exact: true })
    .click();
  await editor.getByLabel('Relato del operador', { exact: true }).fill('Relato rectificado');
  await editor.getByLabel('Motivo', { exact: true }).fill('Precision comunicada');
  await confirmResult(page);
  await expect(resultDetail(page)).toContainText('Relato rectificado');
  await resultDetail(page).getByRole('button', { name: 'Retirar registro', exact: true }).click();
  await editor.getByLabel('Motivo', { exact: true }).fill('Captura duplicada');
  await confirmResult(page);
  await expect(resultDetail(page)).toContainText('Registro retirado');
  await resultDetail(page)
    .getByRole('button', { name: 'Ver historial del registro', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar resultado revisi\u00f3n 1', exact: true })
    .click();
  await expect(resultDetail(page)).toContainText('Sesion parcial comunicada');
  await expect(
    resultDetail(page).getByRole('button', { name: 'Rectificar registro', exact: true }),
  ).toHaveCount(0);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page, 'Otro acto comunicado');
  await confirmResult(page);
  expect(state.records.size).toBe(2);
  expect([...state.records.values()].map((rows) => rows.length).sort()).toEqual([1, 3]);
});

test('continues a withdrawn exact result under another cancelled scheduling reference', async ({
  page,
}) => {
  const first = resultRecord(resultPrepared(resultCommand('withdraw', 1)));
  const other = {
    ...hearingRecord(),
    id: '70000000-0000-4000-8000-000000000007',
    status: 'cancelled',
  };
  const state = await setupResults(page, { hearings: [hearingRecord(), other], results: [first] });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${first.id}`, exact: true })
    .click();
  await resultDetail(page)
    .getByRole('button', { name: 'Registrar continuaci\u00f3n', exact: true })
    .click();
  const editor = resultEditor(page);
  await editor
    .getByRole('button', { name: 'Elegir programaci\u00f3n de origen', exact: true })
    .click();
  await editor
    .getByRole('button', { name: `Consultar programaci\u00f3n ${other.id}`, exact: true })
    .click();
  await editor
    .getByRole('button', { name: 'Usar programaci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await fillResult(page, 'Continuacion informada');
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].hearing_id).toBe(other.id);
  expect(state.submissions[0].change.continuation).toEqual({ result_id: first.id, revision: 2 });
  await expect(resultDetail(page)).toContainText('Continuacion informada');
  await resultDetail(page)
    .getByRole('button', { name: 'Consultar antecedente exacto', exact: true })
    .click();
  await expect(resultDetail(page)).toContainText('Registro retirado');
  expect(state.calls.at(-1).path).toContain(
    `/hearings/${hearingId}/results/${first.id}/revisions/2`,
  );
});
