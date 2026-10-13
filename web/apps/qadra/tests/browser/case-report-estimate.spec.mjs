import { test, expect } from '@playwright/test';
import { report, pending, reportId } from './case-reports-fixtures.mjs';
import {
  setupReports,
  enterReports,
  openReport,
  updateReport,
  details,
} from './case-reports-helpers.mjs';

for (const width of [1440, 390]) {
  test(`report estimate and provider email states remain distinct at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = await setupReports(page);
    state.records.set(reportId, pending({ estimated_total_ms: 65_000, ready_email: null }));
    await enterReports(page);
    await openReport(page);
    const detail = details(page);
    await expect(detail.getByText('Estimaci\u00f3n inicial', { exact: true })).toBeVisible();
    await expect(detail.getByText('1 min 5 s', { exact: true })).toBeVisible();
    await expect(detail.getByText(/estimaci\u00f3n incluye la espera/i)).toBeVisible();
    await expect(detail.getByText('Duraci\u00f3n observada', { exact: true })).toHaveCount(0);
    await expect(detail.getByText(/sin aviso por correo registrado/i)).toBeVisible();

    const messages = [
      ['pending', /aviso por correo pendiente/i],
      ['accepted', /correo aceptado por el proveedor/i],
      ['uncertain', /no se pudo confirmar la aceptaci\u00f3n del correo/i],
      ['rejected', /el proveedor rechaz\u00f3 el correo/i],
      ['access_revoked', /aviso por correo detenido por cambio de acceso/i],
    ];
    for (const [ready_email, message] of messages) {
      state.records.set(reportId, report({ estimated_total_ms: 65_000, ready_email }));
      await updateReport(page);
      await expect(detail.getByText(message)).toBeVisible();
      await expect(detail.getByText('1 min 5 s', { exact: true })).toBeVisible();
      await expect(detail.getByText('Duraci\u00f3n observada', { exact: true })).toBeVisible();
      await expect(detail.getByText('1 min', { exact: true })).toBeVisible();
      await expect(
        detail.getByRole('button', { name: 'Descargar PDF', exact: true }),
      ).toBeEnabled();
      if (ready_email === 'accepted') {
        await expect(
          detail.getByText(/no confirma entrega en la bandeja de entrada/i),
        ).toBeVisible();
      }
    }
    expect(state.requests).toHaveLength(0);
    expect(state.acknowledgments).toHaveLength(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}

test('legacy and insufficient history show unavailable estimation without inventing duration', async ({
  page,
}) => {
  const state = await setupReports(page);
  state.records.set(reportId, pending());
  await enterReports(page);
  await openReport(page);
  const detail = details(page);
  await expect(detail.getByText('Estimaci\u00f3n no disponible.', { exact: true })).toBeVisible();
  await expect(detail.getByText('Duraci\u00f3n observada', { exact: true })).toHaveCount(0);
  state.records.set(reportId, report({ estimated_total_ms: null, ready_email: null }));
  await updateReport(page);
  await expect(detail.getByText('Estimaci\u00f3n no disponible.', { exact: true })).toBeVisible();
  await expect(detail.getByText('1 min', { exact: true })).toBeVisible();
  await expect(detail.getByText(/sin aviso por correo registrado/i)).toBeVisible();
});
