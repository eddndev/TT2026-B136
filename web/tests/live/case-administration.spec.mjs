import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  createPenal,
  fillProfile,
  openCase,
  capture,
  archive,
  permissions,
} from './case-administration-helpers.mjs';

test('penal profiles preserve legacy, explicit conflicts and evidence across closure with real permissions', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  const created = await createPenal(page, 'Browser penal profile', 'PENAL-NEW-001');
  expect(created.administration.revision).toBe(1);
  expect(created.administration.profile.offenses).toEqual([
    'Descripci\u00f3n manual, sin cat\u00e1logo',
  ]);
  expect(created.administration.profile.general_information).toBe('Primera linea\nSegunda linea');
  expect(created.initial_stage.administration_digest).toBe(created.administration.values_digest);
  await expect(
    page.getByRole('heading', { name: 'Investigaci\u00f3n', exact: true }),
  ).toBeVisible();
  await capture(page, testInfo, 'penal-summary-desktop', page.locator('.case-stage'));
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'penal-summary-mobile', page.locator('.case-stage'));
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.setViewportSize({ width: 1280, height: 720 });

  await openCase(page, accounts.legacyCase);
  await expect(
    page.getByText(
      'Datos originales; a\u00fan no hay una revisi\u00f3n administrativa registrada.',
      { exact: true },
    ),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Ver historial administrativo', exact: true }).click();
  await expect(
    page.getByText('Todav\u00eda no hay revisiones administrativas registradas.', { exact: true }),
  ).toBeVisible();
  await capture(page, testInfo, 'penal-legacy-pending');
  await page.getByRole('button', { name: 'Editar datos b\u00e1sicos', exact: true }).click();
  await page.getByLabel('Referencia interna', { exact: true }).fill('LEGACY-REVIEWED');
  await page.getByRole('button', { name: 'Guardar datos b\u00e1sicos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Ficha penal pendiente', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Completar ficha penal', exact: true }).click();
  await fillProfile(page, 'PENAL-LEGACY-001');
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Etapa sin registrar', exact: true }),
  ).toBeVisible();

  await openCase(page);
  await expect(
    page.getByRole('heading', { name: 'Ficha penal pendiente', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Completar ficha penal', exact: true }).click();
  await fillProfile(page, 'PENAL-BASIC-001');
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Ficha penal completa', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Etapa sin registrar', exact: true }),
  ).toBeVisible();

  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await context.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await loginAs(other, accounts.owner, 1);
    await openCase(other);
    await other.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
    await other
      .getByLabel('Informaci\u00f3n general (opcional)', { exact: true })
      .fill('Borrador revisado');
    await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
    await page
      .getByLabel('Informaci\u00f3n general (opcional)', { exact: true })
      .fill('Nota concurrente');
    await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
    await expect(
      page.locator('.case-summary').getByText('Nota concurrente', { exact: true }),
    ).toBeVisible();
    await other.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
    await expect(other.locator('.case-editor').getByRole('alert')).toContainText('cambi');
    await expect(
      other.getByLabel('Informaci\u00f3n general (opcional)', { exact: true }),
    ).toHaveValue('Borrador revisado');
    await other.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
    await expect(other.locator('.case-comparison')).toContainText('Nota concurrente');
    await capture(
      other,
      testInfo,
      'penal-edit-conflict',
      other.getByRole('button', { name: 'Guardar mis cambios', exact: true }),
    );
    await other.getByRole('button', { name: 'Guardar mis cambios', exact: true }).click();
    await expect(
      other.locator('.case-summary').getByText('Borrador revisado', { exact: true }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Actualizar resumen', exact: true }).click();
    await expect(
      page.locator('.case-summary').getByText('Borrador revisado', { exact: true }),
    ).toBeVisible();

    await page.getByRole('link', { name: 'Documentos', exact: true }).click();
    await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
    await page.getByLabel('Archivo', { exact: true }).setInputFiles({
      name: 'case-evidence.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('Immutable case evidence.\n'),
    });
    await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
    await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
    await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
    await expect(
      page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
    ).toBeEnabled();
    const originalZip = await archive(page, testInfo, 'before-case-closed.zip');
    await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
    await page.getByLabel('Archivo de la nueva versi\u00f3n', { exact: true }).setInputFiles({
      name: 'case-pending.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('Second immutable version.\n'),
    });
    await page.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'case-pending.txt', exact: true }),
    ).toBeVisible();
    await page.getByRole('link', { name: 'Participantes', exact: true }).click();
    await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
    await page
      .getByLabel('Nombre del participante', { exact: true })
      .fill('Persona del expediente');
    await page.getByLabel('Rol en el expediente', { exact: true }).fill('Defensa');
    await page.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Persona del expediente', exact: true }),
    ).toBeVisible();

    await other.getByRole('link', { name: 'Documentos', exact: true }).click();
    await other.getByRole('button', { name: 'Subir documento', exact: true }).click();
    const upload = other.getByRole('dialog', { name: 'Subir documento', exact: true });
    await upload.getByLabel('Archivo', { exact: true }).setInputFiles({
      name: 'preserved-draft.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('Draft'),
    });
    await page.getByRole('link', { name: 'Resumen', exact: true }).click();
    await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
    await capture(page, testInfo, 'penal-close-confirmation');
    await page
      .getByRole('button', { name: 'Confirmar cierre administrativo', exact: true })
      .click();
    await expect(
      page.getByRole('button', { name: 'Reactivar expediente', exact: true }),
    ).toBeVisible();
    await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
    await expect(upload.getByRole('alert')).toContainText('cerrado');
    await expect(
      upload.getByRole('button', { name: 'Cargar documento', exact: true }),
    ).toBeDisabled();
    await expect(upload.getByLabel('Nombre del documento')).toHaveValue('preserved-draft.txt');
    await other.screenshot({ path: testInfo.outputPath('penal-closed-upload-draft.png') });
    await upload.getByRole('button', { name: 'Cancelar', exact: true }).click();

    await page.getByRole('button', { name: 'Ver historial administrativo', exact: true }).click();
    await expect(page.locator('.case-history details')).toHaveCount(5);
    await page.locator('.case-history summary').first().click();
    await capture(page, testInfo, 'penal-history-desktop', page.locator('.case-history'));
    await page.setViewportSize({ width: 390, height: 844 });
    await capture(page, testInfo, 'penal-history-mobile', page.locator('.case-history'));
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole('link', { name: 'Documentos', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Subir documento', exact: true })).toBeDisabled();
    await page.getByRole('button', { name: 'Abrir case-pending.txt', exact: true }).click();
    for (const name of ['Agregar versi\u00f3n', 'Editar clasificaci\u00f3n', 'Sellar documento'])
      await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
    await page.getByRole('button', { name: /Versi\u00f3n 1.*case-evidence.txt/ }).click();
    await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
    await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
    expect(await archive(page, testInfo, 'after-case-closed.zip')).toEqual(originalZip);
    await page.getByRole('link', { name: 'Participantes', exact: true }).click();
    await page.getByRole('button', { name: 'Abrir Persona del expediente', exact: true }).click();
    for (const name of ['Agregar participante', 'Editar participante', 'Archivar participante'])
      await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
    await page.getByRole('link', { name: 'Resumen', exact: true }).click();
    await page.getByRole('button', { name: 'Reactivar expediente', exact: true }).click();
    await page.getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true }).click();
    await expect(
      page.getByRole('button', { name: 'Editar ficha penal', exact: true }),
    ).toBeEnabled();
  } finally {
    await context.close();
  }

  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await loginAs(page, accounts.owner, 2);
  await openCase(page);
  await expect(
    page.locator('.case-summary').getByText('Borrador revisado', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Etapa sin registrar', exact: true }),
  ).toBeVisible();
  await permissions(browser, testInfo, errors);
  expect(errors).toEqual([]);
});
