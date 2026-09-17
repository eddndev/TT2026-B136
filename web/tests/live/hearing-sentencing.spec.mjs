import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
import { archive, capture } from './case-administration-helpers.mjs';
import {
  accounts,
  editor,
  detail,
  openHearings,
  fillHearing,
  prepare,
  submit,
} from './hearing-helpers.mjs';
async function supportEvidence(page, testInfo, name) {
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page
    .getByRole('button', { name: /hearing-conviction-later.pdf/ })
    .first()
    .click();
  await page.getByRole('button', { name: /Versi\u00f3n 1.*hearing-conviction.pdf/ }).click();
  await expect(
    page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
  ).toBeEnabled();
  return archive(page, testInfo, name);
}
test('real sentencing uses declared conviction and exact sealed support without changing historical evidence', async ({
  page,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 5);
  await openCase(page, accounts.trialCase);
  const before = await supportEvidence(page, testInfo, 'hearing-support-before.zip');
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  await editor(page)
    .getByRole('combobox', { name: 'Tipo de audiencia', exact: true })
    .selectOption('sentencing');
  await editor(page)
    .getByLabel('Declaraci\u00f3n del antecedente', { exact: true })
    .fill('Antecedente sintetico declarado con soporte documental exacto');
  await editor(page)
    .getByRole('button', { name: 'Elegir soporte del antecedente', exact: true })
    .click();
  const picker = editor(page).getByRole('region', {
    name: 'Seleccionar soporte exacto',
    exact: true,
  });
  await picker.getByRole('button', { name: /hearing-conviction-later.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ hearing-conviction.pdf/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  const record = await submit(page, await prepare(page));
  expect(record.values.conviction_basis.support).toEqual({
    document_id: accounts.trialSupport.document_id,
    version: 1,
    digest: accounts.trialSupport.digest,
  });
  expect(record.support.format).toBe('pdf');
  expect(record.scheduling_context.stage).toBe('trial');
  await expect(detail(page)).toContainText('Antecedente de condena declarado');
  await capture(page, testInfo, 'hearing-sentencing-desktop', detail(page));
  await page.getByRole('button', { name: 'Cancelar audiencia', exact: true }).click();
  await editor(page)
    .getByLabel('Motivo de cancelaci\u00f3n', { exact: true })
    .fill('Cancelacion sin alterar soporte historico');
  const cancelled = await submit(page, await prepare(page));
  expect(cancelled.values).toEqual(record.values);
  expect(cancelled.support).toEqual(record.support);
  expect(await supportEvidence(page, testInfo, 'hearing-support-after.zip')).toEqual(before);
  expect(errors).toEqual([]);
});
