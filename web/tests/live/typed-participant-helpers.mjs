import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fixture } from './helpers.mjs';
export const accounts = fixture.typedParticipants;
export const personName = 'Persona sintetica del expediente';
export const pdf = readFileSync(
  new URL('../../../crates/infrastructure/tests/fixtures/stage-support.pdf', import.meta.url),
);
export const directory = (page) =>
  page.getByRole('region', { name: 'Directorio del expediente', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Datos del participante', exact: true });
export async function participants(page) {
  await page.getByRole('link', { name: 'Participantes', exact: true }).click();
  await expect(directory(page)).toHaveAttribute('aria-busy', 'false');
}
export async function chooseSupport(page, label, name = 'participant-identity.pdf', version = 1) {
  const group = page.getByRole('group', { name: label, exact: true });
  await group.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  const picker = group.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: new RegExp(`${name} / versi`) }).click();
  await picker.getByRole('button', { name: new RegExp(`Versi\u00f3n ${version} /`) }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await group.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill('Pagina 1');
}
export async function uploadIdentity(page, name = 'participant-identity.pdf') {
  const group = page.getByRole('group', { name: 'Soporte de identidad', exact: true });
  await group.getByRole('button', { name: 'Cargar soporte', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await modal
    .getByLabel('Archivo', { exact: true })
    .setInputFiles({ name, mimeType: 'application/pdf', buffer: pdf });
  const pending = page.waitForResponse(
    (response) =>
      response.url().endsWith('/documents/with-metadata') && response.request().method() === 'POST',
  );
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(201);
  await expect(modal).not.toBeVisible();
  await group.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill('Pagina 1');
  return response.json();
}
export async function prepare(page, dialog, afterReview) {
  const review = page.waitForResponse((response) =>
    response.url().endsWith('/participants/proposals/review'),
  );
  await dialog
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  const reviewed = await review;
  expect(reviewed.status()).toBe(200);
  if (afterReview) await afterReview(await reviewed.json());
  await dialog
    .getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true })
    .fill('Identidad sintetica revisada mediante soporte');
  const preparation = page.waitForResponse((response) =>
    response.url().endsWith('/participants/proposals/prepare'),
  );
  await dialog.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  const response = await preparation;
  expect(response.status()).toBe(200);
  await expect(dialog).toHaveAttribute('aria-busy', 'false');
  return response.json();
}
export async function submit(page, dialog, status = 201) {
  const pending = page.waitForResponse((response) =>
    response.url().endsWith('/participants/proposals/commit'),
  );
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(status);
  await expect(dialog).not.toBeVisible();
  return response.json();
}
export async function sign(page, dialog, testInfo) {
  const pending = page.waitForEvent('download');
  await dialog
    .getByRole('button', { name: 'Descargar declaraci\u00f3n binaria', exact: true })
    .click();
  const statementPath = testInfo.outputPath('participant-declaration.bin');
  await (await pending).saveAs(statementPath);
  const statement = readFileSync(statementPath);
  expect(statement.length).toBe(218);
  expect(statement.subarray(0, 6).toString()).toBe('PCRED1');
  const keyPath = process.env.TT_LIVE_PARTICIPANT_PRIVATE_KEY;
  if (!keyPath) throw new Error('An isolated external participant signing key is required.');
  const signaturePath = testInfo.outputPath('participant-declaration.sig');
  execFileSync('openssl', [
    'dgst',
    '-sha256',
    '-sign',
    keyPath,
    '-out',
    signaturePath,
    statementPath,
  ]);
  const signature = readFileSync(signaturePath);
  expect(signature.length).toBe(384);
  await dialog.getByLabel('Firma separada', { exact: true }).setInputFiles(signaturePath);
  return { statement, signature };
}
export async function openTyped(page, label) {
  await directory(page)
    .getByRole('button', { name: `Abrir ${personName}`, exact: true })
    .filter({ hasText: label })
    .click();
  await expect(detail(page).getByRole('heading', { name: personName, exact: true })).toBeVisible();
}
export async function publicEvidence(page, testInfo, filename) {
  await detail(page)
    .getByRole('button', { name: 'Consultar evidencia de firma personal', exact: true })
    .click();
  await expect(
    detail(page).getByRole('heading', { name: 'Firma comprobada con CA interna', exact: true }),
  ).toBeVisible();
  const pending = page.waitForEvent('download');
  await detail(page)
    .getByRole('button', { name: 'Descargar evidencia p\u00fablica', exact: true })
    .click();
  const path = testInfo.outputPath(filename);
  await (await pending).saveAs(path);
  return JSON.parse(readFileSync(path, 'utf8'));
}
