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

test('lost response consults an exact receipt and keeps temporary absence uncertain without re-sending', async ({
  page,
}) => {
  const state = await setupResults(page);
  let missing = true;
  state.handle = async (route, call) => {
    if (call.method === 'POST' && !call.path.endsWith('/prepare')) {
      state.submissions.push(call.body.command);
      state.commit(state.prepare(call.body.command));
      await route.abort('failed');
      return true;
    }
    if (call.path.includes('/revisions/') && missing) {
      await failResult(route, 'hearing_result_not_found', 404);
      return true;
    }
  };
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  await confirmResult(page);
  const editor = resultEditor(page);
  await expect(editor).toContainText('Resultado incierto');
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(editor.getByRole('alert')).toContainText('sigue incierto');
  expect(state.submissions).toHaveLength(1);
  missing = false;
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(resultDetail(page)).toContainText('Consultada exactamente');
  expect(state.submissions).toHaveLength(1);
});

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

test('a case closed before prepare keeps the draft and disables writes', async ({ page }) => {
  const state = await setupResults(page);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  state.scheduling.context.administrative_status = 'closed';
  state.scheduling.admin.administration.administrative_status = 'closed';
  await resultEditor(page).getByRole('button', { name: 'Revisar resultado', exact: true }).click();
  await expect(resultEditor(page).getByLabel('Relato del operador', { exact: true })).toHaveValue(
    'Sesion parcial comunicada',
  );
  await expect(
    resultEditor(page).getByRole('button', { name: 'Revisar resultado', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(0);
});
