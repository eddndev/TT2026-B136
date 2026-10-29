import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { validateEvidence } from './evidence-validation-helpers.mjs';
import {
  bindingsPath,
  documentPath,
  navigate,
  button,
  downloaded,
  responseTo,
  jsonResponse,
  openSigningDocument,
  registerSigningCertificate,
  preparePersonalSignature,
  screenshotSigning,
} from './individual-signing-real-helpers.mjs';

for (const [name, width, height] of [
  ['desktop', 1440, 1000],
  ['mobile', 390, 844],
]) {
  test(
    'real individual signature preserves exact public evidence and receipt recovery on ' + name,
    async ({ page }, testInfo) => {
      test.setTimeout(180000);
      const scenario = fixture.individualSigning?.[name];
      expect(
        scenario?.litigator?.id,
        'The isolated Litigator fixture must be provisioned',
      ).toBeTruthy();
      expect(scenario.certificatePath).toBeTruthy();
      expect(scenario.privateKeyPath).toBeTruthy();
      if (name === 'desktop') {
        expect(scenario.localSigning?.encryptedKeyPath).toBeTruthy();
        expect(scenario.localSigning?.keyPasswordPath).toBeTruthy();
      } else expect(scenario.localSigning).toBeNull();
      const writes = [],
        errors = [];
      page.on('pageerror', (error) => errors.push(error.message));
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (
          request.method() === 'POST' &&
          (path.startsWith(bindingsPath + '/') || path.includes('/individual-signing/'))
        )
          writes.push({ path, data: request.postData() ? request.postDataJSON() : null });
      });
      await page.setViewportSize({ width, height });
      await page.goto('/');
      const authenticated = responseTo(page, '/api/v1/auth/mfa/recovery', 'POST');
      await loginAs(page, scenario.litigator, 0);
      const accessToken = (await jsonResponse(authenticated)).access_token;
      const binding = await registerSigningCertificate(page, scenario, testInfo);
      await screenshotSigning(page, testInfo, name + '-binding');
      const { prepared, input } = await preparePersonalSignature(page, scenario, testInfo);
      await screenshotSigning(page, testInfo, name + '-consent-review');
      const submitPath = documentPath(scenario) + '/individual-signing/submit';
      let finishInterception;
      const interception = new Promise((resolve) => {
        finishInterception = resolve;
      });
      await page.route(
        '**' + submitPath,
        async (route) => {
          try {
            const response = await route.fetch({ timeout: 30000 });
            const captured = { status: response.status(), receipt: await response.json() };
            await route.abort('failed');
            finishInterception(captured);
          } catch (error) {
            await route.abort('failed').catch(() => {});
            finishInterception({ error });
          }
        },
        { times: 1 },
      );
      let captured;
      try {
        await button(page, 'Enviar firma personal').click();
        captured = await interception;
      } finally {
        await page.unrouteAll({ behavior: 'wait' });
      }
      if (captured.error) throw captured.error;
      expect(captured.status).toBe(200);
      const original = captured.receipt;
      await expect(button(page, 'Consultar recibo de firma')).toBeVisible();
      expect(writes.filter((call) => call.path === submitPath)).toEqual([
        { path: submitPath, data: input },
      ]);
      expect(original).toMatchObject({
        status: 'accepted',
        operation_id: prepared.operation_id,
        consent_base64: prepared.consent_base64,
        account_id: scenario.litigator.id,
      });
      const revoked = await page.request.post('/api/v1/auth/logout', {
        headers: { Authorization: 'Bearer ' + accessToken },
      });
      expect(revoked.status()).toBe(204);
      await button(page, 'Consultar recibo de firma').click();
      await expect(
        page.getByRole('heading', { name: 'Accede a tu despacho.', exact: true }),
      ).toBeVisible();
      await loginAs(page, scenario.litigator, 1);
      await openSigningDocument(page, scenario);
      await button(page, 'Recuperar intento de firma').click();
      expect(writes.filter((call) => call.path === submitPath)).toHaveLength(1);
      const recovered = responseTo(
        page,
        documentPath(scenario) + '/individual-signing/receipts/' + prepared.operation_id,
      );
      await button(page, 'Consultar recibo de firma').click();
      expect(await jsonResponse(recovered)).toEqual(original);
      await expect(page.getByText('Firma personal aceptada', { exact: true })).toBeVisible();
      const receipt = await downloaded(
        page,
        button(page, 'Descargar recibo de firma'),
        testInfo,
        'document-signing-receipt.json',
      );
      expect(JSON.parse(receipt.bytes)).toEqual(original);
      await screenshotSigning(page, testInfo, name + '-receipt-recovered');
      const evidence = await downloaded(
        page,
        button(page, 'Descargar evidencia'),
        testInfo,
        'individual-evidence.zip',
      );
      expect(evidence.bytes.subarray(0, 2).toString()).toBe('PK');
      // The package carries the sealed manifest; the real engine in the panel
      // must read it as complete.
      if (name === 'desktop') await validateEvidence(page, evidence.path);
      await navigate(page, 'Mi firma documental');
      await expect(page.getByText('Vinculo registrado', { exact: true })).toBeVisible();
      await button(page, 'Retirar vinculo').click();
      await page.getByLabel('Confirmo retirar el vinculo para futuras firmas').check();
      const withdrawn = responseTo(
        page,
        bindingsPath + '/' + binding.binding_id + '/withdraw',
        'POST',
      );
      await button(page, 'Confirmar retiro').click();
      expect((await jsonResponse(withdrawn)).withdrawn_at).not.toBeNull();
      await expect(page.getByText('Vinculo retirado', { exact: true })).toBeVisible();
      await openSigningDocument(page, scenario);
      const checked = responseTo(page, documentPath(scenario) + '/individual-signing/verification');
      await button(page, 'Consultar estado de la firma').click();
      const report = await jsonResponse(checked);
      expect(report.kind).toBe('individual');
      expect(report.historical.verdict).toBe('valid');
      expect(report.signing).toEqual(original);
      expect(report.binding_withdrawn_at).not.toBeNull();
      await expect(page.getByText('Validez historica: valida', { exact: true })).toBeVisible();
      await expect(
        page.getByText('Vinculo retirado posteriormente', { exact: true }),
      ).toBeVisible();
      await screenshotSigning(page, testInfo, name + '-historical-after-withdrawal');
      expect(writes.map((call) => call.path)).toEqual([
        bindingsPath + '/' + binding.binding_id + '/prepare',
        bindingsPath + '/' + binding.binding_id + '/register',
        documentPath(scenario) + '/individual-signing/prepare',
        submitPath,
        bindingsPath + '/' + binding.binding_id + '/withdraw',
      ]);
      expect(Object.keys(writes[0].data)).toEqual(['certificate_base64']);
      expect(Object.keys(writes[1].data).sort()).toEqual([
        'certificate_der_base64',
        'signature_base64',
        'statement_base64',
      ]);
      expect(writes.at(-1).data).toBeNull();
      expect(errors).toEqual([]);
    },
  );
}
