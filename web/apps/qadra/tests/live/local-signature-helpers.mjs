import { expect } from '@playwright/test';
import { readFileSync, statSync } from 'node:fs';

export async function signLocally(scope, material, certificatePath) {
  expect(material?.encryptedKeyPath).toBeTruthy();
  expect(material?.keyPasswordPath).toBeTruthy();
  expect(statSync(material.encryptedKeyPath).mode & 0o777).toBe(0o600);
  expect(statSync(material.keyPasswordPath).mode & 0o777).toBe(0o600);
  const region = scope.getByRole('region', { name: /^Firma local:/ });
  await expect(region).toHaveCount(1);
  await expect(region).toBeVisible();
  const certificate = region.getByLabel('Certificado para firma local', { exact: true });
  if (await certificate.count()) await certificate.setInputFiles(certificatePath);
  const key = region.getByLabel('Clave privada cifrada', { exact: true });
  const password = region.getByLabel('Contrasena de la clave', { exact: true });
  await key.setInputFiles(material.encryptedKeyPath);
  await password.fill(readFileSync(material.keyPasswordPath, 'utf8'));
  await region.getByRole('button', { name: 'Firmar localmente', exact: true }).click();
  await expect(region.getByRole('status')).toHaveText(
    'Firma local preparada. Revisa y confirma la operacion.',
  );
  await expect(password).toHaveValue('');
  await expect(key).toHaveValue('');
}
