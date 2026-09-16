import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarDetail,
  calendarEditor,
  fillCalendar,
} from './judicial-calendar-helpers.mjs';
import { calendarId } from '../fixtures/judicial-calendars.mjs';
for (const width of [390, 1440])
  test(`calendar detail and form preserve Qadra without horizontal overflow at ${width}`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    await setupCalendars(page);
    await openCalendars(page);
    await calendarPanel(page)
      .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
      .click();
    await expect(
      calendarDetail(page).getByRole('button', {
        name: '29 de febrero de 2000: Excluido',
        exact: true,
      }),
    ).toBeVisible();
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const output = '../output/judicial-calendars-implementation';
    await mkdir(output, { recursive: true });
    await page.screenshot({ path: `${output}/calendar-detail-${width}.png`, fullPage: true });
    await calendarDetail(page)
      .getByRole('button', { name: 'Ver lista de d\u00edas', exact: true })
      .click();
    await expect(
      calendarDetail(page).getByRole('button', {
        name: '29 de febrero de 2000: Excluido',
        exact: true,
      }),
    ).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await calendarPanel(page)
      .getByRole('button', { name: 'Publicar calendario', exact: true })
      .click();
    await fillCalendar(page);
    await expect(
      calendarEditor(page).getByRole('button', { name: 'Revisar calendario', exact: true }),
    ).toBeVisible();
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: `${output}/calendar-form-${width}.png`, fullPage: true });
  });
