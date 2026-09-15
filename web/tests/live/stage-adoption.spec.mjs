import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate, openCase } from '../case-administration-workflow.mjs';
import { capture } from './case-administration-helpers.mjs';
import {
  accounts,
  openStages,
  fillDate,
  uploadSupport,
  submitStage,
  revokeLitigator,
} from './stage-helpers.mjs';
test('legacy adoption keeps its own origin and current staff permissions', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.litigator, 0);
  await openCase(page, accounts.legacyCase);
  await expect(
    page.getByRole('heading', { name: 'Etapa sin registrar', exact: true }),
  ).toBeVisible();
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar etapa actual', exact: true }).click();
  await page.getByLabel('Etapa conocida', { exact: true }).selectOption('intermediate');
  await fillDate(page, 'Fecha de la etapa conocida');
  await page
    .getByLabel('Motivo de adopci\u00f3n')
    .fill('Registro de etapa conocida al incorporar el expediente.');
  await uploadSupport(page, 'Soporte de adopci\u00f3n', 'stage-adoption.pdf');
  const result = await submitStage(page, accounts.legacyCase.id, true);
  expect(result.current.stage_revision).toBe(1);
  expect(result.current.from_stage).toBe(null);
  expect(result.current.values.kind).toBe('adoption');
  expect(result.current.values.known_at).toEqual({
    precision: 'date',
    date: '2026-09-01',
    offset: '-06:00',
  });
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history .stage-entry')).toHaveCount(1);
  await expect(page.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
  await capture(page, testInfo, 'stage-adoption-desktop', page.locator('.stage-current'));
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'stage-adoption-mobile', page.locator('.stage-current'));
  await page.setViewportSize({ width: 1280, height: 720 });
  for (const role of ['paralegal', 'client']) {
    const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
    try {
      const other = await context.newPage(),
        paths = [];
      other.on('pageerror', (error) => errors.push(error.message));
      other.on('request', (r) => {
        const path = new URL(r.url()).pathname;
        if (path.startsWith('/api/v1/')) paths.push(path);
      });
      await other.goto('/');
      await loginAs(other, accounts[role], 0);
      await navigate(other, 'Expedientes');
      await expect(
        other.getByRole('button', { name: new RegExp(accounts.hiddenCase.title) }),
      ).toHaveCount(0);
      await other.getByRole('button', { name: new RegExp(accounts.legacyCase.title) }).click();
      if (role === 'paralegal') {
        await openStages(other);
        await expect(other.locator('.stage-current')).toContainText('Intermedia');
        await expect(other.getByRole('button', { name: 'Registrar paso a Juicio' })).toHaveCount(0);
        await other.getByRole('button', { name: 'Ver historial de etapas' }).click();
        await expect(other.locator('.stage-history .stage-entry')).toHaveCount(1);
        await expect(other.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
      } else {
        await expect(other.getByRole('link', { name: 'Etapas', exact: true })).toHaveCount(0);
        await other.evaluate(() => {
          location.hash = 'stages';
        });
        await expect(other.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
        expect(paths.filter((path) => /\/cases\/[^/]+\/stage(?:\/|$)/.test(path))).toEqual([]);
      }
    } finally {
      await context.close();
    }
  }
  await revokeLitigator();
  await page.getByRole('button', { name: 'Actualizar etapa', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Expediente no disponible' })).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveCount(0);
  expect(errors).toEqual([]);
});
