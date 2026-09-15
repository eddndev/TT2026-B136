import { test, expect } from '@playwright/test';
import {
  setupStages,
  openStages,
  chooseSupport,
  fillDate,
  initial,
  caseId,
  entry,
  document,
} from './stage-helpers.mjs';
const command = {
  expected_revision: 1,
  target: 'intermediate',
  accusation_declared_at: {
    precision: 'date',
    date: '2026-09-01',
    offset: '-06:00',
  },
  accusation: { document_id: document.id, version: 1, digest: document.digest },
};
test('denied support upload clears the same protected context as denied selection', async ({
  page,
}) => {
  await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await page.getByRole('button', { name: 'Cargar soporte' }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'soporte.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('file'),
  });
  await page.route(`**/api/v1/cases/${caseId}/documents/with-metadata`, (route) =>
    route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } }),
  );
  await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Expediente no disponible' })).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveCount(0);
});
test('success waits for visible history refresh without additional document reads', async ({
  page,
}) => {
  const { state, requests } = await setupStages(page);
  await openStages(page);
  let release,
    defer = false;
  await page.route(`**/api/v1/cases/${caseId}/stage/history?*`, async (route) => {
    if (defer) await new Promise((resolve) => (release = resolve));
    await route.fulfill({
      json: { entries: state.history, has_more: false, next_before_revision: null },
    });
  });
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await expect(page.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await chooseSupport(page);
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  defer = true;
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect.poll(() => !!release).toBe(true);
  await expect(page.locator('.stage-confirmation .primary')).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Ocultar historial de etapas' })).toBeDisabled();
  release();
  await expect(page.locator('.stage-form')).toHaveCount(0);
  await expect(page.locator('.stage-history')).toContainText('contrato.pdf');
  expect(requests.filter((r) => r.path.endsWith('/versions/1'))).toHaveLength(1);
});
test('closure during registration preserves draft and queries only administration', async ({
  page,
}) => {
  const { detail, state } = await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await chooseSupport(page);
  await page.getByLabel('Nota (opcional)').fill('Guardar mi borrador');
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await page.route(`**/api/v1/cases/${caseId}/stage/transitions`, async (route) => {
    detail.administration = {
      ...detail.administration,
      revision: 2,
      administrative_status: 'closed',
    };
    await route.fulfill({ status: 409, json: { error: { code: 'case_closed' } } });
  });
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.getByText(/Expediente cerrado administrativamente\. Puedes/)).toBeVisible();
  await expect(page.getByLabel('Nota (opcional)')).toHaveValue('Guardar mi borrador');
  await expect(page.getByRole('button', { name: 'Revisar registro' })).toBeDisabled();
  expect(state.requests.filter((r) => r.method === 'GET')).toHaveLength(1);
});
test('history uses exclusive revision cursor and retains initial original provenance', async ({
  page,
}) => {
  await setupStages(page, { current: entry(command) });
  const cursors = [];
  await page.route(`**/api/v1/cases/${caseId}/stage/history?*`, (route) => {
    const cursor = new URL(route.request().url()).searchParams.get('before_revision');
    cursors.push(cursor);
    return route.fulfill({
      json: cursor
        ? { entries: [initial], has_more: false, next_before_revision: null }
        : { entries: [entry(command)], has_more: true, next_before_revision: 2 },
    });
  });
  await openStages(page);
  await page.getByRole('button', { name: 'Ver historial de etapas' }).click();
  await page.getByRole('button', { name: 'Cargar etapas anteriores' }).click();
  await expect(page.locator('.stage-history .stage-entry')).toHaveCount(2);
  expect(cursors).toEqual([null, '2']);
  await expect(page.locator('.stage-history')).toContainText('original@example.com');
});
