import { test, expect } from '@playwright/test';
import {
  setupProceduralResources,
  openResources,
  resourceEditor,
  resourceDetail,
  browserResource,
} from './procedural-resources-helpers.mjs';
for (const width of [1440, 390])
  test(`archives and reactivates a resource without a withdrawal act at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupProceduralResources(page, { resources: [browserResource()] });
    await openResources(page);
    await page
      .getByRole('button', { name: 'Consultar recurso Recurso declarado', exact: true })
      .click();
    for (const [action, reason] of [
      ['Archivar recurso', 'Organizacion interna'],
      ['Reactivar recurso', 'Retomar seguimiento'],
    ]) {
      await resourceDetail(page).getByRole('button', { name: action, exact: true }).click();
      await resourceEditor(page).getByLabel('Motivo', { exact: true }).fill(reason);
      await resourceEditor(page)
        .getByRole('button', { name: 'Preparar registro', exact: true })
        .click();
      await resourceEditor(page)
        .getByRole('button', { name: 'Confirmar registro', exact: true })
        .click();
      await expect(resourceEditor(page)).toHaveCount(0);
    }
    expect(state.submissions.map((v) => v.change.action)).toEqual(['archive', 'reactivate']);
    expect([...state.records.values()][0].every((v) => v.act === null)).toBe(true);
    await resourceDetail(page)
      .getByRole('button', { name: 'Ver historial de recurso', exact: true })
      .click();
    await page.getByRole('button', { name: 'Consultar recurso revision 1', exact: true }).click();
    await expect(resourceDetail(page)).toContainText('Consultada exactamente');
    await expect(
      resourceDetail(page).getByRole('button', { name: 'Archivar recurso', exact: true }),
    ).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: testInfo.outputPath('resource-history.png'), fullPage: true });
  });
