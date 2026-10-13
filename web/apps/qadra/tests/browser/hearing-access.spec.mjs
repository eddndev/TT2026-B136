import { test, expect } from '@playwright/test';
import { setupHearings, openHearings, hearingEditor, fillHearing } from './hearing-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { login, navigate } from './helpers.mjs';
for (const role of ['paralegal', 'litigator']) {
  test(`${role} has the intended hearing actions`, async ({ page }) => {
    const record = hearingRecord();
    await setupHearings(page, { role, records: [record] });
    await openHearings(page);
    await page
      .getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true })
      .click();
    await expect(
      page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Programar audiencia', exact: true }),
    ).toHaveCount(role === 'paralegal' ? 0 : 1);
    await expect(page.getByRole('button', { name: 'Cancelar audiencia', exact: true })).toHaveCount(
      role === 'paralegal' ? 0 : 1,
    );
    await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
    await expect(
      page.getByRole('region', { name: 'Historial de audiencia', exact: true }),
    ).toBeVisible();
  });
}
test('client has no hearing or agenda routes', async ({ page }) => {
  const { state } = await setupHearings(page, { role: 'client' });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Audiencias', exact: true })).toHaveCount(0);
  await expect(
    page.getByRole('navigation').getByRole('button', { name: 'Agenda', exact: true }),
  ).toHaveCount(0);
  expect(state.calls).toHaveLength(0);
});
test('closed cases retain hearing detail and history while mutations are disabled', async ({
  page,
}) => {
  const record = hearingRecord();
  await setupHearings(page, { records: [record], closed: true });
  await openHearings(page);
  await page.getByRole('button', { name: `Consultar audiencia ${record.id}`, exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Programar audiencia', exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }),
  ).toHaveCount(0);
  await page.getByRole('button', { name: 'Ver historial de audiencias', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Historial de audiencia', exact: true }),
  ).toBeVisible();
});
test('lost assignment clears the hearing form and protected context', async ({ page }) => {
  const { state } = await setupHearings(page, { role: 'litigator' });
  await openHearings(page);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await fillHearing(page);
  state.denied = true;
  await hearingEditor(page).getByRole('button', { name: 'Revisar registro', exact: true }).click();
  await expect(hearingEditor(page)).toHaveCount(0);
  await expect(
    page.getByRole('heading', { name: 'Audiencias del expediente', exact: true }),
  ).toHaveCount(0);
  await expect(page.getByText('Sala privada declarada', { exact: true })).toHaveCount(0);
  expect(state.submissions).toHaveLength(0);
});
