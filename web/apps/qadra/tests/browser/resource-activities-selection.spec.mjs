import { test, expect } from '@playwright/test';
import {
  setupResourceActivities,
  openResourceActivities,
  selectTarget,
  activityPanel,
  activityEditor,
} from './resource-activities-helpers.mjs';

test('changing target family clears exact target and an optional act can be removed explicitly', async ({
  page,
}) => {
  const state = await setupResourceActivities(page);
  await openResourceActivities(page, state);
  await activityPanel(page)
    .getByRole('button', { name: 'Vincular actividad', exact: true })
    .click();
  const editor = activityEditor(page);
  await selectTarget(page, state, 'hearing');
  await editor
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('deadline');
  await expect(
    editor.getByRole('combobox', { name: 'Actividad existente', exact: true }),
  ).toHaveValue('');
  await expect(editor).not.toContainText('Sala historica uno');
  await selectTarget(page, state, 'deadline');
  await editor.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await editor.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await editor.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  await editor.getByRole('button', { name: 'Quitar acto', exact: true }).click();
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  const call = state.calls.findLast(
    (row) => row.method === 'POST' && row.path.endsWith('/prepare'),
  );
  expect(call.body.change.act).toBeNull();
  expect(call.body.change.target).toEqual({
    kind: 'deadline',
    id: state.deadline[0].id,
    revision: 1,
    capture_digest: state.deadline[0].receipt.capture_digest,
  });
  expect(state.submissions).toHaveLength(0);
});
