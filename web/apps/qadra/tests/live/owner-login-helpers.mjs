import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash, X509Certificate, verify } from 'node:crypto';
import { button, downloaded, responseTo, jsonResponse } from './owner-certificate-helpers.mjs';

const path = '/api/v1/auth/certificate-login';
const uuidBytes = (value) => Buffer.from(value.replaceAll('-', ''), 'hex');

async function signedLogin(page, region, capture, own, receipt, testInfo) {
  const material = await downloaded(
    page,
    button(region, 'Descargar bytes de acceso'),
    testInfo,
    'owner-login-statement.bin',
  );
  const bytes = material.bytes;
  expect(bytes).toHaveLength(182);
  expect(bytes.equals(Buffer.from(capture.statement_base64, 'base64'))).toBe(true);
  expect(bytes.subarray(0, 10).equals(Buffer.from('OWNAUTH1\x01\x01', 'binary'))).toBe(true);
  expect(bytes.subarray(62, 78).equals(uuidBytes(own.owner.id))).toBe(true);
  expect(bytes.subarray(86, 102).equals(uuidBytes(receipt.binding_id))).toBe(true);
  const certificate = new X509Certificate(readFileSync(own.certificatePath));
  const leaf = createHash('sha256').update(certificate.raw).digest();
  expect(bytes.subarray(102, 134).equals(leaf)).toBe(true);
  expect(leaf.toString('hex') === receipt.registration.certificate.fingerprint).toBe(true);
  expect(bytes.readBigUInt64BE(78) <= 9223372036854775807n).toBe(true);
  expect(bytes.readUInt32BE(58)).toBeGreaterThan(0);
  const issued = bytes.readBigInt64BE(166),
    expires = bytes.readBigInt64BE(174);
  expect(issued >= 0n && expires > issued && expires - issued <= 300n).toBe(true);
  expect(BigInt(capture.expires_in_seconds) <= expires - issued).toBe(true);
  const keyPath = process.env.TT_LIVE_OWNER_CERTIFICATE_PRIVATE_KEY;
  if (!keyPath) throw new Error('An isolated external Owner signing key is required');
  const signaturePath = testInfo.outputPath('owner-login-statement.sig');
  execFileSync(
    'openssl',
    [
      'dgst',
      '-sha256',
      '-sign',
      keyPath,
      '-sigopt',
      'rsa_padding_mode:pkcs1',
      '-out',
      signaturePath,
      material.path,
    ],
    { timeout: 30000, stdio: 'ignore' },
  );
  const signature = readFileSync(signaturePath);
  expect(signature).toHaveLength(384);
  expect(verify('RSA-SHA256', bytes, certificate.publicKey, signature)).toBe(true);
  await region.getByLabel('Firma separada de acceso', { exact: true }).setInputFiles(signaturePath);
  return signature.toString('base64');
}

export async function loginWithOwnerCertificate(page, own, receipt, testInfo) {
  const calls = [];
  const observe = (request) => {
    const url = new URL(request.url());
    if (url.pathname.startsWith('/api/v1/auth/'))
      calls.push({ path: url.pathname, method: request.method(), search: url.search });
  };
  page.on('request', observe);
  try {
    const availability = responseTo(page, `${path}/availability`);
    await button(page, 'Ingresar con certificado').click();
    expect(await jsonResponse(availability)).toEqual({ enabled: true });
    const region = page.getByRole('region', { name: 'Acceso con certificado', exact: true });
    await region
      .getByLabel('Recibo p\u00fablico del v\u00ednculo', { exact: true })
      .setInputFiles(testInfo.outputPath('owner-registration.json'));
    await expect(button(region, 'Preparar acceso')).toBeEnabled();
    expect(calls).toEqual([{ path: `${path}/availability`, method: 'GET', search: '' }]);
    const start = responseTo(page, `${path}/start`, 'POST');
    await button(region, 'Preparar acceso').click();
    const startResponse = await start;
    const capture = await jsonResponse(Promise.resolve(startResponse));
    expect(Object.keys(capture).sort()).toEqual([
      'challenge_token',
      'expires_in_seconds',
      'statement_base64',
    ]);
    expect(
      typeof capture.challenge_token === 'string' &&
        /^[A-Za-z0-9_-]{43}$/.test(capture.challenge_token),
    ).toBe(true);
    expect(
      Number.isSafeInteger(capture.expires_in_seconds) &&
        capture.expires_in_seconds > 0 &&
        capture.expires_in_seconds <= 300,
    ).toBe(true);
    expect(startResponse.request().postDataJSON()).toEqual({
      owner_id: own.owner.id,
      binding_id: receipt.binding_id,
    });
    expect(startResponse.request().headers().authorization).toBeUndefined();
    const signature = await signedLogin(page, region, capture, own, receipt, testInfo);
    const proof = responseTo(page, `${path}/proof`, 'POST');
    await expect(button(region, 'Comprobar firma')).toBeEnabled();
    await button(region, 'Comprobar firma').click();
    const proofResponse = await proof,
      mfa = await jsonResponse(Promise.resolve(proofResponse));
    const input = proofResponse.request().postDataJSON();
    expect(Object.keys(input).sort()).toEqual(['challenge_token', 'signature_base64']);
    expect(input.challenge_token === capture.challenge_token).toBe(true);
    expect(input.signature_base64 === signature).toBe(true);
    expect(proofResponse.request().headers().authorization).toBeUndefined();
    expect(Object.keys(mfa).sort()).toEqual(['challenge_token', 'expires_in_seconds']);
    await expect(
      page.getByRole('heading', { name: 'Un paso m\u00e1s.', exact: true }),
    ).toBeVisible();
    expect(calls.filter((call) => call.method === 'POST').map((call) => call.path)).toEqual([
      `${path}/start`,
      `${path}/proof`,
    ]);
    await button(page, 'Usar c\u00f3digo de recuperaci\u00f3n').click();
    await page
      .getByLabel('C\u00f3digo de recuperaci\u00f3n', { exact: true })
      .fill(own.owner.recoveryCodes[1]);
    const verified = responseTo(page, '/api/v1/auth/mfa/recovery', 'POST');
    await button(page, 'Verificar y entrar').click();
    const verifiedResponse = await verified;
    const session = await jsonResponse(Promise.resolve(verifiedResponse));
    expect(session.user).toMatchObject({ id: own.owner.id, role: 'owner' });
    expect(typeof session.access_token === 'string' && session.access_token.length > 0).toBe(true);
    const factor = verifiedResponse.request().postDataJSON();
    expect(Object.keys(factor).sort()).toEqual(['challenge_token', 'code']);
    expect(factor.challenge_token === mfa.challenge_token).toBe(true);
    expect(factor.code === own.owner.recoveryCodes[1]).toBe(true);
    expect(verifiedResponse.request().headers().authorization).toBeUndefined();
    await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
    expect(calls.filter((call) => call.method === 'POST').map((call) => call.path)).toEqual([
      `${path}/start`,
      `${path}/proof`,
      '/api/v1/auth/mfa/recovery',
    ]);
    expect(calls.every((call) => call.search === '')).toBe(true);
  } finally {
    page.off('request', observe);
  }
}
