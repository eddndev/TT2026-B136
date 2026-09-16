import { test, expect } from '@playwright/test';
import {
  setupResults,
  openResults,
  resultPanel,
  resultEditor,
  resultDetail,
  fillResult,
} from './hearing-result-helpers.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';

function gate() {
  let release;
  const promise = new Promise((resolve) => (release = resolve));
  return { promise, release };
}

test('late preparation cannot restore a draft after leaving the case', async ({ page }) => {
  const state = await setupResults(page),
    held = gate();
  let started = false;
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/prepare')) return;
    started = true;
    const prepared = state.prepare(call.body);
    await held.promise;
    await route.fulfill({ json: prepared });
    return true;
  };
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
  await fillResult(page);
  await resultEditor(page).getByRole('button', { name: 'Revisar resultado', exact: true }).click();
  await expect.poll(() => started).toBe(true);
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  held.release();
  await expect(resultEditor(page)).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Confirmar resultado', exact: true })).toHaveCount(
    0,
  );
  expect(state.submissions).toHaveLength(0);
});

test('late exact result detail cannot restore private data after case navigation', async ({
  page,
}) => {
  const record = resultRecord(),
    state = await setupResults(page, { results: [record] }),
    held = gate();
  let started = false;
  state.handle = async (route, call) => {
    if (call.method !== 'GET' || !call.path.endsWith(`/${record.id}`)) return;
    started = true;
    await held.promise;
    await route.fulfill({ json: record });
    return true;
  };
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${record.id}`, exact: true })
    .click();
  await expect.poll(() => started).toBe(true);
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  held.release();
  await expect(resultDetail(page)).toHaveCount(0);
  await expect(page.getByText(record.values.summary, { exact: true })).toHaveCount(0);
});

test('result panel loads only after explicit opening and does not fan out to other hearings', async ({
  page,
}) => {
  const state = await setupResults(page);
  const { openHearings } = await import('./hearing-helpers.mjs');
  await openHearings(page);
  await page.getByRole('button', { name: /^Consultar audiencia / }).click();
  expect(state.calls).toHaveLength(0);
  await page.getByRole('button', { name: 'Ver sesiones y resultados', exact: true }).click();
  await expect(resultPanel(page)).toBeVisible();
  await expect.poll(() => state.calls.length).toBe(1);
  expect(state.calls[0].method).toBe('GET');
  expect(state.calls[0].path).toMatch(/\/hearings\/[^/]+\/results$/);
});
