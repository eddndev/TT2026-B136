import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarEditor,
  calendarDetail,
} from './judicial-calendar-helpers.mjs';
import { calendarId, calendarFixtureCommand } from '../fixtures/judicial-calendars.mjs';
async function editing(page, state) {
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await calendarDetail(page)
    .getByRole('button', { name: 'Reemplazar calendario', exact: true })
    .click();
  const editor = calendarEditor(page);
  await editor.getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-10');
  await editor.getByLabel('Motivo', { exact: true }).fill('Borrador del operador');
  return editor;
}
test('revision conflict preserves draft until explicit comparison and accepted current base', async ({
  page,
}) => {
  const { state } = await setupCalendars(page),
    editor = await editing(page, state);
  const concurrent = calendarFixtureCommand('replace', 1);
  concurrent.change.reason = 'Cambio de otro Owner';
  state.commit(state.prepare(concurrent));
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await expect(editor).toContainText('La cabeza del calendario cambi\u00f3');
  await expect(editor.getByLabel('Cobertura hasta', { exact: true })).toHaveValue('2000-03-10');
  await expect(
    editor.getByRole('button', { name: 'Revisar calendario', exact: true }),
  ).toBeDisabled();
  await editor
    .getByRole('button', { name: 'Consultar base actual del calendario', exact: true })
    .click();
  await expect(editor).toContainText('Revisi\u00f3n 2 / Publicado');
  await editor
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await expect(editor).toHaveCount(0);
  expect(state.submissions.at(-1).change.expected_revision).toBe(2);
  expect(state.submissions.at(-1).change.values.coverage.through).toBe('2000-03-10');
});
