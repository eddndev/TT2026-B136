import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  openDeadlines,
  editor,
  detail,
  fill,
  prepare,
  submit,
} from './deadline-helpers.mjs';

async function captureDeadline(page, testInfo, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(0);
  const skipLink = await page.getByRole('link', { name: 'Saltar al contenido' }).boundingBox();
  expect(skipLink).not.toBeNull();
  expect(skipLink.y + skipLink.height).toBeLessThanOrEqual(0);
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
}

test('real daily monthly and hourly deadlines preserve exact sources calendar history and declared attention', async ({
  page,
}, testInfo) => {
  test.setTimeout(120000);
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openDeadlines(page);
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fill(page, 'daily', 'Respuesta diaria con calendario exacto');
  const dailyDraft = await prepare(page);
  expect(dailyDraft.calculation.material.source.reference.revision).toBe(1);
  expect(dailyDraft.calculation.material.source_head.reference.revision).toBe(2);
  expect(dailyDraft.calculation.material.calendar.revision).toBe(1);
  expect(dailyDraft.calculation.material.calendar_head.revision).toBe(2);
  expect(dailyDraft.calculation.result.arithmetic.outcome).toEqual({
    kind: 'civil_candidate',
    date: '2026-01-31',
  });
  expect(dailyDraft.calculation.result.blocks).toEqual([]);
  const daily = await submit(page, dailyDraft);
  expect(daily.responsible.id).toBe(accounts.litigator.id);
  expect(daily.receipt.submission_digest).toBe(dailyDraft.submission_digest);
  await expect(detail(page)).toContainText('2026-01-31');
  await detail(page)
    .getByText('Consultar insumos y fuentes de esta revisi\u00f3n', { exact: true })
    .click();
  await detail(page)
    .getByRole('button', { name: 'Consultar fuente seleccionada', exact: true })
    .click();
  await expect(detail(page)).toContainText(accounts.sources.case.initial.values.summary);
  await detail(page)
    .getByRole('button', { name: 'Consultar fuente observada al registrar', exact: true })
    .click();
  await expect(detail(page)).toContainText(accounts.sources.case.head.values.summary);
  await captureDeadline(page, testInfo, 'deadline-daily-desktop');
  await detail(page).getByRole('button', { name: 'Declarar atenci\u00f3n', exact: true }).click();
  await editor(page)
    .getByRole('combobox', { name: 'Estado de atenci\u00f3n', exact: true })
    .selectOption('recorded');
  await editor(page)
    .getByRole('combobox', { name: 'Precisi\u00f3n de atenci\u00f3n', exact: true })
    .selectOption('unknown');
  await editor(page)
    .getByLabel('Declaraci\u00f3n de atenci\u00f3n', { exact: true })
    .fill('Presentacion declarada sin presumir cumplimiento');
  await editor(page)
    .getByLabel('Localizador de atenci\u00f3n', { exact: true })
    .fill('Constancia declarada');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Informar atencion');
  const attended = await submit(page, await prepare(page));
  expect(attended.calculation).toEqual(daily.calculation);
  await detail(page).getByRole('button', { name: 'Retirar plazo', exact: true }).click();
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Retirar seguimiento sintetico');
  const retired = await submit(page, await prepare(page));
  expect(retired.status).toBe('retired');
  expect(retired.calculation).toEqual(daily.calculation);
  expect(retired.attention).toEqual(attended.attention);
  await detail(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
  const exact = page.waitForResponse((response) =>
    response.url().endsWith(`/deadlines/${daily.id}/revisions/1`),
  );
  await page.getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true }).click();
  expect(await (await exact).json()).toEqual(daily);
  await expect(detail(page)).toContainText('Pendiente de declaraci\u00f3n de atenci\u00f3n');
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fill(page, 'monthly', 'Mes civil sin dia homologo');
  const monthDraft = await prepare(page);
  expect(monthDraft.calculation.result.due_at).toBeNull();
  expect(monthDraft.calculation.result.arithmetic.outcome).toEqual({
    kind: 'blocked',
    block: { kind: 'missing_homologous_day', year: 2026, month: 2, requested_day: 31 },
  });
  await expect(editor(page)).toContainText('no existe en 2026-02');
  const month = await submit(page, monthDraft);
  expect(month.calculation.result.due_at).toBeNull();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await fill(page, 'hourly', 'Horas con cantidad expresamente concedida');
  const missingDraft = await prepare(page);
  expect(missingDraft.definition.input.ordered_quantity).toBeNull();
  expect(missingDraft.calculation.result.rule).toBeNull();
  await expect(editor(page)).toContainText(
    'Falta declarar la duraci\u00f3n efectivamente concedida',
  );
  const missing = await submit(page, missingDraft);
  await detail(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  await editor(page)
    .getByRole('combobox', { name: 'Cantidad ordenada', exact: true })
    .selectOption('known');
  await editor(page).getByLabel('Cantidad declarada', { exact: true }).fill('24');
  await editor(page)
    .getByLabel('Motivo', { exact: true })
    .fill('Declarar cantidad concedida sin sustituirla por el maximo');
  const hoursDraft = await prepare(page);
  expect(hoursDraft.calculation.result.due_at).toEqual({
    unix_seconds: 1767817807,
    nanosecond: 0,
    offset_seconds: 0,
  });
  expect(hoursDraft.calculation.result.arithmetic.trace[0].quantity).toBe(24);
  expect(hoursDraft.definition.input.selection.qualification.at.offset_seconds).toBe(-21600);
  const hours = await submit(page, hoursDraft);
  expect(hours.revision).toBe(2);
  await expect(detail(page)).toContainText('24 horas transcurridas');
  await captureDeadline(page, testInfo, 'deadline-hours-mobile');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await detail(page).getByRole('button', { name: 'Ver historial de plazo', exact: true }).click();
  const oldHours = page.waitForResponse((response) =>
    response.url().endsWith(`/deadlines/${hours.id}/revisions/1`),
  );
  await page.getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true }).click();
  expect(await (await oldHours).json()).toEqual(missing);
  expect(errors).toEqual([]);
});
