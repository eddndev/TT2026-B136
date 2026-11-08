import { test, expect } from '@playwright/test';
import { setup, login, selectCase } from './helpers.mjs';

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

test('the desktop sidebar collapses to icons and keeps keyboard focus on its toggle', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  const sidebar = page.locator('.desktop-sidebar');
  const nav = sidebar.getByRole('navigation');
  const collapse = sidebar.getByRole('button', { name: 'Contraer men\u00fa', exact: true });
  await expect(collapse).toHaveAttribute('aria-expanded', 'true');
  const wide = (await sidebar.boundingBox()).width;
  await collapse.focus();
  await page.keyboard.press('Enter');
  const expand = sidebar.getByRole('button', { name: 'Expandir men\u00fa', exact: true });
  await expect(expand).toBeFocused();
  await expect(expand).toHaveAttribute('aria-expanded', 'false');
  await expect.poll(async () => (await sidebar.boundingBox()).width).toBeLessThan(wide / 2);
  // The logo keeps its full 24px icon plus padding while collapsed.
  await expect.poll(async () => (await expand.boundingBox()).width).toBe(40);
  const home = nav.getByRole('button', { name: 'Inicio', exact: true });
  await expect(home).toBeVisible();
  await expect(home).toHaveAttribute('title', 'Inicio');
  // The label stays in the accessible name but fades out.
  await expect(nav.getByText('Inicio', { exact: true })).toHaveCSS('opacity', '0');
  const main = page.locator('#main-content');
  expect((await main.boundingBox()).x).toBeLessThan(wide);
  await page.keyboard.press('Enter');
  await expect(collapse).toBeFocused();
  await expect(nav.getByText('Inicio', { exact: true })).toHaveCSS('opacity', '1');
});

test('the sidebar keeps only fixed destinations after a case is opened', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  await selectCase(page);
  const sidebar = page.locator('.desktop-sidebar');
  await expect(sidebar.getByRole('link')).toHaveCount(0);
  await expect(sidebar.getByText('Defensa inicial')).toHaveCount(0);
});

// Samples the top and height of every sidebar row on each animation frame
// while the toggle runs, so a layout jump at any point of the transition shows.
async function sampleRows(page, toggle) {
  await page.evaluate(() => {
    const rows = () => [...document.querySelectorAll('.desktop-sidebar nav li > :is(button, a)')];
    window.sidebarSamples = [];
    window.sidebarSampling = true;
    const start = performance.now();
    const frame = () => {
      window.sidebarSamples.push(
        rows().map((node) => {
          const box = node.getBoundingClientRect();
          return [Math.round(box.top), Math.round(box.height)];
        }),
      );
      if (performance.now() - start < 700) requestAnimationFrame(frame);
      else window.sidebarSampling = false;
    };
    requestAnimationFrame(frame);
  });
  await toggle.click();
  await page.waitForFunction(() => !window.sidebarSampling);
  return page.evaluate(() => window.sidebarSamples);
}

test('rows keep their height and position while the sidebar expands or collapses', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await setup(page);
  await login(page, false, false);
  await selectCase(page);
  const sidebar = page.locator('.desktop-sidebar');
  for (const name of [/^Contraer men/, /^Expandir men/]) {
    const samples = await sampleRows(page, sidebar.getByRole('button', { name }));
    expect(samples.length).toBeGreaterThan(5);
    for (const sample of samples) expect(sample).toEqual(samples[0]);
  }
});
