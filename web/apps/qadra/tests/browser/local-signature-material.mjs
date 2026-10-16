import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import { expect } from '@playwright/test';

export const localSigningViewports = [
  { name: 'desktop', viewport: { width: 1440, height: 1000 } },
  { name: 'mobile', viewport: { width: 390, height: 844 } },
];

function der(tag, ...items) {
  const content = Buffer.concat(items);
  const size = content.length;
  const length =
    size < 128
      ? Buffer.from([size])
      : Buffer.from(
          size.toString(16).padStart(Math.ceil(size.toString(16).length / 2) * 2, '0'),
          'hex',
        );
  return Buffer.concat([
    Buffer.from([tag]),
    size < 128 ? length : Buffer.concat([Buffer.from([0x80 | length.length]), length]),
    content,
  ]);
}

const sequence = (...items) => der(0x30, ...items);
const integer = (value) => der(0x02, Buffer.from([value]));
const text = (value) => der(0x0c, Buffer.from(value));
const algorithm = sequence(Buffer.from('06092a864886f70d01010b0500', 'hex'));

// Generated keys and certificates belong only to disposable browser tests.
export function localSignatureMaterial() {
  const { privateKey, publicKey } = generateKeyPairSync('rsa', { modulusLength: 3072 });
  const name = sequence(
    der(0x31, sequence(Buffer.from('0603550403', 'hex'), text('Browser test'))),
  );
  const validity = sequence(
    der(0x17, Buffer.from('260101000000Z')),
    der(0x17, Buffer.from('360101000000Z')),
  );
  const body = sequence(
    der(0xa0, integer(2)),
    integer(1),
    algorithm,
    name,
    validity,
    name,
    publicKey.export({ format: 'der', type: 'spki' }),
  );
  const certificate = sequence(
    body,
    algorithm,
    der(0x03, Buffer.from([0]), sign('sha256', body, privateKey)),
  );
  const password = 'Browser-only local key password 7149';
  const encryptedKey = Buffer.from(
    privateKey.export({
      format: 'pem',
      type: 'pkcs8',
      cipher: 'aes-256-cbc',
      passphrase: password,
    }),
  );
  const certificatePem = Buffer.from(
    '-----BEGIN CERTIFICATE-----\n' +
      certificate
        .toString('base64')
        .match(/.{1,64}/g)
        .join('\n') +
      '\n-----END CERTIFICATE-----\n',
  );
  return {
    certificate,
    certificatePem,
    fingerprint: createHash('sha256').update(certificate).digest('hex'),
    encryptedKey,
    password,
    publicKey,
  };
}

export const localFile = (name, buffer) => ({ name, mimeType: 'application/x-pem-file', buffer });

export async function signLocally(region, material) {
  await region
    .getByLabel('Clave privada cifrada', { exact: true })
    .setInputFiles(localFile('browser-test-encrypted-key.pem', material.encryptedKey));
  await region.getByLabel('Contrasena de la clave', { exact: true }).fill(material.password);
  await region.getByRole('button', { name: 'Firmar localmente', exact: true }).click();
}

export async function captureLocalSignature(region, testInfo, name) {
  await expect(region.getByLabel('Clave privada cifrada', { exact: true })).toHaveValue('');
  await expect(region.getByLabel('Contrasena de la clave', { exact: true })).toHaveValue('');
  const path = testInfo.outputPath(`${name}.png`);
  await region.screenshot({ path, animations: 'disabled' });
  await testInfo.attach(name, { path, contentType: 'image/png' });
}
