import { test, expect } from '@playwright/test';
import { createPrivateKey, createPublicKey } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { login } from './helpers.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
} from './owner-certificate-fixtures.mjs';
import { openOwnerCertificates, downloadedBytes } from './owner-certificate-ui.mjs';
import { localFile } from './local-signature-material.mjs';
import {
  verifyExistingCaIssuer,
  observeStorage,
  expectNoStoredSecrets,
} from './local-key-request-ca.mjs';

const password = 'Disposable browser backup password 9817';
const region = (page) =>
  page.getByRole('region', { name: 'Generar clave y solicitud', exact: true });
const generation = (form) =>
  form.getByRole('button', { name: 'Generar clave y solicitud', exact: true });
const csrDownload = (form) =>
  form.getByRole('button', { name: 'Descargar solicitud CSR', exact: true });
const savedPassword = (form) =>
  form.getByLabel('Contrase\u00f1a de la clave guardada', { exact: true });

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

async function openRequest(page) {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  await openOwnerCertificates(page);
  const form = region(page);
  await expect(form).toBeVisible();
  return { state, form };
}

async function fillRequest(form) {
  await form
    .getByLabel('Nombre com\u00fan (CN)', { exact: true })
    .fill('Disposable browser holder');
  await form.getByLabel('Organizaci\u00f3n (O)', { exact: true }).fill('Despacho Juridico Demo');
  await form.getByLabel('Pa\u00eds (C)', { exact: true }).fill('MX');
  await form.getByLabel('Contrase\u00f1a de la nueva clave', { exact: true }).fill(password);
  await form.getByLabel('Confirmar contrase\u00f1a', { exact: true }).fill(password);
}

function opensslRequest(csr, arguments_) {
  const result = spawnSync('openssl', ['req', ...arguments_], {
    input: csr,
    encoding: 'utf8',
    timeout: 10_000,
    maxBuffer: 65_536,
  });
  expect(
    result.error,
    'OpenSSL must be available for independent CSR verification',
  ).toBeUndefined();
  expect(result.status, result.stderr).toBe(0);
  return result;
}

function verifyDownloadedMaterial(encrypted, csr) {
  expect(encrypted.toString()).toMatch(/^-----BEGIN ENCRYPTED PRIVATE KEY-----/);
  expect(csr.toString()).toMatch(/^-----BEGIN CERTIFICATE REQUEST-----/);
  const key = createPrivateKey({
    key: encrypted,
    format: 'pem',
    type: 'pkcs8',
    passphrase: password,
  });
  expect(key.asymmetricKeyType).toBe('rsa');
  expect(key.asymmetricKeyDetails.modulusLength).toBe(3072);
  expect(key.asymmetricKeyDetails.publicExponent).toBe(65537n);
  const verification = opensslRequest(csr, ['-verify', '-noout']);
  expect(verification.stderr + verification.stdout).toMatch(/verify OK|verify ok/i);
  const publicPem = opensslRequest(csr, ['-pubkey', '-noout']).stdout;
  expect(createPublicKey(publicPem).export({ type: 'spki', format: 'der' })).toEqual(
    createPublicKey(key).export({ type: 'spki', format: 'der' }),
  );
  const subject = opensslRequest(csr, ['-subject', '-nameopt', 'RFC2253', '-noout']).stdout;
  expect(subject).toContain('CN=Disposable browser holder');
  expect(subject).toContain('O=Despacho Juridico Demo');
  expect(subject).toContain('C=MX');
  return createPublicKey(key).export({ type: 'spki', format: 'der' });
}

for (const [layout, width, height] of [
  ['desktop', 1440, 1000],
  ['mobile', 390, 844],
]) {
  test(`Owner generates and checks an encrypted key backup before exporting the CSR on ${layout}`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width, height });
    await observeStorage(page);
    const { state, form } = await openRequest(page);
    const outgoing = [];
    page.on('request', (request) => outgoing.push(`${request.url()}\n${request.postData() ?? ''}`));
    await fillRequest(form);
    await generation(form).click();
    const keyDownload = form.getByRole('button', { name: 'Descargar clave cifrada', exact: true });
    await expect(keyDownload).toBeVisible({ timeout: 60_000 });
    await form.screenshot({ path: testInfo.outputPath(`key-request-${layout}.png`) });
    await expect(csrDownload(form)).toHaveCount(0);
    expect(state.ownerWrites).toEqual([]);
    expect(state.ownerPreparations).toEqual([]);
    const encrypted = await downloadedBytes(page, keyDownload);
    await expectNoStoredSecrets(page, password, encrypted);
    await form
      .getByLabel('Archivo de clave cifrada guardado', { exact: true })
      .setInputFiles(localFile('disposable-browser-key.encrypted.pem', encrypted));
    await savedPassword(form).fill('Wrong backup password 9817');
    await form.getByRole('button', { name: 'Comprobar copia guardada', exact: true }).click();
    await expect(form.getByRole('alert')).toContainText('No se pudo comprobar la copia');
    await expect(savedPassword(form)).toHaveValue('');
    await expect(csrDownload(form)).toHaveCount(0);
    await savedPassword(form).fill(password);
    await form.getByRole('button', { name: 'Comprobar copia guardada', exact: true }).click();
    await expect(form.getByRole('status')).toContainText('La copia cifrada se puede abrir');
    await expect(savedPassword(form)).toHaveValue('');
    const csr = await downloadedBytes(page, csrDownload(form));
    const publicSpki = verifyDownloadedMaterial(encrypted, csr);
    await testInfo.attach(`browser-${layout}-request.csr.pem`, {
      body: csr,
      contentType: 'application/pkcs10',
    });
    await expectNoStoredSecrets(page, password, encrypted);
    if (layout === 'desktop') await verifyExistingCaIssuer(csr, publicSpki, testInfo);
    expect(state.ownerWrites).toEqual([]);
    expect(state.ownerPreparations).toEqual([]);
    const transmitted = outgoing.join('\n');
    expect(transmitted).not.toContain(password);
    expect(transmitted).not.toContain(encrypted.toString());
    expect(transmitted).not.toContain(encrypted.toString('base64'));
    expect(transmitted).not.toContain('PRIVATE KEY');
    await form.getByRole('button', { name: 'Limpiar datos locales', exact: true }).click();
    await expect(keyDownload).toHaveCount(0);
    await expect(csrDownload(form)).toHaveCount(0);
    await expect(form.getByLabel('Nombre com\u00fan (CN)', { exact: true })).toHaveValue('');
    await expect(form.getByLabel('Contrase\u00f1a de la nueva clave', { exact: true })).toHaveValue(
      '',
    );
    await expect(form.getByLabel('Confirmar contrase\u00f1a', { exact: true })).toHaveValue('');
    await expect(form.getByRole('alert')).toHaveCount(0);
    await expectNoStoredSecrets(page, password, encrypted);
  });
}

test('clearing a pending key request terminates its worker and preserves an empty local form', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const post = Worker.prototype.postMessage;
    const terminate = Worker.prototype.terminate;
    const pending = new WeakSet();
    Worker.prototype.postMessage = function (value, ...arguments_) {
      if (value?.type === 'generate') {
        window.localGenerationEntered = true;
        pending.add(this);
        return;
      }
      return post.call(this, value, ...arguments_);
    };
    Worker.prototype.terminate = function () {
      if (pending.has(this)) window.localGenerationTerminated = true;
      return terminate.call(this);
    };
  });
  const { state, form } = await openRequest(page);
  await fillRequest(form);
  await generation(form).click();
  await expect.poll(() => page.evaluate(() => window.localGenerationEntered)).toBe(true);
  await expect(form.getByRole('button', { name: 'Generando...', exact: true })).toBeDisabled();
  await expect(form.getByLabel('Contrase\u00f1a de la nueva clave', { exact: true })).toHaveValue(
    '',
  );
  await form.getByRole('button', { name: 'Limpiar datos locales', exact: true }).click();
  await expect.poll(() => page.evaluate(() => window.localGenerationTerminated)).toBe(true);
  await expect(generation(form)).toBeEnabled();
  await expect(form.getByLabel('Nombre com\u00fan (CN)', { exact: true })).toHaveValue('');
  await expect(form.getByLabel('Confirmar contrase\u00f1a', { exact: true })).toHaveValue('');
  await expect(
    form.getByRole('button', { name: 'Descargar clave cifrada', exact: true }),
  ).toHaveCount(0);
  await expect(csrDownload(form)).toHaveCount(0);
  await expect(form.getByRole('alert')).toHaveCount(0);
  expect(state.ownerWrites).toEqual([]);
  expect(state.ownerPreparations).toEqual([]);
});
