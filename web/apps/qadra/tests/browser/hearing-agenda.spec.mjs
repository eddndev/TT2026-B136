import { test, expect } from '@playwright/test';
import { hearingRecord, hearingRow } from '../fixtures/hearings.mjs';
import { login, navigate } from './helpers.mjs';
import {
  agendaPage,
  queryHearingAgenda as query,
  setupHearingAgenda,
} from './combined-agenda-helpers.mjs';

test('agenda queries a single transversal endpoint and opens an exact one-use hearing intent', async ({
  page,
}) => {
  const record = hearingRecord(),
    { state, requests, agenda } = await setupHearingAgenda(page, [record]);
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await query(page);
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }),
  ).toBeVisible();
  expect(
    requests.filter(
      (row) => row.path.includes('/cases') || row.path.includes('/case-administrations'),
    ),
  ).toHaveLength(0);
  expect(agenda.calls.length).toBeGreaterThan(0);
  expect(agenda.calls.every((url) => url.pathname === '/api/v1/agenda')).toBeTruthy();
  expect(state.calls).toHaveLength(0);
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('consultada exactamente');
  expect(
    state.calls.some((row) => row.path.endsWith(`/hearings/${record.id}/revisions/1`)),
  ).toBeTruthy();
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  const exactReads = state.calls.filter((row) => row.path.includes('/revisions/')).length;
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Detalle de audiencia', exact: true })).toHaveCount(
    0,
  );
  expect(state.calls.filter((row) => row.path.includes('/revisions/'))).toHaveLength(exactReads);
});

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

test('an old agenda query cannot replace a newer explicit range', async ({ page }) => {
  const record = hearingRecord(),
    { agenda } = await setupHearingAgenda(page, [record]);
  let release;
  await login(page, false, false);
  await navigate(page, 'Agenda');
  agenda.handle = async (route, url) => {
    if (!url.searchParams.get('from')?.startsWith('2026-10-01')) return false;
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({
      json: agendaPage(url, [
        {
          kind: 'hearing',
          hearing: hearingRow(record),
          at: {
            unix_seconds: 1790866923,
            nanosecond: 0,
            offset_seconds: 0,
          },
        },
      ]),
    });
    return true;
  };
  await query(page);
  await expect.poll(() => !!release).toBeTruthy();
  await query(page, '2026-10-05', '2026-10-06');
  await expect(
    page.getByText('No hay actividades en esta consulta.', { exact: true }),
  ).toBeVisible();
  const late = page.waitForResponse((response) => response.url().includes('from=2026-10-01'));
  release();
  await late;
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }),
  ).toHaveCount(0);
});

test('agenda exact intent waits for its case context and listing before reading a third resource', async ({
  page,
}) => {
  const record = hearingRecord(),
    { state } = await setupHearingAgenda(page, [record]);
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await query(page);
  let release,
    started = 0;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  state.handle = async (_route, call) => {
    if (
      call.path.endsWith('/hearings/context') ||
      (call.path.includes('/cases/') && call.path.endsWith('/hearings'))
    ) {
      started++;
      await gate;
    }
    return false;
  };
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await expect.poll(() => started).toBe(2);
  try {
    expect(state.calls.filter((call) => call.path.includes('/revisions/'))).toHaveLength(0);
  } finally {
    release();
  }
  await expect(
    page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
  ).toContainText('consultada exactamente');
});
