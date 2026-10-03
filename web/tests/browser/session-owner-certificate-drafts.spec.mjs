import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
  holdOwnerRead,
  bindingsPath,
  currentPath,
  principalPath,
} from './owner-certificate-fixtures.mjs';
import {
  openOwnerCertificates,
  beginOwnerRegistration,
  continueOwnerRegistration,
  prepareOwnerRegistration,
  downloadedBytes,
  register,
  editor,
  current,
} from './owner-certificate-ui.mjs';

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

async function reopen(page) {
  await login(page, false, false);
  await openOwnerCertificates(page);
  return continueOwnerRegistration(page);
}

async function expectBlankRegistration(page) {
  await expect(page.getByRole('button', { name: 'Continuar registro', exact: true })).toHaveCount(
    0,
  );
  const form = await beginOwnerRegistration(page);
  await expect(form.getByLabel('Certificado p\u00fablico PEM', { exact: true })).toHaveValue('');
  await expect(form.getByText('owner-external-signature.sig', { exact: true })).toHaveCount(0);
  await expect(register(form)).toBeDisabled();
  return form;
}

test('same Owner MFA restores only admitted public intent after fresh principal and current reads without preparing or submitting again', async ({
  page,
}) => {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  await openOwnerCertificates(page);
  const { form, prepared } = await prepareOwnerRegistration(page, state);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  const start = state.calls.length;
  const principal = holdOwnerRead(state, principalPath),
    head = holdOwnerRead(state, currentPath);
  await openOwnerCertificates(page);
  await expect.poll(() => principal.entered).toBe(true);
  expect(state.calls.slice(start).filter((call) => call.path === currentPath)).toEqual([]);
  await expect(editor(page)).toHaveCount(0);
  principal.release();
  await expect.poll(() => head.entered).toBe(true);
  await expect(editor(page)).toHaveCount(0);
  head.release();
  const restored = await continueOwnerRegistration(page);
  await expect(restored).toContainText('owner-external-signature.sig');
  const bytes = await downloadedBytes(
    page,
    restored.getByRole('button', { name: 'Descargar bytes para firma', exact: true }),
  );
  expect(bytes).toEqual(Buffer.from(prepared.statement_base64, 'base64'));
  expect(state.ownerPreparations).toHaveLength(1);
  expect(state.ownerWrites).toEqual([]);
  await expect(register(restored)).toBeEnabled();
});

test('a 401 after submission retains exact intent through absent and committed receipts with no automatic retry', async ({
  page,
}) => {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  await openOwnerCertificates(page);
  const { form, prepared } = await prepareOwnerRegistration(page, state);
  state.nextOwnerWrite = { action: 'register', commit: false, status: 401 };
  await register(form).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  const original = structuredClone(state.ownerWrites[0]);
  let restored = await reopen(page);
  await expect(register(restored)).toBeDisabled();
  await expect(
    restored.getByRole('button', { name: 'Reenviar este registro', exact: true }),
  ).toHaveCount(0);
  expect(state.ownerWrites).toHaveLength(1);
  const exactPath = `${bindingsPath}/${prepared.binding_id}`;
  let read = holdOwnerRead(state, exactPath);
  await restored.getByRole('button', { name: 'Comprobar registro', exact: true }).click();
  await expect.poll(() => read.entered).toBe(true);
  expect(state.ownerWrites).toHaveLength(1);
  read.release();
  const retry = restored.getByRole('button', { name: 'Reenviar este registro', exact: true });
  await expect(retry).toBeEnabled();
  expect(state.ownerWrites).toHaveLength(1);
  state.nextOwnerWrite = { action: 'register', commit: true, status: 401 };
  await retry.click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(state.ownerWrites[1].values).toEqual(original.values);
  expect(state.ownerWrites[1].id).toBe(original.id);
  restored = await reopen(page);
  await expect(register(restored)).toBeDisabled();
  expect(state.ownerWrites).toHaveLength(2);
  read = holdOwnerRead(state, exactPath);
  await restored.getByRole('button', { name: 'Comprobar registro', exact: true }).click();
  await expect.poll(() => read.entered).toBe(true);
  await expect(restored).toBeVisible();
  expect(state.ownerPreparations).toHaveLength(1);
  expect(state.ownerWrites).toHaveLength(2);
  read.release();
  await expect(editor(page)).toHaveCount(0);
  await expect(current(page)).toContainText(original.id);
  expect(state.ownerWrites.map(({ id, values }) => ({ id, values }))).toEqual([
    { id: original.id, values: original.values },
    { id: original.id, values: original.values },
  ]);
});

test('explicit logout discards public draft and another Owner never recovers a later interrupted intent', async ({
  page,
}) => {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  await openOwnerCertificates(page);
  await prepareOwnerRegistration(page, state);
  await page
    .getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true })
    .filter({ visible: true })
    .click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page, false, false);
  await openOwnerCertificates(page);
  const blank = await expectBlankRegistration(page);
  const { form } = await prepareOwnerRegistration(page, state, blank);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  const start = state.calls.length;
  await openOwnerCertificates(page);
  await expectBlankRegistration(page);
  expect(
    state.calls
      .slice(start)
      .filter((call) => call.path.startsWith(`${bindingsPath}/`) && call.path !== currentPath),
  ).toEqual([]);
  expect(state.ownerPreparations).toHaveLength(2);
  expect(state.ownerWrites).toEqual([]);
});
