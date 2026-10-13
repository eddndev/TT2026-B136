import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord, document } from './helpers.mjs';
import { administration, profile } from './case-administration-helpers.mjs';

async function summary(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}

test('profile conflict retains draft and explicit replacement preserves independent status and stage', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord, 1, profile),
    writes = [];
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT') {
      const body = route.request().postDataJSON();
      writes.push(body);
      if (writes.length === 1) {
        current = administration({ ...caseRecord, title: 'Cambio concurrente' }, 2, profile);
        return route.fulfill({ status: 409, json: { error: { code: 'case_revision_conflict' } } });
      }
      current = administration({ ...caseRecord, ...body }, 3, body.profile);
    }
    await route.fulfill({ json: current });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await page.getByLabel('T\u00edtulo del expediente', { exact: true }).fill('Mi borrador');
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(page.getByLabel('T\u00edtulo del expediente', { exact: true })).toHaveValue(
    'Mi borrador',
  );
  await page.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Valores actuales guardados' })).toContainText(
    'Cambio concurrente',
  );
  expect(writes.length).toBe(1);
  await page.getByRole('button', { name: 'Guardar mis cambios', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Mi borrador', exact: true })).toBeVisible();
  expect(writes[1].expected_revision).toBe(2);
  expect(Object.keys(writes[1]).sort()).toEqual([
    'expected_revision',
    'profile',
    'reference',
    'title',
  ]);
});

test('status conflict refresh retains only intended status and skips identical status', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord),
    writes = [];
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: current }),
  );
  await page.route('**/administrative-status', async (route) => {
    writes.push(route.request().postDataJSON());
    current = administration({ ...caseRecord, title: 'Titulo concurrente' }, 2, null, 'closed');
    await route.fulfill({ status: 409, json: { error: { code: 'case_revision_conflict' } } });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar cierre administrativo', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(
    dialog.getByText('El expediente ya tiene el estado solicitado.', { exact: false }),
  ).toBeVisible();
  expect(writes).toEqual([{ expected_revision: 1, administrative_status: 'closed' }]);
  await expect(
    dialog.getByRole('button', { name: 'Confirmar cierre administrativo', exact: true }),
  ).toBeDisabled();
});

test('closed document write refreshes case and preserves selected upload without automatic retry', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord),
    writes = 0;
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: current }),
  );
  await page.route('**/documents/with-metadata', async (route) => {
    writes++;
    current = administration(caseRecord, 2, null, 'closed');
    await route.fulfill({ status: 409, json: { error: { code: 'case_closed' } } });
  });
  await login(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'conservado.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('x'),
  });
  await dialog.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Cargar documento', exact: true }),
  ).toBeDisabled();
  await expect(dialog.getByLabel('Nombre del documento')).toHaveValue('conservado.pdf');
  expect(writes).toBe(1);
  await dialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Subir documento', exact: true })).toBeDisabled();
});

test('closed case retains version reading but disables every document mutation', async ({
  page,
}) => {
  await setup(page);
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: administration(caseRecord, 2, profile, 'closed') }),
  );
  await login(page);
  await page.getByRole('button', { name: `Abrir ${document.name}`, exact: true }).click();
  for (const name of [
    'Subir documento',
    'Agregar versi\u00f3n',
    'Editar clasificaci\u00f3n',
    'Sellar documento',
  ])
    await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
});

test('closed participant mutation retains draft and disables creation editing and directory status', async ({
  page,
}) => {
  const { participantSetup, openParticipant } = await import('./participant-helpers.mjs');
  await participantSetup(page);
  let current = administration(caseRecord),
    writes = 0;
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: current }),
  );
  await page.route('**/participants/*', async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    writes++;
    current = administration(caseRecord, 2, null, 'closed');
    return route.fulfill({ status: 409, json: { error: { code: 'case_closed' } } });
  });
  await openParticipant(page);
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Nombre del participante').fill('Borrador conservado');
  await dialog.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Guardar participante', exact: true }),
  ).toBeDisabled();
  await expect(dialog.getByLabel('Nombre del participante')).toHaveValue('Borrador conservado');
  expect(writes).toBe(1);
  await dialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
  for (const name of ['Agregar participante', 'Editar participante', 'Archivar participante'])
    await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Ver historial de cambios', exact: true }),
  ).toBeEnabled();
});

test('explicit state refresh after external reopening keeps upload draft and never resubmits', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord),
    writes = 0;
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: current }),
  );
  await page.route('**/documents/with-metadata', (route) => {
    writes++;
    current = administration(caseRecord, 2, null, 'closed');
    return route.fulfill({ status: 409, json: { error: { code: 'case_closed' } } });
  });
  await login(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog
    .getByLabel('Archivo', { exact: true })
    .setInputFiles({ name: 'draft.txt', mimeType: 'text/plain', buffer: Buffer.from('preserved') });
  await dialog.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Cargar documento', exact: true }),
  ).toBeDisabled();
  current = administration(caseRecord, 3, null, 'active');
  await dialog
    .getByRole('button', { name: 'Consultar estado del expediente', exact: true })
    .click();
  await expect(dialog.getByRole('button', { name: 'Cargar documento', exact: true })).toBeEnabled();
  await expect(dialog.getByLabel('Nombre del documento')).toHaveValue('draft.txt');
  expect(
    await dialog.getByLabel('Archivo', { exact: true }).evaluate((input) => input.files[0].name),
  ).toBe('draft.txt');
  expect(writes).toBe(1);
});
