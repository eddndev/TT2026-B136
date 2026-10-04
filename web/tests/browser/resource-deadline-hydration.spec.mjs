import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import {
  setupResourceDeadline,
  fillDeadline,
  editor,
  prepareResourceDeadline,
} from './resource-deadline-helpers.mjs';

test('contextual deadline harness waits for delayed application focus', async ({ page }) => {
  const state = await setupResourceDeadline(page);
  let releaseApp;
  let appRequested;
  const gate = new Promise((resolve) => {
    releaseApp = resolve;
  });
  const requested = new Promise((resolve) => {
    appRequested = resolve;
  });
  await page.route('**/src/components/App.svelte', async (route) => {
    appRequested();
    await gate;
    await route.continue();
  });
  let releaseTimer;
  try {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await requested;
    // Load all harness dependencies while application hydration stays blocked.
    await page.evaluate(async () => {
      window.deadlineHarness = await import('/tests/browser/resource-deadline-harness.mjs');
    });
    const readiness = [];
    await page.route('**/api/v1/auth/mfa/**', async (route) => {
      readiness.push(
        await page.evaluate(() => ({
          hydrated: !document
            .querySelector('astro-island[component-url*="/App.svelte"]')
            .hasAttribute('ssr'),
          focusedEmail: document.activeElement?.getAttribute('type') === 'email',
        })),
      );
      await route.fallback();
    });
    const mounting = page.evaluate(
      async ({ resource, caseId }) => {
        await window.deadlineHarness.mountResourceDeadline(resource, caseId);
      },
      { resource: state.activities.resource, caseId },
    );
    // Controlled slow hydration exercises ordering without changing test timeouts.
    releaseTimer = setTimeout(releaseApp, 1000);
    await mounting;
    expect(readiness).toEqual([{ hydrated: true, focusedEmail: true }]);
    await fillDeadline(page, 'Plazo contextual');
    await expect(
      editor(page).getByRole('textbox', {
        name: 'T\u00edtulo del plazo',
        exact: true,
      }),
    ).toHaveValue('Plazo contextual');
    await expect(
      page.getByRole('textbox', {
        name: 'Correo electr\u00f3nico',
        exact: true,
      }),
    ).toHaveValue('');
    await editor(page)
      .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
      .selectOption('follow');
    await prepareResourceDeadline(page);
    expect(state.calls.filter((call) => call.path.endsWith('/prepare'))).toHaveLength(1);
    expect(state.submissions).toHaveLength(0);
  } finally {
    clearTimeout(releaseTimer);
    releaseApp();
  }
});
