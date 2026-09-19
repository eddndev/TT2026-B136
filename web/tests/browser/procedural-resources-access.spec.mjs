import { test, expect } from '@playwright/test';
import {
  setupProceduralResources,
  openResources,
  resourceEditor,
  resourceDetail,
  browserResource,
} from './procedural-resources-helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import { login, navigate } from './helpers.mjs';

const mutationNames = [
  'Registrar recurso',
  'Registrar acto',
  'Corregir recurso',
  'Corregir este acto',
  'Archivar recurso',
  'Reactivar recurso',
];

async function openRecord(page, row) {
  await page
    .getByRole('button', {
      name: `Consultar recurso ${row.values.title}`,
      exact: true,
    })
    .click();
  await expect(resourceDetail(page)).toContainText(row.values.grounds);
}

for (const role of ['owner', 'litigator']) {
  test(`${role} can open management without changing a resource on consultation`, async ({
    page,
  }) => {
    const row = browserResource();
    const state = await setupProceduralResources(page, { role, resources: [row] });
    await openResources(page);
    await expect(
      page.getByRole('button', { name: 'Registrar recurso', exact: true }),
    ).toBeEnabled();
    await openRecord(page, row);
    for (const name of ['Registrar acto', 'Corregir recurso', 'Archivar recurso']) {
      await expect(resourceDetail(page).getByRole('button', { name, exact: true })).toBeEnabled();
    }
    expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
    expect(state.submissions).toHaveLength(0);
  });
}

test('assigned paralegal reads exact resource history without management controls', async ({
  page,
}) => {
  const row = browserResource();
  const state = await setupProceduralResources(page, { role: 'paralegal', resources: [row] });
  await openResources(page);
  await openRecord(page, row);
  for (const name of mutationNames) {
    await expect(page.getByRole('button', { name, exact: true })).toHaveCount(0);
  }
  await resourceDetail(page)
    .getByRole('button', { name: 'Ver historial de recurso', exact: true })
    .click();
  await page.getByRole('button', { name: 'Consultar recurso revision 1', exact: true }).click();
  await expect(resourceDetail(page)).toContainText('Consultada exactamente');
  await expect(resourceDetail(page)).toContainText(row.values.grounds);
  await expect(resourceEditor(page)).toHaveCount(0);
  expect(state.calls.some((call) => call.path.endsWith('/revisions/1'))).toBe(true);
  expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
  expect(state.submissions).toHaveLength(0);
});

test('client has no resources navigation or API access through a protected hash', async ({
  page,
}) => {
  const state = await setupProceduralResources(page, {
    role: 'client',
    resources: [browserResource()],
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Recursos', exact: true })).toHaveCount(0);
  await page.evaluate(async () => {
    location.hash = 'resources';
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  });
  await expect(page.getByRole('heading', { name: 'Recursos procesales', exact: true })).toHaveCount(
    0,
  );
  await expect(resourceDetail(page)).toHaveCount(0);
  await expect(resourceEditor(page)).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});

test('revocation clears private resource rows and detail on refresh', async ({ page }) => {
  const row = browserResource();
  row.values.title = 'Recurso de acceso revocado';
  row.values.grounds = 'Motivos reservados del recurso';
  const state = await setupProceduralResources(page, { role: 'litigator', resources: [row] });
  await openResources(page);
  await openRecord(page, row);
  state.facts.results.scheduling.denied = true;
  const denied = page.waitForResponse(
    (response) => response.url().includes('/procedural-resources') && response.status() === 403,
  );
  await page.getByRole('button', { name: 'Actualizar recursos', exact: true }).click();
  await denied;
  await expect(resourceDetail(page)).toHaveCount(0);
  await expect(resourceEditor(page)).toHaveCount(0);
  await expect(page.getByText(row.values.grounds, { exact: true })).toHaveCount(0);
  await expect(
    page.getByRole('button', {
      name: `Consultar recurso ${row.values.title}`,
      exact: true,
    }),
  ).toHaveCount(0);
  expect(state.submissions).toHaveLength(0);
});

test('closed case keeps exact resource history without available mutations', async ({ page }) => {
  const row = browserResource();
  const state = await setupProceduralResources(page, { closed: true, resources: [row] });
  await openResources(page);
  await expect(page.getByRole('button', { name: 'Registrar recurso', exact: true })).toBeDisabled();
  await openRecord(page, row);
  for (const name of mutationNames.filter((name) => name !== 'Registrar recurso')) {
    await expect(page.getByRole('button', { name, exact: true })).toHaveCount(0);
  }
  await resourceDetail(page)
    .getByRole('button', { name: 'Ver historial de recurso', exact: true })
    .click();
  await page.getByRole('button', { name: 'Consultar recurso revision 1', exact: true }).click();
  await expect(resourceDetail(page)).toContainText('Consultada exactamente');
  await expect(resourceDetail(page)).toContainText(row.values.grounds);
  expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
  expect(state.submissions).toHaveLength(0);
});

test('resource conflict keeps the draft until the operator explicitly accepts the current base', async ({
  page,
}) => {
  const row = browserResource();
  const state = await setupProceduralResources(page, { resources: [row] });
  await openResources(page);
  await openRecord(page, row);
  await resourceDetail(page).getByRole('button', { name: 'Corregir recurso', exact: true }).click();
  const editor = resourceEditor(page);
  await editor.getByLabel('Titulo organizativo', { exact: true }).fill('Mi borrador de recurso');
  await editor.getByLabel('Motivo', { exact: true }).fill('Precision declarada');
  const concurrent = resourceCommandFixture('correct');
  concurrent.operation_id = 'c0000000-0000-4000-8000-000000000002';
  concurrent.resource_id = row.id;
  concurrent.change.expected_revision = row.revision;
  concurrent.change.values = {
    ...structuredClone(row.values),
    title: 'Cambio concurrente del recurso',
  };
  state.commit(state.prepare(concurrent));
  await editor.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(
    editor.getByRole('button', { name: 'Comparar con registro actual', exact: true }),
  ).toBeVisible();
  await expect(editor.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    'Mi borrador de recurso',
  );
  await expect(editor.getByRole('button', { name: 'Preparar registro', exact: true })).toHaveCount(
    0,
  );
  await expect(editor.getByRole('button', { name: 'Confirmar registro', exact: true })).toHaveCount(
    0,
  );
  expect(state.submissions).toHaveLength(0);
  await editor.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(editor).toContainText('Cambio concurrente del recurso');
  await expect(editor.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    'Mi borrador de recurso',
  );
  await expect(editor.getByRole('button', { name: 'Preparar registro', exact: true })).toHaveCount(
    0,
  );
  expect(state.submissions).toHaveLength(0);
  await editor
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await editor.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(editor).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.values.title).toBe('Mi borrador de recurso');
  expect(state.submissions[0].change.reason).toBe('Precision declarada');
});
