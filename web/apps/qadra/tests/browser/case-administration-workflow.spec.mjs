import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration, profile, overview, fillPenal } from './case-administration-helpers.mjs';
import { openCase, saveAdministration } from '../case-administration-workflow.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

// Finish browser rendering while the server response remains explicitly withheld.
const render = (page) =>
  page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );

test('the administrative save workflow waits for its exact confirmed revision', async ({
  page,
}) => {
  await setup(page);
  const started = deferred(),
    release = deferred();
  await page.route(`**/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT') {
      started.resolve();
      await release.promise;
      return route.fulfill({ json: administration(caseRecord, 2, profile) });
    }
    return route.fulfill({ json: administration() });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('button', { name: 'Completar ficha penal', exact: true }).click();
  await fillPenal(page, caseRecord.title);
  let finished = false;
  const saving = saveAdministration(page, caseRecord, 'Guardar ficha penal', 2).then(() => {
    finished = true;
  });
  await started.promise;
  await expect(
    page.getByRole('heading', { name: 'Etapa sin registrar', exact: true }),
  ).toBeVisible();
  await render(page);
  try {
    expect(finished).toBe(false);
  } finally {
    release.resolve();
    await saving;
  }
  await expect(page.locator('.case-editor')).toHaveCount(0);
  await expect(page.locator('.case-summary')).toContainText('Revisi\u00f3n 2');
});

test('opening a case applies filters only after the initial index settles', async ({ page }) => {
  await setup(page);
  const started = deferred(),
    release = deferred();
  let initialPending = false,
    overlap = false,
    filtered = 0;
  await page.route('**/api/v1/case-administrations?*', async (route) => {
    const status = new URL(route.request().url()).searchParams.get('status');
    if (status === 'active') {
      initialPending = true;
      started.resolve();
      await release.promise;
      initialPending = false;
    } else {
      filtered++;
      overlap ||= initialPending;
      if (initialPending)
        return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
    }
    return route.fulfill({
      json: { cases: [overview(administration())], has_more: false, next_after_id: null },
    });
  });
  await login(page, false, false);
  const opening = openCase(page, caseRecord);
  await started.promise;
  await expect(
    page.getByRole('combobox', { name: 'Estado administrativo', exact: true }),
  ).toHaveValue('all');
  await render(page);
  try {
    expect(filtered).toBe(0);
  } finally {
    release.resolve();
  }
  await opening;
  expect(overlap).toBe(false);
  expect(filtered).toBe(1);
});
