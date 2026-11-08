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
    const legend = calendarDetail(page).getByRole('list', {
      name: 'Leyenda de estados',
      exact: true,
    });
    await expect(legend.getByRole('listitem')).toHaveText([
      /Computable/,
      /Excluido/,
      /Sin resolver/,
      /Fuera de cobertura/,
    ]);
    const day = calendarDetail(page).getByRole('button', {
      name: '29 de febrero de 2000: Excluido',
      exact: true,
    });
    await expect(day).toHaveAttribute('aria-pressed', 'false');
    await day.click();
    await expect(day).toHaveAttribute('aria-pressed', 'true');
    await page.evaluate(() => {
      document.activeElement?.blur();
      window.scrollTo(0, 0);
    });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    const output = '../../../output/judicial-calendars-implementation';
    await mkdir(output, { recursive: true });
    await page.screenshot({ path: `${output}/calendar-detail-${width}.png`, fullPage: true });
    await calendarDetail(page)
      .getByRole('button', { name: 'Ver lista de d\u00edas', exact: true })
      .click();
    const listed = calendarDetail(page).getByRole('button', {
      name: '29 de febrero de 2000: Excluido',
      exact: true,
    });
    await expect(listed).toBeVisible();
    await expect(listed).toHaveAttribute('aria-pressed', 'true');
    await expect(calendarDetail(page).getByRole('group', { name: 'febrero de 2000' })).toHaveCount(
      0,
    );
    await expect(
      calendarDetail(page).getByRole('region', { name: 'Fechas clasificadas', exact: true }),
    ).toContainText('Todo el d\u00eda');
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
