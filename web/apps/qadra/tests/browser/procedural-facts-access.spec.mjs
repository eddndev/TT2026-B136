import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  fillResolution,
  confirmFact,
} from './procedural-facts-helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';
import { login, navigate, expectClientWorkspace } from './helpers.mjs';

test('assigned litigator records without requiring a complete penal profile or stage', async ({
  page,
}) => {
  const state = await setupFacts(page, { role: 'litigator' });
  state.results.scheduling.context.profile_complete = false;
  state.results.scheduling.context.stage_revision = null;
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  await confirmFact(page);
  await expect(factEditor(page)).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0].change).not.toHaveProperty('expected_stage_revision');
});

test('assigned paralegal reads history without resolution or notification mutation controls', async ({
  page,
}) => {
  const row = factRecord(),
    state = await setupFacts(page, { role: 'paralegal', facts: [row] });
  await openFacts(page);
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await expect(factDetail(page)).toContainText(row.values.summary);
  for (const verb of ['Registrar', 'Corregir', 'Retirar'])
    await expect(
      page.getByRole('button', { name: `${verb} resoluci\u00f3n`, exact: true }),
    ).toHaveCount(0);
  await factDetail(page)
    .getByRole('button', { name: 'Ver historial de resoluci\u00f3n', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
});
