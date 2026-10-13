import { test, expect } from '@playwright/test';
import { login, setup } from './helpers.mjs';
import {
  publicPem,
  signature,
  preparation,
  receipt,
  submission,
} from '../fixtures/owner-certificates.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
  bindingsPath,
} from './owner-certificate-fixtures.mjs';
import {
  openOwnerCertificates,
  beginOwnerRegistration,
  prepareOwnerRegistration,
  downloadedBytes,
  register,
  current,
  editor,
  captureOwnerLayouts,
} from './owner-certificate-ui.mjs';

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

test('Owner discovers an existing binding without its UUID then retires and registers exact externally signed public bytes', async ({
  page,
}, testInfo) => {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  const original = receipt(preparation(state.current.user.id));
  state.ownerRecords.set(original.binding_id, original);
  await openOwnerCertificates(page);
  await expect(current(page)).toContainText(original.binding_id);
  expect(state.ownerWrites).toEqual([]);
  await current(page).getByRole('button', { name: 'Retirar v\u00ednculo', exact: true }).click();
  const confirmation = page.getByRole('dialog', { name: 'Retirar mi certificado', exact: true });
  await expect(confirmation).toContainText(original.binding_id);
  state.nextOwnerWrite = { action: 'withdraw' };
  await confirmation.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect(confirmation).toBeHidden();
  expect(state.ownerWrites[0]).toMatchObject({
    id: original.binding_id,
    values: { expected_revision: 1 },
  });
  const draft = await beginOwnerRegistration(page);
  for (const material of [
    publicPem.toString().replaceAll('CERTIFICATE', 'PRIVATE KEY'),
    publicPem.toString().replaceAll('CERTIFICATE', 'ENCRYPTED PRIVATE KEY'),
    Buffer.concat([publicPem, publicPem]),
  ]) {
    await draft.getByLabel('Certificado p\u00fablico PEM', { exact: true }).setInputFiles({
      name: 'rejected-input.pem',
      mimeType: 'application/x-pem-file',
      buffer: Buffer.from(material),
    });
    await expect(draft.getByRole('alert')).toBeVisible();
    await expect(
      draft.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }),
    ).toBeDisabled();
    await expect(draft.getByLabel('Certificado p\u00fablico PEM', { exact: true })).toHaveValue('');
    expect(state.ownerPreparations).toEqual([]);
    expect(state.ownerWrites).toHaveLength(1);
  }
  const { form, prepared } = await prepareOwnerRegistration(page, state, draft);
  const bytes = await downloadedBytes(
    page,
    form.getByRole('button', { name: 'Descargar bytes para firma', exact: true }),
  );
  expect(bytes).toHaveLength(150);
  expect(bytes).toEqual(Buffer.from(prepared.statement_base64, 'base64'));
  expect(prepared.binding_id).not.toBe(original.binding_id);
  expect(state.ownerWrites).toHaveLength(1);
  await captureOwnerLayouts(page, testInfo, 'owner-certificate-prepared');
  state.nextOwnerWrite = { action: 'register' };
  await register(form).click();
  await expect(editor(page)).toHaveCount(0);
  await expect(current(page)).toContainText(prepared.binding_id);
  expect(state.ownerWrites[1].values).toEqual(submission(prepared, signature.toString('base64')));
  const before = state.calls.length;
  await current(page).getByRole('button', { name: 'Consultar recibo', exact: true }).click();
  await expect
    .poll(() =>
      state.calls
        .slice(before)
        .some(
          (call) => call.method === 'GET' && call.path === `${bindingsPath}/${prepared.binding_id}`,
        ),
    )
    .toBe(true);
  expect(state.ownerWrites).toHaveLength(2);
  const receiptBytes = await downloadedBytes(
    page,
    current(page).getByRole('button', { name: 'Descargar recibo', exact: true }),
  );
  expect(JSON.parse(receiptBytes.toString()).registration.statement_base64).toBe(
    prepared.statement_base64,
  );
  await captureOwnerLayouts(page, testInfo, 'owner-certificate-receipt');
});

test('non Owner navigation and a direct account-certificate hash never open the private feature', async ({
  page,
}) => {
  const requests = await setup(page, 'litigator');
  await login(page, false, false);
  await expect(page.getByRole('button', { name: 'Mi certificado', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    window.location.hash = 'owner-certificates';
  });
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  await expect(editor(page)).toHaveCount(0);
  expect(requests.filter((call) => call.path.startsWith(bindingsPath))).toEqual([]);
});
