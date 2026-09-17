import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  fillResolution,
} from './procedural-facts-helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';

function gate() {
  let release;
  const promise = new Promise((resolve) => {
    release = resolve;
  });
  return { promise, release };
}

test('late preparation cannot restore a resolution draft after leaving the case', async ({
  page,
}) => {
  const state = await setupFacts(page),
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
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  await factEditor(page).getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect.poll(() => started).toBe(true);
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  held.release();
  await expect(factEditor(page)).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Confirmar registro', exact: true })).toHaveCount(
    0,
  );
  expect(state.submissions).toHaveLength(0);
});

test('late detail cannot restore a private resolution after leaving its case', async ({ page }) => {
  const row = factRecord(),
    state = await setupFacts(page, { facts: [row] }),
    held = gate();
  let started = false;
  state.handle = async (route, call) => {
    if (call.method !== 'GET' || !call.path.endsWith(`/${row.id}`)) return;
    started = true;
    await held.promise;
    await route.fulfill({ json: row });
    return true;
  };
  await openFacts(page);
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await expect.poll(() => started).toBe(true);
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  held.release();
  await expect(factDetail(page)).toHaveCount(0);
  await expect(page.getByText(row.values.summary, { exact: true })).toHaveCount(0);
});

test('revocation removes protected details and does not preserve a stale notification collection', async ({
  page,
}) => {
  const row = factRecord(),
    state = await setupFacts(page, { facts: [row] });
  await openFacts(page);
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await expect(factDetail(page)).toContainText(row.values.summary);
  state.results.scheduling.denied = true;
  await page.getByRole('button', { name: 'Actualizar registros', exact: true }).click();
  await expect(factDetail(page)).toHaveCount(0);
  await expect(page.getByText(row.values.summary, { exact: true })).toHaveCount(0);
});
