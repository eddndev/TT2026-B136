import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  editor,
  confirmDeadline,
  deadlineFixture,
  deadlineError,
} from './deadline-editor-helpers.mjs';
import { id } from '../fixtures/deadline-unit.mjs';
import { v2Record as detail } from '../fixtures/deadline-v2-unit.mjs';
const current = () => detail(deadlineFixture());
const detailPanel = (page) => page.getByRole('region', { name: 'Detalle de plazo', exact: true });
async function openCurrent(page, state) {
  await openDeadlines(page);
  await page.getByRole('button', { name: `Consultar plazo ${id(6)}`, exact: true }).click();
  await expect(detailPanel(page)).toBeVisible();
}

test('attention records explicit precision and retirement preserves the historical calculation', async ({
  page,
}) => {
  const initial = current(),
    state = await setupDeadlines(page, { deadlines: [initial] });
  await openCurrent(page, state);
  await detailPanel(page)
    .getByRole('button', { name: 'Declarar atenci\u00f3n', exact: true })
    .click();
  const form = editor(page);
  await expect(
    form.getByRole('combobox', { name: 'Estado de atenci\u00f3n', exact: true }),
  ).toHaveValue('');
  await form
    .getByRole('combobox', { name: 'Estado de atenci\u00f3n', exact: true })
    .selectOption('recorded');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de atenci\u00f3n', exact: true })
    .selectOption('date');
  await form.getByLabel('Fecha de atenci\u00f3n', { exact: true }).fill('2026-02-05');
  await form
    .getByLabel('Declaraci\u00f3n de atenci\u00f3n', { exact: true })
    .fill('Se declaro la presentacion');
  await form.getByLabel('Localizador de atenci\u00f3n', { exact: true }).fill('Acuse, pagina 1');
  await form.getByLabel('Motivo', { exact: true }).fill('Incorporar el acuse declarado');
  await confirmDeadline(page);
  await expect(form).toHaveCount(0);
  const attended = state.records.get(initial.id).at(-1);
  expect(attended.attention.occurred_at).toEqual({
    precision: 'date',
    year: 2026,
    month: 2,
    day: 5,
    offset_seconds: null,
  });
  expect(attended.calculation).toEqual(initial.calculation);
  await detailPanel(page).getByRole('button', { name: 'Retirar plazo', exact: true }).click();
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Retiro administrativo declarado');
  await confirmDeadline(page);
  await expect(editor(page)).toHaveCount(0);
  const retired = state.records.get(initial.id).at(-1);
  expect(retired.status).toBe('retired');
  expect(retired.calculation).toEqual(initial.calculation);
  expect(retired.attention).toEqual(attended.attention);
  expect(state.submissions.map((row) => row.change.action)).toEqual(['set_attention', 'retire']);
});

test('conflicting correction preserves its draft and requires an explicit current-base comparison', async ({
  page,
}) => {
  const initial = current(),
    state = await setupDeadlines(page, { deadlines: [initial] });
  await openCurrent(page, state);
  await detailPanel(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  await editor(page)
    .getByLabel('T\u00edtulo del plazo', { exact: true })
    .fill('Borrador que debe conservarse');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Aclarar el titulo');
  let rejected = false;
  state.handle = async (route, call) => {
    if (!rejected && call.path.endsWith('/prepare')) {
      rejected = true;
      const command = structuredClone(call.body);
      command.operation_id = id(71);
      const latest = state.prepare(command);
      latest.definition.title = 'Cambio concurrente';
      state.commit(latest);
      await deadlineError(route, 'deadline_revision_conflict');
      return true;
    }
  };
  await editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(
    editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }),
  ).toBeDisabled();
  await expect(editor(page).getByLabel('T\u00edtulo del plazo', { exact: true })).toHaveValue(
    'Borrador que debe conservarse',
  );
  await editor(page).getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await confirmDeadline(page);
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.definition.title).toBe('Borrador que debe conservarse');
});
