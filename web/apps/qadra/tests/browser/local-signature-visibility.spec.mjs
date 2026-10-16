import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { visibility } from './session-visibility-helpers.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
} from './owner-certificate-fixtures.mjs';
import { openOwnerCertificates, prepareOwnerRegistration } from './owner-certificate-ui.mjs';
import { ownerLoginSetup, checkOwnerLoginRequests } from './owner-login-fixtures.mjs';
import { panel, openOwnerLogin, prepareLogin } from './owner-login-ui.mjs';
import { localSignatureMaterial, localFile, signLocally } from './local-signature-material.mjs';

async function holdLocalWork(page) {
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
}

for (const stage of ['selected-secret', 'pending-worker']) {
  test('hidden authenticated workspace clears local signing ' + stage, async ({ page }) => {
    await holdLocalWork(page);
    const material = localSignatureMaterial();
    const state = await ownerCertificateSetup(page);
    await login(page, false, false);
    await openOwnerCertificates(page);
    const { form } = await prepareOwnerRegistration(page, state);
    const local = form.getByRole('region', { name: /^Firma local:/ });
    if (stage === 'pending-worker') {
      await signLocally(local, material);
      await expect.poll(() => page.evaluate(() => window.localSigningEntered)).toBe(true);
    } else {
      await local
        .getByLabel('Clave privada cifrada', { exact: true })
        .setInputFiles(localFile('browser-test-key.pem', material.encryptedKey));
      await local.getByLabel('Contrasena de la clave', { exact: true }).fill(material.password);
    }
    await visibility(page, 'hidden');
    await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
    await expect(local.getByLabel('Clave privada cifrada', { exact: true })).toHaveValue('');
    await expect(local.getByLabel('Contrasena de la clave', { exact: true })).toHaveValue('');
    if (stage === 'pending-worker')
      await expect.poll(() => page.evaluate(() => window.localSigningTerminated)).toBe(true);
    expect(state.ownerWrites).toEqual([]);
    await visibility(page, 'visible');
    await expect(page.locator('.app-layout')).not.toHaveAttribute('inert', '');
    await expect(
      local.getByRole('button', { name: 'Firmar localmente', exact: true }),
    ).toBeDisabled();
    await checkOwnerCertificateRequests(page);
  });
}

test('hiding unauthenticated certificate login cancels its local worker without proof', async ({
  page,
}) => {
  await holdLocalWork(page);
  const material = localSignatureMaterial();
  const state = await ownerLoginSetup(page);
  await openOwnerLogin(page);
  await prepareLogin(page);
  const local = panel(page).getByRole('region', { name: /^Firma local:/ });
  await local
    .getByLabel('Certificado para firma local', { exact: true })
    .setInputFiles(localFile('browser-test-certificate.pem', material.certificatePem));
  await signLocally(local, material);
  await expect.poll(() => page.evaluate(() => window.localSigningEntered)).toBe(true);
  await visibility(page, 'hidden');
  await expect.poll(() => page.evaluate(() => window.localSigningTerminated)).toBe(true);
  await expect(local.getByLabel('Clave privada cifrada', { exact: true })).toHaveValue('');
  await expect(local.getByLabel('Contrasena de la clave', { exact: true })).toHaveValue('');
  expect(state.proofs).toEqual([]);
  await checkOwnerLoginRequests(page);
});
