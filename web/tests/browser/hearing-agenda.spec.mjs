import { test, expect } from '@playwright/test';
import { setupHearings } from './hearing-helpers.mjs';
import { hearingRecord, hearingRow } from '../fixtures/hearings.mjs';
import { login, navigate } from './helpers.mjs';
async function query(page, from = '2026-10-01', until = '2026-10-03') {
  await page.getByLabel('Desde (incluido)', { exact: true }).fill(from);
  await page.getByLabel('Hasta (excluido)', { exact: true }).fill(until);
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
}

test('agenda queries a single transversal endpoint and opens an exact one-use hearing intent', async ({
  page,
}) => {
  const record = hearingRecord(),
    { state, requests } = await setupHearings(page, { records: [record] });
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
  expect(state.calls.every((row) => row.path === '/api/v1/hearings')).toBeTruthy();
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

test('agenda paginates by the paired UTC cursor and preserves applied filters on return', async ({
  page,
}) => {
  const first = hearingRecord(),
    second = { ...structuredClone(first), id: '30000000-0000-4000-8000-000000000003' };
  const { state } = await setupHearings(page, { records: [first, second] });
  state.pageSize = 1;
  await login(page, false, false);
  await navigate(page, 'Agenda');
  await query(page);
  await page.getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${second.id}`, exact: true }),
  ).toBeVisible();
  const cursor = state.calls.filter((row) => row.path === '/api/v1/hearings').at(-1).search;
  expect(cursor).toContain('after_time=2026-10-01T15%3A02%3A03Z');
  expect(cursor).toContain(`after_id=${first.id}`);
  await page.getByRole('button', { name: `Consultar audiencia ${second.id}`, exact: true }).click();
  await page.getByRole('button', { name: 'Ir a Agenda', exact: true }).click();
  await expect(page.getByLabel('Desde (incluido)', { exact: true })).toHaveValue('2026-10-01');
  await expect(
    page.getByRole('button', { name: `Consultar audiencia ${first.id}`, exact: true }),
  ).toBeVisible();
});

test('an old agenda query cannot replace a newer explicit range', async ({ page }) => {
  const record = hearingRecord(),
    { state } = await setupHearings(page, { records: [record] });
  let release;
  await login(page, false, false);
  await navigate(page, 'Agenda');
  state.handle = async (route, call) => {
    if (call.path !== '/api/v1/hearings' || !call.search.includes('from=2026-10-01')) return false;
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({
      json: { hearings: [hearingRow(record)], has_more: false, next_after: null },
    });
    return true;
  };
  await query(page);
  await expect.poll(() => !!release).toBeTruthy();
  await query(page, '2026-10-05', '2026-10-06');
  await expect(
    page.getByText('No hay audiencias en esta consulta.', { exact: true }),
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
    { state } = await setupHearings(page, { records: [record] });
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
