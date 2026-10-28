// Validates a downloaded evidence package in the browser with the real
// engine. scripts/web-demo.sh pins its disposable CA with the label "CI" and
// writes the CRL before the server starts, so the server seal of a version
// sealed during the run is complete.
import { readFileSync } from 'node:fs';
import { expect } from '@playwright/test';

export async function validateEvidence(page, archive) {
  const bytes = readFileSync(archive);
  const sent = [];
  const record = (request) => sent.push(request.postDataBuffer());
  page.on('request', record);
  await page.getByRole('tab', { name: 'Evidencia', exact: true }).click();
  const panel = page.locator('.evidence-validation');
  await panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true }).setInputFiles({
    name: 'evidencia.zip',
    mimeType: 'application/zip',
    buffer: bytes,
  });
  await panel.getByRole('button', { name: 'Validar paquete', exact: true }).click();
  const result = panel.locator('.evidence-result');
  await expect(result.locator('.evidence-result-overall')).toHaveText('Completo', {
    timeout: 30000,
  });
  await expect(result).toContainText('Firma correcta');
  await expect(result).toContainText('Ra\u00edz fija: CI');
  page.off('request', record);
  for (const body of sent) if (body) expect(body.includes(bytes)).toBe(false);
}
