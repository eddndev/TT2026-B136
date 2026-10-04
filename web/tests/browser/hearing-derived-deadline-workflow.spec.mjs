import { test, expect } from '@playwright/test';
import { setupResults, openResults, resultPanel } from './hearing-result-helpers.mjs';

const editor = (page) =>
  page.getByRole('region', { name: 'Resultado y plazo configurado', exact: true });

test('starts a combined result and deadline with its source fixed to the prospective result', async ({
  page,
}) => {
  const state = await setupResults(page);
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar resultado y plazo', exact: true })
    .click();
  await expect(editor(page)).toBeVisible();
  await expect(editor(page)).toContainText('Resultado prospectivo / Revision 1');
  await expect(
    editor(page).getByRole('combobox', { name: 'Tipo de fuente', exact: true }),
  ).toHaveCount(0);
  await expect(
    editor(page).getByRole('button', { name: 'Preparar resultado y plazo', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(0);
  expect(state.calls.filter((row) => row.method === 'POST')).toHaveLength(0);
});
