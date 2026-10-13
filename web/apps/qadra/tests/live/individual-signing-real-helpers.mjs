import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash, X509Certificate, verify } from 'node:crypto';
import { expect } from '@playwright/test';
import {
  navigate,
  button,
  downloaded,
  responseTo,
  jsonResponse,
} from './owner-certificate-helpers.mjs';
export { navigate, button, downloaded, responseTo, jsonResponse };
export const bindingsPath = '/api/v1/identity/signing-certificates';
export const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const documentPath = (scenario) =>
  `/api/v1/cases/${scenario.caseId}/documents/${scenario.document.id}/versions/${scenario.document.version}`;

export async function openSigningDocument(page, scenario) {
  await navigate(page, 'Expedientes');
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByLabel('Buscar por t\u00edtulo', { exact: true }).fill(scenario.caseTitle);
  const listed = page.waitForResponse(
    (response) => new URL(response.url()).pathname === '/api/v1/case-administrations',
  );
  await button(page, 'Buscar expedientes').click();
  expect((await listed).status()).toBe(200);
  await expect(page.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: new RegExp(scenario.caseTitle) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page
    .locator('.reference-panel')
    .getByLabel('Identificador del documento')
    .fill(scenario.document.id);
  await button(page, 'Abrir documento').click();
  await expect(
    page.getByRole('heading', { name: scenario.document.name, exact: true }),
  ).toBeVisible();
  await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
}

export async function signedDownload(page, control, scenario, testInfo, name, label) {
  const result = await downloaded(page, button(page, control), testInfo, name);
  const signaturePath = testInfo.outputPath(name + '.sig');
  execFileSync(
    'openssl',
    [
      'dgst',
      '-sha256',
      '-sign',
      scenario.privateKeyPath,
      '-sigopt',
      'rsa_padding_mode:pkcs1',
      '-out',
      signaturePath,
      result.path,
    ],
    { timeout: 30000, stdio: 'ignore' },
  );
  const signature = readFileSync(signaturePath);
  const certificate = new X509Certificate(readFileSync(scenario.certificatePath));
  expect(signature).toHaveLength(384);
  expect(verify('RSA-SHA256', result.bytes, certificate.publicKey, signature)).toBe(true);
  await page.getByLabel(label, { exact: true }).setInputFiles(signaturePath);
  return { ...result, signature, certificate };
}

export async function registerSigningCertificate(page, scenario, testInfo) {
  const current = responseTo(page, bindingsPath + '/current');
  await navigate(page, 'Mi firma documental');
  expect(await jsonResponse(current)).toEqual({ binding: null });
  await page
    .getByLabel('Certificado publico PEM', { exact: true })
    .setInputFiles(scenario.certificatePath);
  const preparation = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname.startsWith(bindingsPath + '/') &&
      response.url().endsWith('/prepare'),
  );
  await button(page, 'Preparar vinculo').click();
  const prepared = await jsonResponse(preparation);
  expect(prepared.account_id).toBe(scenario.litigator.id);
  const proof = await signedDownload(
    page,
    'Descargar declaracion del vinculo',
    scenario,
    testInfo,
    'certificate-binding.bin',
    'Firma externa del vinculo',
  );
  expect(proof.bytes).toHaveLength(142);
  expect(proof.bytes.subarray(0, 10)).toEqual(Buffer.from('DOCBIND1\x01\x01', 'binary'));
  expect(proof.bytes).toEqual(Buffer.from(prepared.statement_base64, 'base64'));
  expect(prepared.certificate.fingerprint).toBe(digest(proof.certificate.raw));
  expect(prepared.certificate.der_base64).toBe(proof.certificate.raw.toString('base64'));
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  const registration = responseTo(
    page,
    bindingsPath + '/' + prepared.binding_id + '/register',
    'POST',
  );
  await button(page, 'Registrar vinculo').click();
  const receipt = await jsonResponse(registration);
  expect(receipt).toMatchObject({
    account_id: scenario.litigator.id,
    binding_id: prepared.binding_id,
    statement_base64: prepared.statement_base64,
    signature_base64: proof.signature.toString('base64'),
    withdrawn_at: null,
  });
  await expect(page.getByText('Vinculo registrado', { exact: true })).toBeVisible();
  return receipt;
}

export async function preparePersonalSignature(page, scenario, testInfo) {
  await openSigningDocument(page, scenario);
  await button(page, 'Firmar personalmente').click();
  const pdf = await downloaded(
    page,
    button(page, 'Descargar PDF para firmar'),
    testInfo,
    'document.pdf',
  );
  expect(digest(pdf.bytes)).toBe(scenario.document.digest);
  expect(pdf.bytes.subarray(0, 5).toString()).toBe('%PDF-');
  await expect(page.getByTitle('PDF exacto de esta version')).toBeVisible();
  await page.getByLabel('He revisado este PDF y confirmo su contenido').check();
  const signaturePath = testInfo.outputPath('document.sig');
  execFileSync(
    'openssl',
    [
      'dgst',
      '-sha256',
      '-sign',
      scenario.privateKeyPath,
      '-sigopt',
      'rsa_padding_mode:pkcs1',
      '-out',
      signaturePath,
      pdf.path,
    ],
    { timeout: 30000, stdio: 'ignore' },
  );
  const pdfSignature = readFileSync(signaturePath);
  expect(pdfSignature).toHaveLength(384);
  await page.getByLabel('Firma externa del PDF', { exact: true }).setInputFiles(signaturePath);
  const preparation = responseTo(
    page,
    documentPath(scenario) + '/individual-signing/prepare',
    'POST',
  );
  await button(page, 'Preparar consentimiento').click();
  const prepared = await jsonResponse(preparation);
  expect(prepared).toMatchObject({
    account_id: scenario.litigator.id,
    case_id: scenario.caseId,
    document_id: scenario.document.id,
    version: scenario.document.version,
    document_digest: scenario.document.digest,
    pdf_signature_digest: digest(pdfSignature),
  });
  const proof = await signedDownload(
    page,
    'Descargar consentimiento',
    scenario,
    testInfo,
    'document-consent.bin',
    'Firma externa del consentimiento',
  );
  expect(proof.bytes).toHaveLength(258);
  expect(proof.bytes.subarray(0, 10)).toEqual(Buffer.from('DOCSIGN1\x01\x01', 'binary'));
  expect(proof.bytes).toEqual(Buffer.from(prepared.consent_base64, 'base64'));
  for (const [uuid, offset] of [
    [scenario.litigator.id, 62],
    [prepared.operation_id, 142],
    [scenario.caseId, 158],
    [scenario.document.id, 174],
  ])
    expect(proof.bytes.subarray(offset, offset + 16)).toEqual(
      Buffer.from(uuid.replaceAll('-', ''), 'hex'),
    );
  expect(proof.bytes.readUInt32BE(190)).toBe(scenario.document.version);
  expect(verify('RSA-SHA256', pdf.bytes, proof.certificate.publicKey, pdfSignature)).toBe(true);
  await page.getByLabel('Confirmo firmar esta version con mi certificado').check();
  return {
    prepared,
    input: {
      consent_base64: prepared.consent_base64,
      pdf_signature_base64: pdfSignature.toString('base64'),
      consent_signature_base64: proof.signature.toString('base64'),
    },
  };
}

export async function screenshotSigning(page, testInfo, name) {
  await page.evaluate(() => document.activeElement?.blur());
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath(name + '.png'), fullPage: true });
}
