import { test, expect } from '@playwright/test';
import { verify } from 'node:crypto';
import { login } from './helpers.mjs';
import { preparation } from '../fixtures/owner-certificates.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
} from './owner-certificate-fixtures.mjs';
import {
  openOwnerCertificates,
  beginOwnerRegistration,
  register,
  current,
} from './owner-certificate-ui.mjs';
import {
  localSignatureMaterial,
  localFile,
  signLocally,
  localSigningViewports,
  captureLocalSignature,
} from './local-signature-material.mjs';

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

for (const { name, viewport } of localSigningViewports) {
  test(`Owner signs the exact binding locally and submits only public proof after confirmation (${name})`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    const material = localSignatureMaterial();
    const state = await ownerCertificateSetup(page);
    const bodies = [];
    page.on('request', (request) => {
      if (request.postData()) bodies.push(request.postData());
    });
    await page.route('**/auth/certificate-bindings/*/prepare', async (route) => {
      const input = route.request().postDataJSON();
      expect(Buffer.from(input.certificate_base64, 'base64')).toEqual(material.certificatePem);
      const id = new URL(route.request().url()).pathname.split('/').at(-2);
      const prepared = preparation(state.current.user.id, id);
      const bytes = Buffer.from(prepared.statement_base64, 'base64');
      Buffer.from(material.fingerprint, 'hex').copy(bytes, 118);
      prepared.statement_base64 = bytes.toString('base64');
      prepared.certificate.der_base64 = material.certificate.toString('base64');
      prepared.certificate.fingerprint = material.fingerprint;
      state.preparedBindings.set(id, prepared);
      state.ownerPreparations.push({ id, prepared });
      await route.fulfill({ json: prepared });
    });
    await login(page, false, false);
    await openOwnerCertificates(page);
    const form = await beginOwnerRegistration(page);
    await form
      .getByLabel('Certificado p\u00fablico PEM', { exact: true })
      .setInputFiles(localFile('browser-test-public.pem', material.certificatePem));
    await form.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
    const local = form.getByRole('region', { name: /^Firma local:/ });
    await expect(local).toBeVisible();
    for (const rejected of [
      { ...material, password: 'Wrong browser test password 1234' },
      localSignatureMaterial(),
    ]) {
      await signLocally(local, rejected);
      await expect(local.getByRole('alert')).toBeVisible();
      await expect(register(form)).toBeDisabled();
      await expect(local.getByLabel('Clave privada cifrada', { exact: true })).toHaveValue('');
      await expect(local.getByLabel('Contrasena de la clave', { exact: true })).toHaveValue('');
      expect(state.ownerWrites).toEqual([]);
    }
    await signLocally(local, material);
    await expect(register(form)).toBeEnabled();
    await expect(local.getByLabel('Clave privada cifrada', { exact: true })).toHaveValue('');
    await expect(local.getByLabel('Contrasena de la clave', { exact: true })).toHaveValue('');
    expect(state.ownerWrites).toEqual([]);
    await captureLocalSignature(local, testInfo, `owner-binding-${name}`);
    state.nextOwnerWrite = { action: 'register' };
    await register(form).click();
    await expect(current(page)).toBeVisible();
    expect(state.ownerWrites).toHaveLength(1);
    const submitted = state.ownerWrites[0].values;
    expect(
      verify(
        'sha256',
        Buffer.from(submitted.statement_base64, 'base64'),
        material.publicKey,
        Buffer.from(submitted.signature_base64, 'base64'),
      ),
    ).toBe(true);
    expect(Buffer.from(submitted.statement_base64, 'base64')).toEqual(
      Buffer.from(state.ownerPreparations[0].prepared.statement_base64, 'base64'),
    );
    expect(Object.keys(submitted).sort()).toEqual([
      'certificate_der_base64',
      'signature_base64',
      'statement_base64',
    ]);
    expect(bodies.join('\n')).not.toContain(material.password);
    expect(bodies.join('\n')).not.toContain(material.encryptedKey.toString());
    expect(bodies.join('\n')).not.toContain('ENCRYPTED PRIVATE KEY');
  });
}
