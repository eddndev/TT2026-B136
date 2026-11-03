import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { createHash, verify } from 'node:crypto';
import { contractSetup, openRequest } from './contract-helpers.mjs';
import { login, navigate } from './helpers.mjs';
import { localSignatureMaterial, signLocally } from './local-signature-material.mjs';

test.use({ channel: 'chromium' });
const stub = readFileSync(new URL('./evidence-worker-stub.js', import.meta.url), 'utf8');

test('the firm signer validates the partial acknowledgement and counter-signs', async ({
  page,
}) => {
  await page.route('**/evidence-verify-worker.mjs*', (route) =>
    route.fulfill({ contentType: 'text/javascript', body: stub }),
  );
  const material = localSignatureMaterial();
  const state = await contractSetup(page, 'litigator', material);
  await login(page, false, false);
  await navigate(page, 'Por firmar');
  const region = await openRequest(page, 'Por firmar');
  await expect(region.getByText('Cliente Demo')).toBeVisible();
  const downloaded = page.waitForEvent('download');
  await region.getByRole('button', { name: 'Descargar acuse parcial' }).click();
  expect((await downloaded).suggestedFilename()).toBe('contrato-acuse-parcial.zip');
  await region.getByRole('button', { name: 'Cargar y comprobar el PDF' }).click();
  await region.getByLabel(/Confirmo firmar por el despacho/).check();
  const step = region.getByRole('region', { name: 'Firma el contrato' });
  const local = region.getByRole('region', { name: /^Firma local:/ });
  await expect(step.getByText('Completa los pasos anteriores para firmar.')).toBeVisible();
  await expect(local).toHaveCount(0);
  const panel = region.locator('.evidence-validation');
  const requestDigest = createHash('sha256')
    .update(Buffer.from(state.detail.statement_base64, 'base64'))
    .digest('hex');
  // A partial package of another request does not unlock the counter-signature.
  await validatePartial(panel, 'cd'.repeat(32));
  await expect(panel.locator('.evidence-result-overall')).toHaveText('Parcial');
  await expect(local).toHaveCount(0);
  expect(state.writes.filter((row) => row.path.endsWith('/prepare'))).toEqual([]);
  await validatePartial(panel, requestDigest);
  const result = panel.locator('.evidence-result');
  await expect(result.locator('.evidence-result-overall')).toHaveText('Parcial');
  await expect(result).toContainText('Acuse parcial: no acredita contrato');
  await expect(local).toHaveCount(1);
  await signLocally(local, material);
  await expect(local.getByText('Firma local preparada')).toBeVisible();
  await region.getByRole('button', { name: 'Enviar firma' }).click();
  await expect(region.getByText('el contrato qued\u00f3 formalizado')).toBeVisible();
  const sent = state.writes.find((row) => row.path.endsWith('/sign')).body;
  expect(
    verify(
      'sha256',
      Buffer.from(sent.statement_base64, 'base64'),
      material.publicKey,
      Buffer.from(sent.signature_base64, 'base64'),
    ),
  ).toBe(true);
  expect(state.prepared.position).toBe(2);
  const final = page.waitForEvent('download');
  await region.getByRole('button', { name: 'Descargar contrato firmado' }).click();
  expect((await final).suggestedFilename()).toBe('contrato-firmado.zip');
});

async function validatePartial(panel, requestDigest) {
  await panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true }).setInputFiles({
    name: 'contrato-acuse-parcial.zip',
    mimeType: 'application/zip',
    buffer: Buffer.from(
      JSON.stringify({
        overall: 'indeterminate',
        delay: 0,
        profile: 'bilateral-contract-partial-v1',
        digests: [{ name: 'contrato-solicitud.bin', sha256: requestDigest }],
      }),
    ),
  });
  await panel.getByRole('button', { name: 'Validar paquete', exact: true }).click();
}

test('the firm signer declines with a separate confirmation', async ({ page }) => {
  const state = await contractSetup(page, 'litigator', localSignatureMaterial());
  await login(page, false, false);
  await navigate(page, 'Por firmar');
  const region = await openRequest(page, 'Por firmar');
  await region.getByRole('button', { name: 'Declinar firma' }).click();
  await region.getByRole('button', { name: 'Confirmar declinaci\u00f3n' }).click();
  await expect(region.getByText('Declinaste la firma.')).toBeVisible();
  expect(state.writes.find((row) => row.path.endsWith('/close')).body.outcome).toBe('decline');
});

test('the requester cancels an open request from the case contracts with a reason', async ({
  page,
}) => {
  const state = await contractSetup(page, 'litigator', localSignatureMaterial(), {
    caseRequester: true,
    status: 'awaiting_client',
  });
  await login(page);
  await page.getByRole('link', { name: 'Contratos', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Contratos', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Abrir Contrato de prestacion de servicios' }).click();
  const detail = page.getByRole('region', { name: 'Detalle del contrato' });
  await expect(detail.getByText('Espera la firma del Cliente')).toBeVisible();
  await expect(detail.getByText('Firmas solicitadas')).toBeVisible();
  await detail.getByRole('button', { name: 'Cancelar solicitud' }).click();
  await detail.getByRole('button', { name: 'Confirmar cancelaci\u00f3n' }).click();
  await expect(detail.getByRole('alert')).toContainText('motivo');
  expect(state.writes).toEqual([]);
  await detail.getByLabel('Motivo', { exact: true }).fill('Se corrigio el documento');
  await detail.getByRole('button', { name: 'Confirmar cancelaci\u00f3n' }).click();
  await expect(detail.getByText('Solicitud cancelada.')).toBeVisible();
  const closed = state.writes.find((row) => row.path.endsWith('/close')).body;
  expect(closed).toMatchObject({ outcome: 'cancel', reason: 'Se corrigio el documento' });
});
