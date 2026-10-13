import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultEditor,
  resultDetail,
  fillResult,
  confirmResult,
} from './hearing-result-helpers.mjs';

test('an exact revision from a different operation does not confirm the lost submission', async ({
  page,
}) => {
  const state = await setupResults(page);
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || call.path.endsWith('/prepare')) return;
    const prepared = state.prepare(call.body.command);
    prepared.command.operation_id = '90000000-0000-4000-8000-000000000009';
    state.submissions.push(call.body.command);
    state.commit(prepared);
    await route.abort('failed');
    return true;
  };
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  await confirmResult(page);
  await resultEditor(page)
    .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
    .click();
  await expect(resultEditor(page).getByRole('alert')).toContainText('otro env\u00edo');
  await expect(resultEditor(page).getByLabel('Relato del operador', { exact: true })).toHaveValue(
    'Sesion parcial comunicada',
  );
  await expect(resultDetail(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
});

test('receipt matching an older exact revision never labels it as the current head', async ({
  page,
}) => {
  const state = await setupResults(page);
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || call.path.endsWith('/prepare')) return;
    const prepared = state.prepare(call.body.command);
    state.submissions.push(call.body.command);
    const first = state.commit(prepared);
    const second = {
      ...structuredClone(first),
      revision: 2,
      reason: 'Cambio posterior',
      values: { ...first.values, summary: 'Registro actual posterior' },
    };
    state.records.get(first.id).push(second);
    await route.abort('failed');
    return true;
  };
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  await confirmResult(page);
  await resultEditor(page)
    .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
    .click();
  await expect(resultEditor(page)).toHaveCount(0);
  await expect(resultDetail(page)).toContainText('Consultada exactamente');
  await expect(resultDetail(page)).toContainText('Sesion parcial comunicada');
  await expect(
    resultDetail(page).getByRole('button', { name: 'Rectificar registro', exact: true }),
  ).toHaveCount(0);
  await resultDetail(page)
    .getByRole('button', { name: 'Consultar resultado actual', exact: true })
    .click();
  await expect(resultDetail(page)).toContainText('Registro actual posterior');
  expect(state.submissions).toHaveLength(1);
});
