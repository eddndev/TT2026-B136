import { test, expect } from '@playwright/test';
import { hearingRecord, hearingRow } from '../fixtures/hearings.mjs';
import { dashboard, setupDashboard, enterDashboard } from './dashboard-helpers.mjs';

// The coming agenda of the overview starts at the local civil day, so the
// zone and the clock are fixed.
test.use({ timezoneId: 'America/Mexico_City' });

const block = (page) =>
  page.getByRole('region', { name: 'Pr\u00f3ximos plazos y audiencias', exact: true });
const checkedAt = { unix_seconds: 1767225600, nanosecond: 0, offset_seconds: 0 };

function hearingItem(id, scheduledAt) {
  const record = hearingRecord();
  record.id = id;
  record.values.scheduled_at = scheduledAt;
  return {
    kind: 'hearing',
    at: { unix_seconds: Date.parse(scheduledAt) / 1000, nanosecond: 0, offset_seconds: 0 },
    hearing: hearingRow(record),
  };
}

async function routeAgenda(page) {
  const state = { calls: [], items: [], status: 200 };
  await page.route('**/api/v1/agenda?*', (route) => {
    const url = new URL(route.request().url());
    state.calls.push(url);
    if (state.status !== 200)
      return route.fulfill({ status: state.status, json: { error: { code: 'unavailable' } } });
    const from = Date.parse(url.searchParams.get('from')) / 1000;
    const until = Date.parse(url.searchParams.get('until')) / 1000;
    return route.fulfill({
      json: {
        from: url.searchParams.get('from'),
        until: url.searchParams.get('until'),
        kind: url.searchParams.get('kind') || 'all',
        hearing_status: url.searchParams.get('hearing_status') || 'scheduled',
        checked_at: checkedAt,
        items: state.items.filter(
          (item) => item.at.unix_seconds >= from && item.at.unix_seconds < until,
        ),
        complete: true,
        next_cursor: null,
      },
    });
  });
  return state;
}

test.beforeEach(async ({ page }) => {
  await page.clock.install({ time: new Date('2026-01-01T12:00:00Z') });
});

test('the overview lists the coming agenda with the case reference and marks its days', async ({
  page,
}) => {
  await setupDashboard(page);
  const agenda = await routeAgenda(page);
  agenda.items = [
    hearingItem('aaaaaaaa-0000-4000-8000-000000000001', '2026-01-02T17:00:00Z'),
    hearingItem('aaaaaaaa-0000-4000-8000-000000000002', '2026-01-05T16:00:00Z'),
    hearingItem('aaaaaaaa-0000-4000-8000-000000000003', '2026-03-02T16:00:00Z'),
  ];
  await enterDashboard(page);
  const rows = block(page).getByRole('listitem');
  await expect(rows).toHaveCount(2);
  // The list heading and the calendar carry decoded accessible names.
  await expect(
    block(page).getByRole('heading', { name: 'Actividades pr\u00f3ximas', level: 3, exact: true }),
  ).toBeAttached();
  await expect(
    block(page).getByRole('region', {
      name: 'Calendario de pr\u00f3ximas actividades',
      exact: true,
    }),
  ).toBeVisible();
  // Thirty local days from today, every kind, scheduled hearings.
  const query = agenda.calls[0].searchParams;
  expect(query.get('from')).toBe('2026-01-01T06:00:00Z');
  expect(query.get('until')).toBe('2026-01-31T06:00:00Z');
  expect(query.get('kind')).toBe('all');
  expect(query.get('hearing_status')).toBe('scheduled');
  await expect(rows.first().locator('.font-mono')).toHaveText('NUC-123');
  await expect(rows.first()).toContainText('Defensa inicial');
  await expect(rows.first().locator('time').first()).toHaveAttribute('datetime', '2026-01-02');
  await expect(block(page).locator('[data-agenda-day]')).toHaveCount(2);
  await expect(block(page).locator('[data-agenda-day="2026-01-05"]')).toHaveAttribute(
    'aria-label',
    '5 de enero de 2026: 1 actividad',
  );
  await rows.first().getByRole('button').click();
  await expect(page).toHaveURL(/#agenda$/);
  await expect(page.getByRole('region', { name: 'Agenda combinada', exact: true })).toBeVisible();
});

test('the coming agenda reports a failure and an empty period', async ({ page }) => {
  await setupDashboard(page);
  const agenda = await routeAgenda(page);
  agenda.status = 503;
  await enterDashboard(page);
  await expect(block(page).getByRole('alert')).toBeVisible();
  await expect(block(page).getByRole('listitem')).toHaveCount(0);
  agenda.status = 200;
  await dashboard(page)
    .getByRole('button', { name: 'Actualizar indicadores', exact: true })
    .click();
  await expect(block(page)).toContainText(
    'Sin plazos ni audiencias en los pr\u00f3ximos d\u00edas',
  );
  await expect(block(page).getByRole('alert')).toHaveCount(0);
  expect(agenda.calls).toHaveLength(2);
});
