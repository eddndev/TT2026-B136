import { expect } from '@playwright/test';
import {
  calendarEditor,
  calendarPanel,
  calendarDetail,
  fillCalendar,
} from './judicial-calendar-helpers.mjs';
import { calendarId } from '../fixtures/judicial-calendars.mjs';
export { calendarEditor, calendarPanel, calendarDetail, fillCalendar };
export const reviewCalendar = (form) =>
  form.getByRole('button', { name: 'Revisar calendario', exact: true });
export const confirmCalendar = (form) =>
  form.getByRole('button', { name: 'Confirmar calendario', exact: true });
export const exactCalendar = (form) =>
  form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true });
export const rawCalendar = {
  title: '  Calendario sin concluir  ',
  reason: '  Motivo\npendiente de revisar  ',
  source: '  Fuente declarada  ',
  url: 'https://',
  explanation: '  Regla escrita\npendiente  ',
};
export async function enterCalendars(page) {
  await page.evaluate(() => {
    location.hash = 'judicial-calendars';
  });
  await expect(
    page.getByRole('heading', { name: 'Calendarios jurisdiccionales', exact: true }),
  ).toBeVisible();
  await expect(
    calendarPanel(page).getByRole('button', { name: 'Actualizar calendarios', exact: true }),
  ).toBeEnabled();
}
export async function beginCalendar(page, action = 'publish', id = calendarId) {
  if (action === 'publish') {
    await calendarPanel(page)
      .getByRole('button', { name: 'Publicar calendario', exact: true })
      .click();
  } else {
    await calendarPanel(page)
      .getByRole('button', { name: `Consultar calendario ${id}`, exact: true })
      .click();
    await calendarDetail(page)
      .getByRole('button', {
        name: action === 'replace' ? 'Reemplazar calendario' : 'Retirar calendario',
        exact: true,
      })
      .click();
  }
  const form = calendarEditor(page);
  await expect(form).toBeVisible();
  return form;
}
export async function prepareCalendar(state, form) {
  state.calendarBudget++;
  await reviewCalendar(form).click();
  await expect(confirmCalendar(form)).toBeEnabled();
}
export async function partialCalendar(page) {
  await fillCalendar(page);
  const form = calendarEditor(page);
  await form.getByLabel('T\u00edtulo del calendario', { exact: true }).fill(rawCalendar.title);
  await form.getByRole('button', { name: 'Agregar fuente', exact: true }).click();
  const source = form.getByRole('group', { name: 'Fuente 1', exact: true });
  await source.getByLabel('T\u00edtulo de fuente', { exact: true }).fill(rawCalendar.source);
  await source.getByLabel('Emisor', { exact: true }).fill('  Emisor declarado  ');
  await source.getByLabel('Referencia HTTPS', { exact: true }).fill(rawCalendar.url);
  await source.getByLabel('Localizador', { exact: true }).fill('  apartado parcial  ');
  await source.getByLabel('Consulta declarada', { exact: true }).fill('2000-01-01');
  const monday = form.getByRole('group', { name: 'Regla de Lunes', exact: true });
  await monday
    .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
    .selectOption('countable');
  await monday.getByRole('checkbox', { name: rawCalendar.source.trim(), exact: true }).check();
  await monday.getByLabel('Explicaci\u00f3n', { exact: true }).fill(rawCalendar.explanation);
  await form.getByRole('button', { name: 'Agregar excepci\u00f3n', exact: true }).click();
  const exception = form.getByRole('group', { name: 'Excepci\u00f3n 1', exact: true });
  await exception.getByLabel('Excepci\u00f3n desde', { exact: true }).fill('2000-02-29');
  await exception
    .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
    .selectOption('excluded');
  await exception.getByLabel('Explicaci\u00f3n', { exact: true }).fill(rawCalendar.explanation);
  await exception.getByRole('checkbox', { name: rawCalendar.source.trim(), exact: true }).check();
  await source.getByText('Identidad de esta referencia', { exact: true }).click();
  await exception.getByText('Identidad de la excepci\u00f3n', { exact: true }).click();
  return {
    source: await source.locator('details > code').textContent(),
    exception: await exception
      .locator('details')
      .filter({
        hasText: 'Identidad de la excepci\u00f3n',
      })
      .locator('code')
      .textContent(),
  };
}
