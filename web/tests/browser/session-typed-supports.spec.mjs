import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { typedDraftSetup, checkTypedRequests } from './session-typed-draft-fixtures.mjs';
import { createTyped, fillTypedOwn, roleSupport, confirmTyped } from './session-typed-draft-ui.mjs';
import { openChildUpload, fillChild } from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkTypedRequests(page));

test('a profile support and role upload keep separate owners while changing the declared state discards only its child', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal);
  await modal.getByLabel('Tipo de participante', { exact: true }).selectOption('victim');
  await modal
    .getByLabel('Estado de Contacto seguro declarado', { exact: true })
    .selectOption('documented');
  let contact = modal.getByRole('group', { name: 'Contacto seguro declarado', exact: true });
  let upload = await openChildUpload(page, contact);
  await expire(page, state, await fillChild(upload, 'typed-contact.txt', 'Contact-only bytes\n'));
  await login(page);
  modal = await createTyped(page);
  await expect(
    modal.getByLabel('Estado de Contacto seguro declarado', { exact: true }),
  ).toHaveValue('documented');
  upload = await openChildUpload(page, roleSupport(modal));
  await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expire(page, state, await fillChild(upload, 'typed-role-owned.txt', 'Role-only bytes\n'));
  await login(page);
  modal = await createTyped(page);
  await modal
    .getByLabel('Estado de Contacto seguro declarado', { exact: true })
    .selectOption('unknown');
  await modal
    .getByLabel('Motivo de Contacto seguro declarado', { exact: true })
    .fill('  Decision nueva sin soporte  ');
  await modal
    .getByLabel('Estado de Contacto seguro declarado', { exact: true })
    .selectOption('documented');
  contact = modal.getByRole('group', { name: 'Contacto seguro declarado', exact: true });
  upload = await openChildUpload(page, contact);
  await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(upload.getByText('typed-contact.txt', { exact: true })).toHaveCount(0);
  await upload.getByRole('button', { name: 'Cancelar', exact: true }).click();
  upload = await openChildUpload(page, roleSupport(modal));
  await expect(upload.getByText('typed-role-owned.txt', { exact: true })).toBeVisible();
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta parcial  ',
  );
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).toBeHidden();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({
    filename: 'typed-role-owned.txt',
    text: 'Role-only bytes\n',
    type: 'text/plain',
    metadata: { classification: 'Soporte en revision', tags: ['etiqueta parcial'] },
  });
  await expect(confirmTyped(modal)).toBeDisabled();
  expect(state.typedCommits).toEqual([]);
});
