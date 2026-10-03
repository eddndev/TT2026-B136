import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { loginWithOwnerCertificate } from './owner-login-helpers.mjs';
import {
  bindingsPath,
  button,
  editorRegion,
  receiptRegion,
  responseTo,
  jsonResponse,
  navigate,
  freshOwner,
  openOwner,
  readReceipt,
  downloadReceipt,
  canonicalStatement,
  signPreparation,
  expectRegistration,
} from './owner-certificate-helpers.mjs';

test('Owner registers public proof, discovers it after fresh MFA and withdraws it', async ({
  page,
}, testInfo) => {
  const own = fixture.ownerCertificates;
  expect(own?.owner?.id, 'The isolated Owner fixture must be provisioned').toBeTruthy();
  expect(own.certificatePath).toBeTruthy();
  const writes = [];
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (request.method() === 'POST' && url.pathname.startsWith(`${bindingsPath}/`))
      writes.push({ path: url.pathname, data: request.postDataJSON() });
  });
  await page.goto('/');
  await loginAs(page, own.owner, 0);
  expect(await openOwner(page, own.owner)).toBeNull();
  expect(writes).toEqual([]);
  expect(
    await freshOwner(page, own.owner, () => button(page, 'Registrar certificado').click()),
  ).toBeNull();
  const form = editorRegion(page);
  await expect(form).toHaveAttribute('aria-busy', 'false');
  await form
    .getByLabel('Certificado p\u00fablico PEM', { exact: true })
    .setInputFiles(own.certificatePath);
  await expect(button(form, 'Preparar v\u00ednculo')).toBeEnabled();
  const id = (
    await form.getByText('Identificador de este intento:', { exact: false }).innerText()
  ).match(/[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}/)?.[0];
  expect(id).toBeTruthy();
  const preparation = responseTo(page, `${bindingsPath}/${id}/prepare`, 'POST');
  await button(form, 'Preparar v\u00ednculo').click();
  const prepared = await jsonResponse(preparation);
  expect(prepared).toMatchObject({ owner_id: own.owner.id, binding_id: id });
  await expect(button(form, 'Descargar bytes para firma')).toBeEnabled();
  const data = await signPreparation(page, form, prepared, own.certificatePath, testInfo);
  await expect(button(form, 'Registrar v\u00ednculo')).toBeEnabled();
  expect(writes.map((call) => call.path)).toEqual([`${bindingsPath}/${id}/prepare`]);
  const registration = responseTo(page, `${bindingsPath}/${id}/register`, 'POST');
  await button(form, 'Registrar v\u00ednculo').click();
  const original = await jsonResponse(registration);
  expectRegistration(original, prepared, data);
  expect(writes[1]).toEqual({ path: `${bindingsPath}/${id}/register`, data });
  await expect(form).toHaveCount(0);
  await expect(receiptRegion(page)).toContainText(id);
  expect(await readReceipt(page, id)).toEqual(original);
  expect(await downloadReceipt(page, testInfo, 'owner-registration.json')).toEqual(original);

  const logout = responseTo(page, '/api/v1/auth/logout', 'POST');
  await navigate(page, 'Cerrar sesi\u00f3n');
  expect((await logout).status()).toBe(204);
  await loginWithOwnerCertificate(page, own, original, testInfo);
  expect(await openOwner(page, own.owner)).toEqual(original);
  await expect(receiptRegion(page)).toContainText(id);
  expect(await readReceipt(page, id)).toEqual(original);
  expect(writes.map((call) => call.path)).toEqual([
    `${bindingsPath}/${id}/prepare`,
    `${bindingsPath}/${id}/register`,
  ]);

  const certificateLogout = responseTo(page, '/api/v1/auth/logout', 'POST');
  await navigate(page, 'Cerrar sesi\u00f3n');
  expect((await certificateLogout).status()).toBe(204);
  await loginAs(page, own.owner, 2);
  expect(await openOwner(page, own.owner)).toEqual(original);

  expect(
    await freshOwner(page, own.owner, () =>
      button(receiptRegion(page), 'Retirar v\u00ednculo').click(),
    ),
  ).toEqual(original);
  const confirmation = page.getByRole('dialog', { name: 'Retirar mi certificado', exact: true });
  await expect(confirmation).toContainText(id);
  await expect(button(confirmation, 'Confirmar retiro')).toBeEnabled();
  const withdrawal = responseTo(page, `${bindingsPath}/${id}/withdraw`, 'POST');
  await button(confirmation, 'Confirmar retiro').click();
  const terminal = await jsonResponse(withdrawal);
  expect(terminal).toMatchObject({
    binding_id: id,
    owner_id: own.owner.id,
    revision: 2,
    policy: original.policy,
    registration: original.registration,
  });
  expect(terminal.registration).toEqual(original.registration);
  expect(Buffer.from(terminal.withdrawal.statement_base64, 'base64')).toEqual(
    canonicalStatement(prepared, terminal.withdrawal),
  );
  expect(BigInt(terminal.withdrawal.account_revision)).toBeGreaterThan(
    BigInt(original.registration.account_revision),
  );
  await expect(confirmation).toHaveCount(0);
  await expect(receiptRegion(page)).toContainText('Retirado');
  expect(await readReceipt(page, id)).toEqual(terminal);
  expect(await downloadReceipt(page, testInfo, 'owner-withdrawal.json')).toEqual(terminal);
  await navigate(page, 'Inicio');
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  expect(await openOwner(page, own.owner)).toBeNull();
  await expect(button(page, 'Registrar certificado')).toBeEnabled();
  await expect(receiptRegion(page)).toHaveCount(0);
  expect(writes).toHaveLength(3);
  expect(writes[2]).toEqual({
    path: `${bindingsPath}/${id}/withdraw`,
    data: { expected_revision: 1 },
  });
});
