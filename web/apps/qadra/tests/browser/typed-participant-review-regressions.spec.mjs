import { test, expect } from '@playwright/test';
import { typedSetup, fillTyped, typed, subject } from './typed-participant-helpers.mjs';
import { detail, openParticipant } from './participant-helpers.mjs';
test('changing represented identity clears its certificate while edits of the same identity retain it', async ({
  page,
}) => {
  await typedSetup(page);
  const dialog = await fillTyped(page, 'control_judge');
  const certificate = dialog.getByLabel('Certificado p\u00fablico PEM o DER', { exact: true });
  await certificate.setInputFiles('tests/fixtures/participant-public-certificate.pem');
  await dialog
    .getByLabel('Nombre de la persona', { exact: true })
    .fill('Cambio de valores de la misma identidad nueva');
  await expect(
    dialog.getByRole('button', { name: 'Quitar certificado', exact: true }),
  ).toBeVisible();
  await dialog.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  const picker = dialog.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: 'Consultar identidad: Persona tipificada', exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Quitar certificado', exact: true })).toHaveCount(
    0,
  );
});
test('a missing exact subject revision preserves an uncertain draft and permits explicit current consultation', async ({
  page,
}) => {
  await typedSetup(page, 'owner', [typed]);
  await page.route('**/subjects/*/review', async (route) =>
    route.fulfill({
      json: {
        case_id: subject.case_id,
        id: subject.id,
        ...route.request().postDataJSON(),
        directory_stamp: 'c'.repeat(64),
        candidates: [],
      },
    }),
  );
  let writes = 0;
  await page.route('**/subjects/*', async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    writes++;
    await route.fulfill({ status: 503, json: { error: { code: 'service_busy' } } });
  });
  await page.route('**/subjects/*/revisions/2', (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'subject_not_found' } } }),
  );
  await openParticipant(page, typed.display_name);
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Editar identidad', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Editar identidad del expediente', exact: true });
  await dialog
    .getByLabel('Nombre de la persona', { exact: true })
    .fill('Nombre conservado tras envio incierto');
  await dialog
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  await dialog
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill('Actualizacion sintetica');
  await dialog
    .getByRole('button', { name: 'Guardar revisi\u00f3n de identidad', exact: true })
    .click();
  const missing = page.waitForResponse((response) => response.url().endsWith('/revisions/2'));
  await dialog
    .getByRole('button', { name: 'Consultar revisi\u00f3n enviada de identidad', exact: true })
    .click();
  expect((await missing).status()).toBe(404);
  await expect(detail(page)).toHaveCount(1);
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    'Nombre conservado tras envio incierto',
  );
  await expect(dialog.getByRole('alert')).toContainText('no encontr');
  await dialog
    .getByRole('button', { name: 'Consultar identidad actual antes de decidir', exact: true })
    .click();
  await expect(
    dialog.getByRole('region', { name: 'Revisi\u00f3n de identidad consultada', exact: true }),
  ).toContainText('Persona tipificada');
  await expect(
    dialog.getByRole('button', { name: 'Guardar revisi\u00f3n de identidad', exact: true }),
  ).toBeDisabled();
  await dialog
    .getByRole('button', {
      name: 'Usar esta base y conservar el formulario de identidad',
      exact: true,
    })
    .click();
  await expect(dialog.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    'Nombre conservado tras envio incierto',
  );
  expect(writes).toBe(1);
});
