import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  directory,
  detail,
  openCase,
  openPerson,
  edit,
  capture,
  archive,
  exercisePermissions,
} from './participant-helpers.mjs';

test('participants persist with concurrent edits, precise archive, history and live case permissions', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openCase(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
  const upload = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await upload.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'participant-evidence.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Participants preserve document evidence.\n'),
  });
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
  const originalZip = await archive(page, testInfo, 'before-participants.zip');
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  let modal = page.getByRole('dialog', { name: 'Agregar participante', exact: true });
  await modal.getByLabel('Nombre del participante', { exact: true }).fill('Ana Mu\u00f1oz');
  await modal.getByLabel('Rol en el expediente', { exact: true }).fill('Defensa registrada');
  await modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true }).fill('Despacho');
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(detail(page).getByText('Revisi\u00f3n 1', { exact: true })).toBeVisible();
  modal = await edit(page);
  await modal
    .getByLabel('Situaci\u00f3n jur\u00eddica registrada (opcional)', { exact: true })
    .fill('Nota manual');
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(detail(page).getByText('Revisi\u00f3n 2', { exact: true })).toBeVisible();
  const otherContext = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await otherContext.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await loginAs(other, accounts.owner, 1);
    await openCase(other);
    await other.getByRole('link', { name: 'Participantes', exact: true }).click();
    await openPerson(other, 'Ana Mu\u00f1oz');
    const otherModal = await edit(other);
    await otherModal
      .getByLabel('Nombre del participante', { exact: true })
      .fill('Ana Mu\u00f1oz revisada');
    modal = await edit(page);
    await modal
      .getByLabel('Organizaci\u00f3n (opcional)', { exact: true })
      .fill('Oficina actualizada');
    await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await expect(detail(page).getByText('Revisi\u00f3n 3', { exact: true })).toBeVisible();
    await otherModal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await expect(otherModal.getByRole('alert')).toContainText('cambiaron');
    await expect(otherModal.getByLabel('Nombre del participante', { exact: true })).toHaveValue(
      'Ana Mu\u00f1oz revisada',
    );
    await otherModal.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
    await expect(otherModal.getByText('Oficina actualizada', { exact: true })).toBeVisible();
    await otherModal.evaluate((element) => {
      element.scrollTop = 0;
    });
    await other.screenshot({ path: testInfo.outputPath('participants-edit-conflict.png') });
    await otherModal
      .getByRole('button', { name: 'Guardar mis cambios', exact: true })
      .scrollIntoViewIfNeeded();
    await other.screenshot({ path: testInfo.outputPath('participants-edit-confirmation.png') });
    await otherModal.getByRole('button', { name: 'Guardar mis cambios', exact: true }).click();
    await expect(detail(other).getByText('Revisi\u00f3n 4', { exact: true })).toBeVisible();
    await directory(page).getByRole('button', { name: 'Actualizar', exact: true }).click();
    await openPerson(page, 'Ana Mu\u00f1oz revisada');
    await page.getByRole('button', { name: 'Archivar participante', exact: true }).click();
    const statusModal = page.getByRole('dialog', { name: 'Archivar participante', exact: true });
    const roleEdit = await edit(other);
    await roleEdit.getByLabel('Rol en el expediente', { exact: true }).fill('Rol actualizado');
    await roleEdit.getByRole('button', { name: 'Guardar participante', exact: true }).click();
    await expect(detail(other).getByText('Revisi\u00f3n 5', { exact: true })).toBeVisible();
    await statusModal.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
    await expect(statusModal.getByRole('alert')).toContainText('cambi');
    await statusModal
      .getByRole('button', { name: 'Consultar datos actuales', exact: true })
      .click();
    await expect(statusModal.getByText('Rol actualizado', { exact: true })).toBeVisible();
    await statusModal.evaluate((element) => {
      element.scrollTop = 0;
    });
    await page.screenshot({ path: testInfo.outputPath('participants-archive-conflict.png') });
    await statusModal
      .getByRole('button', { name: 'Confirmar archivo', exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({ path: testInfo.outputPath('participants-archive-confirmation.png') });
    await statusModal.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
    await expect(directory(page).locator('.participant-row')).toHaveCount(0);
  } finally {
    await otherContext.close();
  }
  await page.getByLabel('Estado del directorio', { exact: true }).selectOption('archived');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await openPerson(page, 'Ana Mu\u00f1oz revisada');
  await expect(detail(page).getByText('Rol actualizado', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(page.locator('.participant-revision')).toHaveCount(6);
  await page.getByText('Cambio 1', { exact: true }).click();
  await expect(
    page.locator('.participant-revision[open]').getByText('Ana Mu\u00f1oz', { exact: true }),
  ).toBeVisible();
  await capture(page, testInfo, 'participants-history-desktop');
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'participants-history-mobile');
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('button', { name: 'Reactivar participante', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar reactivaci\u00f3n', exact: true }).click();
  await page.getByRole('button', { name: 'Limpiar filtros', exact: true }).click();
  await openPerson(page, 'Ana Mu\u00f1oz revisada');
  await expect(detail(page).getByText('Revisi\u00f3n 7', { exact: true })).toBeVisible();
  await capture(page, testInfo, 'participants-current-desktop');
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'participants-current-mobile');
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page.getByRole('button', { name: /Abrir participant-evidence.txt/ }).click();
  expect(await archive(page, testInfo, 'after-participants.zip')).toEqual(originalZip);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await loginAs(page, accounts.owner, 2);
  await openCase(page);
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await openPerson(page, 'Ana Mu\u00f1oz revisada');
  await expect(detail(page).getByText('Revisi\u00f3n 7', { exact: true })).toBeVisible();
  await exercisePermissions(browser, testInfo, 'Ana Mu\u00f1oz revisada', errors);
  expect(errors).toEqual([]);
});
