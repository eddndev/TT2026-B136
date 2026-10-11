import { test, expect } from '@playwright/test';
import { panel, auditPage, setupAudit, enterAudit, consult } from './audit-events-helpers.mjs';

const account = '20000000-0000-4000-8000-000000000001';
const request = '30000000-0000-4000-8000-000000000001';
const context = (change = {}) => ({
  user_id: account,
  ip_address: '2001:db8::17',
  source: 'http',
  request_id: request,
  ...change,
});
const field = (record, label) =>
  record.getByText(label, { exact: true }).locator('..').locator('dd');

for (const width of [1440, 390]) {
  test(`Owner reads verified context beside captured history at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupAudit(page);
    state.value = auditPage(0, 2);
    Object.assign(state.value.events[0], {
      actor: 'captured-owner@example.test',
      action: 'access.denied',
      context: context(),
    });
    Object.assign(state.value.events[1], {
      actor: 'historical-actor@example.test',
      action: 'document.read',
      context: null,
    });
    await enterAudit(page);
    await consult(page);
    const current = panel(page).locator('[data-sequence="0"]');
    const historical = panel(page).locator('[data-sequence="1"]');
    await expect(field(current, 'Actor registrado')).toHaveText('captured-owner@example.test');
    await expect(field(current, 'Identidad de cuenta')).toHaveText(account);
    await expect(field(current, 'IP de origen')).toHaveText('2001:db8::17');
    await expect(field(current, 'Origen')).toHaveText('HTTP');
    await expect(field(current, 'Resultado')).toHaveText('Denegado');
    await expect(field(historical, 'Actor registrado')).toHaveText('historical-actor@example.test');
    for (const label of ['Identidad de cuenta', 'IP de origen', 'Origen', 'Resultado'])
      await expect(field(historical, label)).toHaveText('No disponible');
    await expect(historical).not.toContainText(account);
    await expect(page.getByRole('button', { name: 'Verificar cadena', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
  });
}

test('access outcomes preserve rejected ambiguity and internal origins do not fabricate identity', async ({
  page,
}) => {
  const state = await setupAudit(page);
  const outcomes = [
    ['access.granted', 'Concedido'],
    ['access.denied', 'Denegado'],
    ['access.rejected', 'Rechazado'],
    ['access.failed', 'Fallo'],
  ];
  state.value = auditPage(0, 6);
  outcomes.forEach(([action], index) => {
    Object.assign(state.value.events[index], { action, context: context() });
  });
  for (const [index, source] of [
    [4, 'cli'],
    [5, 'internal'],
  ])
    Object.assign(state.value.events[index], {
      action: 'audit.appended',
      context: context({ source, user_id: null, ip_address: null, request_id: null }),
    });
  await enterAudit(page);
  await consult(page);
  for (const [index, [, result]] of outcomes.entries())
    await expect(field(panel(page).locator(`[data-sequence="${index}"]`), 'Resultado')).toHaveText(
      result,
    );
  for (const [index, source] of [
    [4, 'CLI'],
    [5, 'Interno'],
  ]) {
    const record = panel(page).locator(`[data-sequence="${index}"]`);
    await expect(field(record, 'Origen')).toHaveText(source);
    await expect(field(record, 'Identidad de cuenta')).toHaveText('No disponible');
    await expect(field(record, 'IP de origen')).toHaveText('No disponible');
    await expect(field(record, 'Resultado')).toHaveText('No disponible');
  }
});

test('Owner filters by verified account UUID using the existing actor input and route', async ({
  page,
}) => {
  const state = await setupAudit(page);
  Object.assign(state.value.events[0], {
    actor: 'captured-owner@example.test',
    context: context(),
  });
  await enterAudit(page);
  await panel(page).getByLabel('Actor registrado', { exact: true }).fill(account);
  await consult(page);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(1);
  expect(state.calls).toHaveLength(1);
  expect(state.calls[0].pathname).toBe('/api/v1/audit/events');
  expect(state.calls[0].searchParams.get('actor')).toBe(account);
  expect(state.calls[0].searchParams.has('user_id')).toBe(false);
  await expect(field(panel(page).locator('[data-sequence="0"]'), 'Actor registrado')).toHaveText(
    'captured-owner@example.test',
  );
});

test('malformed context clears previous private results without rendering forged metadata', async ({
  page,
}) => {
  const state = await setupAudit(page);
  state.value.events[0].context = null;
  await enterAudit(page);
  await consult(page);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(1);
  state.value.events[0].context = context({ ip_address: '<img src=x onerror=alert(1)>' });
  await panel(page).getByRole('button', { name: 'Actualizar actividad', exact: true }).click();
  await expect(panel(page).getByRole('alert')).toBeVisible();
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(0);
  await expect(panel(page).locator('img, script')).toHaveCount(0);
});
