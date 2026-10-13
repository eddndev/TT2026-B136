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
import { login, navigate } from './helpers.mjs';

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

test('closed case retains exact history with no enabled writes', async ({ page }) => {
  const row = factRecord();
  await setupFacts(page, { closed: true, facts: [row] });
  await openFacts(page);
  const create = page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true });
  await expect(create).toBeDisabled();
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await expect(factDetail(page)).toContainText(row.values.summary);
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  await factDetail(page)
    .getByRole('button', { name: 'Ver historial de resoluci\u00f3n', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true }),
  ).toBeVisible();
});

test('client never queries either family even through a protected hash', async ({ page }) => {
  const state = await setupFacts(page, { role: 'client' });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Resoluciones', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'resolutions';
  });
  await expect(
    page.getByRole('heading', { name: 'Resoluciones y notificaciones', exact: true }),
  ).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});
