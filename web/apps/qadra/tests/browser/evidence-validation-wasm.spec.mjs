// The evidence validation panel with the real Web Worker and WebAssembly
// module. tests/fixtures/evidence/repacked.zip holds evidence-like entries
// recompressed with deflate by a general ZIP tool, as happens when an export
// is unpacked and packed again, so the engine does not admit it.
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { versionSetup } from './version-helpers.mjs';

const repacked = readFileSync(new URL('../fixtures/evidence/repacked.zip', import.meta.url));

test('the real engine explains that a repacked ZIP is not the Qadra export', async ({ page }) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  const sent = [];
  page.on('request', (request) => sent.push(request.postDataBuffer()));
  await versionSetup(page);
  await page.getByRole('tab', { name: 'Evidencia', exact: true }).click();
  const panel = page.locator('.evidence-validation');
  await panel.getByLabel('Paquete de evidencia (ZIP)', { exact: true }).setInputFiles({
    name: 'repacked.zip',
    mimeType: 'application/zip',
    buffer: repacked,
  });
  await panel.getByRole('button', { name: 'Validar paquete', exact: true }).click();
  const result = panel.locator('.evidence-result');
  await expect(result.locator('.evidence-result-overall')).toHaveText('No compatible', {
    timeout: 30000,
  });
  await expect(result).toContainText(
    'Este archivo no es el paquete que export\u00f3 Qadra; si lo descomprimiste y volviste a comprimir, descarga de nuevo la evidencia.',
  );
  await expect(result).toContainText('evidence-verify');
  for (const body of sent) if (body) expect(body.includes(repacked)).toBe(false);
  expect(errors).toEqual([]);
});
