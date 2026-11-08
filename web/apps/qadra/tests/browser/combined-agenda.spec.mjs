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
    const calendar = page.getByRole('region', { name: 'Calendario de la agenda', exact: true });
    const list = page.getByRole('region', { name: 'Actividades en el periodo', exact: true });
    // The calendar opens on the month of the reference date (2026-01-02), so
    // the week from 2025-12-29 only names its four January days; the three
    // December days fall in the inert leading cells of January.
    for (const [view, days, from, until] of [
      ['day', 1, '2026-01-02T00:00:00Z', '2026-01-03T00:00:00Z'],
      ['week', 4, '2025-12-29T00:00:00Z', '2026-01-05T00:00:00Z'],
      ['month', 31, '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z'],
    ]) {
      await queryAgenda(page, view);
      await expect.poll(() => state.calls.at(-1)?.searchParams.get('from')).toBe(from);
      expect(state.calls.at(-1).searchParams.get('until')).toBe(until);
      await expect(calendar).toBeVisible();
      await expect(list).toBeVisible();
      await expect(
        calendar.getByRole('button', { name: '2 de enero de 2026: 2 actividades', exact: true }),
      ).toBeVisible();
      await expect(calendar.locator('[data-agenda-day]')).toHaveCount(days);
      await expect(activity(page, 'hearing', state.hearing.id)).toBeVisible();
      await expect(activity(page, 'deadline', state.deadline.id)).toBeVisible();
      expect(
        await list
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

test('the agenda calendar marks the queried day and queries a clicked day', async ({ page }) => {
  const state = await setupCombinedAgenda(page);
  await openCombinedAgenda(page);
  await queryAgenda(page);
  const calendar = page.getByRole('region', { name: 'Calendario de la agenda', exact: true });
  const day = (name) => calendar.getByRole('button', { name, exact: true });
  await expect(day('2 de enero de 2026: 2 actividades')).toHaveAttribute('aria-pressed', 'true');
  await expect(day('3 de enero de 2026: fuera del periodo consultado')).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  const count = state.calls.length;
  await calendar.getByRole('button', { name: 'Mes siguiente', exact: true }).click();
  await expect(calendar.getByRole('heading', { name: 'marzo de 2026', exact: true })).toHaveCount(
    1,
  );
  await expect(calendar.getByRole('heading', { name: 'enero de 2026', exact: true })).toHaveCount(
    0,
  );
  expect(state.calls).toHaveLength(count);
  await calendar.getByRole('button', { name: 'Mes anterior', exact: true }).click();
  await expect(calendar.getByRole('heading', { name: 'enero de 2026', exact: true })).toHaveCount(
    1,
  );
  expect(state.calls).toHaveLength(count);
  await day('5 de enero de 2026: fuera del periodo consultado').click();
  await expect
    .poll(() => state.calls.at(-1)?.searchParams.get('from'))
    .toBe('2026-01-05T00:00:00Z');
  expect(state.calls.at(-1).searchParams.get('until')).toBe('2026-01-06T00:00:00Z');
  await expect(page.getByLabel('Fecha de referencia', { exact: true })).toHaveValue('2026-01-05');
  await expect(page.getByRole('combobox', { name: 'Vista de agenda', exact: true })).toHaveValue(
    'day',
  );
  await expect(day('5 de enero de 2026: sin actividades')).toHaveAttribute('aria-pressed', 'true');
  await expect(
    page
      .getByRole('region', { name: 'Actividades en el periodo', exact: true })
      .getByText('No hay actividades en esta consulta.', { exact: true }),
  ).toBeVisible();
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
