import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
import { createPenal, capture, archive } from './case-administration-helpers.mjs';
import {
  accounts,
  pdf,
  openStages,
  fillDate,
  uploadSupport,
  chooseSupport,
  submitStage,
} from './stage-helpers.mjs';
test('stage transitions retain exact historic supports, explicit conflict and original initial registration', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  const created = await createPenal(page, 'Browser stage case', 'STAGE-NEW-001');
  const record = { id: created.id, title: created.administration.title };
  await openStages(page);
  await expect(page.locator('.stage-current')).toContainText('Registro inicial');
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n', true);
  const support = await uploadSupport(page, 'Acusaci\u00f3n');
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await context.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await loginAs(other, accounts.owner, 1);
    await openCase(other, record);
    await openStages(other);
    await other.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
    await fillDate(other, 'Fecha de la acusaci\u00f3n');
    await chooseSupport(other, 'Acusaci\u00f3n');
    await other.getByLabel('Nota (opcional)').fill('Borrador de otra sesion');
    const first = await submitStage(page, record.id);
    expect(first.current.stage).toBe('intermediate');
    expect(first.current.stage_revision).toBe(2);
    expect(first.current.supports[0].digest).toBe(support.digest);
    await other.getByRole('button', { name: 'Revisar registro' }).click();
    const rejected = other.waitForResponse(
      (r) => r.url().endsWith('/stage/transitions') && r.request().method() === 'POST',
    );
    await other.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
    expect((await rejected).status()).toBe(409);
    await expect(other.getByLabel('Nota (opcional)')).toHaveValue('Borrador de otra sesion');
    await other.getByRole('button', { name: 'Consultar etapa e historial' }).click();
    await expect(other.locator('.stage-reconciliation')).toContainText(
      'El avance del borrador ya no corresponde',
    );
    await expect(other.locator('.stage-form')).toHaveAttribute('aria-busy', 'false');
    await capture(
      other,
      testInfo,
      'stage-conflict-desktop',
      other.locator('.stage-reconciliation'),
    );
  } finally {
    await context.close();
  }
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page
    .getByRole('button', { name: /stage-accusation.pdf/ })
    .first()
    .click();
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
  ).toBeEnabled();
  const zip = await archive(page, testInfo, 'stage-support-original.zip');
  await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
  await page
    .getByLabel('Archivo de la nueva versi\u00f3n', { exact: true })
    .setInputFiles({ name: 'stage-later.pdf', mimeType: 'application/pdf', buffer: pdf });
  await page.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'stage-later.pdf', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toBeEnabled();
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Juicio' }).click();
  await fillDate(page, 'Fecha de emisi\u00f3n del auto');
  await fillDate(page, 'Fecha de recepci\u00f3n');
  await page.getByLabel('Tribunal receptor', { exact: true }).fill('Tribunal receptor declarado');
  await page.getByLabel('Referencia de recepci\u00f3n (opcional)').fill('REC-001');
  await chooseSupport(page, 'Auto de apertura', 'stage-later.pdf', 1);
  await chooseSupport(page, 'Constancia de recepci\u00f3n', 'stage-later.pdf', 1);
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await capture(
    page,
    testInfo,
    'stage-trial-confirmation-desktop',
    page.locator('.stage-confirmation'),
  );
  await page.getByRole('button', { name: 'Volver al borrador' }).click();
  const trial = await submitStage(page, record.id);
  expect(trial.current.stage).toBe('trial');
  expect(trial.current.stage_revision).toBe(3);
  expect(trial.current.supports).toHaveLength(1);
  expect(trial.current.supports[0].version).toBe(1);
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history .stage-entry')).toHaveCount(3);
  await expect(page.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
  await capture(page, testInfo, 'stage-history-desktop', page.locator('.stage-history'));
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'stage-history-mobile', page.locator('.stage-history'));
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  const links = await page
    .getByRole('navigation', { name: 'Secciones del expediente' })
    .getByRole('link')
    .all();
  for (let i = 0; i < links.length - 1; i++) {
    const a = await links[i].boundingBox(),
      b = await links[i + 1].boundingBox();
    expect(a.x + a.width <= b.x + 1 || a.y + a.height <= b.y + 1).toBe(true);
  }
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Investigaci\u00f3n', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar cierre administrativo', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Reactivar expediente', exact: true }),
  ).toBeVisible();
  await openStages(page);
  await expect(page.locator('.stage-current')).toContainText('Juicio');
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page
    .getByRole('button', { name: /stage-later.pdf/ })
    .first()
    .click();
  await page.getByRole('button', { name: /Versi\u00f3n 1.*stage-accusation.pdf/ }).click();
  await expect(
    page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
  ).toBeEnabled();
  expect(await archive(page, testInfo, 'stage-support-after-transitions.zip')).toEqual(zip);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await loginAs(page, accounts.owner, 2);
  await openCase(page, record);
  await openStages(page);
  await expect(page.locator('.stage-current')).toContainText('Juicio');
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history .stage-entry')).toHaveCount(3);
  expect(errors).toEqual([]);
});
