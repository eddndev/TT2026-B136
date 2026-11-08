import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultEditor,
  resultDetail,
  fillResult,
  confirmResult,
  failResult,
} from './hearing-result-helpers.mjs';
import { resultRecord, resultCommand } from '../fixtures/hearing-results.mjs';

test('another exact receipt stays a conflict and the operator compares before adopting a new base', async ({
  page,
}) => {
  const first = resultRecord(),
    state = await setupResults(page, { results: [first] });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${first.id}`, exact: true })
    .click();
  await resultDetail(page)
    .getByRole('button', { name: 'Rectificar registro', exact: true })
    .click();
  const editor = resultEditor(page);
  await editor.getByLabel('Relato del operador', { exact: true }).fill('Mi borrador');
  await editor.getByLabel('Motivo', { exact: true }).fill('Mi precision');
  const other = resultCommand('correct', 1);
  other.change.values.summary = 'Otro cambio';
  state.commit(state.prepare(other));
  await editor.getByRole('button', { name: 'Revisar resultado', exact: true }).click();
  await expect(editor.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    'Mi borrador',
  );
  await expect(
    editor.getByRole('button', { name: 'Revisar resultado', exact: true }),
  ).toBeDisabled();
  await editor
    .getByRole('button', { name: 'Consultar base actual del resultado', exact: true })
    .click();
  await expect(editor).toContainText('Otro cambio');
  await editor
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await confirmResult(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.values.summary).toBe('Mi borrador');
});
