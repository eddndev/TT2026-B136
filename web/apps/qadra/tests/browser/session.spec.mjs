import { test, expect } from '@playwright/test';
import {
  caseId,
  otherCaseId,
  id,
  document,
  setup,
  login,
  navigate,
  selectCase,
  signOut,
  openSection,
} from './helpers.mjs';

async function waitForRendering(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

test('revoked access clears document rows and open metadata on the next read', async ({ page }) => {
  await setup(page);
  await login(page);
  await page.getByRole('button', { name: /contrato.pdf/ }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toBeVisible();
  await page.route(`**/cases/${caseId}/documents?**`, (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'case_not_found' } } }),
  );
  await page.getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('ya no tienes acceso');
  await expect(page.getByText('contrato.pdf', { exact: true })).toHaveCount(0);
});
