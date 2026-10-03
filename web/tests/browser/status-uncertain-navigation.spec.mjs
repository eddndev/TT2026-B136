import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';

for (const applied of [false, true]) {
  test(`uncertain case status survives section navigation until an explicit successful read, applied=${applied}`, async ({
    page,
  }) => {
    await setup(page);
    let current = administration(caseRecord);
    let reads = 0;
    let holdRead = false;
    let readEntered = false;
    let readCompleted = false;
    let release;
    const writes = [];
    const gate = new Promise((resolve) => {
      release = resolve;
    });
    await page.route(`**/cases/${caseRecord.id}/administration`, async (route) => {
      expect(route.request().method()).toBe('GET');
      reads++;
      if (holdRead) {
        readEntered = true;
        await gate;
      }
      await route.fulfill({ json: current });
      if (holdRead) readCompleted = true;
    });
    await page.route(`**/cases/${caseRecord.id}/administrative-status`, async (route) => {
      expect(route.request().method()).toBe('PUT');
      const command = route.request().postDataJSON();
      expect(command).toEqual({
        expected_revision: 1,
        administrative_status: 'closed',
      });
      writes.push(command);
      if (writes.length === 1) {
        if (applied) current = administration(caseRecord, 2, null, 'closed');
        return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
      }
      expect(writes).toHaveLength(2);
      expect(applied).toBe(false);
      expect(readCompleted).toBe(true);
      current = administration(caseRecord, 2, null, 'closed');
      return route.fulfill({ json: current });
    });
    await page.route(`**/cases/${caseRecord.id}/participants?*`, (route) => {
      expect(route.request().method()).toBe('GET');
      return route.fulfill({
        json: { participants: [], has_more: false, next_after_id: null },
      });
    });

    try {
      await login(page, false, false);
      await navigate(page, 'Expedientes');
      await page.getByRole('button', { name: /Defensa inicial/ }).click();
      const start = page.getByRole('button', {
        name: 'Cerrar administrativamente',
        exact: true,
      });
      await start.click();
      const dialog = page.getByRole('dialog', {
        name: 'Cerrar administrativamente',
        exact: true,
      });
      const confirm = dialog.getByRole('button', {
        name: 'Confirmar cierre administrativo',
        exact: true,
      });
      const before = reads;
      await confirm.click();
      await expect(dialog.getByRole('alert')).toBeVisible();
      await expect(confirm).toBeDisabled();
      expect(writes).toHaveLength(1);
      expect(reads).toBe(before);
      await dialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
      await expect(dialog).not.toBeVisible();

      await page.getByRole('link', { name: 'Participantes', exact: true }).click();
      await expect(
        page.getByRole('region', { name: 'Directorio del expediente', exact: true }),
      ).toHaveAttribute('aria-busy', 'false');
      await page.getByRole('link', { name: 'Resumen', exact: true }).click();
      await expect(
        page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
      ).toBeVisible();
      await start.click();
      await expect(confirm).toBeDisabled();
      expect(reads).toBe(before);
      expect(writes).toHaveLength(1);

      holdRead = true;
      await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
      await expect.poll(() => readEntered).toBe(true);
      await expect(
        dialog.getByRole('button', { name: 'Guardando...', exact: true }),
      ).toBeDisabled();
      expect(reads).toBe(before + 1);
      expect(readCompleted).toBe(false);
      expect(writes).toHaveLength(1);
      release();
      await expect(dialog.getByRole('status')).toContainText(
        applied ? 'ya tiene el estado' : 'Datos actuales consultados',
      );
      expect(writes).toHaveLength(1);
      if (applied) {
        await expect(dialog).toContainText('Revisi\u00f3n 2');
        await expect(confirm).toBeDisabled();
      } else {
        await expect(confirm).toBeEnabled();
        await confirm.click();
        await expect(dialog).not.toBeVisible();
        expect(writes).toHaveLength(2);
        await expect(
          page.getByRole('button', { name: 'Reactivar expediente', exact: true }),
        ).toBeVisible();
      }
      expect(reads).toBe(before + 1);
    } finally {
      release();
    }
  });
}
