import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factList,
  factEditor,
  failFact,
} from './procedural-facts-helpers.mjs';
import { factRecord, factPrepared, factCommand } from '../fixtures/procedural-facts.mjs';

test('failed parent revision lookup removes the previous selectable reference until a new exact read succeeds', async ({
  page,
}) => {
  const first = factRecord();
  const command = factCommand('resolution', 'correct', 1);
  command.change.values.summary = 'Otra revision del mismo padre';
  const second = factRecord(factPrepared(command));
  const state = await setupFacts(page, { facts: [first, second] });
  state.handle = async (route, call) => {
    if (call.method !== 'GET' || !call.path.endsWith(`/resolutions/${first.id}/revisions/2`))
      return;
    await failFact(route, 'internal_error', 500);
    return true;
  };
  await openFacts(page);
  await factList(page)
    .getByRole('button', {
      name: `Consultar resoluci\u00f3n ${first.id}`,
      exact: true,
    })
    .click();
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }).click();
  const editor = factEditor(page, 'notification');
  await editor
    .getByRole('button', {
      name: 'Elegir revisi\u00f3n de la resoluci\u00f3n',
      exact: true,
    })
    .click();
  const picker = editor.getByRole('region', {
    name: 'Elegir revisi\u00f3n de la resoluci\u00f3n',
    exact: true,
  });
  const read = (revision) =>
    picker
      .getByRole('button', {
        name: `Consultar resoluci\u00f3n revisi\u00f3n ${revision}`,
        exact: true,
      })
      .click();
  const link = picker.getByRole('button', {
    name: 'Vincular esta revisi\u00f3n de resoluci\u00f3n',
    exact: true,
  });
  await read(1);
  await expect(link).toBeVisible();
  await read(2);
  await expect(picker.getByRole('alert')).toBeVisible();
  await expect(link).toHaveCount(0);
  await expect(picker).not.toContainText(first.values.summary);
  await read(1);
  await expect(link).toBeVisible();
  await expect(picker).toContainText('Revisi\u00f3n exacta 1 / Registrada');
  await expect(picker).toContainText(first.values.summary);
  expect(state.submissions).toHaveLength(0);
});
