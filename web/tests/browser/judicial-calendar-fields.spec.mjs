import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarEditor,
  fillCalendar,
  confirmCalendar,
} from './judicial-calendar-helpers.mjs';
test('explicit sources and exceptions survive preparation with exact identifiers', async ({
  page,
}) => {
  const { state } = await setupCalendars(page, { records: [] });
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: 'Publicar calendario', exact: true })
    .click();
  await fillCalendar(page);
  const editor = calendarEditor(page);
  await editor.getByRole('button', { name: 'Agregar fuente', exact: true }).click();
  const source = editor.getByRole('group', { name: 'Fuente 1', exact: true });
  await source.getByLabel('T\u00edtulo de fuente', { exact: true }).fill('Disposicion declarada');
  await source.getByLabel('Emisor', { exact: true }).fill('Autoridad declarada');
  await source.getByLabel('Referencia HTTPS', { exact: true }).fill('https://example.org/calendar');
  await source.getByLabel('Localizador', { exact: true }).fill('Apartado 3');
  await source.getByLabel('Consulta declarada', { exact: true }).fill('2000-01-01');
  const monday = editor.getByRole('group', { name: 'Regla de Lunes', exact: true });
  await monday
    .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
    .selectOption('countable');
  await monday.getByRole('checkbox', { name: 'Disposicion declarada', exact: true }).check();
  await editor.getByRole('button', { name: 'Agregar excepci\u00f3n', exact: true }).click();
  const exception = editor.getByRole('group', { name: 'Excepci\u00f3n 1', exact: true });
  await exception.getByLabel('Excepci\u00f3n desde', { exact: true }).fill('2000-02-29');
  await exception.getByLabel('Excepci\u00f3n hasta', { exact: true }).fill('2000-02-29');
  await exception
    .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
    .selectOption('excluded');
  await exception
    .getByLabel('Explicaci\u00f3n', { exact: true })
    .fill('Excepcion declarada para este ambito');
  await exception.getByRole('checkbox', { name: 'Disposicion declarada', exact: true }).check();
  await expect(source.getByRole('button', { name: 'Quitar fuente 1', exact: true })).toBeDisabled();
  await confirmCalendar(page);
  const values = state.submissions[0].change.values;
  expect(values.sources[0].published_on).toBe(null);
  expect(values.weekly_pattern[0].source_ids).toEqual([values.sources[0].id]);
  expect(values.exceptions[0].source_ids).toEqual([values.sources[0].id]);
  expect(values.exceptions[0].from).toBe('2000-02-29');
});
test('invalid civil date coverage or unresolved empty explanation prevents network preparation', async ({
  page,
}) => {
  const { state } = await setupCalendars(page, { records: [] });
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: 'Publicar calendario', exact: true })
    .click();
  await fillCalendar(page);
  const editor = calendarEditor(page);
  await editor.getByLabel('Cobertura hasta', { exact: true }).fill('1999-12-31');
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await expect(editor.getByRole('alert')).toContainText('cobertura');
  expect(state.calls.filter((c) => c.path.endsWith('/prepare'))).toHaveLength(0);
  await editor.getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-31');
  await editor
    .getByRole('group', { name: 'Regla de Lunes', exact: true })
    .getByLabel('Explicaci\u00f3n', { exact: true })
    .fill('');
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await expect(editor.getByRole('alert')).toContainText('explicaci\u00f3n');
  expect(state.calls.filter((c) => c.path.endsWith('/prepare'))).toHaveLength(0);
});
