import { test, expect } from '@playwright/test';
import { setupRelated, openRelatedFromAlert, related } from './activity-resources-helpers.mjs';

test('shrinking from desktop to phone leaves no horizontal overflow at once', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const state = await setupRelated(page, 'hearing');
  await openRelatedFromAlert(page, state);
  await page.setViewportSize({ width: 390, height: 1000 });
  await expect(related(page)).toBeVisible();
  // Measured right away: the sidebar offset must not animate out on a phone.
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});
