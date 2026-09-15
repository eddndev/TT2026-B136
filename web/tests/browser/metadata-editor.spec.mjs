import { test, expect } from '@playwright/test';
import { metadataSetup } from './metadata-helpers.mjs';

test('metadata conflict keeps the draft and requires a reviewed explicit replacement', async ({
  page,
}) => {
  const { calls } = await metadataSetup(page, { conflict: true });
  await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n' });
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Mis cambios');
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n' }).click();
  await expect(modal.getByRole('alert')).toContainText('cambi');
  await expect(modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    'Mis cambios',
  );
  await modal.getByRole('button', { name: 'Consultar clasificaci\u00f3n actual' }).click();
  await expect(modal.getByText('Otro cambio', { exact: true })).toBeVisible();
  expect(calls.filter((call) => call.method === 'PUT')).toHaveLength(1);
  await modal.getByRole('button', { name: 'Guardar mis cambios' }).click();
  await expect(modal).not.toBeVisible();
  await expect(
    page
      .getByRole('region', { name: 'Clasificaci\u00f3n actual del documento' })
      .getByText('Mis cambios', { exact: true }),
  ).toBeVisible();
  expect(
    calls
      .filter((call) => call.method === 'PUT')
      .map((call) => JSON.parse(call.body).expected_metadata_revision),
  ).toEqual([1, 2]);
});

test('metadata history shows captured authors and empty revisions without changing the content version', async ({
  page,
}) => {
  const { state } = await metadataSetup(page);
  state.history.unshift({
    metadata_revision: 2,
    document_type: null,
    classification: null,
    tags: [],
    changed_by: { id: 'author', email: 'old@example.com' },
    changed_at: '2026-09-15T11:00:00Z',
    metadata_digest: 'c'.repeat(64),
  });
  await page.getByRole('button', { name: 'Ver historial de clasificaci\u00f3n' }).click();
  await expect(page.getByText('old@example.com', { exact: true })).toBeVisible();
  await page.getByText('Cambio 2', { exact: true }).click();
  await expect(page.getByText('Sin valores de clasificaci\u00f3n', { exact: true })).toBeVisible();
  await expect(page.getByText('Consultando versi\u00f3n actual: 1', { exact: true })).toBeVisible();
});

test('metadata history requests the returned exclusive cursor before appending older revisions', async ({
  page,
}) => {
  const { state, historical } = await metadataSetup(page);
  const cursors = [];
  await page.route('**/metadata/history?**', (route) => {
    const before = new URL(route.request().url()).searchParams.get('before_revision');
    cursors.push(before);
    return route.fulfill({
      json: {
        case_id: state.records[0].case_id,
        id: state.records[0].id,
        revisions: [historical({ ...state.current, metadata_revision: before ? 1 : 2 })],
        has_more: !before,
        next_before_revision: before ? null : 2,
      },
    });
  });
  await page.getByRole('button', { name: 'Ver historial de clasificaci\u00f3n' }).click();
  await page.getByRole('button', { name: 'Cargar cambios anteriores', exact: true }).click();
  const history = page.getByRole('region', { name: 'Historial de clasificaci\u00f3n' });
  await expect(history.locator('summary strong')).toHaveText(['Cambio 2', 'Cambio 1']);
  expect(cursors).toEqual([null, '2']);
  await expect(
    page.getByRole('button', { name: 'Cargar cambios anteriores', exact: true }),
  ).toHaveCount(0);
});
