import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
import { capture } from './case-administration-helpers.mjs';
import {
  accounts,
  editor,
  detail,
  openHearings,
  openHearing,
  fillHearing,
  prepare,
  submit,
  selectParticipant,
  changeDirectory,
  advanceForCancellation,
} from './hearing-helpers.mjs';
test('real hearings reconcile an exact receipt and retain immutable references through conflict and cancellation', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openCase(page, accounts.case);
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  await selectParticipant(page, accounts.manual.display_name);
  await selectParticipant(page, accounts.typed.display_name);
  const prepared = await prepare(page);
  let actual,
    writes = 0;
  const endpoint = `**/api/v1/cases/${accounts.case.id}/hearings`;
  await page.route(endpoint, async (route) => {
    if (route.request().method() !== 'POST') return route.continue();
    writes++;
    const response = await route.fetch();
    expect(response.status()).toBe(201);
    actual = await response.json();
    await route.abort('failed');
  });
  await editor(page).getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(editor(page)).toContainText('Resultado del env\u00edo pendiente de confirmar');
  await editor(page)
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  await expect(editor(page)).toHaveCount(0);
  await expect(detail(page)).toContainText('consultada exactamente');
  expect(writes).toBe(1);
  expect(actual.receipt.submission_digest).toBe(prepared.submission_digest);
  expect(actual.recorded_by.id).toBe(accounts.owner.id);
  await page.unroute(endpoint);
  await page.getByRole('button', { name: 'Consultar registro actual', exact: true }).click();
  await changeDirectory();
  await expect(detail(page)).toContainText(accounts.manual.display_name);
  await expect(detail(page)).toContainText(accounts.typed.display_name);
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await context.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await loginAs(other, accounts.owner, 1);
    await openCase(other, accounts.case);
    await openHearings(other);
    await openHearing(other, actual.id);
    await other.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
    await editor(other)
      .getByLabel('Sede o conexi\u00f3n', { exact: true })
      .fill('Borrador concurrente preservado');
    await editor(other)
      .getByLabel('Motivo del cambio', { exact: true })
      .fill('Correccion de otra sesion');
    const stale = await prepare(other);
    await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
    await editor(page).getByLabel('Fecha', { exact: true }).fill('2030-10-02');
    await editor(page)
      .getByLabel('Motivo del cambio', { exact: true })
      .fill('Reprogramacion comunicada');
    const second = await submit(page, await prepare(page));
    expect(second.values.participants).toEqual(actual.values.participants);
    expect(second.participants).toEqual(actual.participants);
    expect(second.values.scheduled_at).toBe('2030-10-02T09:02:03-06:00');
    await submit(other, stale, 409);
    await expect(editor(other).getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
      'Borrador concurrente preservado',
    );
    await editor(other)
      .getByRole('button', { name: 'Consultar audiencia y contexto actuales', exact: true })
      .click();
    await editor(other)
      .getByRole('button', { name: 'Usar base consultada y revisar borrador', exact: true })
      .click();
    expect((await submit(other, await prepare(other))).revision).toBe(3);
  } finally {
    await context.close();
  }
  await advanceForCancellation();
  await page.getByRole('button', { name: 'Actualizar audiencias', exact: true }).click();
  await openHearing(page, actual.id);
  await page.getByRole('button', { name: 'Cancelar audiencia', exact: true }).click();
  await editor(page)
    .getByLabel('Motivo de cancelaci\u00f3n', { exact: true })
    .fill('Cancelacion organizativa declarada');
  const cancelled = await submit(page, await prepare(page));
  expect(cancelled.revision).toBe(4);
  expect(cancelled.scheduling_context.stage).toBe('investigation');
  expect(cancelled.participants).toEqual(actual.participants);
  await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
  await expect(page.locator('.hearing-revision')).toHaveCount(4);
  await capture(page, testInfo, 'hearing-history-desktop', detail(page));
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'hearing-history-mobile', detail(page));
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await loginAs(page, accounts.owner, 2);
  await openCase(page, accounts.case);
  await openHearings(page);
  await openHearing(page, actual.id);
  await expect(detail(page)).toContainText('Cancelada');
  await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
  await expect(page.locator('.hearing-revision')).toHaveCount(4);
  expect(errors).toEqual([]);
});
