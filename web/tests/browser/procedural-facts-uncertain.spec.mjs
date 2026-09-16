import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  fillResolution,
  confirmFact,
  failFact,
} from './procedural-facts-helpers.mjs';

async function submitLost(page, state, mutate) {
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || call.path.endsWith('/prepare')) return;
    const prepared = state.prepare(call.body.command);
    state.submissions.push(call.body.command);
    const row = state.commit(prepared);
    if (mutate) mutate(row);
    await route.abort('failed');
    return true;
  };
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  await confirmFact(page);
  await expect(factEditor(page)).toContainText('Resultado incierto');
}

test('temporary target absence remains uncertain and never re-sends the recorded command', async ({
  page,
}) => {
  const state = await setupFacts(page);
  await submitLost(page, state);
  const submitHandler = state.handle;
  let missing = true;
  state.handle = async (route, call) => {
    if (call.path.includes('/revisions/') && missing) {
      await failFact(route, 'procedural_fact_not_found', 404);
      return true;
    }
    return submitHandler(route, call);
  };
  const read = () =>
    factEditor(page)
      .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
      .click();
  await read();
  await expect(factEditor(page).getByRole('alert')).toContainText('incierto');
  await expect(
    factEditor(page).getByRole('button', { name: 'Confirmar registro', exact: true }),
  ).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  missing = false;
  await read();
  await expect(factEditor(page)).toHaveCount(0);
  await expect(factDetail(page)).toContainText('Declaracion de resolucion capturada');
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  const command = state.submissions[0];
  expect(
    state.calls.some(
      (call) =>
        call.method === 'GET' && call.path.endsWith(`/resolutions/${command.id}/revisions/1`),
    ),
  ).toBe(true);
  expect(state.submissions).toHaveLength(1);
});

for (const mismatch of ['operation', 'actor', 'sources', 'family'])
  test(`an exact revision with a different ${mismatch} cannot confirm a lost submission`, async ({
    page,
  }) => {
    const state = await setupFacts(page);
    await submitLost(page, state, (row) => {
      if (mismatch === 'operation')
        row.receipt.operation_id = '90000000-0000-4000-8000-000000000099';
      if (mismatch === 'actor') row.recorded_by.id = 'another-user';
      if (mismatch === 'sources') row.receipt.sources_digest = 'a'.repeat(64);
      if (mismatch === 'family') row.family = 'notification';
    });
    await factEditor(page)
      .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
      .click();
    await expect(factEditor(page).getByRole('alert')).toBeVisible();
    await expect(factDetail(page)).toHaveCount(0);
    await expect(
      factEditor(page).getByLabel('Resumen de la resoluci\u00f3n', { exact: true }),
    ).toHaveValue('Declaracion de resolucion capturada');
    expect(state.submissions).toHaveLength(1);
  });
