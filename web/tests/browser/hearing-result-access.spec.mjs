import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultDetail,
  resultEditor,
  fillResult,
  confirmResult,
} from './hearing-result-helpers.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';
import { login, navigate } from './helpers.mjs';

for (const role of ['owner', 'litigator'])
  test(`${role} registers declared results without requiring the current scheduling stage`, async ({
    page,
  }) => {
    const state = await setupResults(page, { role });
    state.scheduling.context.profile_complete = false;
    state.scheduling.context.stage_revision = null;
    await openResults(page);
    await resultPanel(page)
      .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
      .click();
    await fillResult(page);
    await confirmResult(page);
    await expect(resultEditor(page)).toHaveCount(0);
    expect(state.submissions[0].change).not.toHaveProperty('expected_case_revision');
    expect(state.submissions[0].change).not.toHaveProperty('expected_stage_revision');
  });

test('assigned paralegal reads exact results and history without management actions', async ({
  page,
}) => {
  const record = resultRecord();
  await setupResults(page, { role: 'paralegal', results: [record] });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${record.id}`, exact: true })
    .click();
  await expect(resultDetail(page)).toContainText(record.values.summary);
  for (const name of [
    'Registrar sesi\u00f3n o acto',
    'Rectificar registro',
    'Retirar registro',
    'Registrar continuaci\u00f3n',
  ])
    await expect(resultPanel(page).getByRole('button', { name, exact: true })).toHaveCount(0);
  await resultDetail(page)
    .getByRole('button', { name: 'Ver historial del registro', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Consultar resultado revisi\u00f3n 1', exact: true }),
  ).toBeVisible();
});

test('closed case retains reads and history with management disabled', async ({ page }) => {
  const record = resultRecord();
  await setupResults(page, { closed: true, results: [record] });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${record.id}`, exact: true })
    .click();
  await expect(
    resultPanel(page).getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true }),
  ).toBeDisabled();
  await expect(
    resultDetail(page).getByRole('button', { name: 'Rectificar registro', exact: true }),
  ).toHaveCount(0);
  await expect(resultDetail(page)).toContainText(record.values.summary);
});

test('revocation on refresh removes protected results and the selected case context', async ({
  page,
}) => {
  const record = resultRecord(),
    state = await setupResults(page, { results: [record] });
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${record.id}`, exact: true })
    .click();
  state.scheduling.denied = true;
  await resultPanel(page)
    .getByRole('button', { name: 'Actualizar resultados', exact: true })
    .click();
  await expect(resultDetail(page)).toHaveCount(0);
  await expect(page.getByText(record.values.summary, { exact: true })).toHaveCount(0);
});

test('client navigation never queries protected result endpoints', async ({ page }) => {
  const state = await setupResults(page, { role: 'client' });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Audiencias', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'hearings';
  });
  await expect(resultPanel(page)).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});
