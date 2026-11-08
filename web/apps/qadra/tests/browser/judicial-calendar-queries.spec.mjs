import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarDetail,
} from './judicial-calendar-helpers.mjs';
import {
  calendarId,
  calendarRecord,
  calendarPrepared,
  calendarFixtureCommand,
} from '../fixtures/judicial-calendars.mjs';
const uuid = (n) => `00000000-0000-0000-0000-${String(n).padStart(12, '0')}`;
test('current roots are paginated globally with explicit filters and homonyms stay distinguishable', async ({
  page,
}) => {
  const records = Array.from({ length: 21 }, (_, i) => {
    const r = calendarRecord();
    r.id = uuid(i);
    r.values.scope.title = 'Mismo titulo';
    if (i === 20) {
      r.values.scope.jurisdiction = 'federal';
      r.values.scope.entity_codes = ['09'];
    }
    return r;
  });
  const { state } = await setupCalendars(page, { records });
  await openCalendars(page);
  const panel = calendarPanel(page);
  await expect(panel.getByRole('button', { name: /^Consultar calendario / })).toHaveCount(20);
  await panel
    .getByRole('button', { name: 'P\u00e1gina siguiente de calendarios', exact: true })
    .click();
  await expect(
    panel.getByRole('button', { name: `Consultar calendario ${uuid(20)}`, exact: true }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain(`after_id=${uuid(19)}`);
  await panel.getByRole('combobox', { name: 'Filtrar fuero', exact: true }).selectOption('federal');
  await panel.getByRole('combobox', { name: 'Filtrar entidad', exact: true }).selectOption('09');
  await panel.getByRole('button', { name: 'Aplicar filtros de calendarios', exact: true }).click();
  await expect(panel.getByRole('button', { name: /^Consultar calendario / })).toHaveCount(1);
  expect(state.calls.at(-1).search).toContain('entity_code=09');
  expect(state.calls.at(-1).search).not.toContain('after_id');
  await panel.getByRole('combobox', { name: 'Filtrar entidad', exact: true }).selectOption('32');
  await panel.getByRole('button', { name: 'Aplicar filtros de calendarios', exact: true }).click();
  await expect(panel).toContainText('No hay calendarios en esta consulta');
});
test('history pages remain light and exact retired revisions can classify dates', async ({
  page,
}) => {
  const records = Array.from({ length: 12 }, (_, i) => {
    const c = calendarFixtureCommand(i === 0 ? 'publish' : i === 11 ? 'retire' : 'replace', i);
    c.operation_id = uuid(i);
    return calendarRecord(calendarPrepared(c));
  });
  const { state } = await setupCalendars(page, { records });
  await openCalendars(page);
  const panel = calendarPanel(page);
  await expect(panel).toContainText('No hay calendarios');
  await panel
    .getByRole('combobox', { name: 'Estado del calendario', exact: true })
    .selectOption('all');
  await panel.getByRole('button', { name: 'Aplicar filtros de calendarios', exact: true }).click();
  await panel
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await expect(calendarDetail(page)).toContainText('Retirado');
  await expect(
    calendarDetail(page).getByRole('button', {
      name: '29 de febrero de 2000: Excluido',
      exact: true,
    }),
  ).toBeVisible();
  await calendarDetail(page)
    .getByRole('button', { name: 'Ver historial del calendario', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Cargar revisiones anteriores del calendario', exact: true })
    .click();
  expect(state.calls.at(-1).search).toContain('before_revision=3');
  await page
    .getByRole('button', { name: 'Consultar calendario revisi\u00f3n 1', exact: true })
    .click();
  await expect(calendarDetail(page)).toContainText('Revisi\u00f3n hist\u00f3rica');
  expect(state.calls.some((c) => c.path.endsWith('/revisions/1/days'))).toBe(true);
});
test('month controls stay within the declared coverage and mark today', async ({ page }) => {
  await page.clock.setFixedTime(new Date(2026, 10, 5, 12));
  const { state } = await setupCalendars(page);
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  const detail = calendarDetail(page);
  await expect(
    detail.getByRole('button', { name: '29 de febrero de 2000: Excluido', exact: true }),
  ).toBeVisible();
  await expect(detail.getByRole('button', { name: /^1 de marzo de 2000/ })).toHaveCount(0);
  await expect(detail.getByRole('button', { name: 'Mes anterior', exact: true })).toBeDisabled();
  const calls = state.calls.filter((c) => c.path.endsWith('/days')).length;
  await detail.getByRole('button', { name: 'Mes siguiente', exact: true }).click();
  await expect(
    detail.getByRole('button', { name: '1 de marzo de 2000: Computable', exact: true }),
  ).toBeVisible();
  expect(state.calls.filter((c) => c.path.endsWith('/days'))).toHaveLength(calls + 1);
  expect(state.calls.at(-1).search).toContain('from=2000-03-01');
  expect(state.calls.at(-1).search).toContain('through=2000-03-31');
  await expect(detail.getByLabel('Mes de consulta', { exact: true })).toHaveValue('2000-03');
  await expect(detail.getByRole('button', { name: 'Mes siguiente', exact: true })).toBeDisabled();
  await expect(detail.getByRole('button', { name: 'Mes anterior', exact: true })).toBeEnabled();
  await detail.getByLabel('Mes de consulta', { exact: true }).fill('2026-11');
  await detail.getByRole('button', { name: 'Consultar mes', exact: true }).click();
  await expect(
    detail.getByRole('button', { name: '5 de noviembre de 2026: Fuera de cobertura', exact: true }),
  ).toHaveAttribute('aria-current', 'date');
  await expect(detail.locator('[aria-current="date"]')).toHaveCount(1);
  await expect(detail.getByRole('button', { name: 'Mes siguiente', exact: true })).toBeDisabled();
  await expect(detail.getByRole('button', { name: 'Mes anterior', exact: true })).toBeEnabled();
});
