import { test, expect } from '@playwright/test';
import { requestCompletion } from './request-completion.mjs';
import { login, navigate, caseId, id } from './helpers.mjs';
import { incidentPage } from '../fixtures/document-integrity-incidents.mjs';
import {
  setupIncidents,
  enterIncidents,
  inbox,
  incidentCard,
} from './document-integrity-incidents-helpers.mjs';

for (const width of [1440, 390]) {
  test(`Owner sees persistent incidents and opens their exact document version at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const state = await setupIncidents(page);
    await login(page, false, false);
    const notice = page.getByRole('region', { name: 'Avisos de integridad', exact: true });
    await expect(notice).toContainText('Hay incidentes de integridad registrados.');
    expect(state.calls.map((call) => call.limit)).toEqual(['1']);
    await notice.getByRole('button', { name: 'Consultar incidentes', exact: true }).click();
    await expect(inbox(page)).toBeVisible();
    await expect(incidentCard(page, state.rows[0])).toBeVisible();
    await expect(inbox(page)).toContainText('Orden por identificador');
    await expect(inbox(page)).toContainText('validaci\u00f3n');
    await expect(
      inbox(page).getByRole('button', { name: /Marcar|Reconocer|Enviar correo/ }),
    ).toHaveCount(0);
    await inbox(page)
      .getByRole('button', { name: 'Cargar m\u00e1s incidentes', exact: true })
      .click();
    await expect(inbox(page).locator('[data-incident-id]')).toHaveCount(2);
    expect(
      await inbox(page)
        .locator('[data-incident-id]')
        .evaluateAll((elements) => elements.map((element) => element.dataset.incidentId)),
    ).toEqual([state.rows[0].id, state.rows[1].id]);
    expect(state.calls.filter((call) => call.after).map((call) => call.after)).toEqual([
      state.rows[0].id,
    ]);
    expect(state.calls.every((call) => call.method === 'GET')).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({
      path: testInfo.outputPath('document-integrity-inbox.png'),
      fullPage: true,
    });
    await incidentCard(page, state.rows[0])
      .getByRole('button', { name: 'Abrir versi\u00f3n exacta', exact: true })
      .click();
    await expect(page.getByRole('heading', { name: 'contrato.pdf', exact: true })).toBeVisible();
    await expect(
      page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
    ).toBeVisible();
    expect(
      state.baseCalls.some(
        (call) => call.path === `/api/v1/cases/${caseId}/documents/${id}/versions/1`,
      ),
    ).toBe(true);
    expect(state.calls.filter((call) => call.path.endsWith(`/${state.rows[0].id}`))).toHaveLength(
      1,
    );
    await page.getByRole('button', { name: 'Volver a incidentes', exact: true }).click();
    await expect(inbox(page)).toBeVisible();
  });
}

test('Owner incident service failure stays visible until explicit refresh, distinct from an empty inbox', async ({
  page,
}) => {
  const state = await setupIncidents(page);
  state.handle = (route) =>
    route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } }).then(() => true);
  await login(page, false, false);
  await expect(
    page.getByRole('region', { name: 'Avisos de integridad', exact: true }).getByRole('alert'),
  ).toBeVisible();
  await navigate(page, 'Incidentes de integridad');
  await expect(inbox(page).getByRole('alert')).toBeVisible();
  await expect(
    inbox(page).getByText('No hay incidentes registrados.', { exact: true }),
  ).toHaveCount(0);
  const before = state.calls.length;
  state.handle = (route) => route.fulfill({ json: incidentPage() }).then(() => true);
  await inbox(page).getByRole('button', { name: 'Actualizar incidentes', exact: true }).click();
  await expect(
    inbox(page).getByText('No hay incidentes registrados.', { exact: true }),
  ).toBeVisible();
  await expect(inbox(page).getByRole('alert')).toHaveCount(0);
  expect(state.calls.length).toBe(before + 1);
});

test('a late Owner incident list is discarded after logout', async ({ page }) => {
  const state = await setupIncidents(page);
  await enterIncidents(page);
  await expect(incidentCard(page, state.rows[0])).toBeVisible();
  let release;
  state.handle = (route) =>
    new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: incidentPage(state.rows) });
        resolve(true);
      };
    });
  await inbox(page).getByRole('button', { name: 'Actualizar incidentes', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  const finished = requestCompletion(page, (request) =>
    request.url().includes('/document-integrity-incidents?'),
  );
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await release();
  const completed = await finished;
  if (completed.failed) expect(completed.request.failure()?.errorText).toMatch(/abort|cancel/i);
  await expect(inbox(page)).toHaveCount(0);
  await expect(page.locator('[data-incident-id]')).toHaveCount(0);
});

for (const role of ['litigator', 'paralegal', 'client']) {
  test(`${role} cannot enter the Owner incident inbox or trigger its initial query`, async ({
    page,
  }) => {
    const state = await setupIncidents(page, role);
    await login(page, false, false);
    await expect(
      page
        .getByRole('navigation')
        .getByRole('button', { name: 'Incidentes de integridad', exact: true }),
    ).toHaveCount(0);
    await page.evaluate(() => {
      location.hash = '#integrity-incidents';
    });
    await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
    await expect(inbox(page)).toHaveCount(0);
    expect(state.calls).toHaveLength(0);
  });
}
