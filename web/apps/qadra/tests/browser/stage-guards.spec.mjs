import { test, expect } from '@playwright/test';
import {
  setupStages,
  openStages,
  chooseSupport,
  fillDate,
  initial,
  caseId,
  entry,
  document,
} from './stage-helpers.mjs';
const command = {
  expected_revision: 1,
  target: 'intermediate',
  accusation_declared_at: {
    precision: 'date',
    date: '2026-09-01',
    offset: '-06:00',
  },
  accusation: { document_id: document.id, version: 1, digest: document.digest },
};
test('history uses exclusive revision cursor and retains initial original provenance', async ({
  page,
}) => {
  await setupStages(page, { current: entry(command) });
  const cursors = [];
  await page.route(`**/api/v1/cases/${caseId}/stage/history?*`, (route) => {
    const cursor = new URL(route.request().url()).searchParams.get('before_revision');
    cursors.push(cursor);
    return route.fulfill({
      json: cursor
        ? { entries: [initial], has_more: false, next_before_revision: null }
        : { entries: [entry(command)], has_more: true, next_before_revision: 2 },
    });
  });
  await openStages(page);
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await page.getByRole('button', { name: 'Cargar etapas anteriores' }).click();
  await expect(page.locator('.stage-history .stage-entry')).toHaveCount(2);
  expect(cursors).toEqual([null, '2']);
  await expect(page.locator('.stage-history')).toContainText('original@example.com');
});
