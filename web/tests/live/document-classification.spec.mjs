import { createPenal } from './case-administration-helpers.mjs';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { test, expect } from '@playwright/test';
import { fixture, login, capture } from './helpers.mjs';
const title = 'Browser classification case';
const metadataCard = (page) =>
  page.getByRole('region', { name: 'Clasificaci\u00f3n actual del documento' });
const metadataDialog = (page) => page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n' });
async function openCase(page) {
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: new RegExp(title) }).click();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
}
async function edit(page, value) {
  await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
  const modal = metadataDialog(page);
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill(value);
  return modal;
}
async function seal(page) {
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
  await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
}
async function archive(page, testInfo, name) {
  const event = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const path = testInfo.outputPath(name);
  await (await event).saveAs(path);
  execFileSync('unzip', ['-t', path]);
  return readFileSync(path);
}

test('classification is atomic, concurrent, persistent and independent of content evidence', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await login(page, fixture.recoveryCodes[2]);
  await createPenal(page, title, 'CLASSIFICATION-001');
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
  const upload = page.getByRole('dialog', { name: 'Subir documento' });
  await upload.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'classified-v1.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Classified immutable content one.\n'),
  });
  await upload.getByLabel('Tipo de documento (opcional)', { exact: true }).fill('Escrito');
  await upload.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Civil');
  await upload.getByLabel('Nueva etiqueta', { exact: true }).fill('acci\u00f3n, prueba');
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(
    metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 1', { exact: true }),
  ).toBeVisible();
  await expect(metadataCard(page).getByText('acci\u00f3n, prueba', { exact: true })).toBeVisible();
  await seal(page);
  const firstArchive = await archive(page, testInfo, 'classified-original-v1.zip');
  let modal = await edit(page, 'Laboral');
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(
    metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 2', { exact: true }),
  ).toBeVisible();
  expect(await archive(page, testInfo, 'classified-after-edit-v1.zip')).toEqual(firstArchive);

  const otherContext = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await otherContext.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await login(other, fixture.recoveryCodes[3]);
    await openCase(other);
    await other.getByRole('button', { name: /Abrir classified-v1.txt/ }).click();
    await expect(
      metadataCard(other).getByText('Revisi\u00f3n de clasificaci\u00f3n: 2', { exact: true }),
    ).toBeVisible();
    const otherModal = await edit(other, 'Cambio revisado');
    modal = await edit(page, 'Cambio concurrente');
    await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
    await expect(
      metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 3', { exact: true }),
    ).toBeVisible();
    await otherModal
      .getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true })
      .click();
    await expect(otherModal.getByRole('alert')).toContainText('cambi');
    await expect(
      otherModal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }),
    ).toHaveValue('Cambio revisado');
    await otherModal.getByRole('button', { name: 'Consultar clasificaci\u00f3n actual' }).click();
    await expect(otherModal.getByText('Cambio concurrente', { exact: true })).toBeVisible();
    await other.screenshot({ path: testInfo.outputPath('classification-conflict-desktop.png') });
    await otherModal.getByRole('button', { name: 'Guardar mis cambios' }).scrollIntoViewIfNeeded();
    await other.screenshot({
      path: testInfo.outputPath('classification-conflict-confirmation-desktop.png'),
    });
    await otherModal.getByRole('button', { name: 'Guardar mis cambios' }).click();
    await expect(
      metadataCard(other).getByText('Revisi\u00f3n de clasificaci\u00f3n: 4', { exact: true }),
    ).toBeVisible();
  } finally {
    await otherContext.close();
  }
  await page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }).click();
  await expect(metadataCard(page).getByText('Cambio revisado', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
  await page.getByLabel('Archivo de la nueva versi\u00f3n', { exact: true }).setInputFiles({
    name: 'classified-v2.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Classified immutable content two.\n'),
  });
  await page.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
  await expect(page.getByText('Consultando versi\u00f3n actual: 2', { exact: true })).toBeVisible();
  await expect(
    metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 4', { exact: true }),
  ).toBeVisible();
  await seal(page);
  const secondArchive = await archive(page, testInfo, 'classified-original-v2.zip');
  await capture(page, testInfo, 'classification-current-desktop');
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'classification-current-mobile');
  await page.setViewportSize({ width: 1280, height: 720 });
  modal = await edit(page, '');
  await modal.getByLabel('Tipo de documento (opcional)', { exact: true }).fill('');
  await modal.getByRole('button', { name: 'Quitar etiqueta: acci\u00f3n, prueba' }).click();
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(
    metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 5', { exact: true }),
  ).toBeVisible();
  await expect(
    metadataCard(page).getByText('Sin valores de clasificaci\u00f3n', { exact: true }),
  ).toBeVisible();
  expect(await archive(page, testInfo, 'classified-cleared-v2.zip')).toEqual(secondArchive);
  await page.getByRole('button', { name: /Versi\u00f3n 1.*classified-v1.txt/ }).click();
  expect(await archive(page, testInfo, 'classified-cleared-v1.zip')).toEqual(firstArchive);
  await page.getByRole('button', { name: 'Ver historial de clasificaci\u00f3n' }).click();
  const history = page.getByRole('region', { name: 'Historial de clasificaci\u00f3n' });
  await expect(history.locator('.metadata-revision')).toHaveCount(5);
  await history.getByText('Cambio 1', { exact: true }).click();
  await expect(
    history.locator('details[open]').getByText('acci\u00f3n, prueba', { exact: true }),
  ).toBeVisible();
  await expect(
    history.locator('details[open]').getByText(fixture.email, { exact: true }),
  ).toBeVisible();
  await capture(page, testInfo, 'classification-history-desktop');
  await page.getByText('Filtros de clasificaci\u00f3n', { exact: true }).click();
  await page.getByLabel('Etiqueta exacta', { exact: true }).fill('acci\u00f3n, prueba');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await expect(page.getByText('No encontramos coincidencias', { exact: true })).toBeVisible();
  await page
    .locator('.metadata-filters')
    .getByRole('button', { name: 'Limpiar filtros', exact: true })
    .click();
  await expect(
    page.locator('.document-list').getByText('classified-v2.txt', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await login(page, fixture.recoveryCodes[4]);
  await openCase(page);
  await page.getByRole('button', { name: /Abrir classified-v2.txt/ }).click();
  await expect(
    metadataCard(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 5', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /Versi\u00f3n 1.*classified-v1.txt/ }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Ver historial de clasificaci\u00f3n' }).click();
  await expect(page.locator('.metadata-revision')).toHaveCount(5);
  expect(errors).toEqual([]);
});
