import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash, X509Certificate, verify } from 'node:crypto';

export const bindingsPath = '/api/v1/auth/certificate-bindings';
export const currentPath = `${bindingsPath}/current`;
export const receiptRegion = (page) =>
  page.getByRole('region', {
    name: 'V\u00ednculo de mi cuenta',
    exact: true,
  });
export const editorRegion = (page) =>
  page.getByRole('region', {
    name: 'Registro de mi certificado',
    exact: true,
  });
export const button = (scope, name) => scope.getByRole('button', { name, exact: true });

export function responseTo(page, path, method = 'GET') {
  return page.waitForResponse((response) => {
    const url = new URL(response.url());
    return url.pathname === path && url.search === '' && response.request().method() === method;
  });
}

export async function jsonResponse(pending) {
  const response = await pending;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  return response.json();
}

export async function navigate(page, label) {
  const menu = button(page, 'Abrir men\u00fa');
  if (await menu.isVisible()) await menu.click();
  await button(page, label).filter({ visible: true }).click();
}

export async function freshOwner(page, owner, action) {
  const me = responseTo(page, '/api/v1/auth/me');
  const current = responseTo(page, currentPath);
  await action();
  expect(await jsonResponse(me)).toMatchObject({ id: owner.id, role: 'owner' });
  return jsonResponse(current);
}

export async function openOwner(page, owner) {
  const current = await freshOwner(page, owner, () => navigate(page, 'Mi certificado'));
  await expect(page.getByRole('heading', { name: 'Mi certificado', exact: true })).toBeVisible();
  return current;
}

export async function downloaded(page, control, testInfo, filename) {
  const pending = page.waitForEvent('download');
  await control.click();
  const path = testInfo.outputPath(filename);
  await (await pending).saveAs(path);
  return { path, bytes: readFileSync(path) };
}

export async function readReceipt(page, bindingId) {
  const pending = responseTo(page, `${bindingsPath}/${bindingId}`);
  await button(receiptRegion(page), 'Consultar recibo').click();
  const result = await jsonResponse(pending);
  await expect(button(receiptRegion(page), 'Descargar recibo')).toBeEnabled();
  return result;
}

export async function downloadReceipt(page, testInfo, filename) {
  const result = await downloaded(
    page,
    button(receiptRegion(page), 'Descargar recibo'),
    testInfo,
    filename,
  );
  return JSON.parse(result.bytes.toString('utf8'));
}

export function canonicalStatement(prepared, withdrawal = null) {
  const result = Buffer.alloc(150);
  result.write('OWNCERT1', 0, 'ascii');
  result[8] = withdrawal ? 2 : 1;
  result[9] = 1;
  Buffer.from(prepared.deployment_id.replaceAll('-', ''), 'hex').copy(result, 10);
  Buffer.from(prepared.root_fingerprint, 'hex').copy(result, 26);
  result.writeUInt32BE(prepared.trust_revision, 58);
  Buffer.from(prepared.owner_id.replaceAll('-', ''), 'hex').copy(result, 62);
  const account = withdrawal ?? prepared;
  result.writeBigUInt64BE(BigInt(account.account_revision), 78);
  result.writeBigUInt64BE(BigInt(account.auth_generation), 86);
  Buffer.from(prepared.binding_id.replaceAll('-', ''), 'hex').copy(result, 94);
  result.writeUInt32BE(withdrawal ? 1 : 0, 110);
  result.writeUInt32BE(withdrawal ? 2 : 1, 114);
  Buffer.from(prepared.certificate.fingerprint, 'hex').copy(result, 118);
  return result;
}

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

export async function signPreparation(page, form, prepared, certificatePath, testInfo) {
  const certificate = new X509Certificate(readFileSync(certificatePath));
  expect(prepared.policy).toBe('internal_partner_binding_v1');
  expect(prepared.certificate.der_base64).toBe(certificate.raw.toString('base64'));
  expect(prepared.certificate.fingerprint).toBe(digest(certificate.raw));
  const statement = await downloaded(
    page,
    button(form, 'Descargar bytes para firma'),
    testInfo,
    'owner-statement.bin',
  );
  expect(statement.bytes).toHaveLength(150);
  expect(statement.bytes).toEqual(Buffer.from(prepared.statement_base64, 'base64'));
  expect(statement.bytes).toEqual(canonicalStatement(prepared));
  const keyPath = process.env.TT_LIVE_OWNER_CERTIFICATE_PRIVATE_KEY;
  if (!keyPath) throw new Error('An isolated external Owner signing key is required');
  const signaturePath = testInfo.outputPath('owner-statement.sig');
  execFileSync(
    'openssl',
    ['dgst', '-sha256', '-sign', keyPath, '-out', signaturePath, statement.path],
    { timeout: 30000, stdio: 'ignore' },
  );
  const signature = readFileSync(signaturePath);
  expect(signature).toHaveLength(384);
  expect(verify('RSA-SHA256', statement.bytes, certificate.publicKey, signature)).toBe(true);
  await form.getByLabel('Firma separada', { exact: true }).setInputFiles(signaturePath);
  return {
    statement_base64: statement.bytes.toString('base64'),
    certificate_der_base64: certificate.raw.toString('base64'),
    signature_base64: signature.toString('base64'),
  };
}

export function expectRegistration(receipt, prepared, data) {
  expect(receipt).toMatchObject({
    binding_id: prepared.binding_id,
    owner_id: prepared.owner_id,
    revision: 1,
    policy: prepared.policy,
    withdrawal: null,
  });
  expect(receipt.registration).toMatchObject({
    statement_base64: data.statement_base64,
    statement_digest: digest(Buffer.from(data.statement_base64, 'base64')),
    signature_base64: data.signature_base64,
    certificate: prepared.certificate,
    account_revision: prepared.account_revision,
    auth_generation: prepared.auth_generation,
    trust: {
      deployment_id: prepared.deployment_id,
      revision: prepared.trust_revision,
      root_fingerprint: prepared.root_fingerprint,
    },
  });
  expect(typeof receipt.registration.trust.crl_number).toBe('string');
}
