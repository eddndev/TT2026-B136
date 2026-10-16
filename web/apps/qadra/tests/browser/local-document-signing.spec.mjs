import { test, expect } from '@playwright/test';
import { verify } from 'node:crypto';
import { login, selectCase, openDocument } from './helpers.mjs';
import { signingSetup, pdf } from './individual-signing-fixture.mjs';
import {
  localSignatureMaterial,
  localFile,
  signLocally,
  localSigningViewports,
  captureLocalSignature,
} from './local-signature-material.mjs';

// The full Chromium headless mode supports embedded PDF viewing without forced downloads.
test.use({ channel: 'chromium' });

for (const { name, viewport } of localSigningViewports) {
  test(`Litigator locally signs binding, exact PDF and consent without automatic submission (${name})`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    const material = localSignatureMaterial();
    const state = await signingSetup(page, 'litigator', material);
    const bodies = [];
    const downloads = [];
    page.on('download', (value) => downloads.push(value.suggestedFilename()));
    page.on('request', (request) => {
      if (request.postData()) bodies.push(request.postData());
    });
    await login(page, false, false);
    const menu = page.getByRole('button', { name: 'Abrir men\u00fa', exact: true });
    if (await menu.isVisible()) await menu.click();
    await page.getByRole('button', { name: 'Mi firma documental', exact: true }).click();
    await page
      .getByLabel('Certificado publico PEM', { exact: true })
      .setInputFiles(localFile('browser-test-certificate.pem', material.certificatePem));
    await page.getByRole('button', { name: 'Preparar vinculo', exact: true }).click();
    const local = page.getByRole('region', { name: /^Firma local:/ });
    await expect(local).toHaveCount(1);
    await signLocally(local, material);
    const confirmation = page.getByLabel('Confirmo vincular este certificado a mi cuenta');
    await expect(confirmation).toBeEnabled();
    await expect(confirmation).not.toBeChecked();
    expect(state.writes.filter((value) => value.path.endsWith('/register'))).toEqual([]);
    await captureLocalSignature(local, testInfo, `litigator-binding-${name}`);
    await confirmation.check();
    await page.getByRole('button', { name: 'Registrar vinculo', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Vinculo registrado', exact: true }),
    ).toBeVisible();
    const registered = JSON.parse(
      state.writes.find((value) => value.path.endsWith('/register')).body,
    );
    expect(
      verify(
        'sha256',
        Buffer.from(registered.statement_base64, 'base64'),
        material.publicKey,
        Buffer.from(registered.signature_base64, 'base64'),
      ),
    ).toBe(true);

    await selectCase(page);
    await openDocument(page);
    await page.getByRole('button', { name: 'Firmar personalmente', exact: true }).click();
    await page.getByRole('button', { name: 'Revisar PDF', exact: true }).click();
    await expect(page.getByTitle('PDF exacto de esta version')).toBeVisible();
    await page.getByLabel('He revisado este PDF y confirmo su contenido').check();
    const localPdf = page.getByRole('region', { name: 'Firma local: PDF', exact: true });
    await signLocally(localPdf, material);
    const consentButton = page.getByRole('button', {
      name: 'Preparar consentimiento',
      exact: true,
    });
    await expect(consentButton).toBeEnabled();
    expect(state.writes.filter((value) => value.path.includes('/individual-signing/'))).toEqual([]);
    await captureLocalSignature(localPdf, testInfo, `document-pdf-${name}`);
    await consentButton.click();
    const localConsent = page.getByRole('region', {
      name: 'Firma local: consentimiento',
      exact: true,
    });
    await expect(localConsent).toBeVisible();
    const prepared = JSON.parse(
      state.writes.find((value) => value.path.endsWith('/individual-signing/prepare')).body,
    );
    expect(
      verify(
        'sha256',
        pdf,
        material.publicKey,
        Buffer.from(prepared.pdf_signature_base64, 'base64'),
      ),
    ).toBe(true);
    const finalConfirmation = page.getByLabel('Confirmo firmar esta version con mi certificado');
    await expect(finalConfirmation).toBeEnabled();
    await expect(finalConfirmation).not.toBeChecked();
    await expect(
      localConsent.getByRole('button', { name: 'Firmar localmente', exact: true }),
    ).toBeDisabled();
    await expect(localConsent.getByLabel('Clave privada cifrada', { exact: true })).toBeDisabled();
    expect(state.writes.filter((value) => value.path.endsWith('/submit'))).toEqual([]);
    await finalConfirmation.check();
    await signLocally(localConsent, material);
    await expect(
      page.getByRole('button', { name: 'Enviar firma personal', exact: true }),
    ).toBeEnabled();
    await expect(finalConfirmation).toBeChecked();
    expect(state.writes.filter((value) => value.path.endsWith('/submit'))).toEqual([]);
    await captureLocalSignature(localConsent, testInfo, `document-consent-${name}`);
    await page.getByRole('button', { name: 'Enviar firma personal', exact: true }).click();
    await expect(
      page.getByRole('heading', { name: 'Firma personal aceptada', exact: true }),
    ).toBeVisible();
    const submitted = JSON.parse(state.writes.find((value) => value.path.endsWith('/submit')).body);
    expect(
      verify(
        'sha256',
        Buffer.from(submitted.consent_base64, 'base64'),
        material.publicKey,
        Buffer.from(submitted.consent_signature_base64, 'base64'),
      ),
    ).toBe(true);
    expect(submitted.pdf_signature_base64).toBe(prepared.pdf_signature_base64);
    expect(Object.keys(submitted).sort()).toEqual([
      'consent_base64',
      'consent_signature_base64',
      'pdf_signature_base64',
    ]);
    expect(bodies.join('\n')).not.toContain(material.password);
    expect(bodies.join('\n')).not.toContain('ENCRYPTED PRIVATE KEY');
    expect(downloads).toEqual([]);
  });
}
