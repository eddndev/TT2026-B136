import { test, expect } from '@playwright/test';
import { fixture, loginAs } from './helpers.mjs';

const accounts = fixture.auditEvents;
if (!accounts) throw new Error('Audit event fixtures must be provisioned by web-demo.sh');
const panel = (page) => page.getByRole('region', { name: 'Actividad registrada', exact: true });
const query = () =>
  new URLSearchParams({
    from: `${accounts.from}T00:00:00Z`,
    until: `${accounts.until}T00:00:00Z`,
    actor: accounts.owner.email,
    action: 'case.read',
    resource: accounts.resource,
    limit: '20',
  });
const responseTo = (page, path, method = 'GET') =>
  page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `/api/v1${path}` &&
      response.request().method() === method,
  );

async function enter(page, account) {
  await page.goto('/');
  const signed = responseTo(page, '/auth/mfa/recovery', 'POST');
  await loginAs(page, account, 0);
  return (await (await signed).json()).access_token;
}

async function load(page, name) {
  const response = responseTo(page, '/audit/events');
  await panel(page).getByRole('button', { name, exact: true }).click();
  const received = await response;
  expect(received.status()).toBe(200);
  expect(received.headers()['cache-control']).toBe('no-store');
  await expect(panel(page)).toHaveAttribute('aria-busy', 'false');
  return received.json();
}

async function visibleEvents(page, events) {
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(events.length);
  for (const event of events) {
    const row = panel(page).locator(`[data-sequence="${event.sequence}"]`);
    await expect(row.locator('time')).toHaveAttribute('datetime', event.timestamp);
    await expect(row.locator('time')).toHaveText(event.timestamp);
    await expect(row.locator('dd')).toHaveText([
      event.actor,
      event.action,
      event.resource,
      event.sequence,
    ]);
    expect(Object.keys(event).sort()).toEqual([
      'action',
      'actor',
      'resource',
      'sequence',
      'timestamp',
    ]);
  }
}

test('real Owner reads exact historical events with stable pages and separate chain verification', async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  const token = await enter(page, accounts.owner);
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name: 'Auditor\u00eda', exact: true })
    .click();
  await expect(panel(page)).toBeVisible();
  await panel(page).getByLabel('Desde (UTC, incluido)', { exact: true }).fill(accounts.from);
  await panel(page).getByLabel('Hasta (UTC, excluido)', { exact: true }).fill(accounts.until);
  await panel(page).getByLabel('Actor registrado', { exact: true }).fill(accounts.owner.email);
  await panel(page).getByLabel('Operaci\u00f3n registrada', { exact: true }).fill('case.read');
  await panel(page).getByLabel('Recurso registrado', { exact: true }).fill(accounts.resource);
  const first = await load(page, 'Consultar actividad');
  expect(first.events).toEqual(accounts.events.slice(0, 20));
  expect(first.has_more).toBe(true);
  await visibleEvents(page, first.events);
  await page.screenshot({ path: testInfo.outputPath('audit-events-desktop.png'), fullPage: true });

  const appended = await page.request.get(
    `${process.env.API_PROXY_TARGET}/api/v1/cases/${accounts.caseId}`,
    {
      headers: { Authorization: `Bearer ${token}` },
    },
  );
  expect(appended.status()).toBe(200);
  const second = await load(page, 'Cargar siguiente p\u00e1gina');
  expect(second.snapshot_max_sequence).toBe(first.snapshot_max_sequence);
  expect(second.events).toEqual(accounts.events.slice(20));
  expect(second.has_more).toBe(false);
  expect(second.next_cursor).toBeNull();
  await visibleEvents(page, second.events);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: testInfo.outputPath('audit-events-mobile.png'), fullPage: true });
  await page.setViewportSize({ width: 1440, height: 1000 });

  const refreshed = await load(page, 'Actualizar actividad');
  expect(BigInt(refreshed.snapshot_max_sequence)).toBeGreaterThan(
    BigInt(first.snapshot_max_sequence),
  );
  expect(refreshed.events).toEqual(accounts.events.slice(0, 20));
  const final = await load(page, 'Cargar siguiente p\u00e1gina');
  expect(final.events).toHaveLength(2);
  expect(final.events[0]).toEqual(accounts.events[20]);
  expect(BigInt(final.events[1].sequence)).toBeGreaterThan(BigInt(first.snapshot_max_sequence));
  expect(final.events[1]).toMatchObject({
    actor: accounts.owner.email,
    action: 'case.read',
    resource: accounts.resource,
  });
  await visibleEvents(page, final.events);

  await panel(page)
    .getByLabel('Recurso registrado', { exact: true })
    .fill(`${accounts.resource}:absent`);
  await expect(panel(page).locator('[data-sequence]')).toHaveCount(0);
  expect((await load(page, 'Consultar actividad')).events).toEqual([]);
  await expect(
    panel(page).getByText('No hay eventos que coincidan con estos filtros.'),
  ).toBeVisible();
  const verified = responseTo(page, '/audit/verify');
  await page.getByRole('button', { name: 'Verificar cadena', exact: true }).click();
  expect((await (await verified).json()).valid).toBe(true);
  await expect(page.getByRole('heading', { name: 'Cadena \u00edntegra' })).toBeVisible();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(panel(page)).toHaveCount(0);
});

test('real Litigator cannot read global audit through API or forced navigation', async ({
  page,
}) => {
  const token = await enter(page, accounts.litigator);
  await expect(
    page.getByRole('navigation').getByRole('button', { name: 'Auditor\u00eda', exact: true }),
  ).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = '#audit';
  });
  await expect(page).toHaveURL(/#overview$/);
  await expect(panel(page)).toHaveCount(0);
  for (const resource of [accounts.resource, `${accounts.resource}:absent`]) {
    const params = query();
    params.set('resource', resource);
    const response = await page.request.get(
      `${process.env.API_PROXY_TARGET}/api/v1/audit/events?${params}`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    expect(response.status()).toBe(403);
    expect(response.headers()['cache-control']).toBe('no-store');
    const body = await response.json();
    expect(Object.keys(body)).toEqual(['error']);
    expect(body.error.code).toBe('permission_denied');
  }
});
