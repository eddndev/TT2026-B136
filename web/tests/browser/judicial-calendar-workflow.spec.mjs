import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarEditor,
  calendarDetail,
  fillCalendar,
  confirmCalendar,
} from './judicial-calendar-helpers.mjs';
import { calendarId } from '../fixtures/judicial-calendars.mjs';
test('publishes explicit rules then replaces retires and reads exact history', async ({ page }) => {
  const { state } = await setupCalendars(page, { records: [] });
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: 'Publicar calendario', exact: true })
    .click();
  const editor = calendarEditor(page);
  await expect(calendarPanel(page).getByRole('status')).toHaveCount(0);
  await expect(editor.getByRole('combobox', { name: 'Fuero', exact: true })).toHaveValue('');
  await expect(editor.getByRole('checkbox', { checked: true })).toHaveCount(0);
  await expect(editor.getByLabel('Cobertura desde', { exact: true })).toHaveValue('');
  await fillCalendar(page);
  await confirmCalendar(page);
  expect(
    state.submissions[0].change.values.weekly_pattern.every(
      (r) => r.classification === 'unresolved',
    ),
  ).toBe(true);
  await calendarDetail(page)
    .getByRole('button', { name: 'Reemplazar calendario', exact: true })
    .click();
  await expect(editor.getByLabel('T\u00edtulo del calendario', { exact: true })).toBeDisabled();
  await editor.getByLabel('Cobertura hasta', { exact: true }).fill('2000-04-30');
  await editor.getByLabel('Motivo', { exact: true }).fill('Ampliacion de cobertura declarada');
  await confirmCalendar(page);
  await calendarDetail(page)
    .getByRole('button', { name: 'Retirar calendario', exact: true })
    .click();
  await editor.getByLabel('Motivo', { exact: true }).fill('Configuracion sustituida');
  await confirmCalendar(page);
  await expect(calendarDetail(page)).toContainText('Retirado');
  await calendarDetail(page)
    .getByRole('button', { name: 'Ver historial del calendario', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar calendario revisi\u00f3n 1', exact: true })
    .click();
  await expect(calendarDetail(page)).toContainText('Revisi\u00f3n hist\u00f3rica');
  await expect(
    calendarDetail(page).getByRole('button', { name: 'Reemplazar calendario', exact: true }),
  ).toHaveCount(0);
  expect([...state.records.values()][0]).toHaveLength(3);
});
test('shows declared day explanations and sources from one exact range query', async ({ page }) => {
  const { state } = await setupCalendars(page);
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  const detail = calendarDetail(page);
  await expect(detail).toContainText('Sin resolver');
  await detail
    .getByRole('button', { name: '29 de febrero de 2000: Excluido', exact: true })
    .click();
  await expect(detail.getByRole('region', { name: 'Detalle del d\u00eda' })).toContainText(
    'Excepcion declarada',
  );
  await expect(detail).toContainText('La copia de estas fuentes no est\u00e1 archivada');
  const links = detail.getByRole('link', { name: /Abrir referencia/ });
  await expect(links.first()).toHaveAttribute('target', '_blank');
  await expect(links.first()).toHaveAttribute('rel', /noopener/);
  expect(state.calls.filter((c) => c.path.endsWith('/days'))).toHaveLength(1);
});
test('paralegal reads global catalog without a selected case and Client cannot open it', async ({
  page,
}) => {
  const { state } = await setupCalendars(page, { role: 'paralegal' });
  await openCalendars(page);
  await expect(
    calendarPanel(page).getByRole('button', { name: 'Publicar calendario', exact: true }),
  ).toHaveCount(0);
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await expect(calendarDetail(page)).toBeVisible();
  expect(state.calls.every((c) => !c.path.includes('/cases/'))).toBe(true);
});

test('reviews the immutable title before publishing or retiring', async ({ page }) => {
  await setupCalendars(page, { records: [] });
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: 'Publicar calendario', exact: true })
    .click();
  await fillCalendar(page);
  const editor = calendarEditor(page);
  await editor
    .getByLabel('T\u00edtulo del calendario', { exact: true })
    .fill('Titulo que debe revisarse');
  await editor.getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  await expect(editor.locator('dd').filter({ hasText: 'Titulo que debe revisarse' })).toBeVisible();
  await editor.getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await calendarDetail(page)
    .getByRole('button', { name: 'Retirar calendario', exact: true })
    .click();
  await expect(editor.locator('dd').filter({ hasText: 'Titulo que debe revisarse' })).toBeVisible();
});
