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
