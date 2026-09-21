import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  accounts,
  inbox,
  card,
  downloadButton,
  contentPath,
  responseTo,
  accountAction,
  openHistoricalContent,
  downloadExact,
  findIncident,
  capture,
} from './document-content-helpers.mjs';

for (const [name, width] of [
  ['desktop', 1440],
  ['mobile', 390],
]) {
  test(`real historical content and persistent Owner incidents at ${width}px`, async ({
    page,
  }, testInfo) => {
    const scenario = accounts[name],
      pageErrors = [],
      documentMutations = [];
    page.on('pageerror', (error) => pageErrors.push(error.message));
    page.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (!path.startsWith('/api/v1/')) return;
      if (path.includes('/documents/') && request.method() !== 'GET') documentMutations.push(path);
    });
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    await loginAs(page, accounts[`${name}Owner`], 0);
    await expect(
      page.getByRole('region', { name: 'Avisos de integridad', exact: true }),
    ).toContainText('Hay incidentes de integridad registrados.');
    await openHistoricalContent(page, scenario);
    await expect(
      page.getByRole('button', { name: 'Verificar integridad', exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
    ).toBeDisabled();
    await downloadExact(page, scenario);
    await expect(
      page.locator('.detail-panel').getByText('Pendiente de sello', { exact: true }),
    ).toBeVisible();
    await capture(page, testInfo, `${name}-exact-content`);

    await navigate(page, 'Incidentes de integridad');
    await findIncident(page, scenario.incident);
    await expect(inbox(page)).toContainText('Orden por identificador');
    await expect(inbox(page)).toContainText('No determina su causa ni atribuye responsabilidad');
    const technical = card(page, scenario.incident).locator('details');
    await technical.locator('summary').click();
    await expect(technical).toContainText(scenario.incident.expected_digest);
    await expect(technical).toContainText(scenario.incident.observed_snapshot_digest);
    await capture(page, testInfo, `${name}-integrity-inbox`);
    const incidentResponse = responseTo(
      page,
      `/document-integrity-incidents/${scenario.incident.id}`,
    );
    const contentResponse = responseTo(page, contentPath(scenario).replace('/content', ''));
    await card(page, scenario.incident)
      .getByRole('button', { name: 'Abrir versi\u00f3n exacta', exact: true })
      .click();
    expect(await (await incidentResponse).json()).toEqual(scenario.incident);
    expect(await (await contentResponse).json()).toEqual(scenario.first);
    await expect(
      page.getByRole('heading', { name: scenario.first.name, exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Volver a incidentes', exact: true }).click();
    await findIncident(page, scenario.incident);
    await inbox(page).getByRole('button', { name: 'Actualizar incidentes', exact: true }).click();
    await findIncident(page, scenario.incident);
    expect(documentMutations).toEqual([]);
    expect(pageErrors).toEqual([]);
  });
}

test('real content permits assigned staff, denies Client and clears revoked history', async ({
  page,
  browser,
}, testInfo) => {
  const scenario = accounts.policy;
  await page.goto('/');
  await loginAs(page, accounts.paralegal, 0);
  await expect(page.getByRole('region', { name: 'Avisos de integridad', exact: true })).toHaveCount(
    0,
  );
  await openHistoricalContent(page, scenario);
  await downloadExact(page, scenario);
  await accountAction(accounts.paralegal, 1, async (call) => {
    for (const path of [
      '/document-integrity-incidents',
      `/document-integrity-incidents/${accounts.desktop.incident.id}`,
    ]) {
      expect((await call('GET', path, undefined, 403)).error.code).toBe('permission_denied');
    }
  });

  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const client = await context.newPage(),
      protectedRequests = [];
    client.on('request', (request) => {
      const path = new URL(request.url()).pathname;
      if (!path.startsWith('/api/v1/')) return;
      if (path.includes('/document-integrity-incidents') || path.endsWith('/content'))
        protectedRequests.push(path);
    });
    await client.goto('/');
    await loginAs(client, accounts.client, 0);
    await navigate(client, 'Expedientes');
    await expect(client.locator('.case-list')).toHaveAttribute('aria-busy', 'false');
    await client.getByRole('button', { name: new RegExp(scenario.case.title) }).click();
    await expect(
      client.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
    ).toBeVisible();
    await expect(downloadButton(client)).toHaveCount(0);
    await expect(
      client.getByRole('region', { name: 'Avisos de integridad', exact: true }),
    ).toHaveCount(0);
    expect(protectedRequests).toEqual([]);
    await accountAction(accounts.client, 1, async (call) => {
      for (const path of [
        contentPath(scenario),
        '/document-integrity-incidents',
        `/document-integrity-incidents/${accounts.desktop.incident.id}`,
      ]) {
        expect((await call('GET', path, undefined, 403)).error.code).toBe('permission_denied');
      }
    });
  } finally {
    await context.close();
  }

  await accountAction(accounts.owner, 0, async (call) => {
    const before = await call('GET', '/document-integrity-incidents?limit=100');
    expect(before.has_more).toBe(false);
    await call(
      'DELETE',
      `/cases/${scenario.case.id}/members/${accounts.paralegal.id}`,
      undefined,
      204,
    );
    const downloads = [];
    page.on('download', (value) => downloads.push(value));
    const denied = responseTo(page, contentPath(scenario));
    await downloadButton(page).click();
    const response = await denied;
    expect(response.status()).toBe(404);
    expect((await response.json()).error.code).toBe('document_not_found');
    expect(response.headers()['content-disposition']).toBeUndefined();
    expect(response.headers()['x-document-digest']).toBeUndefined();
    await expect(page.locator('.document-focus')).toHaveCount(0);
    await expect(page.getByRole('alert')).toHaveText(
      'No se encontr\u00f3 un documento con ese identificador.',
    );
    expect(downloads).toEqual([]);
    expect(await call('GET', '/document-integrity-incidents?limit=100')).toEqual(before);
  });
});
