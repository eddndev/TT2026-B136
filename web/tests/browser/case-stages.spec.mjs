import { test, expect } from '@playwright/test';
import {
  setupStages,
  openStages,
  chooseSupport,
  fillDate,
  initial,
  caseId,
} from './stage-helpers.mjs';
test('stage head is independent of initial registration and history is on demand', async ({
  page,
}) => {
  const { state, requests } = await setupStages(page);
  await openStages(page);
  expect(state.requests).toHaveLength(1);
  await expect(page.getByText('Registro inicial', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history')).toContainText('original@example.com');
  expect(state.requests.filter((r) => r.path.endsWith('history'))).toHaveLength(1);
  expect(requests.filter((r) => r.path.includes('/documents'))).toHaveLength(0);
});
test('adoption has a mandatory exact support and preserves date precision', async ({ page }) => {
  const { state } = await setupStages(page, { current: null });
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar etapa actual', exact: true }).click();
  await page.getByLabel('Etapa conocida', { exact: true }).selectOption('trial');
  await fillDate(page, 'Fecha de la etapa conocida');
  await page.getByLabel('Motivo de adopci\u00f3n').fill('Registro anterior incorporado');
  await chooseSupport(page, 'Soporte de adopci\u00f3n');
  await page.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await expect(page.locator('.stage-confirmation')).toContainText('sin hora');
  await page
    .locator('.stage-confirmation')
    .getByRole('button', { name: 'Registrar etapa actual' })
    .click();
  await expect(page.locator('.stage-current')).toContainText('Juicio');
  expect(state.posts[0].expected_revision).toBe(0);
  expect(state.posts[0].known_at).toEqual({
    precision: 'date',
    date: '2026-09-01',
    offset: '-06:00',
  });
  expect(state.posts[0].support.version).toBe(1);
});
test('ordinary transition awaits readers and commits only an explicitly selected version', async ({
  page,
}) => {
  const { state } = await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await chooseSupport(page);
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.locator('.stage-current')).toContainText('Intermedia');
  expect(state.posts[0].expected_revision).toBe(initial.stage_revision);
  expect(Object.keys(state.posts[0]).sort()).toEqual([
    'accusation',
    'accusation_declared_at',
    'expected_revision',
    'target',
  ]);
  expect(state.requests.filter((r) => r.path.endsWith('/stage'))).toHaveLength(1);
});
for (const mode of ['pending', 'closed', 'paralegal'])
  test(`stage ${mode} permits consultation without mutations`, async ({ page }) => {
    await setupStages(page, {
      complete: mode !== 'pending',
      closed: mode === 'closed',
      role: mode === 'paralegal' ? mode : 'owner',
    });
    await openStages(page);
    await expect(page.getByRole('button', { name: 'Registrar paso a Intermedia' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Ver historial de etapas' })).toBeEnabled();
  });
