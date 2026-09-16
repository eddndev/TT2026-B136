import { test, expect } from '@playwright/test';
import { setupFacts, openFacts, factList, factDetail } from './procedural-facts-helpers.mjs';
import { factRecord, factPrepared, factCommand } from '../fixtures/procedural-facts.mjs';

test('root pagination preserves applied status and lists do not expand private narratives or child collections', async ({
  page,
}) => {
  const template = factRecord();
  const rows = Array.from({ length: 21 }, (_, index) => ({
    ...structuredClone(template),
    id: `10000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`,
  }));
  const secondPage = rows.at(-1);
  secondPage.status = 'withdrawn';
  const state = await setupFacts(page, { facts: rows });
  await openFacts(page);
  await expect(factList(page)).not.toContainText(template.values.summary);
  expect(state.calls.filter((call) => call.family === 'notification')).toHaveLength(0);
  await page
    .getByRole('combobox', { name: 'Estado del registro', exact: true })
    .selectOption('withdrawn');
  await page.getByRole('button', { name: 'Siguientes registros', exact: true }).click();
  await expect(
    factList(page).getByRole('button', {
      name: `Consultar resoluci\u00f3n ${secondPage.id}`,
      exact: true,
    }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain('status=all');
  expect(state.calls.at(-1).search).toContain(`after_id=${rows[19].id}`);
  await page.getByRole('button', { name: 'Aplicar estado', exact: true }).click();
  await expect(
    factList(page).getByRole('button', {
      name: `Consultar resoluci\u00f3n ${secondPage.id}`,
      exact: true,
    }),
  ).toBeVisible();
  expect(state.calls.at(-1).search).toContain('status=withdrawn');
  expect(state.calls.at(-1).search).not.toContain('after_id');
});

test('light history pages metadata and only fetches exact values after explicit selection', async ({
  page,
}) => {
  const first = factRecord(),
    history = [first];
  for (let revision = 2; revision <= 11; revision++) {
    const command = factCommand('resolution', 'correct', revision - 1);
    command.change.values.summary = `Resumen reservado de revision ${revision}`;
    history.push(factRecord(factPrepared(command)));
  }
  const state = await setupFacts(page, { facts: history });
  await openFacts(page);
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${first.id}`, exact: true })
    .click();
  await factDetail(page)
    .getByRole('button', { name: 'Ver historial de resoluci\u00f3n', exact: true })
    .click();
  const panel = page.getByRole('region', { name: 'Historial de resoluci\u00f3n', exact: true });
  await expect(panel).not.toContainText(history.at(-1).values.summary);
  expect(state.calls.filter((call) => call.path.includes('/revisions/'))).toHaveLength(0);
  await panel.getByRole('button', { name: 'Cargar cambios anteriores', exact: true }).click();
  expect(state.calls.at(-1).search).toContain('before_revision=2');
  expect(new URLSearchParams(state.calls.at(-1).search).get('limit')).toBe('10');
  await panel
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await expect(factDetail(page)).toContainText(first.values.summary);
  expect(state.calls.at(-1).path).toContain('/revisions/1');
});
