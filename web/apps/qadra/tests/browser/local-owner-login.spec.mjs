import { test, expect } from '@playwright/test';
import { verify } from 'node:crypto';
import { statement, challenge, mfa } from '../fixtures/owner-login.mjs';
import {
  ownerLoginSetup,
  checkOwnerLoginRequests,
  publicReceipt,
} from './owner-login-fixtures.mjs';
import {
  panel,
  prepare,
  proof,
  openOwnerLogin,
  selectReceipt,
  enterLoginMfa,
  back,
} from './owner-login-ui.mjs';
import {
  localSignatureMaterial,
  localFile,
  signLocally,
  localSigningViewports,
  captureLocalSignature,
} from './local-signature-material.mjs';

test.afterEach(async ({ page }) => checkOwnerLoginRequests(page));

for (const { name, viewport } of localSigningViewports) {
  test(`Owner locally signs a fresh access challenge and still explicitly submits proof and MFA (${name})`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    const material = localSignatureMaterial();
    const state = await ownerLoginSetup(page);
    const receipt = publicReceipt();
    const registration = receipt.registration;
    registration.certificate.der_base64 = material.certificate.toString('base64');
    registration.certificate.fingerprint = material.fingerprint;
    const bindingBytes = Buffer.from(registration.statement_base64, 'base64');
    Buffer.from(material.fingerprint, 'hex').copy(bindingBytes, 118);
    registration.statement_base64 = bindingBytes.toString('base64');
    const loginBytes = Buffer.from(statement);
    Buffer.from(material.fingerprint, 'hex').copy(loginBytes, 102);
    const prepared = challenge(loginBytes);
    const proofs = [];
    await page.route('**/auth/certificate-login/start', (route) =>
      route.fulfill({ json: prepared }),
    );
    await page.route('**/auth/certificate-login/proof', async (route) => {
      const input = route.request().postDataJSON();
      proofs.push(input);
      expect(Object.keys(input).sort()).toEqual(['challenge_token', 'signature_base64']);
      expect(input.challenge_token).toBe(prepared.challenge_token);
      expect(
        verify(
          'sha256',
          loginBytes,
          material.publicKey,
          Buffer.from(input.signature_base64, 'base64'),
        ),
      ).toBe(true);
      const next = mfa();
      state.pendingMfa = { token: next.challenge_token, certificate: true };
      await route.fulfill({ json: next });
    });
    await openOwnerLogin(page);
    await selectReceipt(page, receipt);
    await prepare(page).click();
    const local = panel(page).getByRole('region', { name: /^Firma local:/ });
    await expect(local).toBeVisible();
    await local
      .getByLabel('Certificado para firma local', { exact: true })
      .setInputFiles(localFile('browser-test-certificate.pem', material.certificatePem));
    await signLocally(local, material);
    await expect(proof(page)).toBeEnabled();
    expect(proofs).toEqual([]);
    expect(state.mfas).toEqual([]);
    await captureLocalSignature(local, testInfo, `owner-login-${name}`);
    await proof(page).click();
    await expect(
      page.getByRole('heading', { name: 'Un paso m\u00e1s.', exact: true }),
    ).toBeVisible();
    expect(proofs).toHaveLength(1);
    expect(state.mfas).toEqual([]);
    await enterLoginMfa(page, true);
    await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
    expect(state.mfas).toHaveLength(1);
  });
}

test('changing the login method terminates pending local work without sending proof', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const post = Worker.prototype.postMessage;
    const terminate = Worker.prototype.terminate;
    Worker.prototype.postMessage = function (value, ...args) {
      if (value?.type === 'sign') {
        window.localSigningEntered = true;
        return;
      }
      return post.call(this, value, ...args);
    };
    Worker.prototype.terminate = function () {
      window.localSigningTerminated = true;
      return terminate.call(this);
    };
  });
  const material = localSignatureMaterial();
  const state = await ownerLoginSetup(page);
  await openOwnerLogin(page);
  await selectReceipt(page);
  await prepare(page).click();
  const local = panel(page).getByRole('region', { name: /^Firma local:/ });
  await local
    .getByLabel('Certificado para firma local', { exact: true })
    .setInputFiles(localFile('browser-test-certificate.pem', material.certificatePem));
  await signLocally(local, material);
  await expect.poll(() => page.evaluate(() => window.localSigningEntered)).toBe(true);
  await back(page).click();
  await expect(panel(page)).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => window.localSigningTerminated)).toBe(true);
  expect(state.proofs).toEqual([]);
  expect(state.mfas).toEqual([]);
  await openOwnerLogin(page);
  await expect(panel(page).getByRole('region', { name: /^Firma local:/ })).toHaveCount(0);
  await expect(prepare(page)).toBeDisabled();
});
