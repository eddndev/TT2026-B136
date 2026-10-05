import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import {
  setupPrecautionaryScheduling,
  openPrecautionaryForm,
  fillPrecautionaryForm,
  precautionaryEditor,
} from './precautionary-hearing-scheduling-helpers.mjs';

for (const width of [1440, 390])
  test(`precautionary scheduling fields stay usable at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await setupPrecautionaryScheduling(page);
    await openPrecautionaryForm(page);
    await fillPrecautionaryForm(page);
    const editor = precautionaryEditor(page);
    await expect(editor.getByLabel('Sede o enlace', { exact: true })).toBeVisible();
    await expect(
      editor.getByRole('button', { name: 'Revisar convocatoria', exact: true }),
    ).toBeEnabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    for (const field of await editor.locator('input, textarea, select, button').all()) {
      if (!(await field.isVisible())) continue;
      const box = await field.boundingBox();
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
    }
    await page.evaluate(async () => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    });
    const output = resolve('../output/precautionary-measures/qadra-appointments/visual');
    await mkdir(output, { recursive: true });
    await page.screenshot({
      path: `${output}/precautionary-form-page-${width}.png`,
      fullPage: true,
    });
  });
