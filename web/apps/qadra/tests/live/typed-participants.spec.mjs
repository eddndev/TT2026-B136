import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
import { capture } from './case-administration-helpers.mjs';
import {
  accounts,
  personName,
  directory,
  detail,
  participants,
  uploadIdentity,
  chooseSupport,
  prepare,
  submit,
  sign,
  openTyped,
  publicEvidence,
} from './typed-participant-helpers.mjs';
test('typed identities and personally signed roles persist with exact bound history and public evidence', async ({
  page,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openCase(page, accounts.case);
  await participants(page);
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  let dialog = page.getByRole('dialog', { name: 'Agregar participante tipificado', exact: true });
  await dialog.getByLabel('Nombre de la persona', { exact: true }).fill(personName);
  await dialog.getByLabel('Estado de CURP', { exact: true }).selectOption('unknown');
  await dialog
    .getByLabel('Motivo de CURP', { exact: true })
    .fill('Fixture sintetica sin identificador personal');
  const document = await uploadIdentity(page);
  await dialog.getByLabel('Tipo de participante', { exact: true }).selectOption('defendant');
  await dialog
    .getByLabel('Estado de Situaci\u00f3n de libertad declarada', { exact: true })
    .selectOption('unknown');
  await dialog
    .getByLabel('Motivo de Situaci\u00f3n de libertad declarada', { exact: true })
    .fill('Dato no declarado en la fixture');
  await chooseSupport(page, 'Soporte del rol');
  const unsigned = await prepare(page, dialog);
  expect(unsigned.declaration).toBeNull();
  expect(unsigned.submission_digest).toMatch(/^[0-9a-f]{64}$/);
  const first = await submit(page, dialog);
  expect(first.profile.kind).toBe('defendant');
  expect(first.subject.revision).toBe(1);
  expect(first.role_support.version).toBe(1);
  expect(first.role_support.digest).toBe(document.digest);
  await page.getByRole('button', { name: 'Agregar participante', exact: true }).click();
  dialog = page.getByRole('dialog', { name: 'Agregar participante tipificado', exact: true });
  await dialog.getByRole('button', { name: 'Elegir identidad existente', exact: true }).click();
  const picker = dialog.getByRole('region', { name: 'Elegir identidad existente', exact: true });
  await picker
    .getByRole('button', { name: `Consultar identidad: ${personName}`, exact: true })
    .click();
  await picker.getByRole('button', { name: 'Usar esta identidad', exact: true }).click();
  await dialog.getByLabel('Tipo de participante', { exact: true }).selectOption('defense_counsel');
  await dialog.getByLabel('C\u00e9dula profesional', { exact: true }).fill('000123');
  await dialog
    .getByLabel('Emisor de C\u00e9dula profesional', { exact: true })
    .fill('Emisor sintetico');
  await dialog.getByLabel('Modalidad de defensa', { exact: true }).selectOption('private');
  await chooseSupport(page, 'Soporte del rol');
  await dialog
    .getByLabel('Certificado p\u00fablico PEM o DER', { exact: true })
    .setInputFiles(accounts.certificatePath);
  const prepared = await prepare(page, dialog);
  expect(prepared.declaration.policy).toBe('internal_demo_v1');
  expect(prepared.submission_digest).toBeNull();
  const materials = await sign(page, dialog, testInfo);
  await dialog.evaluate((element) => element.scrollTo(0, element.scrollHeight));
  await page.screenshot({ path: testInfo.outputPath('typed-signature-confirmation.png') });
  const signed = await submit(page, dialog);
  expect(signed.subject.id).toBe(first.subject.id);
  expect(signed.profile.license.number).toBe('000123');
  expect(signed.credential_origin.participant_revision).toBe(1);
  const evidence = await publicEvidence(page, testInfo, 'participant-public-evidence-before.json');
  expect(Buffer.from(evidence.declaration_base64, 'base64')).toEqual(materials.statement);
  expect(Buffer.from(evidence.signature_base64, 'base64')).toEqual(materials.signature);
  expect(evidence.statement_digest).toBe(prepared.declaration.digest);
  expect(typeof evidence.trust.crl_number).toBe('string');
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Editar identidad', exact: true }).click();
  const editor = page.getByRole('dialog', { name: 'Editar identidad del expediente', exact: true });
  await editor
    .getByLabel('Nombre de la persona', { exact: true })
    .fill('Nombre sintetico actualizado');
  await editor
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  await editor
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill('Correccion sintetica con el mismo soporte');
  const identityWrite = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/subjects/${first.subject.id}`) &&
      response.request().method() === 'PUT',
  );
  await editor
    .getByRole('button', { name: 'Guardar revisi\u00f3n de identidad', exact: true })
    .click();
  expect((await identityWrite).status()).toBe(200);
  await expect(editor).not.toBeVisible();
  await expect(detail(page)).toContainText('Identidad vinculada: revisi\u00f3n 1');
  await expect(
    detail(page).getByRole('region', { name: 'Identidad actual consultada', exact: true }),
  ).toContainText('Nombre sintetico actualizado');
  await detail(page).getByRole('button', { name: 'Archivar participante', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar archivo', exact: true }).click();
  await expect(detail(page)).toHaveCount(0);
  await directory(page).getByLabel('Estado del directorio', { exact: true }).selectOption('all');
  await directory(page).getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await openTyped(page, 'Defensor');
  await expect(detail(page)).toContainText('Archivado');
  const after = await publicEvidence(page, testInfo, 'participant-public-evidence-after.json');
  expect(after).toEqual(evidence);
  await detail(page).getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Historial del participante', exact: true }),
  ).toHaveAttribute('aria-busy', 'false');
  await capture(page, testInfo, 'typed-participant-desktop', detail(page));
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'typed-participant-mobile', detail(page));
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.setViewportSize({ width: 1280, height: 720 });
  const loggedOut = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      new URL(response.url()).pathname === '/api/v1/auth/logout',
  );
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  expect((await loggedOut).status()).toBe(204);
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.goto('/');
  await loginAs(page, accounts.owner, 1);
  await openCase(page, accounts.case);
  await participants(page);
  await directory(page).getByLabel('Estado del directorio', { exact: true }).selectOption('all');
  await directory(page).getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await openTyped(page, 'Defensor');
  expect(await publicEvidence(page, testInfo, 'participant-public-evidence-reloaded.json')).toEqual(
    evidence,
  );
  expect(errors).toEqual([]);
});
