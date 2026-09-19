import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import {
  activity,
  agendaPage,
  cursorFor,
  openCombinedAgenda,
  queryAgenda,
  setupCombinedAgenda,
} from './combined-agenda-helpers.mjs';

for (const width of [1440, 390])
  test(`combined day week and month views preserve exact activity order at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupCombinedAgenda(page);
    await openCombinedAgenda(page);
    for (const [view, label, days] of [
      ['day', 'Vista diaria', 1],
      ['week', 'Vista semanal', 7],
      ['month', 'Vista mensual', 31],
    ]) {
      await queryAgenda(page, view);
      const region = page.getByRole('region', { name: label, exact: true });
      await expect(region).toBeVisible();
      await expect(region.locator('[data-agenda-day]')).toHaveCount(days);
      await expect(activity(page, 'hearing', state.hearing.id)).toBeVisible();
      await expect(activity(page, 'deadline', state.deadline.id)).toBeVisible();
      expect(
        await region
          .locator('[data-agenda-kind]')
          .evaluateAll((rows) => rows.map((row) => row.dataset.agendaKind)),
      ).toEqual(['hearing', 'deadline']);
      if (view === 'month')
        await page.screenshot({
          path: test.info().outputPath(`combined-month-compact-${width}.png`),
          fullPage: true,
        });
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
        true,
      );
    }
    await queryAgenda(page, 'month', '2026-01-31');
    await page.getByRole('button', { name: 'Periodo siguiente', exact: true }).click();
    await expect(page.getByLabel('Fecha de referencia', { exact: true })).toHaveValue('2026-02-28');
    await expect
      .poll(() => state.calls.at(-1)?.searchParams.get('from'))
      .toBe('2026-02-01T00:00:00Z');
    expect(state.calls.every((url) => url.searchParams.get('limit') === '20')).toBe(true);
  });

test('an empty partial page stays pending and later pages accumulate both kinds by newest revision', async ({
  page,
}) => {
  const state = await setupCombinedAgenda(page);
  const revised = structuredClone(state.hearingItem);
  revised.hearing.revision = 2;
  revised.hearing.scheduled_at = '2026-01-02T01:00:00Z';
  revised.at.unix_seconds += 3600;
  state.handle = async (route, url) => {
    if (!url.searchParams.get('from')?.startsWith('2026-01-02')) return false;
    const cursor = url.searchParams.get('cursor');
    const first = cursorFor(url, 1767312000, 0, 0, '00000000-0000-0000-0000-000000000000');
    const second = cursorFor(url, 1767312000, 0, 0, state.hearing.id);
    const result =
      cursor === second
        ? agendaPage(url, [state.deadlineItem, revised])
        : cursor === first
          ? agendaPage(url, [state.hearingItem], false, second)
          : agendaPage(url, [], false, first);
    await route.fulfill({ json: result });
    return true;
  };
  await openCombinedAgenda(page);
  await queryAgenda(page);
  await expect(
    page.getByText('Consulta parcial: faltan actividades por consultar.', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('No hay actividades en esta consulta.', { exact: true })).toHaveCount(
    0,
  );
  const more = page.getByRole('button', { name: 'Cargar m\u00e1s actividades', exact: true });
  await more.click();
  await expect(activity(page, 'hearing', state.hearing.id)).toBeVisible();
  await more.click();
  await expect(activity(page, 'hearing', state.hearing.id)).toContainText('Revisi\u00f3n 2');
  await expect(activity(page, 'deadline', state.deadline.id)).toBeVisible();
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(2);
  await expect(more).toHaveCount(0);
});

test('a late page cannot restore activities after applying a different period', async ({
  page,
}) => {
  const state = await setupCombinedAgenda(page);
  let release,
    started = false,
    completed = false;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  state.handle = async (route, url) => {
    if (!url.searchParams.get('from')?.startsWith('2026-01-02')) return false;
    started = true;
    await gate;
    await route.fulfill({ json: agendaPage(url, state.rows) });
    completed = true;
    return true;
  };
  await openCombinedAgenda(page);
  await queryAgenda(page);
  await expect.poll(() => started).toBe(true);
  await queryAgenda(page, 'day', '2026-01-05');
  await expect(
    page.getByText('No hay actividades en esta consulta.', { exact: true }),
  ).toBeVisible();
  release();
  await expect.poll(() => completed).toBe(true);
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(0);
  await expect(page.getByLabel('Fecha de referencia', { exact: true })).toHaveValue('2026-01-05');
});

for (const kind of ['hearing', 'deadline'])
  test(`opening a ${kind} from the agenda reads its exact revision after authorizing the case`, async ({
    page,
  }) => {
    const state = await setupCombinedAgenda(page);
    await openCombinedAgenda(page);
    await queryAgenda(page);
    const record = state[kind];
    await activity(page, kind, record.id).click();
    const label = kind === 'hearing' ? 'Detalle de audiencia' : 'Detalle de plazo';
    await expect(page.getByRole('region', { name: label, exact: true })).toContainText(
      /consultada exactamente/i,
    );
    const calls =
      kind === 'hearing' ? state.deadlines.facts.results.scheduling.calls : state.deadlines.calls;
    expect(
      calls.some((call) =>
        call.path.endsWith(
          `/${kind === 'hearing' ? 'hearings' : 'deadlines'}/${record.id}/revisions/1`,
        ),
      ),
    ).toBe(true);
    await page.getByRole('button', { name: 'Ir a Agenda', exact: true }).click();
    await expect(page.getByRole('combobox', { name: 'Vista de agenda', exact: true })).toHaveValue(
      'day',
    );
    await expect(activity(page, kind, record.id)).toBeVisible();
  });

test('revoked case access removes both kinds from the visible agenda', async ({ page }) => {
  const state = await setupCombinedAgenda(page);
  await openCombinedAgenda(page);
  await queryAgenda(page);
  await page.route(`**/api/v1/cases/${caseId}/administration`, (route) =>
    route.fulfill({
      status: 403,
      json: { error: { code: 'permission_denied' } },
    }),
  );
  await activity(page, 'deadline', state.deadline.id).click();
  await expect(
    page.getByRole('region', { name: 'Agenda combinada', exact: true }).getByRole('alert'),
  ).toBeVisible();
  await expect(page.locator('[data-agenda-kind]')).toHaveCount(0);
});

test('custom ranges preserve the explicit offset and type filters without sending invalid ranges', async ({
  page,
}) => {
  const state = await setupCombinedAgenda(page);
  await openCombinedAgenda(page);
  await page.getByRole('combobox', { name: 'Vista de agenda', exact: true }).selectOption('custom');
  await page.getByLabel('Desde (incluido)', { exact: true }).fill('2026-01-01');
  await page.getByLabel('Hasta (excluido)', { exact: true }).fill('2026-01-03');
  await page.getByLabel('Desfase de consulta', { exact: true }).fill('-06:00');
  await page
    .getByRole('combobox', { name: 'Estado de audiencia', exact: true })
    .selectOption('all');
  await page
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('deadline');
  await expect(
    page.getByRole('combobox', { name: 'Estado de audiencia', exact: true }),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  await expect(activity(page, 'deadline', state.deadline.id)).toBeVisible();
  await expect(activity(page, 'hearing', state.hearing.id)).toHaveCount(0);
  const call = state.calls.at(-1);
  expect(call.searchParams.get('from')).toBe('2026-01-01T06:00:00Z');
  expect(call.searchParams.get('until')).toBe('2026-01-03T06:00:00Z');
  expect(call.searchParams.get('kind')).toBe('deadline');
  expect(call.searchParams.get('hearing_status')).toBe('scheduled');
  const count = state.calls.length;
  await page.getByLabel('Hasta (excluido)', { exact: true }).fill('2027-01-03');
  await page.getByRole('button', { name: 'Consultar Agenda', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Agenda combinada', exact: true }).getByRole('alert'),
  ).toBeVisible();
  expect(state.calls).toHaveLength(count);
});
