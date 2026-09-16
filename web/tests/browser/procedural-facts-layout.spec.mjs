import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  factList,
  fillNotification,
} from './procedural-facts-helpers.mjs';
import {
  factRecord,
  factPrepared,
  factCommand,
  factResolutionSource,
} from '../fixtures/procedural-facts.mjs';

for (const width of [1440, 390])
  test(`Qadra fact details and full capture remain readable at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const parent = factRecord(),
      command = factCommand('notification');
    command.change.values.summary = 'Declaracion historica de practica. '.repeat(12).trim();
    command.change.values.practiced_at = {
      precision: 'minute',
      year: 2026,
      month: 9,
      day: 1,
      hour: 12,
      minute: 34,
      offset_seconds: null,
    };
    command.change.values.received_at = {
      precision: 'second',
      year: 2026,
      month: 9,
      day: 1,
      hour: 12,
      minute: 34,
      second: 56,
      offset_seconds: 0,
    };
    command.change.values.stated_effect = {
      at: { precision: 'unknown' },
      statement: 'Efecto expresamente declarado por la fuente sin eficacia inferida.',
      locator: 'Parrafo 2 de la nota del operador',
    };
    const row = factRecord(factPrepared(command));
    row.sources.resolution = factResolutionSource(parent);
    await setupFacts(page, { facts: [parent, row] });
    await openFacts(page);
    await factList(page)
      .getByRole('button', { name: `Consultar resoluci\u00f3n ${parent.id}`, exact: true })
      .click();
    await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
    await factList(page, 'notification')
      .getByRole('button', { name: `Consultar notificaci\u00f3n ${row.id}`, exact: true })
      .click();
    await expect(factDetail(page, 'notification')).toContainText(
      row.values.stated_effect.statement,
    );
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../output/procedural-facts-web/fact-detail-${width}.png`,
      fullPage: true,
    });
    await page.getByRole('button', { name: 'Registrar notificaci\u00f3n', exact: true }).click();
    await fillNotification(page);
    await expect(
      factEditor(page, 'notification').getByRole('button', {
        name: 'Preparar registro',
        exact: true,
      }),
    ).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    await page.screenshot({
      path: `../output/procedural-facts-web/fact-form-${width}.png`,
      fullPage: true,
    });
  });
