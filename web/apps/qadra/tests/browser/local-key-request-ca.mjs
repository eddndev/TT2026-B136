import { expect } from '@playwright/test';
import { createHash, X509Certificate } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const initScript = fileURLToPath(new URL('../../../../../pki/init-ca.sh', import.meta.url));
const issueScript = fileURLToPath(new URL('../../../../../pki/issue-request.sh', import.meta.url));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

function execute(command, args, directory, environment) {
  const result = spawnSync(command, args, {
    cwd: directory,
    env: environment,
    encoding: 'utf8',
    timeout: 30_000,
    maxBuffer: 65_536,
  });
  expect(result.error, 'The existing isolated CA issuer must complete').toBeUndefined();
  expect(result.status, result.stderr).toBe(0);
}

// This checks the existing issuer scripts, not HTTP authorization or enrollment.
export async function verifyExistingCaIssuer(csr, publicSpki, testInfo) {
  const directory = mkdtempSync(join(tmpdir(), 'qadra-browser-csr-'));
  const authorityDirectory = join(directory, 'authority');
  const environment = { ...process.env, PKI_CA_DIR: authorityDirectory };
  const requestPath = join(directory, 'browser-request.pem');
  try {
    writeFileSync(requestPath, csr, { mode: 0o600 });
    execute('bash', [initScript], directory, environment);
    const rootPath = join(authorityDirectory, 'ca.crt.pem');
    const root = new X509Certificate(readFileSync(rootPath));
    const receipt = {
      source: 'browser-generated-pkcs10',
      csr_sha256: sha256(csr),
      spki_sha256: sha256(publicSpki),
      root_certificate_sha256: sha256(root.raw),
      issued: [],
    };
    for (const purpose of ['partner', 'document-signing']) {
      const name = `browser-${purpose}`;
      execute('bash', [issueScript, name, requestPath, purpose], directory, environment);
      const certificatePath = join(authorityDirectory, 'certs', `${name}.crt.pem`);
      const pem = readFileSync(certificatePath);
      const certificate = new X509Certificate(pem);
      execute('openssl', ['verify', '-CAfile', rootPath, certificatePath], directory, environment);
      expect(certificate.verify(root.publicKey)).toBe(true);
      expect(certificate.ca).toBe(false);
      expect(certificate.publicKey.export({ type: 'spki', format: 'der' })).toEqual(publicSpki);
      expect(certificate.keyUsage ?? []).toEqual(
        purpose === 'partner' ? ['1.3.6.1.5.5.7.3.2', '1.3.6.1.5.5.7.3.4'] : [],
      );
      receipt.issued.push({
        purpose,
        certificate_sha256: sha256(certificate.raw),
        serial: certificate.serialNumber,
      });
      await testInfo.attach(`${name}.certificate.pem`, {
        body: pem,
        contentType: 'application/x-pem-file',
      });
    }
    await testInfo.attach('isolated-issuer-compatibility.json', {
      body: Buffer.from(JSON.stringify(receipt, null, 2)),
      contentType: 'application/json',
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

export async function observeStorage(page) {
  await page.addInitScript(() => {
    const setItem = Storage.prototype.setItem;
    window.localCryptoStorageWrites = [];
    Storage.prototype.setItem = function (key, value) {
      window.localCryptoStorageWrites.push([String(key), String(value)]);
      return setItem.call(this, key, value);
    };
  });
}

export async function expectNoStoredSecrets(page, password, encrypted = null) {
  const stored = await page.evaluate(() =>
    JSON.stringify({
      local: Object.entries(localStorage),
      session: Object.entries(sessionStorage),
      writes: window.localCryptoStorageWrites,
    }),
  );
  expect(stored).not.toContain(password);
  expect(stored).not.toContain('PRIVATE KEY');
  if (encrypted) {
    expect(stored).not.toContain(encrypted.toString());
    expect(stored).not.toContain(encrypted.toString('base64'));
  }
}
