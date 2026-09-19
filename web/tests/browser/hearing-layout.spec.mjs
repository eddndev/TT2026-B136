import { test, expect } from '@playwright/test';
import { openHearings, hearingEditor, fillHearing } from './hearing-helpers.mjs';
import { setupHearingAgenda, queryHearingAgenda } from './combined-agenda-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
async function captureNormal(page, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  const skip = await page.getByRole('link', { name: 'Saltar al contenido' }).boundingBox();
  expect(skip.y + skip.height).toBeLessThanOrEqual(0);
  await page.screenshot({ path: `../output/hearings-implementation/${name}.png`, fullPage: true });
}
for (const width of [1440, 390]) {
  test(`hearing and agenda layouts remain usable at ${width} pixels`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const record = hearingRecord();
    await setupHearingAgenda(page, [record]);
    await openHearings(page);
    await page
      .getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true })
      .click();
    await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
    await expect(
      page.getByRole('region', { name: 'Historial de audiencia', exact: true }),
    ).toContainText('Revisi\u00f3n 1');
    await expect
      .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
      .toBeTruthy();
    await captureNormal(page, `hearings-${width}`);
    await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
    await fillHearing(page);
    await expect(hearingEditor(page)).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
      .toBeTruthy();
    await captureNormal(page, `hearing-form-${width}`);
    await page.getByRole('button', { name: 'Ir a Agenda', exact: true }).click();
    await queryHearingAgenda(page);
    await expect(
      page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }),
    ).toBeVisible();
    await expect
      .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
      .toBeTruthy();
    await captureNormal(page, `agenda-${width}`);
  });
}
