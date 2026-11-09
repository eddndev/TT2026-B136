import { test, expect } from '@playwright/test';
import { login, expectLanding } from './helpers.mjs';
import { panel, auditPage, setupAudit, enterAudit, consult } from './audit-events-helpers.mjs';

for (const width of [1440, 390]) {
  test(`Owner reads exact escaped event values without redesign at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupAudit(page);
    const value = auditPage();
    value.events[0].resource = '<img src=x onerror="window.auditInjected=true">';
    value.events[0].sequence = '9007199254740993';
    state.value = value;
    await enterAudit(page);
    expect(state.calls).toHaveLength(0);
    await consult(page);
    await expect(panel(page).locator('[data-sequence="9007199254740993"]')).toBeVisible();
    await expect(panel(page)).toContainText(value.events[0].resource);
    await expect(panel(page).locator('img, script')).toHaveCount(0);
    expect(await page.evaluate(() => window.auditInjected)).toBeUndefined();
    await expect(panel(page).locator('time').last()).toHaveAttribute(
      'datetime',
      value.events[0].timestamp,
    );
    await expect(page.getByRole('button', { name: 'Verificar cadena', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Verificar cadena', exact: true }).click();
    await expect(
      page
        .getByRole('region', { name: 'Integridad de la cadena', exact: true })
        .getByText('Cadena \u00edntegra', { exact: true }),
    ).toBeVisible();
    expect(state.calls).toHaveLength(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({
      path: testInfo.outputPath(`audit-events-${width}.png`),
      fullPage: true,
    });
  });
}
test('exact filters survive pagination and each page replaces bounded prior data', async ({
  page,
}) => {
  const state = await setupAudit(page);
  await enterAudit(page);
  await panel(page).getByLabel('Actor', { exact: true }).fill('system');
  state.value = auditPage(0, 20, {
    has_more: true,
    next_cursor: 'opaque-first',
  });
  await consult(page);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(20);
  expect(state.calls[0].searchParams.get('actor')).toBe('system');
  expect(state.calls[0].searchParams.get('from')).toBe('2026-10-01T00:00:00Z');
  expect(state.calls[0].searchParams.get('until')).toBe('2026-10-02T00:00:00Z');
  state.value = auditPage(20, 2);
  await panel(page).getByRole('button', { name: 'Siguiente p\u00e1gina', exact: true }).click();
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(2);
  await expect(panel(page).locator('[data-sequence="0"]')).toHaveCount(0);
  await expect(panel(page)).toContainText('No hay m\u00e1s movimientos.');
  expect(state.calls[1].searchParams.get('cursor')).toBe('opaque-first');
  expect(state.calls[1].searchParams.get('actor')).toBe('system');
  await panel(page).getByLabel('Actor', { exact: true }).fill('other');
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(0);
  state.value = auditPage(0, 0);
  await consult(page);
  await expect(panel(page)).toContainText('Ning\u00fan movimiento coincide con estos filtros.');
  expect(state.calls[2].searchParams.has('cursor')).toBe(false);
});
test('permission and capacity failures clear prior results without document guidance', async ({
  page,
}) => {
  const state = await setupAudit(page);
  await enterAudit(page);
  await consult(page);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(1);
  state.handle = (route) =>
    route.fulfill({
      status: 413,
      json: { error: { code: 'audit_query_capacity_exceeded' } },
    });
  await panel(page).getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect(panel(page).getByRole('alert')).toContainText('actividad');
  await expect(panel(page).getByRole('alert')).not.toContainText('16 MiB');
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(0);
  state.handle = (route) =>
    route.fulfill({
      status: 403,
      json: { error: { code: 'permission_denied' } },
    });
  await consult(page);
  await expect(panel(page).getByRole('alert')).toBeVisible();
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(0);
});
test('expired session clears the audit panel and its private results', async ({ page }) => {
  const state = await setupAudit(page);
  await enterAudit(page);
  await consult(page);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(1);
  state.handle = (route) =>
    route.fulfill({
      status: 401,
      json: { error: { code: 'invalid_session' } },
    });
  await panel(page).getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho' })).toBeVisible();
  await expect(panel(page)).toHaveCount(0);
});
for (const role of ['litigator', 'paralegal', 'client']) {
  test(`${role} cannot consult global audit through navigation or forced hash`, async ({
    page,
  }) => {
    const state = await setupAudit(page, role);
    await login(page, false, false);
    await expect(
      page.getByRole('navigation').getByRole('button', { name: 'Bit\u00e1cora', exact: true }),
    ).toHaveCount(0);
    await page.evaluate(() => {
      location.hash = '#audit';
    });
    await expectLanding(page, role);
    await expect(panel(page)).toHaveCount(0);
    expect(state.calls).toHaveLength(0);
  });
}
