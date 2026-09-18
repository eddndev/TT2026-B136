import { test, expect } from '@playwright/test';
import {
  setupDeadlines,
  openDeadlines,
  editor,
  fillDeadline,
  confirmDeadline,
} from './deadline-editor-helpers.mjs';

test('deadline capture retains unknown source and absent quantity and requires explicit blocked confirmation', async ({
  page,
}) => {
  const state = await setupDeadlines(page);
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  const form = editor(page);
  await expect(
    form.getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true }),
  ).toHaveValue('');
  await expect(
    form.getByRole('combobox', { name: 'Existe una incidencia sin resolver', exact: true }),
  ).toHaveValue('');
  await fillDeadline(page);
  await confirmDeadline(page);
  await expect(form).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  const input = state.submissions[0].change.definition.input;
  expect(input.ordered_quantity).toBe(null);
  expect(input.selection.source).toEqual({ kind: 'unknown', reason: 'Falta la fuente de inicio' });
  expect(input.qualification.scope_applies).toEqual({ kind: 'known', value: true });
  expect(input.qualification.unresolved_incident).toEqual({ kind: 'known', value: false });
});

test('unknown applicability requires its reason and is never silently false', async ({ page }) => {
  const state = await setupDeadlines(page);
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fillDeadline(page);
  const form = editor(page);
  await form
    .getByRole('combobox', { name: 'El ambito del perfil aplica', exact: true })
    .selectOption('unknown');
  await form.getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(form.getByRole('alert')).toBeVisible();
  expect(state.calls.filter((row) => row.path.endsWith('/prepare'))).toHaveLength(0);
  await form
    .getByLabel('Motivo: El ambito del perfil aplica', { exact: true })
    .fill('Alcance pendiente de confirmar');
  await form.getByRole('button', { name: 'Preparar plazo', exact: true }).click();
  await expect(form.getByRole('button', { name: 'Confirmar plazo', exact: true })).toBeVisible();
  const command = state.calls.find((row) => row.path.endsWith('/prepare')).body;
  expect(command.change.definition.input.qualification.scope_applies).toEqual({
    kind: 'unknown',
    reason: 'Alcance pendiente de confirmar',
  });
});

test('uncertain commit checks its exact revision without automatically resubmitting', async ({
  page,
}) => {
  const state = await setupDeadlines(page);
  state.handle = async (route, call) => {
    if (call.method === 'POST' && !call.path.endsWith('/prepare')) {
      state.submissions.push(call.body.command);
      state.commit(state.prepare(call.body.command));
      await route.abort('failed');
      return true;
    }
  };
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fillDeadline(page);
  await confirmDeadline(page);
  await expect(
    editor(page).getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(1);
  await editor(page).getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.calls.some((row) => row.path.endsWith('/revisions/1'))).toBeTruthy();
});
