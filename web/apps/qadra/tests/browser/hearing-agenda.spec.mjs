import { test, expect } from '@playwright/test';
import { hearingRecord, hearingRow } from '../fixtures/hearings.mjs';
import { login, navigate, openSection } from './helpers.mjs';
import {
  agendaPage,
  queryHearingAgenda as query,
  setupHearingAgenda,
} from './combined-agenda-helpers.mjs';

test('agenda accumulates pages by the bound UTC cursor and preserves applied filters on return', async ({
  page,
}) => {
  const first = hearingRecord(),
    second = { ...structuredClone(first), id: '30000000-0000-4000-8000-000000000003' };
  const { state, agenda } = await setupHearingAgenda(page, [first, second]);
  state.pageSize = 1;
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await query(page);
  await page.getByRole('button', { name: 'Cargar m\u00e1s actividades', exact: true }).click();
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${second.id}`, exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${first.id}`, exact: true }),
  ).toBeVisible();
  await expect(page.locator('[data-agenda-kind="hearing"]')).toHaveCount(2);
  const cursor = agenda.calls.at(-1).searchParams.get('cursor');
  expect(cursor).toBe(`a1:1790812800:1790985600:hearing:scheduled:1790866923:0:0:${first.id}`);
  await page.getByRole('button', { name: `Consultar audiencia ${second.id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Ir a Agenda', exact: true }).click();
  await expect(page.getByLabel('Desde (incluido)', { exact: true })).toHaveValue('2026-10-01');
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${first.id}`, exact: true }),
  ).toBeVisible();
});
