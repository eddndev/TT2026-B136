import { test, expect } from '@playwright/test';
import { csr, portalFixture } from './portal-fixture.mjs';

for (const width of [1440, 390]) {
  test(`CA issuance recovery, revocation and publication at ${width}px`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 1000 });
    const calls = await portalFixture(page);
    await page.goto('/');
    await page.getByLabel('Correo electr\u00f3nico').fill('ca@example.test');
    await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('SyntheticPassword2026!');
    await page.getByRole('button', { name: 'Continuar', exact: true }).click();
    await page.getByLabel('C\u00f3digo de autenticaci\u00f3n', { exact: true }).fill('123456');
    await page.getByRole('button', { name: 'Verificar y entrar' }).click();
    await expect(page.getByRole('heading', { name: 'Operaciones de Qadra CA.' })).toBeVisible();
    await page.getByLabel('CSR p\u00fablico en formato PEM').fill(csr);
    await page.getByLabel('Titular revisado').fill('Titular Demo');
    await page.getByLabel('Referencia de la solicitud').fill('Expediente demostrativo');
    await page.getByLabel('Criterio de emisi\u00f3n').fill('Identidad revisada en la prueba');
    await page.getByRole('button', { name: 'Revisar solicitud', exact: true }).click();
    await page.getByLabel('He revisado el titular', { exact: false }).check();
    await page.getByRole('button', { name: 'Confirmar y emitir' }).click();
    await page.getByRole('button', { name: 'Retomar esta operaci\u00f3n' }).click();
    await expect(page.getByText('Emisi\u00f3n confirmada.', { exact: true })).toBeVisible();
    expect(calls.filter((value) => value.action === 'issue')).toHaveLength(1);
    await page.getByRole('button', { name: 'Certificados', exact: true }).click();
    const download = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Descargar certificado', exact: true }).click();
    expect((await download).suggestedFilename()).toBe('certificado-1000.crt.pem');
    await page.getByRole('button', { name: 'Revocar certificado', exact: true }).click();
    await page.getByLabel('Motivo de la revocaci\u00f3n').fill('Cierre de la prueba');
    await page.getByLabel('Confirmo la serie', { exact: false }).check();
    await page.getByRole('button', { name: 'Confirmar revocaci\u00f3n local' }).click();
    await expect(page.getByText('Pendiente de publicar en Qadra', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Publicaci\u00f3n', exact: true }).click();
    await page.getByLabel('Confirmo la revisi\u00f3n 3', { exact: false }).check();
    await page.getByRole('button', { name: 'Renovar y publicar CRL' }).click();
    await expect(page.getByText('CRL publicada en Qadra.', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Certificados', exact: true }).click();
    await expect(page.getByText('Revocado en la CRL de Qadra', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Historial', exact: true }).click();
    await expect(page.locator('.ca-record')).toHaveCount(3);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath(`ca-history-${width}.png`), fullPage: true });
    await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
    await expect(page.getByLabel('Contrase\u00f1a', { exact: true })).toBeVisible();
    expect(await page.evaluate(() => [localStorage.length, sessionStorage.length])).toEqual([0, 0]);
  });
}
