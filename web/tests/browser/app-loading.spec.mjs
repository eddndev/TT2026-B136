import { test, expect } from '@playwright/test';

test('the login interface survives repeated reloads with browser caching enabled', async ({
  page,
}, testInfo) => {
  const failures = [];
  page.on('requestfailed', (request) => {
    if (request.resourceType() === 'script')
      failures.push({
        pathname: new URL(request.url()).pathname,
        error: request.failure()?.errorText,
      });
  });
  page.on('response', (response) => {
    if (response.request().resourceType() === 'script' && response.status() >= 400)
      failures.push({ pathname: new URL(response.url()).pathname, status: response.status() });
  });
  try {
    await page.goto('/');
    await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
    for (let reload = 0; reload < 3; reload += 1) {
      await page.reload();
      await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
    }
    expect(failures).toEqual([]);
  } finally {
    await testInfo.attach('script-loading-failures', {
      body: JSON.stringify(failures, null, 2),
      contentType: 'application/json',
    });
  }
});
