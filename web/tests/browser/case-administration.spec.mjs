import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration, overview, fillPenal, profile } from './case-administration-helpers.mjs';

test('staff uses its compact index and Client only basic case APIs', async ({ page }) => {
  const requests = await setup(page, 'client');
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  expect(requests.some((r) => /case-administrations|\/administration/.test(r.path))).toBe(false);
  await expect(page.getByText('Ficha penal pendiente')).toHaveCount(0);
});

test('complete penal creation uses one atomic route and retains initial stage separately', async ({
  page,
}) => {
  const requests = await setup(page);
  let created;
  await page.route('**/api/v1/penal-cases', async (route) => {
    const body = route.request().postDataJSON();
    created = administration({ ...caseRecord, ...body }, 1, body.profile);
    created.initial_stage = {
      case_id: caseRecord.id,
      stage_revision: 1,
      administration_revision: 1,
      stage: 'investigation',
      administration_digest: 'c'.repeat(64),
      recorded_at: '2026-09-15T00:00:00Z',
      recorded_by: { id: 'actor', email: 'owner@example.com' },
    };
    await route.fulfill({ json: created, status: 201 });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await fillPenal(page);
  await page.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Investigaci\u00f3n', { exact: true })).toBeVisible();
  expect(created.administration.profile).toEqual(profile);
  expect(requests.some((r) => r.path.endsWith('/cases') && r.method === 'POST')).toBe(false);
});

for (const revision of [0, 1])
  test(`pending profile revision ${revision} has basic edit and complete actions without invented stage`, async ({
    page,
  }) => {
    await setup(page);
    let current = administration(caseRecord, revision);
    await page.route('**/api/v1/case-administrations?*', (route) =>
      route.fulfill({ json: { cases: [overview(current)], has_more: false, next_after_id: null } }),
    );
    await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, async (route) => {
      if (route.request().method() === 'PUT') {
        const body = route.request().postDataJSON();
        expect(body.expected_revision).toBe(revision);
        current = administration({ ...caseRecord, ...body }, revision + 1, body.profile);
      }
      await route.fulfill({ json: current });
    });
    await login(page, false, false);
    await navigate(page, 'Expedientes');
    await page.getByRole('button', { name: /Defensa inicial/ }).click();
    await expect(page.getByText('Ficha penal pendiente', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Editar datos b\u00e1sicos', exact: true }).click();
    await expect(page.getByLabel('NUC', { exact: true })).toHaveCount(0);
    await page.getByRole('button', { name: 'Cancelar', exact: true }).click();
    await page.getByRole('button', { name: 'Completar ficha penal', exact: true }).click();
    await fillPenal(page);
    await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
    await expect(page.getByText('Etapa sin registrar', { exact: true })).toBeVisible();
    expect(current.initial_stage).toBe(null);
  });
