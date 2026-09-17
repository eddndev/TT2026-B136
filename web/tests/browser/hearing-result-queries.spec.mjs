import { test, expect } from '@playwright/test';
import { setupResults, openResults, resultPanel, resultDetail } from './hearing-result-helpers.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';

test('root pagination keeps applied status and summaries do not expose private narrative', async ({
  page,
}) => {
  const first = resultRecord(),
    second = {
      ...structuredClone(first),
      id: '80000000-0000-4000-8000-000000000008',
      status: 'withdrawn',
    };
  const state = await setupResults(page, { results: [first, second] });
  state.pageSize = 1;
  await openResults(page);
  const list = page.getByRole('region', { name: 'Registros de sesiones y actos', exact: true });
  await expect(list).not.toContainText(first.values.summary);
  await resultPanel(page)
    .getByRole('combobox', { name: 'Estado del registro', exact: true })
    .selectOption('withdrawn');
  await resultPanel(page)
    .getByRole('button', { name: 'Siguientes resultados', exact: true })
    .click();
  await expect(
    list.getByRole('button', { name: `Consultar resultado ${second.id}`, exact: true }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain('status=all');
  expect(state.calls.at(-1).search).toContain(`after_id=${first.id}`);
  await resultPanel(page)
    .getByRole('button', { name: 'Aplicar estado del resultado', exact: true })
    .click();
  await expect(
    list.getByRole('button', { name: `Consultar resultado ${second.id}`, exact: true }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain('status=withdrawn');
  expect(state.calls.at(-1).search).not.toContain('after_id');
});

test('light history pages descending metadata and reads exact content only on selection', async ({
  page,
}) => {
  const first = resultRecord(),
    second = {
      ...structuredClone(first),
      revision: 2,
      reason: 'Texto corregido',
      values: { ...first.values, summary: 'Relato posterior' },
    };
  const state = await setupResults(page, { results: [first, second] });
  state.historySize = 1;
  await openResults(page);
  await resultPanel(page)
    .getByRole('button', { name: `Consultar resultado ${first.id}`, exact: true })
    .click();
  await resultDetail(page)
    .getByRole('button', { name: 'Ver historial del registro', exact: true })
    .click();
  const history = page.getByRole('region', { name: 'Historial del resultado', exact: true });
  await expect(history).not.toContainText('Relato posterior');
  expect(state.calls.filter((call) => call.path.includes('/revisions/'))).toHaveLength(0);
  await history
    .getByRole('button', { name: 'Cargar cambios anteriores del resultado', exact: true })
    .click();
  expect(state.calls.at(-1).search).toContain('before_revision=2');
  expect(state.calls.at(-1).search).toContain('limit=10');
  await history
    .getByRole('button', { name: 'Consultar resultado revisi\u00f3n 1', exact: true })
    .click();
  await expect(resultDetail(page)).toContainText(first.values.summary);
  expect(state.calls.at(-1).path).toContain('/revisions/1');
});
