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
test('a retired concurrent base cannot become a new editable head', async ({ page }) => {
  const { state } = await setupCalendars(page),
    editor = await editing(page, state);
  state.commit(state.prepare(calendarFixtureCommand('retire', 1)));
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await editor
    .getByRole('button', { name: 'Consultar base actual del calendario', exact: true })
    .click();
  await expect(editor).toContainText('Revisi\u00f3n 2 / Retirado');
  await expect(
    editor.getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true }),
  ).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
});
test('lost response reconciles exact receipt after absent response without another mutation', async ({
  page,
}) => {
  const { state } = await setupCalendars(page),
    editor = await editing(page, state);
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  let writes = 0,
    exactReads = 0;
  await page.route(`**/api/v1/judicial-calendars/${calendarId}`, async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    writes++;
    state.commit(state.prepare(route.request().postDataJSON().command));
    return route.abort('failed');
  });
  await page.route(`**/api/v1/judicial-calendars/${calendarId}/revisions/2`, async (route) => {
    exactReads++;
    if (exactReads === 1)
      return route.fulfill({
        status: 404,
        json: { error: { code: 'judicial_calendar_not_found' } },
      });
    return route.fallback();
  });
  await editor.getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await expect(editor).toContainText('Resultado incierto');
  await expect(editor.getByLabel('Cobertura hasta', { exact: true })).toBeDisabled();
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(editor).toContainText('El resultado sigue incierto');
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(calendarDetail(page)).toContainText('Revisi\u00f3n hist\u00f3rica');
  expect(writes).toBe(1);
  expect(exactReads).toBe(2);
});
test('a different operation at the target revision never confirms an uncertain write', async ({
  page,
}) => {
  const { state } = await setupCalendars(page),
    editor = await editing(page, state);
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await page.route(`**/api/v1/judicial-calendars/${calendarId}`, async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    state.commit(state.prepare(calendarFixtureCommand('replace', 1)));
    return route.abort('failed');
  });
  await editor.getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(editor).toContainText('La revisi\u00f3n corresponde a otro env\u00edo');
  await expect(editor.getByLabel('Motivo', { exact: true })).toHaveValue('Borrador del operador');
  await expect(
    editor.getByRole('button', { name: 'Confirmar calendario', exact: true }),
  ).toHaveCount(0);
});
