import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';

if (!fixture.alerts) throw new Error('The live hearing fixture must provision personal alerts');
const scenario = fixture.alerts;
const card = (page, id) => page.locator(`[data-alert-id="${id}"]`);
const responseTo = (page, path, method = 'GET') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === path && response.request().method() === method,
  );

async function refresh(page) {
  const pending = responseTo(page, '/api/v1/alerts');
  await page.getByRole('button', { name: 'Actualizar alertas', exact: true }).click();
  const response = await pending;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
  return response.json();
}

for (const [name, width] of [
  ['desktop', 1440],
  ['mobile', 390],
])
  test(`real personal alerts retain exact hearing history, read state and preferences at ${width}px`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(90000);
    const actor = scenario[name],
      errors = [],
      writes = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    // Hearing permissions use recovery code 0; this independent session uses 1.
    await loginAs(page, actor, 1);
    page.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (path.startsWith('/api/v1/') && request.method() !== 'GET')
        writes.push([request.method(), path]);
    });
    await navigate(page, 'Alertas');
    await expect(page.locator('.alerts-list')).toHaveAttribute('aria-busy', 'false');
    let alert;
    await expect
      .poll(
        async () => {
          const result = await refresh(page);
          alert = result.alerts.find((row) => row.subject.id === scenario.hearing.id);
          return alert !== undefined;
        },
        { timeout: 60000, intervals: [250, 500, 1000] },
      )
      .toBe(true);
    expect(alert).toMatchObject({
      recipient_id: actor.id,
      subject: { kind: 'hearing', case_id: scenario.case.id, id: scenario.hearing.id },
      case_title: scenario.case.title,
      case_reference: scenario.case.reference,
      origin: { revision: 1, evidence_digest: scenario.hearing.receipt.submission_digest },
      kind: { kind: 'upcoming', lead_hours: 48 },
      state: { kind: 'active' },
      email: { kind: 'disabled' },
      read_at: null,
    });
    await expect(card(page, alert.id)).toContainText('48 horas antes');
    await expect(card(page, alert.id)).toContainText('Correo deshabilitado');
    const exactPath = `/api/v1/cases/${scenario.case.id}/hearings/${scenario.hearing.id}/revisions/1`;
    const exactResponse = responseTo(page, exactPath);
    await card(page, alert.id)
      .getByRole('button', { name: 'Abrir audiencia', exact: true })
      .click();
    const exact = await exactResponse;
    expect(exact.status()).toBe(200);
    expect(await exact.json()).toEqual(scenario.hearing);
    await expect(
      page.getByRole('region', { name: 'Detalle de audiencia', exact: true }),
    ).toContainText(/consultada exactamente/i);
    expect(writes).toEqual([]);
    await page.getByRole('button', { name: 'Volver a Alertas', exact: true }).click();
    await expect(card(page, alert.id)).toContainText('Sin leer');
    const reading = responseTo(page, `/api/v1/alerts/${alert.id}/read`, 'POST');
    await card(page, alert.id)
      .getByRole('button', { name: 'Marcar como le\u00edda', exact: true })
      .click();
    const receiptResponse = await reading;
    expect(receiptResponse.status()).toBe(200);
    const receipt = await receiptResponse.json();
    expect(receipt.alert.read_at).not.toBeNull();
    await expect(card(page, alert.id).getByText('Le\u00edda', { exact: true })).toBeVisible();
    expect((await refresh(page)).alerts.find((row) => row.id === alert.id).read_at).toEqual(
      receipt.alert.read_at,
    );
    await page.getByRole('button', { name: 'Preferencias de alertas', exact: true }).click();
    const preferences = page.getByRole('region', { name: 'Preferencias de alertas', exact: true });
    await expect(
      preferences.getByLabel('Anticipaciones de audiencias (horas)', { exact: true }),
    ).toHaveValue('48,24');
    await preferences.getByLabel('Anticipaciones de plazos (horas)', { exact: true }).fill('72,24');
    const saving = responseTo(page, '/api/v1/alert-preferences', 'PUT');
    await preferences.getByRole('button', { name: 'Guardar preferencias', exact: true }).click();
    const savedResponse = await saving;
    expect(savedResponse.status()).toBe(200);
    const saved = (await savedResponse.json()).preferences;
    expect(saved).toMatchObject({ user_id: actor.id, revision: 1, email_transport: 'disabled' });
    expect(saved.values.deadline_upcoming.lead_hours).toEqual([72, 24]);
    await expect(page.getByText('Preferencias guardadas.', { exact: true })).toBeVisible();
    const stored = responseTo(page, '/api/v1/alert-preferences');
    await page.getByRole('button', { name: 'Preferencias de alertas', exact: true }).click();
    const storedResponse = await stored;
    expect(storedResponse.status()).toBe(200);
    expect((await storedResponse.json()).preferences).toEqual(saved);
    await expect(
      preferences.getByLabel('Anticipaciones de plazos (horas)', { exact: true }),
    ).toHaveValue('72,24');
    await preferences.getByRole('button', { name: 'Cancelar', exact: true }).click();
    expect((await refresh(page)).alerts.find((row) => row.id === alert.id).origin).toEqual(
      alert.origin,
    );
    expect(writes).toEqual([
      ['POST', `/api/v1/alerts/${alert.id}/read`],
      ['PUT', '/api/v1/alert-preferences'],
    ]);
    expect((await page.request.get('/api/v1/alerts')).status()).toBe(401);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath(`alerts-real-${name}.png`), fullPage: true });
    expect(errors).toEqual([]);
  });
