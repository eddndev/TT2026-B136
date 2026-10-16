import assert from "node:assert/strict";
import { createPrivateKey, randomBytes } from "node:crypto";
import { readFile, stat, writeFile } from "node:fs/promises";

export async function provisionLocalSigningKey(privateKeyPath) {
  assert.ok(process.env.IDENTITY_TEST_DATABASE_URL);
  assert.ok(privateKeyPath, "An isolated synthetic signing key is required");
  assert.equal((await stat(privateKeyPath)).mode & 0o777, 0o600);
  const privateKey = createPrivateKey(await readFile(privateKeyPath));
  assert.equal(privateKey.asymmetricKeyType, "rsa");
  assert.equal(privateKey.asymmetricKeyDetails.modulusLength, 3072);
  const password = randomBytes(24).toString("hex");
  const encrypted = privateKey.export({
    type: "pkcs8", format: "pem", cipher: "aes-256-cbc", passphrase: password,
  });
  assert.match(encrypted, /^-----BEGIN ENCRYPTED PRIVATE KEY-----/);
  const encryptedKeyPath = privateKeyPath + ".encrypted.pem";
  const keyPasswordPath = privateKeyPath + ".password.txt";
  await writeFile(encryptedKeyPath, encrypted, { mode: 0o600, flag: "wx" });
  await writeFile(keyPasswordPath, password, { mode: 0o600, flag: "wx" });
  assert.equal((await stat(encryptedKeyPath)).mode & 0o777, 0o600);
  assert.equal((await stat(keyPasswordPath)).mode & 0o777, 0o600);
  return { encryptedKeyPath, keyPasswordPath };
}
