import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
export { accountAction } from './hearing-result-helpers.mjs';
export const accounts = fixture.deadlines;
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de plazo', exact: true });
export const detail = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
export const list = (page) => page.getByRole('region', { name: 'Plazos registrados', exact: true });
export async function openDeadlines(page, record = accounts.case) {
  await openCase(page, record);
  await page.getByRole('link', { name: 'Plazos', exact: true }).click();
  await expect(list(page)).toHaveAttribute('aria-busy', 'false');
}
export async function openDeadline(page, row) {
  await list(page)
    .getByRole('button', { name: `Consultar plazo ${row.id}`, exact: true })
    .click();
  await expect(detail(page)).toBeVisible();
}
export async function fill(page, kind, title) {
  const form = editor(page),
    profile = accounts.profiles.case[kind],
    source = accounts.sources.case.initial;
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill(title);
  await form.getByRole('button', { name: 'Elegir perfil exacto', exact: true }).click();
  await form
    .getByRole('button', { name: `Revisiones de ${profile.definition.title}`, exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Consultar perfil revisi\u00f3n 1 (publicado)', exact: true })
    .click();
  await form.getByRole('button', { name: 'Usar este perfil exacto', exact: true }).click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('follow');
  await form.getByRole('button', { name: 'Elegir responsable', exact: true }).click();
  await form
    .getByRole('button', { name: `Elegir responsable ${accounts.litigator.email}`, exact: true })
    .click();
  await form
    .getByRole('combobox', { name: 'Tipo de fuente', exact: true })
    .selectOption('resolution');
  await form.getByRole('button', { name: 'Elegir fuente exacta', exact: true }).click();
  await form
    .getByRole('button', { name: `Consultar revisiones de resolucion ${source.id}`, exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await form
    .getByRole('button', { name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n', exact: true })
    .click();
  await form
    .getByRole('combobox', { name: 'Cuando cambie la fuente', exact: true })
    .selectOption('fixed');
  if (kind === 'daily') {
    await form.getByRole('button', { name: 'Elegir calendario exacto', exact: true }).click();
    await form
      .getByRole('button', {
        name: `Revisiones de ${accounts.calendar.values.scope.title}`,
        exact: true,
      })
      .click();
    await form
      .getByRole('button', {
        name: 'Consultar calendario revisi\u00f3n 1 (publicado)',
        exact: true,
      })
      .click();
    await form.getByRole('button', { name: 'Usar este calendario exacto', exact: true }).click();
    await form
      .getByRole('combobox', { name: 'Cuando cambie el calendario', exact: true })
      .selectOption('fixed');
  }
  if (kind === 'hourly') {
    await form
      .getByRole('checkbox', { name: 'Declarar un inicio calificado', exact: true })
      .check();
    await form
      .getByRole('combobox', { name: 'Finalidad del inicio', exact: true })
      .selectOption('ordered_period_start');
    await form
      .getByRole('combobox', { name: 'Precisi\u00f3n de inicio calificado', exact: true })
      .selectOption('second');
    await form.getByLabel('Fecha de inicio calificado', { exact: true }).fill('2026-01-06');
    await form.getByLabel('Hora de inicio calificado', { exact: true }).fill('14:30:07');
    await form
      .getByRole('combobox', { name: 'Desfase de inicio calificado', exact: true })
      .selectOption('declared');
    await form.getByLabel('Desfase UTC de inicio calificado', { exact: true }).fill('-06:00');
    await form
      .getByLabel('Declaraci\u00f3n del inicio', { exact: true })
      .fill('Inicio expresamente declarado en la fuente sintetica');
    await form
      .getByLabel('Localizador del inicio', { exact: true })
      .fill('Fuente sintetica, pagina 1');
  }
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill('Supuesto declarado para prueba sin afirmar validez juridica');
  await form
    .getByLabel('Localizador de aplicabilidad', { exact: true })
    .fill('Fuente sintetica, pagina 2');
  await form
    .getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true })
    .selectOption('yes');
  await form
    .getByRole('combobox', { name: 'Existe una incidencia sin resolver', exact: true })
    .selectOption('no');
  await form
    .getByRole('combobox', { name: 'Se cumple la condicion 1', exact: true })
    .selectOption('yes');
  await form
    .getByLabel('Localizador de condici\u00f3n 1', { exact: true })
    .fill('Declaracion sintetica');
}
export async function prepare(page, expected = 200) {
  const pending = page.waitForResponse(
    (response) =>
      response.url().endsWith('/deadlines/prepare') && response.request().method() === 'POST',
  );
  await editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(expected);
  return response.json();
}
export async function acknowledge(page) {
  await expect(
    editor(page).getByRole('button', { name: 'Confirmar plazo', exact: true }),
  ).toBeDisabled();
  await editor(page)
    .getByRole('checkbox', { name: /Revise las declaraciones/ })
    .check();
}
export async function submit(page, draft, expected = 201) {
  const { command } = draft,
    action = command.change.action;
  const suffix =
    action === 'set_attention' ? '/attention' : action === 'retire' ? '/retirement' : '';
  const path = `/deadlines${action === 'register' ? '' : `/${command.deadline_id}${suffix}`}`;
  const pending = page.waitForResponse(
    (response) =>
      response.url().endsWith(path) &&
      response.request().method() === (action === 'correct' ? 'PUT' : 'POST'),
  );
  await acknowledge(page);
  await editor(page).getByRole('button', { name: 'Confirmar plazo', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(expected);
  const value = await response.json();
  if (expected === 201) await expect(editor(page)).toHaveCount(0);
  return value;
}
